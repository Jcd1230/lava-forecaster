use crate::java_client::query_rust_rest_service;
use crate::ui::{SummaryCounts, print_summary, record_summary_result};
use chrono::NaiveDate;
use csv::ReaderBuilder;
use lava_forecaster::{
    evaluate_patient_all_groups,
    models::{
        Cvx, Dose, DoseEvaluation, DoseStatus, EvaluationReason, Gender, Patient, SeriesForecast,
        SeriesStatus, UnifiedTestCase,
    },
};
use reqwest::blocking::Client;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CdcExpectedDose {
    pub dose_date: NaiveDate,
    pub cvx: Cvx,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CdcExpectedForecast {
    pub series_status: String,
    pub forecast_number: Option<usize>,
    pub earliest_date: Option<NaiveDate>,
    pub recommended_date: Option<NaiveDate>,
    pub overdue_date: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct CdcExpectedResults {
    pub evaluations: Vec<CdcExpectedDose>,
    pub forecast: CdcExpectedForecast,
}

#[derive(Debug, Clone)]
pub struct CdcCsvCase {
    pub test_id: String,
    pub unified_case: UnifiedTestCase,
    pub expected: CdcExpectedResults,
}

#[derive(Debug, Clone)]
pub struct CdcActualDose {
    pub dose_date: NaiveDate,
    pub cvx: Cvx,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CdcActualForecast {
    pub series_status: Option<String>,
    pub forecast_number: Option<usize>,
    pub earliest_date: Option<NaiveDate>,
    pub recommended_date: Option<NaiveDate>,
    pub overdue_date: Option<NaiveDate>,
}

pub fn sanitize_name(test_id: &str, name: &str) -> String {
    let full_name = format!("cdsi_{}_{}", test_id, name);
    full_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

pub fn parse_cdc_date(date_str: &str) -> Result<Option<NaiveDate>, String> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    for fmt in ["%m/%d/%Y", "%m/%d/%y", "%Y-%m-%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, fmt) {
            return Ok(Some(date));
        }
    }

    Err(format!("Could not parse CDC date '{}'", trimmed))
}

pub fn parse_cdc_forecast_number(value: &str) -> Result<Option<usize>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "-" {
        return Ok(None);
    }

    trimmed
        .parse::<usize>()
        .map(Some)
        .map_err(|_| format!("Could not parse CDC forecast number '{}'", trimmed))
}

pub fn cdc_group_map(csv_group: &str) -> Option<(&'static str, &'static str)> {
    match csv_group {
        "COVID-19" => Some(("COVID19", "850")),
        "DTAP" => Some(("DTP", "200")),
        "FLU" => Some(("INFLUENZA", "800")),
        "HIB" => Some(("HIB", "300")),
        "HPV" => Some(("HPV", "840")),
        "HepA" => Some(("HEP_A", "810")),
        "HepB" => Some(("HEP_B", "100")),
        "MCV" => Some(("MCV", "830")),
        "MENB" => Some(("MENB", "835")),
        "MMR" => Some(("MMR", "500")),
        "PCV" => Some(("PNEUMOCOCCAL", "750")),
        "POL" => Some(("POLIO", "400")),
        "ROTA" => Some(("ROTAVIRUS", "820")),
        "RSV" => Some(("RSV", "875")),
        "VAR" => Some(("VARICELLA", "600")),
        "ZOSTER" => Some(("ZOSTER", "620")),
        _ => None,
    }
}

pub fn map_rust_reason_to_cdc(eval: &DoseEvaluation) -> Option<&'static str> {
    if eval.status == DoseStatus::Valid {
        return None;
    }

    let has_reason = |target: EvaluationReason| eval.reasons.iter().any(|reason| *reason == target);

    if has_reason(EvaluationReason::BelowMinimumAge)
        || has_reason(EvaluationReason::BelowMinimumAgeFinalDose)
    {
        Some("Age: Too Young")
    } else if has_reason(EvaluationReason::AboveRecommendedAgeSeries) {
        Some("Age: Too Old")
    } else if has_reason(EvaluationReason::BelowMinimumInterval)
        || has_reason(EvaluationReason::DuplicateShotSameDay)
    {
        Some("Interval: too Soon")
    } else if has_reason(EvaluationReason::TooEarlyLiveVirus) {
        Some("Live Virus Conflict")
    } else if has_reason(EvaluationReason::BoosterDose)
        || has_reason(EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven)
    {
        Some("Series Already Complete")
    } else if has_reason(EvaluationReason::VaccineNotPartOfSeries)
        || has_reason(EvaluationReason::OutsideRoutineSeries)
        || has_reason(EvaluationReason::VaccineNotLicensedForMales)
        || has_reason(EvaluationReason::MissingAntigen)
        || has_reason(EvaluationReason::InsufficientAntigen)
    {
        Some("Inadvertent Vaccine")
    } else if has_reason(EvaluationReason::VaccineNotAllowedInUs) {
        Some("Not a preferable or allowable vaccine")
    } else {
        None
    }
}

