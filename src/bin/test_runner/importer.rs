use std::fs;
use std::path::Path;
use std::collections::HashMap;
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use lava_forecaster::models::{Patient, Dose, Gender, UnifiedTestCase, Cvx};

// Helper for relative date resolution
pub struct RelativeDateResolver {
    pub dob: NaiveDate,
    pub dose_dates: Vec<NaiveDate>,
}

impl RelativeDateResolver {
    pub fn new(dob: NaiveDate) -> Self {
        Self {
            dob,
            dose_dates: Vec::new(),
        }
    }

    pub fn add_months(d: NaiveDate, months: i32) -> NaiveDate {
        let mut year = d.year();
        let mut month = d.month() as i32 + months - 1;
        year += month / 12;
        month = month % 12 + 1;
        if month <= 0 {
            month += 12;
            year -= 1;
        }
        
        // Handle end-of-month clamping (e.g. Oct 31 + 1 month -> Nov 30)
        let mut day = d.day();
        loop {
            if let Some(date) = NaiveDate::from_ymd_opt(year, month as u32, day) {
                return date;
            }
            day -= 1;
            if day == 0 {
                panic!("Invalid day reached in add_months");
            }
        }
    }

    pub fn add_offset(d: NaiveDate, val: i32, unit: char) -> NaiveDate {
        match unit {
            'y' => Self::add_months(d, val * 12),
            'm' => Self::add_months(d, val),
            'w' => d + chrono::Duration::days(val as i64 * 7),
            'd' => d + chrono::Duration::days(val as i64),
            _ => d,
        }
    }

    pub fn resolve(&mut self, expr: &str) -> NaiveDate {
        let expr = expr.trim();
        if expr == "birth" {
            return self.dob;
        }

        let parts: Vec<&str> = expr.split('+').collect();
        let base_ref = parts[0].trim();

        let base_date = if base_ref == "birth" {
            self.dob
        } else if base_ref == "prev" {
            *self.dose_dates.last().unwrap_or(&self.dob)
        } else if base_ref.starts_with("dose") {
            let idx: usize = base_ref[4..].parse::<usize>().unwrap() - 1;
            self.dose_dates[idx]
        } else {
            match NaiveDate::parse_from_str(base_ref, "%Y-%m-%d") {
                Ok(d) => return d,
                Err(_) => panic!("Unknown date base reference: {}", base_ref),
            }
        };

        if parts.len() == 1 {
            return base_date;
        }

        let offset_str = parts[1].trim();
        // Simple manual parsing of terms like "2m" or "15m" or "-4d"
        let mut current_date = base_date;
        let mut chars = offset_str.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
                continue;
            }
            let mut sign = 1;
            if c == '+' {
                chars.next();
            } else if c == '-' {
                sign = -1;
                chars.next();
            }

            let mut digits = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_digit() {
                    digits.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }

            let val: i32 = digits.parse().unwrap_or(0) * sign;
            if let Some(unit) = chars.next() {
                current_date = Self::add_offset(current_date, val, unit);
            }
        }

        current_date
    }
}

// Relative parsing structure from Python json suites
#[derive(Debug, Deserialize)]
struct RawSuiteCase {
    name: String,
    dob: String,
    gender: String,
    eval_date: String,
    doses: Vec<String>,
    group: String,
    focus: String,
}

#[derive(Debug, Deserialize)]
struct RawTestSuite {
    #[serde(default, rename = "histories")]
    _histories: HashMap<String, serde_json::Value>,
    test_cases: Vec<RawSuiteCase>,
}

pub fn import_python_cases(input_file: &Path, output_dir: &Path) {
    let content = fs::read_to_string(input_file).expect("Failed to read raw suite file");
    let suite: RawTestSuite = serde_json::from_str(&content).expect("Failed to parse raw suite JSON");

    let mut imported = 0;
    for tc in suite.test_cases {
        let dob = NaiveDate::parse_from_str(&tc.dob, "%Y-%m-%d")
            .expect("Failed to parse patient birth_date");
        let gender = match tc.gender.as_str() {
            "F" => Gender::Female,
            "M" => Gender::Male,
            _ => Gender::Unknown,
        };

        let mut resolver = RelativeDateResolver::new(dob);
        let mut resolved_doses = Vec::new();
        for dose_str in tc.doses {
            let parts: Vec<&str> = dose_str.split(':').collect();
            let date_expr = parts[0];
            let cvx_num = parts[1].parse::<u16>().unwrap();
            let resolved_date = resolver.resolve(date_expr);
            resolver.dose_dates.push(resolved_date);
            resolved_doses.push(Dose {
                date: resolved_date,
                cvx: Cvx(cvx_num),
                is_valid: None,
            });
        }

        let resolved_eval_date = resolver.resolve(&tc.eval_date);

        let mut focus_code = tc.focus.clone();
        if focus_code.len() == 1 && focus_code.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            focus_code = format!("0{}", focus_code);
        }

        let unified_case = UnifiedTestCase {
            name: tc.name.clone(),
            group: tc.group,
            focus_code,
            patient: Patient {
                birth_date: dob,
                gender,
                immunities: Vec::new(),
                contraindications: Vec::new(),
            },
            history: resolved_doses,
            execution_date: resolved_eval_date,
            expected: None,
        };

        let out_path = output_dir.join(format!("{}.json", tc.name));
        let out_json = serde_json::to_string_pretty(&unified_case).unwrap();
        fs::write(out_path, out_json).expect("Failed to write unified case JSON");
        imported += 1;
    }

    println!("Imported {} test cases from {:?}", imported, input_file);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_date_resolver() {
        let dob = NaiveDate::from_ymd_opt(2020, 10, 31).unwrap();
        let mut resolver = RelativeDateResolver::new(dob);

        // Test birth reference
        assert_eq!(resolver.resolve("birth"), dob);

        // Test offsets
        assert_eq!(resolver.resolve("birth + 1y"), NaiveDate::from_ymd_opt(2021, 10, 31).unwrap());
        
        // Test end-of-month clamping (Oct 31 + 1m -> Nov 30)
        assert_eq!(resolver.resolve("birth + 1m"), NaiveDate::from_ymd_opt(2020, 11, 30).unwrap());
        
        // Test negative days offset
        assert_eq!(resolver.resolve("birth + -4d"), NaiveDate::from_ymd_opt(2020, 10, 27).unwrap());

        // Test weeks offset
        assert_eq!(resolver.resolve("birth + 4w"), NaiveDate::from_ymd_opt(2020, 11, 28).unwrap());

        // Test prev reference
        resolver.dose_dates.push(NaiveDate::from_ymd_opt(2020, 12, 1).unwrap());
        assert_eq!(resolver.resolve("prev + 2m"), NaiveDate::from_ymd_opt(2021, 2, 1).unwrap());

        // Test dose index reference
        resolver.dose_dates.push(NaiveDate::from_ymd_opt(2021, 1, 15).unwrap());
        assert_eq!(resolver.resolve("dose2 + 3d"), NaiveDate::from_ymd_opt(2021, 1, 18).unwrap());

        // Test absolute date parsing fallback
        assert_eq!(resolver.resolve("2026-06-07"), NaiveDate::from_ymd_opt(2026, 6, 7).unwrap());
    }
}
