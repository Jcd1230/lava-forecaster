use chrono::NaiveDate;
use std::str::FromStr;

use crate::date_utils::TimePeriod;
use crate::models::{
    Cvx, Dose, DoseEvaluation, DoseStatus, ExpectedResults, Gender, Patient, SeriesForecast,
    SeriesStatus, UnifiedTestCase,
};

pub fn parse_test_case_dsl(content: &str) -> Result<UnifiedTestCase, String> {
    let mut name = String::new();
    let mut group = String::new();
    let mut focus_code = String::from("000");

    let mut birth_date = None;
    let gender = Gender::Female;
    let mut execution_date = None;

    let mut doses: Vec<Dose> = Vec::new();
    let mut immunities = Vec::new();
    let mut contraindications = Vec::new();

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
        let cleaned = s
            .replace(" and ", " ")
            .replace("months", "m")
            .replace("month", "m")
            .replace("weeks", "w")
            .replace("week", "w")
            .replace("days", "d")
            .replace("day", "d")
            .replace("years", "y")
            .replace("year", "y")
            .replace(" ", "");
        TimePeriod::parse(&cleaned).map_err(|e| e.to_string())
    };

    let lines: Vec<&str> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    for line in lines {
        let lower = line.to_lowercase();

        if line.starts_with("Test: ") {
            name = line.replace("Test: ", "").trim().to_string();
        } else if line.starts_with("Group: ") {
            group = line.replace("Group: ", "").trim().to_string();
        } else if line.starts_with("Focus: ") {
            focus_code = line.replace("Focus: ", "").trim().to_string();
        } else if lower.contains("patient born on ") {
            let date_str = lower
                .split("patient born on ")
                .nth(1)
                .ok_or("Missing date after 'patient born on'")?
                .trim();
            birth_date = Some(parse_date(date_str)?);
        } else if lower.contains("evaluation date is ") {
            let date_str = lower
                .split("evaluation date is ")
                .nth(1)
                .ok_or("Missing date after 'evaluation date is'")?
                .trim();
            execution_date = Some(parse_date(date_str)?);
        } else if lower.contains("patient is immune to ") {
            let idx = lower.find("patient is immune to ").unwrap() + "patient is immune to ".len();
            let after = &line[idx..].trim();
            let on_idx = after.to_lowercase().find(" on ");
            if on_idx.is_none() {
                return Err("Missing 'on' date in immunity definition".to_string());
            }
            let on_idx = on_idx.unwrap();
            let disease = after[..on_idx].trim().to_string();
            let mut date_part = after[on_idx + 4..].trim();
            let mut reason = "Titer positive".to_string();
            let lower_date_part = date_part.to_lowercase();
            if let Some(r_idx) = lower_date_part.find("(reason:") {
                let r_part = date_part[r_idx..]
                    .replace("(reason:", "")
                    .replace("(Reason:", "")
                    .replace(")", "")
                    .trim()
                    .to_string();
                if !r_part.is_empty() {
                    reason = r_part;
                }
                date_part = date_part[..r_idx].trim();
            }
            let date = parse_date(date_part)?;
            immunities.push(crate::models::DiseaseImmunity {
                disease,
                date,
                reason,
            });
        } else if lower.contains("patient is contraindicated for ") {
            let idx = lower.find("patient is contraindicated for ").unwrap()
                + "patient is contraindicated for ".len();
            let after = &line[idx..].trim();
            let on_idx = after.to_lowercase().find(" on ");
            if on_idx.is_none() {
                return Err("Missing 'on' date in contraindication definition".to_string());
            }
            let on_idx = on_idx.unwrap();
            let target_part = after[..on_idx].trim();
            let mut date_part = after[on_idx + 4..].trim();

            let mut valid_until = None;
            let lower_date_part = date_part.to_lowercase();
            if let Some(u_idx) = lower_date_part.find(" until ") {
                let until_part = date_part[u_idx + 7..].trim();
                valid_until = Some(parse_date(until_part)?);
                date_part = date_part[..u_idx].trim();
            }

            let date = parse_date(date_part)?;

            let mut cvx = None;
            let target = target_part.to_string();
            if target_part.to_lowercase().starts_with("cvx ") {
                let cvx_val_str = target_part[4..].trim();
                if let Ok(c_val) = cvx_val_str.parse::<u16>() {
                    cvx = Some(Cvx(c_val));
                }
            }

            contraindications.push(crate::models::Contraindication {
                date,
                target,
                reason: "Contraindication".to_string(),
                valid_until,
                cvx,
            });
        } else if lower.contains("receive cvx ") {
            let mut clean_line = line.to_string();
            let mut is_valid_override = None;
            let lower_line = clean_line.to_lowercase();
            if lower_line.contains("(is_valid: true)") || lower_line.contains("(isvalid: true)") {
                is_valid_override = Some(true);
                clean_line = clean_line
                    .replace("(is_valid: true)", "")
                    .replace("(is_valid: True)", "")
                    .replace("(isvalid: true)", "")
                    .replace("(isvalid: True)", "");
            } else if lower_line.contains("(is_valid: false)")
                || lower_line.contains("(isvalid: false)")
            {
                is_valid_override = Some(false);
                clean_line = clean_line
                    .replace("(is_valid: false)", "")
                    .replace("(is_valid: False)", "")
                    .replace("(isvalid: false)", "")
                    .replace("(isvalid: False)", "");
            }
            let clean_line = clean_line.trim().to_string();
            let lower_clean = clean_line.to_lowercase();

            let parts: Vec<&str> = lower_clean.split("receive cvx ").collect();
            let after_cvx = parts[1].trim();
            let cvx_str = after_cvx
                .split_whitespace()
                .next()
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
                let period_str = remainder
                    .replace("at ", "")
                    .replace("of age", "")
                    .trim()
                    .to_string();
                let tp = parse_period(&period_str)?;
                let bdate =
                    birth_date.ok_or("Birthdate must be specified before relative doses")?;
                tp.add_to(bdate)
            } else if remainder.contains("after dose ") {
                let parts: Vec<&str> = remainder.split("after dose ").collect();
                let period_str = parts[0].trim();
                let dose_idx_str = parts[1].trim();
                let dose_idx = dose_idx_str
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid dose number: {}", dose_idx_str))?
                    - 1;
                let tp = parse_period(period_str)?;
                if dose_idx >= doses.len() {
                    return Err(format!("Reference to non-existent dose {}", dose_idx + 1));
                }
                let prev_date = doses[dose_idx].date;
                tp.add_to(prev_date)
            } else {
                return Err(format!("Could not parse dose timing: {}", remainder));
            };
            doses.push(Dose {
                cvx: Cvx(cvx),
                date,
                is_valid: is_valid_override,
            });

            if let Some(status) = inline_status {
                expected_evals.push(DoseEvaluation {
                    dose_number: None,
                    status,
                    reasons: Default::default(),
                    dose_date: date,
                    cvx: Cvx(cvx),
                    sources: std::collections::HashMap::new(),
                });
            }
        } else if lower.starts_with("then dose ")
            || (lower.starts_with("and dose ") && lower.contains(" status should be "))
        {
            let after_dose = lower
                .split("dose ")
                .nth(1)
                .ok_or("Missing dose number after 'dose'")?
                .trim();
            let dose_num_str = after_dose
                .split_whitespace()
                .next()
                .ok_or("Missing dose number")?
                .trim();
            let dose_num = dose_num_str
                .parse::<usize>()
                .map_err(|_| format!("Invalid dose number: {}", dose_num_str))?;

            let status_str = after_dose
                .split(" status should be ")
                .nth(1)
                .ok_or("Missing status after 'status should be'")?
                .trim();
            let status = match status_str {
                "valid" => DoseStatus::Valid,
                "invalid" => DoseStatus::Invalid,
                "ignored" => DoseStatus::Ignored,
                "accepted" => DoseStatus::Accepted,
                _ => return Err(format!("Unknown dose status {}", status_str)),
            };
            expected_evals.push(DoseEvaluation {
                dose_number: None,
                status,
                reasons: Default::default(),
                dose_date: doses[dose_num - 1].date,
                cvx: doses[dose_num - 1].cvx,
                sources: std::collections::HashMap::new(),
            });
        } else if lower.contains("series status should be ") {
            let status_str = lower
                .split("series status should be ")
                .nth(1)
                .ok_or("Missing status after 'series status should be'")?
                .trim();
            let status = match status_str {
                "notcomplete" => "NotComplete",
                "complete" => "Complete",
                "notrecommended" => "NotRecommended",
                "conditionallyrecommended" => "ConditionallyRecommended",
                _ => return Err(format!("Unknown series status {}", status_str)),
            };
            expected_status = Some(status.to_string());
        } else if lower.contains("earliest date should be ") {
            let d = lower
                .split("earliest date should be ")
                .nth(1)
                .ok_or("Missing date after 'earliest date should be'")?
                .trim();
            earliest_date = Some(parse_date(d)?);
        } else if lower.contains("recommended date should be ") {
            let d = lower
                .split("recommended date should be ")
                .nth(1)
                .ok_or("Missing date after 'recommended date should be'")?
                .trim();
            recommended_date = Some(parse_date(d)?);
        } else if lower.contains("overdue date should be ") {
            let d = lower
                .split("overdue date should be ")
                .nth(1)
                .ok_or("Missing date after 'overdue date should be'")?
                .trim();
            overdue_date = Some(parse_date(d)?);
        } else if lower.contains("latest date should be ") {
            let d = lower
                .split("latest date should be ")
                .nth(1)
                .ok_or("Missing date after 'latest date should be'")?
                .trim();
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
        sources: std::collections::HashMap::new(),
    };

    let mut resolved_focus_code = focus_code;
    if resolved_focus_code == "000" {
        resolved_focus_code = match group.to_lowercase().as_str() {
            "hepb" | "hep_b" | "hep b" | "hepatitis b" => "100".to_string(),
            "dtp" | "dtap" | "diphtheria" | "tetanus" | "pertussis" => "200".to_string(),
            "hib" => "300".to_string(),
            "polio" | "ipv" | "opv" => "400".to_string(),
            "mmr" => "500".to_string(),
            "varicella" | "chickenpox" => "600".to_string(),
            "zoster" | "shingles" => "620".to_string(),
            "pneumo" | "pneumococcal" => "750".to_string(),
            "flu" | "influenza" => "800".to_string(),
            "hepa" | "hep_a" | "hep a" | "hepatitis a" => "810".to_string(),
            "rota" | "rotavirus" => "820".to_string(),
            "mcv" | "mening" | "meningococcal" => "830".to_string(),
            "menb" => "835".to_string(),
            "hpv" => "840".to_string(),
            "covid" | "covid19" | "covid-19" => "850".to_string(),
            "rsv" => "875".to_string(),
            _ => "000".to_string(),
        };
    }

    Ok(UnifiedTestCase {
        name,
        group,
        focus_code: resolved_focus_code,
        patient: Patient {
            birth_date: bdate,
            gender,
            immunities,
            contraindications,
        },
        history: doses,
        execution_date: edate,
        expected: Some(ExpectedResults {
            evaluations: expected_evals,
            forecasts: vec![forecast],
        }),
    })
}