pub fn map_rust_dose_status_to_cdc(eval: &DoseEvaluation) -> &'static str {
    match eval.status {
        DoseStatus::Valid => "Valid",
        DoseStatus::Invalid => "Not Valid",
        DoseStatus::Accepted | DoseStatus::Ignored => {
            if matches!(
                map_rust_reason_to_cdc(eval),
                Some("Series Already Complete" | "Age: Too Old")
            ) {
                "Extraneous"
            } else {
                "Not Valid"
            }
        }
    }
}

pub fn map_rust_series_status_to_cdc(forecast: &SeriesForecast) -> &'static str {
    match forecast.status {
        SeriesStatus::NotComplete { .. } => "Not complete",
        SeriesStatus::Complete => "Complete",
        SeriesStatus::NotRecommended => "Aged out",
        SeriesStatus::ConditionallyRecommended => {
            if forecast
                .reasons
                .iter()
                .any(|reason| *reason == lava_forecaster::models::ForecastReason::MaxAgeExceeded)
            {
                "Aged out"
            } else {
                "Not complete"
            }
        }
    }
}

pub fn normalize_rust_results_to_cdc(
    rust_evals: &[DoseEvaluation],
    rust_fc: Option<&SeriesForecast>,
) -> (Vec<CdcActualDose>, CdcActualForecast) {
    let evaluations = rust_evals
        .iter()
        .map(|eval| CdcActualDose {
            dose_date: eval.dose_date,
            cvx: eval.cvx,
            status: map_rust_dose_status_to_cdc(eval).to_string(),
            reason: map_rust_reason_to_cdc(eval).map(str::to_string),
        })
        .collect::<Vec<_>>();

    let forecast = if let Some(fc) = rust_fc {
        let forecast_number = if matches!(fc.status, SeriesStatus::NotComplete { .. }) {
            Some(
                rust_evals
                    .iter()
                    .filter(|eval| eval.status == DoseStatus::Valid)
                    .filter_map(|eval| eval.dose_number)
                    .max()
                    .unwrap_or(0)
                    + 1,
            )
        } else {
            None
        };

        CdcActualForecast {
            series_status: Some(map_rust_series_status_to_cdc(fc).to_string()),
            forecast_number,
            earliest_date: fc.status.earliest_date(),
            recommended_date: fc.status.recommended_date(),
            overdue_date: fc.status.overdue_date(),
        }
    } else {
        CdcActualForecast {
            series_status: None,
            forecast_number: None,
            earliest_date: None,
            recommended_date: None,
            overdue_date: None,
        }
    };

    (evaluations, forecast)
}

