use chrono::NaiveDate;
use std::str::FromStr;

use crate::models::{Cvx, Dose, DoseEvaluation, DoseStatus, ExpectedResults, Gender, Patient, SeriesForecast, SeriesStatus, UnifiedTestCase};
use crate::date_utils::TimePeriod;

pub fn parse_test_case_dsl(content: &str) -> Result<UnifiedTestCase, String> {
    let mut name = String::new();
    let mut group = String::new();
    let mut focus_code = String::from("000");
    
    let mut birth_date = None;
    let gender = Gender::Female;
    let mut execution_date = None;
    
    let mut doses: Vec<Dose> = Vec::new();
    
    let mut expected_evals = Vec::new();
    let mut expected_status = None;
    let mut earliest_date = None;
    let mut recommended_date = None;
    let mut overdue_date = None;
    let mut latest_date = None;

    let parse_date = |s: &str| -> Result<NaiveDate, String> {
        NaiveDate::from_str(s).map_err(|e| format!("Invalid date {}: {}", s, e))
    };

    let parse_period = |s: &str| -> Result<TimePeriod, String> {
        let cleaned = s.replace(" and ", " ").replace("months", "m").replace("month", "m")
                       .replace("weeks", "w").replace("week", "w")
                       .replace("days", "d").replace("day", "d")
                       .replace("years", "y").replace("year", "y")
                       .replace(" ", "");
        TimePeriod::parse(&cleaned).map_err(|e| e.to_string())
    };

    let lines: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    
    for line in lines {
        let lower = line.to_lowercase();
        
        if line.starts_with("Test: ") {
            name = line.replace("Test: ", "").trim().to_string();
        } else if line.starts_with("Group: ") {
            group = line.replace("Group: ", "").trim().to_string();
        } else if line.starts_with("Focus: ") {
            focus_code = line.replace("Focus: ", "").trim().to_string();
        } else if lower.contains("patient born on ") {
            let date_str = lower.split("patient born on ").nth(1)
                .ok_or("Missing date after 'patient born on'")?.trim();
            birth_date = Some(parse_date(date_str)?);
        } else if lower.contains("evaluation date is ") {
            let date_str = lower.split("evaluation date is ").nth(1)
                .ok_or("Missing date after 'evaluation date is'")?.trim();
            execution_date = Some(parse_date(date_str)?);
        } else if lower.contains("receive cvx ") {
            // "receive CVX 106 at 2 months of age" or "receive CVX 106 on 2020-03-01"
            // or "receive CVX 106 4 weeks after dose 1"
            let parts: Vec<&str> = lower.split("receive cvx ").collect();
            let after_cvx = parts[1].trim();
            let cvx_str = after_cvx.split_whitespace().next()
                .ok_or("Missing CVX code after 'receive cvx'")?;
            let cvx = cvx_str.parse::<u16>().map_err(|_| "Invalid CVX")?;
            
            let mut remainder = after_cvx[cvx_str.len()..].trim();
            let mut inline_status = None;
            
            if let Some(idx) = remainder.find("->") {
                let status_str = remainder[idx + 2..].trim();
                inline_status = Some(match status_str {
                    "valid" => DoseStatus::Valid,
                    "invalid" => DoseStatus::Invalid,
                    "ignored" => DoseStatus::Ignored,
                    "accepted" => DoseStatus::Accepted,
                    _ => return Err(format!("Unknown inline dose status {}", status_str)),
                });
                remainder = remainder[..idx].trim();
            }
            
            let date = if remainder.starts_with("on ") {
                parse_date(remainder.replace("on ", "").trim())?
            } else if remainder.starts_with("at ") && remainder.contains("of age") {
                let period_str = remainder.replace("at ", "").replace("of age", "").trim().to_string();
                let tp = parse_period(&period_str)?;
                let bdate = birth_date.ok_or("Birthdate must be specified before relative doses")?;
                tp.add_to(bdate)
            } else if remainder.contains("after dose ") {
                let parts: Vec<&str> = remainder.split("after dose ").collect();
                let period_str = parts[0].trim();
                let dose_idx_str = parts[1].trim();
                let dose_idx = dose_idx_str.parse::<usize>()
                    .map_err(|_| format!("Invalid dose number: {}", dose_idx_str))? - 1;
                let tp = parse_period(period_str)?;
                if dose_idx >= doses.len() {
                    return Err(format!("Reference to non-existent dose {}", dose_idx + 1));
                }
                let prev_date = doses[dose_idx].date;
                tp.add_to(prev_date)
            } else {
                return Err(format!("Could not parse dose timing: {}", remainder));
            };
            doses.push(Dose { cvx: Cvx(cvx), date, is_valid: None });
            
            if let Some(status) = inline_status {
                expected_evals.push(DoseEvaluation {
                    dose_number: Some(doses.len()),
                    status,
                    reasons: Default::default(),
                    dose_date: date,
                    cvx: Cvx(cvx),
                });
            }
            
        } else if lower.starts_with("then dose ") || (lower.starts_with("and dose ") && lower.contains(" status should be ")) {
            let after_dose = lower.split("dose ").nth(1)
                .ok_or("Missing dose number after 'dose'")?
                .trim();
            let dose_num_str = after_dose.split_whitespace().next()
                .ok_or("Missing dose number")?
                .trim();
            let dose_num = dose_num_str.parse::<usize>()
                .map_err(|_| format!("Invalid dose number: {}", dose_num_str))?;
            
            let status_str = after_dose.split(" status should be ").nth(1)
                .ok_or("Missing status after 'status should be'")?.trim();
            let status = match status_str {
                "valid" => DoseStatus::Valid,
                "invalid" => DoseStatus::Invalid,
                "ignored" => DoseStatus::Ignored,
                "accepted" => DoseStatus::Accepted,
                _ => return Err(format!("Unknown dose status {}", status_str)),
            };
            expected_evals.push(DoseEvaluation {
                dose_number: Some(dose_num),
                status,
                reasons: Default::default(),
                dose_date: doses[dose_num - 1].date,
                cvx: doses[dose_num - 1].cvx,
            });
        } else if lower.contains("series status should be ") {
            let status_str = lower.split("series status should be ").nth(1)
                .ok_or("Missing status after 'series status should be'")?.trim();
            let status = match status_str {
                "notcomplete" => "NotComplete",
                "complete" => "Complete",
                "notrecommended" => "NotRecommended",
                "conditionallyrecommended" => "ConditionallyRecommended",
                _ => return Err(format!("Unknown series status {}", status_str)),
            };
            expected_status = Some(status.to_string());
        } else if lower.contains("earliest date should be ") {
            let d = lower.split("earliest date should be ").nth(1)
                .ok_or("Missing date after 'earliest date should be'")?.trim();
            earliest_date = Some(parse_date(d)?);
        } else if lower.contains("recommended date should be ") {
            let d = lower.split("recommended date should be ").nth(1)
                .ok_or("Missing date after 'recommended date should be'")?.trim();
            recommended_date = Some(parse_date(d)?);
        } else if lower.contains("overdue date should be ") {
            let d = lower.split("overdue date should be ").nth(1)
                .ok_or("Missing date after 'overdue date should be'")?.trim();
            overdue_date = Some(parse_date(d)?);
        } else if lower.contains("latest date should be ") {
            let d = lower.split("latest date should be ").nth(1)
                .ok_or("Missing date after 'latest date should be'")?.trim();
            latest_date = Some(parse_date(d)?);
        }
    }
    
    let bdate = birth_date.ok_or("Missing patient birth date")?;
    let edate = execution_date.ok_or("Missing evaluation date")?;
    
    let mut forecast_status = SeriesStatus::NotComplete {
        earliest_date,
        recommended_date,
        overdue_date,
        latest_date,
    };
    
    if let Some(s) = expected_status {
        if s == "Complete" {
            forecast_status = SeriesStatus::Complete;
        } else if s == "NotRecommended" {
            forecast_status = SeriesStatus::NotRecommended;
        } else if s == "ConditionallyRecommended" {
            forecast_status = SeriesStatus::ConditionallyRecommended;
        }
    }
    
    let forecast = SeriesForecast {
        series_name: std::borrow::Cow::Owned(group.clone()),
        status: forecast_status,
        reasons: Default::default(),
    };

    Ok(UnifiedTestCase {
        name,
        group,
        focus_code,
        patient: Patient {
            birth_date: bdate,
            gender,
            immunities: Vec::new(),
            contraindications: Vec::new(),
        },
        history: doses,
        execution_date: edate,
        expected: Some(ExpectedResults {
            evaluations: expected_evals,
            forecasts: vec![forecast],
        }),
    })
}