pub fn load_cdc_csv_cases(csv_path: &Path) -> Result<Vec<CdcCsvCase>, String> {
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .from_path(csv_path)
        .map_err(|err| format!("Failed to open CDC CSV {:?}: {}", csv_path, err))?;

    let mut cases = Vec::new();
    for row in reader.deserialize::<HashMap<String, String>>() {
        let row = row.map_err(|err| format!("Failed to parse CDC CSV row: {}", err))?;
        let test_id = row.get("CDC_Test_ID").map(|s| s.trim()).unwrap_or("");
        if test_id.is_empty() {
            continue;
        }

        let csv_group = row.get("Vaccine_Group").map(|s| s.trim()).unwrap_or("");
        let (group_name, focus_code) = cdc_group_map(csv_group).ok_or_else(|| {
            format!(
                "Unknown CDC vaccine group '{}' in test case {}",
                csv_group, test_id
            )
        })?;

        let dob = parse_cdc_date(row.get("DOB").map(|s| s.as_str()).unwrap_or(""))?
            .ok_or_else(|| format!("Missing DOB in test case {}", test_id))?;
        let execution_date =
            parse_cdc_date(row.get("Assessment_Date").map(|s| s.as_str()).unwrap_or(""))?
                .unwrap_or(dob);
        let gender = match row.get("gender").map(|s| s.trim()).unwrap_or("") {
            "F" => Gender::Female,
            "M" => Gender::Male,
            _ => Gender::Unknown,
        };

        let mut history = Vec::new();
        let mut expected_evaluations = Vec::new();
        for idx in 1..=7 {
            let date_key = format!("Date_Administered_{}", idx);
            let cvx_key = format!("CVX_{}", idx);
            let status_key = format!("Evaluation_Status_{}", idx);
            let reason_key = format!("Evaluation_Reason_{}", idx);

            let Some(dose_date) =
                parse_cdc_date(row.get(&date_key).map(|s| s.as_str()).unwrap_or(""))?
            else {
                continue;
            };
            let cvx_str = row.get(&cvx_key).map(|s| s.trim()).unwrap_or("");
            if cvx_str.is_empty() {
                continue;
            }
            let cvx = Cvx(cvx_str.parse::<u16>().map_err(|_| {
                format!(
                    "Invalid CVX '{}' in test case {} dose {}",
                    cvx_str, test_id, idx
                )
            })?);

            history.push(Dose {
                date: dose_date,
                cvx,
                is_valid: None,
            });

            let status = row
                .get(&status_key)
                .map(|s| s.trim())
                .unwrap_or("")
                .to_string();
            let reason = row
                .get(&reason_key)
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(str::to_string);

            expected_evaluations.push(CdcExpectedDose {
                dose_date,
                cvx,
                status,
                reason,
            });
        }

        let series_status = row
            .get("Series_Status")
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        if series_status.is_empty() {
            return Err(format!("Missing Series_Status in test case {}", test_id));
        }

        let expected_forecast = CdcExpectedForecast {
            series_status,
            forecast_number: parse_cdc_forecast_number(
                row.get("Forecast_#").map(|s| s.as_str()).unwrap_or(""),
            )?,
            earliest_date: parse_cdc_date(
                row.get("Earliest_Date").map(|s| s.as_str()).unwrap_or(""),
            )?,
            recommended_date: parse_cdc_date(
                row.get("Recommended_Date")
                    .map(|s| s.as_str())
                    .unwrap_or(""),
            )?,
            overdue_date: parse_cdc_date(
                row.get("Past_Due_Date").map(|s| s.as_str()).unwrap_or(""),
            )?,
        };

        let case_name = sanitize_name(
            test_id,
            row.get("Test_Case_Name").map(|s| s.as_str()).unwrap_or(""),
        );
        cases.push(CdcCsvCase {
            test_id: test_id.to_string(),
            unified_case: UnifiedTestCase {
                name: case_name,
                group: group_name.to_string(),
                focus_code: focus_code.to_string(),
                patient: Patient {
                    birth_date: dob,
                    gender,
                    immunities: Vec::new(),
                    contraindications: Vec::new(),
                },
                history,
                execution_date,
                expected: None,
            },
            expected: CdcExpectedResults {
                evaluations: expected_evaluations,
                forecast: expected_forecast,
            },
        });
    }

    Ok(cases)
}

pub fn print_cdc_comparison_table(
    tc_name: &str,
    rust_evals: &[CdcActualDose],
    rust_fc: &CdcActualForecast,
    exp_evals: &[CdcExpectedDose],
    exp_fc: &CdcExpectedForecast,
    errors: &[String],
) {
    if !errors.is_empty() {
        println!("\nTest Case: \x1b[91m{}\x1b[0m (FAIL)", tc_name);
        for err in errors {
            println!("  - \x1b[91m{}\x1b[0m", err);
        }
    } else {
        println!("\nTest Case: \x1b[92m{}\x1b[0m (PASS)", tc_name);
    }

    println!(
        "{:<12} | {:<4} | {:<32} | {:<32} | Status",
        "Date", "CVX", "Rust CDC Eval", "Expected CDC Eval"
    );
    println!("{}", "-".repeat(96));

    let mut all_keys: Vec<(NaiveDate, Cvx)> =
        rust_evals.iter().map(|e| (e.dose_date, e.cvx)).collect();
    for eval in exp_evals {
        if !all_keys.contains(&(eval.dose_date, eval.cvx)) {
            all_keys.push((eval.dose_date, eval.cvx));
        }
    }
    all_keys.sort_by_key(|key| key.0);

    for (date, cvx) in all_keys {
        let rust_eval = rust_evals
            .iter()
            .find(|eval| eval.dose_date == date && eval.cvx == cvx);
        let exp_eval = exp_evals
            .iter()
            .find(|eval| eval.dose_date == date && eval.cvx == cvx);

        let rust_str = rust_eval
            .map(|eval| match &eval.reason {
                Some(reason) => format!("{} ({})", eval.status, reason),
                None => eval.status.clone(),
            })
            .unwrap_or_else(|| "MISSING".to_string());
        let exp_str = exp_eval
            .map(|eval| match &eval.reason {
                Some(reason) => format!("{} ({})", eval.status, reason),
                None => eval.status.clone(),
            })
            .unwrap_or_else(|| "MISSING".to_string());

        let match_ok = match (rust_eval, exp_eval) {
            (Some(rust_eval), Some(exp_eval)) => {
                rust_eval.status == exp_eval.status
                    && (exp_eval.reason.is_none() || rust_eval.reason == exp_eval.reason)
            }
            _ => false,
        };
        let status_indicator = if match_ok {
            "\x1b[92mOK\x1b[0m"
        } else {
            "\x1b[91mFAIL\x1b[0m"
        };

        println!(
            "{:<12} | {:<4} | {:<32} | {:<32} | {}",
            date.format("%Y-%m-%d"),
            cvx,
            rust_str,
            exp_str,
            status_indicator,
        );
    }

    println!("\nCDC Forecast:");
    println!(
        "{:<18} | {:<22} | {:<22}",
        "Field", "Rust Forecast", "Expected Forecast"
    );
    println!("{}", "-".repeat(70));
    let format_optional_date = |date: Option<NaiveDate>| {
        date.map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "-".to_string())
    };
    let format_optional_number = |value: Option<usize>| {
        value
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string())
    };
    println!(
        "{:<18} | {:<22} | {:<22}",
        "series_status",
        rust_fc
            .series_status
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        exp_fc.series_status.clone()
    );
    println!(
        "{:<18} | {:<22} | {:<22}",
        "forecast_number",
        format_optional_number(rust_fc.forecast_number),
        format_optional_number(exp_fc.forecast_number)
    );
    println!(
        "{:<18} | {:<22} | {:<22}",
        "earliest_date",
        format_optional_date(rust_fc.earliest_date),
        format_optional_date(exp_fc.earliest_date)
    );
    println!(
        "{:<18} | {:<22} | {:<22}",
        "recommended_date",
        format_optional_date(rust_fc.recommended_date),
        format_optional_date(exp_fc.recommended_date)
    );
    println!(
        "{:<18} | {:<22} | {:<22}",
        "past_due_date",
        format_optional_date(rust_fc.overdue_date),
        format_optional_date(exp_fc.overdue_date)
    );
}

pub fn run_cdc_csv_mode(
    client: &Client,
    csv_path: &Path,
    filter_group: Option<String>,
    filter_case: Option<String>,
    rest_url: Option<String>,
    verbose: bool,
) {
    let cases = load_cdc_csv_cases(csv_path).unwrap_or_else(|err| panic!("{}", err));

    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;
    let mut failed_details = Vec::new();
    let mut group_summary: BTreeMap<String, SummaryCounts> = BTreeMap::new();

    for cdc_case in cases {
        let tc = &cdc_case.unified_case;
        if let Some(ref fg) = filter_group {
            if !tc.group.eq_ignore_ascii_case(fg) {
                continue;
            }
        }
        if let Some(ref fc) = filter_case {
            if tc.name != *fc && cdc_case.test_id != *fc {
                continue;
            }
        }

        total += 1;

        let (rust_evals, rust_fc) = if let Some(ref r_url) = rest_url {
            match query_rust_rest_service(client, r_url, tc) {
                Ok(resp) => {
                    let rust_group = resp
                        .vaccine_groups
                        .iter()
                        .find(|rg| rg.vaccine_group == tc.group)
                        .cloned();
                    match rust_group {
                        Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                        None => (Vec::new(), None),
                    }
                }
                Err(e) => {
                    failed += 1;
                    record_summary_result(&mut group_summary, &tc.group, false);
                    failed_details.push((tc.name.clone(), vec![format!("Rust REST Error: {}", e)]));
                    continue;
                }
            }
        } else {
            let rust_results =
                evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
            let rust_group = rust_results.iter().find(|rg| rg.vaccine_group == tc.group);
            match rust_group {
                Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                None => (Vec::new(), None),
            }
        };

        let (cdc_rust_evals, cdc_rust_fc) =
            normalize_rust_results_to_cdc(&rust_evals, rust_fc.as_ref());

        if filter_case.is_some() && verbose {
            println!("DEBUG: patient: {:?}", tc.patient);
            println!("DEBUG: history: {:?}", tc.history);
            println!("DEBUG: execution_date: {:?}", tc.execution_date);
            println!("DEBUG: rust_evals: {:#?}", rust_evals);
            println!("DEBUG: rust_fc: {:#?}", rust_fc);
            println!("DEBUG: cdc_expected: {:#?}", cdc_case.expected);
        }

        let mut is_ok = true;
        let mut errors = Vec::new();

        for expected_eval in &cdc_case.expected.evaluations {
            let rust_eval = cdc_rust_evals.iter().find(|eval| {
                eval.dose_date == expected_eval.dose_date && eval.cvx == expected_eval.cvx
            });
            match rust_eval {
                Some(rust_eval) => {
                    if rust_eval.status != expected_eval.status {
                        is_ok = false;
                        errors.push(format!(
                            "CDC evaluation status mismatch for dose ({:?}, {}): Rust={}, Expected={}",
                            expected_eval.dose_date,
                            expected_eval.cvx,
                            rust_eval.status,
                            expected_eval.status,
                        ));
                    }
                    if let Some(expected_reason) = &expected_eval.reason {
                        if rust_eval.reason.as_ref() != Some(expected_reason) {
                            is_ok = false;
                            errors.push(format!(
                                "CDC evaluation reason mismatch for dose ({:?}, {}): Rust={:?}, Expected={:?}",
                                expected_eval.dose_date,
                                expected_eval.cvx,
                                rust_eval.reason,
                                expected_eval.reason,
                            ));
                        }
                    }
                }
                None => {
                    is_ok = false;
                    errors.push(format!(
                        "Evaluation missing in Rust for dose ({:?}, {})",
                        expected_eval.dose_date, expected_eval.cvx,
                    ));
                }
            }
        }
        for rust_eval in &cdc_rust_evals {
            let expected_eval =
                cdc_case.expected.evaluations.iter().find(|eval| {
                    eval.dose_date == rust_eval.dose_date && eval.cvx == rust_eval.cvx
                });
            if expected_eval.is_none() {
                is_ok = false;
                errors.push(format!(
                    "Evaluation missing in CDC expected results for dose ({:?}, {})",
                    rust_eval.dose_date, rust_eval.cvx,
                ));
            }
        }

        if cdc_rust_fc.series_status.as_ref() != Some(&cdc_case.expected.forecast.series_status) {
            is_ok = false;
            errors.push(format!(
                "CDC forecast status mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.series_status, cdc_case.expected.forecast.series_status,
            ));
        }
        if cdc_rust_fc.forecast_number != cdc_case.expected.forecast.forecast_number {
            is_ok = false;
            errors.push(format!(
                "CDC forecast target mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.forecast_number, cdc_case.expected.forecast.forecast_number,
            ));
        }
        if cdc_rust_fc.earliest_date != cdc_case.expected.forecast.earliest_date {
            is_ok = false;
            errors.push(format!(
                "CDC forecast earliest date mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.earliest_date, cdc_case.expected.forecast.earliest_date,
            ));
        }
        if cdc_rust_fc.recommended_date != cdc_case.expected.forecast.recommended_date {
            is_ok = false;
            errors.push(format!(
                "CDC forecast recommended date mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.recommended_date, cdc_case.expected.forecast.recommended_date,
            ));
        }
        if cdc_rust_fc.overdue_date != cdc_case.expected.forecast.overdue_date {
            is_ok = false;
            errors.push(format!(
                "CDC forecast past due date mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.overdue_date, cdc_case.expected.forecast.overdue_date,
            ));
        }

        if verbose {
            print_cdc_comparison_table(
                &tc.name,
                &cdc_rust_evals,
                &cdc_rust_fc,
                &cdc_case.expected.evaluations,
                &cdc_case.expected.forecast,
                &errors,
            );
        } else if !is_ok {
            println!("\nTest Case: \x1b[91m{}\x1b[0m (FAIL)", tc.name);
            for err in &errors {
                println!("  - \x1b[91m{}\x1b[0m", err);
            }
        }

        if is_ok {
            passed += 1;
            record_summary_result(&mut group_summary, &tc.group, true);
        } else {
            failed += 1;
            record_summary_result(&mut group_summary, &tc.group, false);
            failed_details.push((tc.name.clone(), errors));
        }
    }

    print_summary(
        "CDC Compliance Summary",
        total,
        passed,
        failed,
        &group_summary,
    );

    if failed > 0 {
        println!("\nFailed Cases:");
        for (name, errs) in failed_details {
            println!("  FAIL: {}", name);
            for err in errs {
                println!("    - {}", err);
            }
        }
        std::process::exit(1);
    } else {
        println!("All CDC-checked tests passed successfully!");
        std::process::exit(0);
    }
}
