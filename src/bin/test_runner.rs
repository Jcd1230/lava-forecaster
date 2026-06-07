use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use chrono::{Datelike, NaiveDate};
use csv::ReaderBuilder;
use serde::Deserialize;
use lava_forecaster::{
    evaluate_patient_all_groups,
    engine::is_eval_ignored,
    models::{
        Dose, DoseEvaluation, DoseStatus, EvaluationReason, ExpectedResults,
        Gender, Patient, SeriesForecast, SeriesStatus, UnifiedTestCase, Cvx,
    },
};

#[path = "test_runner/fuzzer.rs"]
mod fuzzer;

#[derive(Debug, Clone)]
struct CdcExpectedDose {
    dose_date: NaiveDate,
    cvx: Cvx,
    status: String,
    reason: Option<String>,
}

#[derive(Debug, Clone)]
struct CdcExpectedForecast {
    series_status: String,
    forecast_number: Option<usize>,
    earliest_date: Option<NaiveDate>,
    recommended_date: Option<NaiveDate>,
    overdue_date: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
struct CdcExpectedResults {
    evaluations: Vec<CdcExpectedDose>,
    forecast: CdcExpectedForecast,
}

#[derive(Debug, Clone)]
struct CdcCsvCase {
    test_id: String,
    unified_case: UnifiedTestCase,
    expected: CdcExpectedResults,
}

#[derive(Debug, Clone)]
struct CdcActualDose {
    dose_date: NaiveDate,
    cvx: Cvx,
    status: String,
    reason: Option<String>,
}

#[derive(Debug, Clone)]
struct CdcActualForecast {
    series_status: Option<String>,
    forecast_number: Option<usize>,
    earliest_date: Option<NaiveDate>,
    recommended_date: Option<NaiveDate>,
    overdue_date: Option<NaiveDate>,
}

#[derive(Debug, Default, Clone, Copy)]
struct SummaryCounts {
    executed: usize,
    passed: usize,
    failed: usize,
}

fn record_summary_result(summary: &mut BTreeMap<String, SummaryCounts>, group: &str, passed: bool) {
    let entry = summary.entry(group.to_string()).or_default();
    entry.executed += 1;
    if passed {
        entry.passed += 1;
    } else {
        entry.failed += 1;
    }
}

fn print_summary(title: &str, total: usize, passed: usize, failed: usize, group_summary: &BTreeMap<String, SummaryCounts>) {
    println!("\n--- {} ---", title);
    println!("Executed: {}", total);
    println!("Passed  : {}", passed);
    println!("Failed  : {}", failed);
    if total > 0 {
        println!("Pass %  : {:.2}", passed as f64 / total as f64 * 100.0);
    }

    if !group_summary.is_empty() {
        println!("\nPer-Group Summary:");
        println!("{:<16} | {:>8} | {:>8} | {:>8} | {:>8}", "Group", "Executed", "Passed", "Failed", "Pass %");
        println!("{}", "-".repeat(62));
        for (group, counts) in group_summary {
            let pass_pct = if counts.executed > 0 {
                counts.passed as f64 / counts.executed as f64 * 100.0
            } else {
                0.0
            };
            println!(
                "{:<16} | {:>8} | {:>8} | {:>8} | {:>7.2}",
                group,
                counts.executed,
                counts.passed,
                counts.failed,
                pass_pct,
            );
        }
    }
}

fn sanitize_name(test_id: &str, name: &str) -> String {
    let full_name = format!("cdsi_{}_{}", test_id, name);
    full_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn parse_cdc_date(date_str: &str) -> Result<Option<NaiveDate>, String> {
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

fn parse_cdc_forecast_number(value: &str) -> Result<Option<usize>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "-" {
        return Ok(None);
    }

    trimmed
        .parse::<usize>()
        .map(Some)
        .map_err(|_| format!("Could not parse CDC forecast number '{}'", trimmed))
}

fn cdc_group_map(csv_group: &str) -> Option<(&'static str, &'static str)> {
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

fn map_rust_reason_to_cdc(eval: &DoseEvaluation) -> Option<&'static str> {
    if eval.status == DoseStatus::Valid {
        return None;
    }

    let has_reason = |target: EvaluationReason| eval.reasons.iter().any(|reason| *reason == target);

    if has_reason(EvaluationReason::BelowMinimumAge) || has_reason(EvaluationReason::BelowMinimumAgeFinalDose) {
        Some("Age: Too Young")
    } else if has_reason(EvaluationReason::AboveRecommendedAgeSeries) {
        Some("Age: Too Old")
    } else if has_reason(EvaluationReason::BelowMinimumInterval) || has_reason(EvaluationReason::DuplicateShotSameDay) {
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

fn map_rust_dose_status_to_cdc(eval: &DoseEvaluation) -> &'static str {
    match eval.status {
        DoseStatus::Valid => "Valid",
        DoseStatus::Invalid => "Not Valid",
        DoseStatus::Accepted | DoseStatus::Ignored => {
            if matches!(map_rust_reason_to_cdc(eval), Some("Series Already Complete" | "Age: Too Old")) {
                "Extraneous"
            } else {
                "Not Valid"
            }
        }
    }
}

fn map_rust_series_status_to_cdc(forecast: &SeriesForecast) -> &'static str {
    match forecast.status {
        SeriesStatus::NotComplete { .. } => "Not complete",
        SeriesStatus::Complete => "Complete",
        SeriesStatus::NotRecommended => "Aged out",
        SeriesStatus::ConditionallyRecommended => {
            if forecast.reasons.iter().any(|reason| reason.as_ref() == "MAX_AGE_EXCEEDED") {
                "Aged out"
            } else {
                "Not complete"
            }
        }
    }
}

fn normalize_rust_results_to_cdc(
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

fn load_cdc_csv_cases(csv_path: &Path) -> Result<Vec<CdcCsvCase>, String> {
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
        let (group_name, focus_code) = cdc_group_map(csv_group)
            .ok_or_else(|| format!("Unknown CDC vaccine group '{}' in test case {}", csv_group, test_id))?;

        let dob = parse_cdc_date(row.get("DOB").map(|s| s.as_str()).unwrap_or(""))?
            .ok_or_else(|| format!("Missing DOB in test case {}", test_id))?;
        let execution_date = parse_cdc_date(row.get("Assessment_Date").map(|s| s.as_str()).unwrap_or(""))?
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

            let Some(dose_date) = parse_cdc_date(row.get(&date_key).map(|s| s.as_str()).unwrap_or(""))? else {
                continue;
            };
            let cvx_str = row.get(&cvx_key).map(|s| s.trim()).unwrap_or("");
            if cvx_str.is_empty() {
                continue;
            }
            let cvx = Cvx(
                cvx_str
                    .parse::<u16>()
                    .map_err(|_| format!("Invalid CVX '{}' in test case {} dose {}", cvx_str, test_id, idx))?,
            );

            history.push(Dose { date: dose_date, cvx, is_valid: None });

            let status = row.get(&status_key).map(|s| s.trim()).unwrap_or("").to_string();
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

        let series_status = row.get("Series_Status").map(|s| s.trim()).unwrap_or("").to_string();
        if series_status.is_empty() {
            return Err(format!("Missing Series_Status in test case {}", test_id));
        }

        let expected_forecast = CdcExpectedForecast {
            series_status,
            forecast_number: parse_cdc_forecast_number(row.get("Forecast_#").map(|s| s.as_str()).unwrap_or(""))?,
            earliest_date: parse_cdc_date(row.get("Earliest_Date").map(|s| s.as_str()).unwrap_or(""))?,
            recommended_date: parse_cdc_date(row.get("Recommended_Date").map(|s| s.as_str()).unwrap_or(""))?,
            overdue_date: parse_cdc_date(row.get("Past_Due_Date").map(|s| s.as_str()).unwrap_or(""))?,
        };

        let case_name = sanitize_name(test_id, row.get("Test_Case_Name").map(|s| s.as_str()).unwrap_or(""));
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

fn print_cdc_comparison_table(
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

    println!("{:<12} | {:<4} | {:<32} | {:<32} | Status", "Date", "CVX", "Rust CDC Eval", "Expected CDC Eval");
    println!("{}", "-".repeat(96));

    let mut all_keys: Vec<(NaiveDate, Cvx)> = rust_evals.iter().map(|e| (e.dose_date, e.cvx)).collect();
    for eval in exp_evals {
        if !all_keys.contains(&(eval.dose_date, eval.cvx)) {
            all_keys.push((eval.dose_date, eval.cvx));
        }
    }
    all_keys.sort_by_key(|key| key.0);

    for (date, cvx) in all_keys {
        let rust_eval = rust_evals.iter().find(|eval| eval.dose_date == date && eval.cvx == cvx);
        let exp_eval = exp_evals.iter().find(|eval| eval.dose_date == date && eval.cvx == cvx);

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
        let status_indicator = if match_ok { "\x1b[92mOK\x1b[0m" } else { "\x1b[91mFAIL\x1b[0m" };

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
    println!("{:<18} | {:<22} | {:<22}", "Field", "Rust Forecast", "Expected Forecast");
    println!("{}", "-".repeat(70));
    let format_optional_date = |date: Option<NaiveDate>| {
        date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string())
    };
    let format_optional_number = |value: Option<usize>| {
        value.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string())
    };
    println!("{:<18} | {:<22} | {:<22}", "series_status", rust_fc.series_status.clone().unwrap_or_else(|| "-".to_string()), exp_fc.series_status.clone());
    println!("{:<18} | {:<22} | {:<22}", "forecast_number", format_optional_number(rust_fc.forecast_number), format_optional_number(exp_fc.forecast_number));
    println!("{:<18} | {:<22} | {:<22}", "earliest_date", format_optional_date(rust_fc.earliest_date), format_optional_date(exp_fc.earliest_date));
    println!("{:<18} | {:<22} | {:<22}", "recommended_date", format_optional_date(rust_fc.recommended_date), format_optional_date(exp_fc.recommended_date));
    println!("{:<18} | {:<22} | {:<22}", "past_due_date", format_optional_date(rust_fc.overdue_date), format_optional_date(exp_fc.overdue_date));
}

fn run_cdc_csv_mode(client: &reqwest::blocking::Client, args: &[String]) {
    let csv_path = Path::new(&args[2]);

    let mut filter_group = None;
    let mut filter_case = None;
    let mut rest_url = None;
    let mut verbose = false;

    let mut idx = 3;
    while idx < args.len() {
        if args[idx] == "--group" {
            filter_group = Some(args[idx + 1].to_uppercase());
            idx += 2;
        } else if args[idx] == "--case" {
            filter_case = Some(args[idx + 1].clone());
            idx += 2;
        } else if args[idx] == "--rest-url" {
            rest_url = Some(args[idx + 1].clone());
            idx += 2;
        } else if args[idx] == "--verbose" || args[idx] == "-v" {
            verbose = true;
            idx += 1;
        } else {
            idx += 1;
        }
    }

    let cases = load_cdc_csv_cases(csv_path).unwrap_or_else(|err| panic!("{}", err));

    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;
    let mut failed_details = Vec::new();
    let mut group_summary: BTreeMap<String, SummaryCounts> = BTreeMap::new();

    for cdc_case in cases {
        let tc = &cdc_case.unified_case;
        if let Some(ref fg) = filter_group {
            if tc.group.to_uppercase() != *fg {
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
                    let rust_group = resp.vaccine_groups.iter().find(|rg| rg.vaccine_group == tc.group).cloned();
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
            let rust_results = evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
            let rust_group = rust_results.iter().find(|rg| rg.vaccine_group == tc.group);
            match rust_group {
                Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                None => (Vec::new(), None),
            }
        };

        let (cdc_rust_evals, cdc_rust_fc) = normalize_rust_results_to_cdc(&rust_evals, rust_fc.as_ref());

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
            let rust_eval = cdc_rust_evals
                .iter()
                .find(|eval| eval.dose_date == expected_eval.dose_date && eval.cvx == expected_eval.cvx);
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
                        expected_eval.dose_date,
                        expected_eval.cvx,
                    ));
                }
            }
        }
        for rust_eval in &cdc_rust_evals {
            let expected_eval = cdc_case
                .expected
                .evaluations
                .iter()
                .find(|eval| eval.dose_date == rust_eval.dose_date && eval.cvx == rust_eval.cvx);
            if expected_eval.is_none() {
                is_ok = false;
                errors.push(format!(
                    "Evaluation missing in CDC expected results for dose ({:?}, {})",
                    rust_eval.dose_date,
                    rust_eval.cvx,
                ));
            }
        }

        if cdc_rust_fc.series_status.as_ref() != Some(&cdc_case.expected.forecast.series_status) {
            is_ok = false;
            errors.push(format!(
                "CDC forecast status mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.series_status,
                cdc_case.expected.forecast.series_status,
            ));
        }
        if cdc_rust_fc.forecast_number != cdc_case.expected.forecast.forecast_number {
            is_ok = false;
            errors.push(format!(
                "CDC forecast target mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.forecast_number,
                cdc_case.expected.forecast.forecast_number,
            ));
        }
        if cdc_rust_fc.earliest_date != cdc_case.expected.forecast.earliest_date {
            is_ok = false;
            errors.push(format!(
                "CDC forecast earliest date mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.earliest_date,
                cdc_case.expected.forecast.earliest_date,
            ));
        }
        if cdc_rust_fc.recommended_date != cdc_case.expected.forecast.recommended_date {
            is_ok = false;
            errors.push(format!(
                "CDC forecast recommended date mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.recommended_date,
                cdc_case.expected.forecast.recommended_date,
            ));
        }
        if cdc_rust_fc.overdue_date != cdc_case.expected.forecast.overdue_date {
            is_ok = false;
            errors.push(format!(
                "CDC forecast past due date mismatch: Rust={:?}, Expected={:?}",
                cdc_rust_fc.overdue_date,
                cdc_case.expected.forecast.overdue_date,
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

    print_summary("CDC Compliance Summary", total, passed, failed, &group_summary);

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

// Helper for relative date resolution
struct RelativeDateResolver {
    dob: NaiveDate,
    dose_dates: Vec<NaiveDate>,
}

impl RelativeDateResolver {
    fn new(dob: NaiveDate) -> Self {
        Self {
            dob,
            dose_dates: Vec::new(),
        }
    }

    fn add_months(d: NaiveDate, months: i32) -> NaiveDate {
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

    fn add_offset(d: NaiveDate, val: i32, unit: char) -> NaiveDate {
        match unit {
            'y' => Self::add_months(d, val * 12),
            'm' => Self::add_months(d, val),
            'w' => d + chrono::Duration::days(val as i64 * 7),
            'd' => d + chrono::Duration::days(val as i64),
            _ => d,
        }
    }

    fn resolve(&mut self, expr: &str) -> NaiveDate {
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

// Java ICE XML Payload Builder
fn generate_xml_payload(patient: &Patient, history: &[Dose]) -> String {
    let mut sae_templates = Vec::new();
    for (idx, dose) in history.iter().enumerate() {
        let dt_str = dose.date.format("%Y%m%d").to_string();
        let is_valid_xml = match dose.is_valid {
            Some(true) => "\n                        <isValid value=\"true\"/>",
            Some(false) => "\n                        <isValid value=\"false\"/>",
            None => "",
        };
        sae_templates.push(format!(
            r#"                    <substanceAdministrationEvent>
                        <templateId root="2.16.840.1.113883.3.795.11.9.1.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.10" extension="{}"/>
                        <substanceAdministrationGeneralPurpose code="384810002" codeSystem="2.16.840.1.113883.6.5"/>
                        <substance>
                            <id root="ab0c489e-782a-4c34-9e4e-9094cc2952d7"/>
                            <substanceCode code="{}" displayName="Vaccine" codeSystem="2.16.840.1.113883.12.292"/>
                        </substance>
                        <administrationTimeInterval high="{}" low="{}" />{}
                    </substanceAdministrationEvent>"#,
            1000 + idx, dose.cvx, dt_str, dt_str, is_valid_xml
        ));
    }

    let sae_str = sae_templates.join("\n");

    // Generate observations (immunities & contraindications)
    let mut obs_templates = Vec::new();
    let mut obs_idx = 0;

    for immunity in &patient.immunities {
        let (focus_code, focus_system) = match immunity.disease.to_lowercase().as_str() {
            "hepb" | "hep_b" | "hep b" | "hepatitis b" => ("070.30".to_string(), "2.16.840.1.113883.6.103".to_string()),
            "varicella" | "chickenpox" => ("052.9".to_string(), "2.16.840.1.113883.6.103".to_string()),
            "measles" | "rubeola" => ("055.9".to_string(), "2.16.840.1.113883.6.103".to_string()),
            "mumps" => ("072.9".to_string(), "2.16.840.1.113883.6.103".to_string()),
            "rubella" | "german measles" => ("056.9".to_string(), "2.16.840.1.113883.6.103".to_string()),
            "hepa" | "hep_a" | "hep a" | "hepatitis a" => ("070.1".to_string(), "2.16.840.1.113883.6.103".to_string()),
            _ => (immunity.disease.clone(), "2.16.840.1.113883.3.795.12.1.1".to_string()),
        };
        let reason_code = match immunity.reason.to_lowercase().as_str() {
            "disease documented" | "disease_documented" => "DISEASE_DOCUMENTED",
            _ => "PROOF_OF_IMMUNITY",
        };
        let dt_str = immunity.date.format("%Y%m%d").to_string();
        obs_templates.push(format!(
            r#"                    <observationResult>
                        <templateId root="2.16.840.1.113883.3.795.11.6.3.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.12" extension="{}"/>
                        <observationFocus code="{}" codeSystem="{}"/>
                        <observationEventTime low="{}" high="{}"/>
                        <observationValue>
                            <concept code="{}" codeSystem="2.16.840.1.113883.3.795.12.100.8"/>
                        </observationValue>
                        <interpretation code="IS_IMMUNE" codeSystem="2.16.840.1.113883.3.795.12.100.9"/>
                    </observationResult>"#,
            2000 + obs_idx, focus_code, focus_system, dt_str, dt_str, reason_code
        ));
        obs_idx += 1;
    }

    for contra in &patient.contraindications {
        let (focus_code, focus_system) = match contra.target.to_lowercase().as_str() {
            "dtp" | "dtap" | "pertussis" => ("70654002".to_string(), "2.16.840.1.113883.6.96".to_string()),
            _ => {
                if contra.target.chars().all(|c| c.is_ascii_digit()) {
                    (contra.target.clone(), "2.16.840.1.113883.6.96".to_string())
                } else {
                    (contra.target.clone(), "2.16.840.1.113883.3.795.12.1.1".to_string())
                }
            }
        };
        let reason_code = if contra.reason.is_empty() {
            "CONTRAINDICATION"
        } else {
            &contra.reason
        };
        let dt_str = contra.date.format("%Y%m%d").to_string();
        obs_templates.push(format!(
            r#"                    <observationResult>
                        <templateId root="2.16.840.1.113883.3.795.11.6.3.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.12" extension="{}"/>
                        <observationFocus code="{}" codeSystem="{}"/>
                        <observationEventTime low="{}" high="{}"/>
                        <observationValue>
                            <concept code="{}" codeSystem="2.16.840.1.113883.3.795.12.100.8"/>
                        </observationValue>
                        <interpretation code="CONTRAINDICATION" codeSystem="2.16.840.1.113883.3.795.12.100.9"/>
                    </observationResult>"#,
            2000 + obs_idx, focus_code, focus_system, dt_str, dt_str, reason_code
        ));
        obs_idx += 1;
    }

    let obs_str = if obs_templates.is_empty() {
        "".to_string()
    } else {
        format!("                <observationResults>\n{}\n                </observationResults>\n", obs_templates.join("\n"))
    };

    let dob_str = patient.birth_date.format("%Y%m%d").to_string();
    let gender_code = match patient.gender {
        Gender::Female => "F",
        Gender::Male => "M",
        Gender::Unknown => "U",
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns3:cdsInput xmlns:ns2="org.opencds.vmr.v1_0.schema.cdsinput.specification"
        xmlns:ns3="org.opencds.vmr.v1_0.schema.cdsinput"
        xmlns:ns4="org.opencds.vmr.v1_0.schema.cdsoutput"
        xmlns:ns5="org.opencds.vmr.v1_0.schema.vmr">
    <templateId root="2.16.840.1.113883.3.795.11.1.1"/>
    <cdsContext>
        <cdsSystemUserPreferredLanguage code="en" codeSystem="2.16.840.1.113883.6.99" displayName="English"/>
    </cdsContext>
    <vmrInput>
        <templateId root="2.16.840.1.113883.3.795.11.1.1"/>
        <patient>
            <templateId root="2.16.840.1.113883.3.795.11.2.1.1"/>
            <id root="2.16.840.1.113883.3.795.12.100.11" extension="43299551" />
            <demographics>
                <birthTime value="{}"/>
                <gender code="{}" codeSystem="2.16.840.1.113883.5.1"/>
            </demographics>
            <clinicalStatements>
{}                <substanceAdministrationEvents>
{}                </substanceAdministrationEvents>
            </clinicalStatements>
        </patient>
    </vmrInput>
</ns3:cdsInput>"#,
        dob_str, gender_code, obs_str, sae_str
    )
}

fn build_evaluate_payload(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
) -> serde_json::Value {
    let xml_content = generate_xml_payload(patient, history);
    let b64_xml = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, xml_content.as_bytes());

    let eval_datetime = eval_date.and_hms_opt(23, 59, 59).unwrap();
    let ts_ms = eval_datetime.and_utc().timestamp_millis();

    serde_json::json!({
        "interactionId": {
            "scopingEntityId": "org.nyc.cir",
            "interactionId": "123456",
            "submissionTime": ts_ms
        },
        "specifiedTime": ts_ms,
        "evaluationRequest": {
            "clientLanguage": "en",
            "clientTimeZoneOffset": "+0000",
            "kmEvaluationRequest": [{
                "kmId": {
                    "scopingEntityId": "org.nyc.cir",
                    "businessId": "ICE",
                    "version": "1.0.0"
                }
            }],
            "dataRequirementItemData": [{
                "driId": {
                    "containingEntityId": {
                        "scopingEntityId": "org.nyc.cir",
                        "businessId": "ICEData",
                        "version": "1.0.0"
                    },
                    "itemId": "cdsPayload"
                },
                "data": {
                    "informationModelSSId": {
                        "scopingEntityId": "org.opencds.vmr",
                        "businessId": "VMR",
                        "version": "1.0"
                    },
                    "base64EncodedPayload": [b64_xml]
                }
            }]
        }
    })
}

// Minimal XML parser to extract expected evaluations & forecast from Java response
fn parse_date_only(date_str: &str) -> Option<NaiveDate> {
    if date_str.len() < 8 {
        return None;
    }
    let clean: String = date_str.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean.len() >= 8 {
        NaiveDate::parse_from_str(&clean[0..8], "%Y%m%d").ok()
    } else {
        None
    }
}

fn map_legacy_status(legacy_status: &str, reasons: &[String]) -> SeriesStatus {
    if reasons.iter().any(|r| r == "PROOF_OF_IMMUNITY" || r == "DISEASE_DOCUMENTED") {
        return SeriesStatus::Complete;
    }
    if legacy_status == "COMPLETE" || reasons.iter().any(|r| r.contains("COMPLETE")) {
        return SeriesStatus::Complete;
    }
    if legacy_status == "CONDITIONAL" {
        return SeriesStatus::ConditionallyRecommended;
    }
    if legacy_status == "NOT_RECOMMENDED" {
        return SeriesStatus::NotRecommended;
    }
    if legacy_status == "RECOMMENDED" || legacy_status == "FUTURE_RECOMMENDED" {
        return SeriesStatus::default();
    }
    SeriesStatus::default()
}

fn is_immune(patient: &Patient, group: &str, eval_date: NaiveDate) -> bool {
    let group_lower = group.to_lowercase();
    patient.immunities.iter().any(|imm| {
        let imm_disease_lower = imm.disease.to_lowercase();
        let matches_group = if group_lower.contains("hepb") || group_lower.contains("hep_b") || group_lower.contains("hep b") || group_lower.contains("hepatitis b") {
            imm_disease_lower.contains("hepb") || imm_disease_lower.contains("hep_b") || imm_disease_lower.contains("hep b") || imm_disease_lower.contains("hepatitis b")
        } else if group_lower.contains("varicella") {
            imm_disease_lower.contains("varicella") || imm_disease_lower.contains("chickenpox")
        } else if group_lower.contains("measles") {
            imm_disease_lower.contains("measles") || imm_disease_lower.contains("rubeola")
        } else if group_lower.contains("mumps") {
            imm_disease_lower.contains("mumps")
        } else if group_lower.contains("rubella") {
            imm_disease_lower.contains("rubella") || imm_disease_lower.contains("german measles")
        } else if group_lower.contains("mmr") {
            imm_disease_lower.contains("mmr") || imm_disease_lower.contains("measles") || imm_disease_lower.contains("mumps") || imm_disease_lower.contains("rubella")
        } else {
            imm_disease_lower == group_lower
        };
        matches_group && eval_date >= imm.date
    })
}

fn is_contraindicated(patient: &Patient, group: &str, eval_date: NaiveDate) -> bool {
    let group_lower = group.to_lowercase();
    patient.contraindications.iter().any(|c| {
        if c.cvx.is_some() {
            return false;
        }
        let target_lower = c.target.to_lowercase();
        let matches_group = if group_lower.contains("dtp") || group_lower.contains("dtap") || group_lower.contains("dt") || group_lower.contains("tdap") || group_lower.contains("td") || group_lower.contains("diphtheria") || group_lower.contains("tetanus") || group_lower.contains("pertussis") {
            target_lower.contains("dtp") || target_lower.contains("dtap") || target_lower.contains("dt") || target_lower.contains("tdap") || target_lower.contains("td") || target_lower.contains("diphtheria") || target_lower.contains("tetanus") || target_lower.contains("pertussis")
        } else {
            target_lower == group_lower
        };
        let active = eval_date >= c.date && c.valid_until.map_or(true, |until| eval_date < until);
        matches_group && active
    })
}



fn map_legacy_dose_status(status: &str) -> DoseStatus {
    match status {
        "VALID" => DoseStatus::Valid,
        "INVALID" => DoseStatus::Invalid,
        "ACCEPTED" => DoseStatus::Accepted,
        _ => DoseStatus::Invalid,
    }
}

fn process_element(
    name: &[u8],
    attributes: quick_xml::events::attributes::Attributes,
    tag_stack: &[String],
    focus_code: &str,
    outer_date: &mut Option<NaiveDate>,
    outer_cvx: &mut Option<Cvx>,
    inner_focus_matched: &mut bool,
    inner_status: &mut Option<String>,
    inner_dose_number: &mut Option<usize>,
    inner_reasons: &mut Vec<String>,
    prop_focus_matched: &mut bool,
    prop_earliest: &mut Option<NaiveDate>,
    prop_recommended: &mut Option<NaiveDate>,
    prop_overdue: &mut Option<NaiveDate>,
    prop_status: &mut Option<String>,
    prop_reasons: &mut Vec<String>,
) {
    let event_depth = tag_stack.iter().filter(|&t| t == "substanceAdministrationEvent").count();
    let in_proposal = tag_stack.iter().any(|t| t == "substanceAdministrationProposal");

    match name {
        b"substanceAdministrationEvent" => {
            if event_depth == 0 {
                *outer_date = None;
                *outer_cvx = None;
            } else if event_depth == 1 {
                *inner_focus_matched = false;
                *inner_status = None;
                *inner_dose_number = None;
                inner_reasons.clear();
            }
        }
        b"substanceAdministrationProposal" => {
            *prop_focus_matched = false;
            *prop_earliest = None;
            *prop_recommended = None;
            *prop_overdue = None;
            *prop_status = None;
            prop_reasons.clear();
        }
        b"administrationTimeInterval" => {
            if event_depth == 1 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        let key = a.key.as_ref();
                        if key == b"low" || key == b"high" {
                            let val = String::from_utf8_lossy(a.value.as_ref());
                            if let Some(d) = parse_date_only(&val) {
                                *outer_date = Some(d);
                            }
                        }
                    }
                }
            }
        }
        b"substanceCode" => {
            if event_depth == 1 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            let raw_cvx = String::from_utf8_lossy(a.value.as_ref());
                            if let Ok(n) = raw_cvx.parse::<u16>() {
                                *outer_cvx = Some(Cvx(n));
                            }
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            let code = String::from_utf8_lossy(a.value.as_ref());
                            if code == focus_code {
                                *prop_focus_matched = true;
                            }
                        }
                    }
                }
            }
        }
        b"observationFocus" => {
            if event_depth == 2 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            let code = String::from_utf8_lossy(a.value.as_ref());
                            if code == focus_code {
                                *inner_focus_matched = true;
                            }
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            let code = String::from_utf8_lossy(a.value.as_ref());
                            if code == focus_code {
                                *prop_focus_matched = true;
                            }
                        }
                    }
                }
            }
        }
        b"concept" => {
            if event_depth == 2 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            *inner_status = Some(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            *prop_status = Some(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            }
        }
        b"interpretation" => {
            if event_depth == 2 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            inner_reasons.push(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            prop_reasons.push(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            }
        }
        b"doseNumber" => {
            if event_depth == 2 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"value" {
                            let val_str = String::from_utf8_lossy(a.value.as_ref());
                            if let Ok(val) = val_str.parse::<usize>() {
                                *inner_dose_number = Some(val);
                            }
                        }
                    }
                }
            }
        }
        b"validAdministrationTimeInterval" => {
            if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"low" {
                            let val = String::from_utf8_lossy(a.value.as_ref());
                            *prop_earliest = parse_date_only(&val);
                        }
                    }
                }
            }
        }
        b"proposedAdministrationTimeInterval" => {
            if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        let key = a.key.as_ref();
                        let val = String::from_utf8_lossy(a.value.as_ref());
                        if key == b"low" {
                            *prop_recommended = parse_date_only(&val);
                        } else if key == b"high" {
                            *prop_overdue = parse_date_only(&val);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn parse_legacy_xml(xml_content: &str, focus_code: &str) -> ExpectedResults {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut evaluations = Vec::new();
    let mut forecasts = Vec::new();

    let mut tag_stack: Vec<String> = Vec::new();

    let mut outer_date = None;
    let mut outer_cvx = None;

    let mut inner_focus_matched = false;
    let mut inner_status = None;
    let mut inner_dose_number = None;
    let mut inner_reasons = Vec::new();

    let mut prop_focus_matched = false;
    let mut prop_earliest = None;
    let mut prop_recommended = None;
    let mut prop_overdue = None;
    let mut prop_status = None;
    let mut prop_reasons = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error parsing XML at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) => {
                let name = e.local_name();
                let name_bytes = name.as_ref();
                let name_str = String::from_utf8_lossy(name_bytes).into_owned();

                process_element(
                    name_bytes,
                    e.attributes(),
                    &tag_stack,
                    focus_code,
                    &mut outer_date,
                    &mut outer_cvx,
                    &mut inner_focus_matched,
                    &mut inner_status,
                    &mut inner_dose_number,
                    &mut inner_reasons,
                    &mut prop_focus_matched,
                    &mut prop_earliest,
                    &mut prop_recommended,
                    &mut prop_overdue,
                    &mut prop_status,
                    &mut prop_reasons,
                );

                tag_stack.push(name_str);
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.local_name();
                let name_bytes = name.as_ref();

                process_element(
                    name_bytes,
                    e.attributes(),
                    &tag_stack,
                    focus_code,
                    &mut outer_date,
                    &mut outer_cvx,
                    &mut inner_focus_matched,
                    &mut inner_status,
                    &mut inner_dose_number,
                    &mut inner_reasons,
                    &mut prop_focus_matched,
                    &mut prop_earliest,
                    &mut prop_recommended,
                    &mut prop_overdue,
                    &mut prop_status,
                    &mut prop_reasons,
                );
            }
            Ok(Event::End(ref e)) => {
                let name = e.local_name();
                let name_bytes = name.as_ref();
                let name_str = String::from_utf8_lossy(name_bytes);

                if name_bytes == b"substanceAdministrationEvent" {
                    let event_depth = tag_stack.iter().filter(|&t| t == "substanceAdministrationEvent").count();
                    if event_depth == 2 {
                        if inner_focus_matched {
                            if let (Some(dt), Some(cvx), Some(ref st)) = (outer_date, outer_cvx.as_ref(), inner_status.as_ref()) {
                                let mapped_reasons: Vec<EvaluationReason> = inner_reasons
                                    .iter()
                                    .filter_map(|r| match r.as_str() {
                                        "BelowMinimumAge" => Some(EvaluationReason::BelowMinimumAge),
                                        "BelowMinimumInterval" => Some(EvaluationReason::BelowMinimumInterval),
                                        "TooEarlyLiveVirus" => Some(EvaluationReason::TooEarlyLiveVirus),
                                        "DuplicateShotSameDay" => Some(EvaluationReason::DuplicateShotSameDay),
                                        "VaccineNotPartOfSeries" => Some(EvaluationReason::VaccineNotPartOfSeries),
                                        "AboveRecommendedAgeSeries" => Some(EvaluationReason::AboveRecommendedAgeSeries),
                                        _ => None,
                                    })
                                    .collect();

                                evaluations.push(DoseEvaluation {
                                    dose_date: dt,
                                    cvx: *cvx,
                                    status: map_legacy_dose_status(st),
                                    dose_number: inner_dose_number,
                                    reasons: mapped_reasons.into(),
                                });
                            }
                        }
                    }
                } else if name_bytes == b"substanceAdministrationProposal" {
                    if prop_focus_matched {
                        let st = prop_status.as_deref().unwrap_or("RECOMMENDED");
                        let legacy_status = map_legacy_status(st, &prop_reasons);
                        forecasts.push(SeriesForecast {
                            series_name: "".into(),
                            status: if matches!(legacy_status, SeriesStatus::NotComplete { .. }) {
                                SeriesStatus::NotComplete {
                                    earliest_date: prop_earliest,
                                    recommended_date: prop_recommended,
                                    overdue_date: prop_overdue,
                                    latest_date: None,
                                }
                            } else {
                                legacy_status
                            },
                            reasons: prop_reasons.iter().map(|r| r.clone().into()).collect(),
                        });
                    }
                }

                if let Some(last) = tag_stack.last() {
                    if last == name_str.as_ref() {
                        tag_stack.pop();
                    }
                }
            }
            _ => {}
        }
        buf.clear();
    }

    ExpectedResults {
        evaluations,
        forecasts,
    }
}

// REST call to Java ICE
fn query_java_service(client: &reqwest::blocking::Client, java_endpoint: &str, payload: serde_json::Value) -> String {
    let resp = client
        .post(java_endpoint)
        .json(&payload)
        .send()
        .expect("Failed to send request to Java service");
    
    let resp_json: serde_json::Value = resp.json().expect("Failed to parse Java JSON response");
    
    let b64_payload = resp_json["finalKMEvaluationResponse"][0]["kmEvaluationResultData"][0]["data"]["base64EncodedPayload"][0]
        .as_str()
        .expect("Failed to extract base64 payload from Java response");

    let clean_b64: String = b64_payload.chars().filter(|c| !c.is_whitespace()).collect();
    let xml_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, clean_b64.as_bytes())
        .expect("Failed to decode base64 XML payload");

    String::from_utf8(xml_bytes).expect("Failed to decode UTF-8 XML string")
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

fn import_python_cases(input_file: &Path, output_dir: &Path) {
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

fn print_comparison_table(
    tc_name: &str,
    group: &str,
    rust_evals: &[DoseEvaluation],
    rust_fc: Option<&SeriesForecast>,
    exp_evals: &[DoseEvaluation],
    exp_fc: Option<&SeriesForecast>,
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

    println!("{:<12} | {:<4} | {:<28} | {:<28} | Status", "Date", "CVX", "Rust Evaluation", "Expected Evaluation");
    println!("{}", "-".repeat(90));

    let mut all_keys: Vec<(NaiveDate, Cvx)> = Vec::new();
    for e in rust_evals {
        all_keys.push((e.dose_date, e.cvx));
    }
    for e in exp_evals {
        if !all_keys.contains(&(e.dose_date, e.cvx)) {
            all_keys.push((e.dose_date, e.cvx));
        }
    }
    all_keys.sort_by_key(|k| k.0);

    for (dt, cvx) in all_keys {
        let r = rust_evals.iter().find(|e| e.dose_date == dt && e.cvx == cvx);
        let e = exp_evals.iter().find(|e| e.dose_date == dt && e.cvx == cvx);

        let format_eval = |eval: &DoseEvaluation| -> String {
            let dose_num = eval.dose_number.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());
            if eval.status == DoseStatus::Invalid {
                let ignored = is_eval_ignored(group, eval.cvx, eval.dose_date, eval.status);
                let tag = if ignored { "Ignored" } else { "Not Ignored" };
                format!("{:?} ({}) #{}", eval.status, tag, dose_num)
            } else {
                format!("{:?} #{}", eval.status, dose_num)
            }
        };

        let r_str = match r {
            Some(re) => format_eval(re),
            None => "MISSING".to_string(),
        };
        let e_str = match e {
            Some(ee) => format_eval(ee),
            None => "MISSING".to_string(),
        };

        let match_ok = match (r, e) {
            (Some(re), Some(ee)) => re.status == ee.status && (ee.dose_number.is_none() || re.dose_number == ee.dose_number),
            _ => false,
        };

        let status_indicator = if match_ok { "\x1b[92mOK\x1b[0m" } else { "\x1b[91mFAIL\x1b[0m" };
        println!("{:<12} | {:<4} | {:<28} | {:<28} | {}", dt.format("%Y-%m-%d"), cvx, r_str, e_str, status_indicator);
    }

    println!("\nForecasts:");
    println!("{:<15} | {:<22} | {:<22}", "Field", "Rust Forecast", "Expected Forecast");
    println!("{}", "-".repeat(65));

    let fields = ["status", "earliest_date", "recommended_date", "overdue_date"];
    for field in &fields {
        let r_val = match (field, rust_fc) {
            (&"status", Some(fc)) => format!("{:?}", fc.status),
            (&"earliest_date", Some(fc)) => fc.status.earliest_date().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string()),
            (&"recommended_date", Some(fc)) => fc.status.recommended_date().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string()),
            (&"overdue_date", Some(fc)) => fc.status.overdue_date().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string()),
            _ => "-".to_string(),
        };
        let e_val = match (field, exp_fc) {
            (&"status", Some(fc)) => format!("{:?}", fc.status),
            (&"earliest_date", Some(fc)) => fc.status.earliest_date().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string()),
            (&"recommended_date", Some(fc)) => fc.status.recommended_date().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string()),
            (&"overdue_date", Some(fc)) => fc.status.overdue_date().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string()),
            _ => "-".to_string(),
        };

        println!("{:<15} | {:<22} | {:<22}", field, r_val, e_val);
    }
    println!();
}

fn query_rust_rest_service(client: &reqwest::blocking::Client, rust_url: &str, tc: &UnifiedTestCase) -> Result<lava_forecaster::models::ForecastResponse, String> {
    let req_payload = lava_forecaster::models::ForecastRequest {
        patient: tc.patient.clone(),
        history: tc.history.clone(),
        execution_date: tc.execution_date,
    };
    let endpoint = format!("{}/evaluate", rust_url);
    let resp = client.post(&endpoint)
        .json(&req_payload)
        .send()
        .map_err(|e| format!("Failed to connect to Rust REST server: {}", e))?;
    
    if !resp.status().is_success() {
        return Err(format!("Rust REST server returned error: {}", resp.status()));
    }
    
    let resp_data: lava_forecaster::models::ForecastResponse = resp.json()
        .map_err(|e| format!("Failed to parse Rust REST response: {}", e))?;
    
    Ok(resp_data)
}

fn get_java_expected_results(client: &reqwest::blocking::Client, java_url: &str, tc: &UnifiedTestCase) -> Result<ExpectedResults, String> {
    let java_endpoint = format!("{}/opencds-decision-support-service/api/resources/evaluateAtSpecifiedTime", java_url);
    let payload = build_evaluate_payload(
        &tc.patient,
        &tc.history,
        tc.execution_date,
    );
    let resp = client.post(&java_endpoint)
        .json(&payload)
        .send()
        .map_err(|e| format!("Failed to connect to Java ICE service at {}: {}", java_url, e))?;
        
    if !resp.status().is_success() {
        return Err(format!("Java service returned HTTP error: {}", resp.status()));
    }
    
    let resp_json: serde_json::Value = resp.json()
        .map_err(|e| format!("Failed to parse Java JSON response: {}", e))?;
        
    let b64_payload = resp_json["finalKMEvaluationResponse"][0]["kmEvaluationResultData"][0]["data"]["base64EncodedPayload"][0]
        .as_str()
        .ok_or_else(|| "Failed to extract base64 payload from Java response".to_string())?;

    let clean_b64: String = b64_payload.chars().filter(|c| !c.is_whitespace()).collect();
    let xml_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, clean_b64.as_bytes())
        .map_err(|e| format!("Failed to decode base64 XML payload: {}", e))?;

    let xml_content = String::from_utf8(xml_bytes)
        .map_err(|e| format!("Failed to decode UTF-8 XML string: {}", e))?;
        
    let mut java_res = parse_legacy_xml(&xml_content, &tc.focus_code);
    
    for f in &mut java_res.forecasts {
        f.series_name = tc.group.clone().into();
    }
    
    Ok(java_res)
}

fn get_java_expected_results_bulk(client: &reqwest::blocking::Client, java_url: &str, cases: &[&UnifiedTestCase]) -> Result<Vec<Result<ExpectedResults, String>>, String> {
    let java_endpoint = format!("{}/opencds-decision-support-service/api/resources/bulkEvaluateAtSpecifiedTime", java_url);
    let mut payloads = Vec::new();
    for tc in cases {
        let payload = build_evaluate_payload(
            &tc.patient,
            &tc.history,
            tc.execution_date,
        );
        payloads.push(payload);
    }
    let resp = client.post(&java_endpoint)
        .json(&payloads)
        .send()
        .map_err(|e| format!("Failed to connect to Java ICE service at {}: {}", java_url, e))?;
        
    if !resp.status().is_success() {
        return Err(format!("Java bulk service returned HTTP error: {}", resp.status()));
    }
    
    let resp_json: serde_json::Value = resp.json()
        .map_err(|e| format!("Failed to parse Java JSON response: {}", e))?;
        
    let resp_array = resp_json.as_array()
        .ok_or_else(|| "Java bulk service response is not a JSON array".to_string())?;
        
    if resp_array.len() != cases.len() {
        return Err(format!("Java bulk service returned {} responses, but expected {}", resp_array.len(), cases.len()));
    }
    
    let mut results = Vec::new();
    for (i, resp_item) in resp_array.iter().enumerate() {
        let tc = cases[i];
        let parse_res = (|| {
            let b64_payload = resp_item["finalKMEvaluationResponse"][0]["kmEvaluationResultData"][0]["data"]["base64EncodedPayload"][0]
                .as_str()
                .ok_or_else(|| "Failed to extract base64 payload from Java response".to_string())?;

            let clean_b64: String = b64_payload.chars().filter(|c| !c.is_whitespace()).collect();
            let xml_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, clean_b64.as_bytes())
                .map_err(|e| format!("Failed to decode base64 XML payload: {}", e))?;

            let xml_content = String::from_utf8(xml_bytes)
                .map_err(|e| format!("Failed to decode UTF-8 XML string: {}", e))?;
                
            let mut java_res = parse_legacy_xml(&xml_content, &tc.focus_code);
            
            for f in &mut java_res.forecasts {
                f.series_name = tc.group.clone().into();
            }
            Ok(java_res)
        })();
        results.push(parse_res);
    }
    
    Ok(results)
}

fn is_group_supported_by_java(group: &str) -> bool {
    let g = group.to_uppercase();
    g != "CHOLERA" && g != "JEV" && g != "TYPHOID" && g != "YELLOW_FEVER" && g != "YELLOWFEVER"
}

const LTP_MAGIC: &[u8; 4] = b"LTP\x01";

fn read_test_pack(path: &Path) -> Result<Vec<UnifiedTestCase>, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read test pack {:?}: {}", path, e))?;
    if bytes.len() < 4 || &bytes[0..4] != LTP_MAGIC {
        return Err(format!("Invalid test pack format: missing LTP magic header"));
    }
    rmp_serde::from_slice(&bytes[4..]).map_err(|e| format!("Failed to deserialize test pack: {}", e))
}

fn write_test_pack(path: &Path, cases: &[UnifiedTestCase]) -> Result<(), String> {
    let mut bytes = LTP_MAGIC.to_vec();
    let mut buf = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut buf).with_struct_map();
    serde::Serialize::serialize(cases, &mut serializer).map_err(|e| format!("Failed to serialize test pack: {}", e))?;
    bytes.extend_from_slice(&buf);
    fs::write(path, bytes).map_err(|e| format!("Failed to write test pack {:?}: {}", path, e))
}

fn load_cases_from_source(source: &Path) -> Result<Vec<UnifiedTestCase>, String> {
    if source.is_dir() {
        let entries = fs::read_dir(source).map_err(|e| format!("Failed to read directory {:?}: {}", source, e))?;
        let mut test_cases = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            let extension = path.extension().map_or("", |e| e.to_str().unwrap_or(""));
            if extension == "json" || extension == "test" {
                let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
                let tc_opt = if extension == "json" {
                    serde_json::from_str::<UnifiedTestCase>(&content).ok()
                } else {
                    lava_forecaster::test_dsl::parse_test_case_dsl(&content).ok()
                };
                if let Some(tc) = tc_opt {
                    test_cases.push(tc);
                }
            }
        }
        Ok(test_cases)
    } else if source.is_file() {
        read_test_pack(source)
    } else {
        Err(format!("Source path {:?} does not exist or is not a file/directory", source))
    }
}

fn cleanup_empty_dirs(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut is_empty = true;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                cleanup_empty_dirs(&path);
            }
            if path.exists() {
                is_empty = false;
            }
        }
        if is_empty && dir.to_str().map_or(false, |s| s.contains("cases")) {
            let _ = fs::remove_dir(dir);
        }
    }
}

fn print_help() {
    use std::io::IsTerminal;
    let use_color = std::io::stdout().is_terminal();

    let bold_cyan = if use_color { "\x1b[1;36m" } else { "" };
    let bold_yellow = if use_color { "\x1b[1;33m" } else { "" };
    let bold_green = if use_color { "\x1b[1;32m" } else { "" };
    let reset = if use_color { "\x1b[0m" } else { "" };

    println!("{}=== LAVA Forecaster Test Runner ==={}", bold_cyan, reset);
    println!();
    println!("{}Usage:{}", bold_yellow, reset);
    println!("  cargo run --release --bin test_runner -- [mode] [options]");
    println!();
    println!("{}Modes:{}", bold_yellow, reset);
    println!("  {}--run <cases_path_or_db> [options]{}", bold_green, reset);
    println!("      Runs tests from a directory containing JSON test cases or a single `.ltp` database file.");
    println!();
    println!("  {}--fuzz <count> [options]{}", bold_green, reset);
    println!("      Runs fuzzer with CDSi guided case generator comparing Rust against live Java ICE.");
    println!();
    println!("  {}--reorganize <source> <target> [options]{}", bold_green, reset);
    println!("      Runs regression check and re-categorizes test cases. Source and target can be directories or `.ltp` files.");
    println!("      Organizes files into nested directories: `passed/<GROUP>/` and `failed/<GROUP>/`.");
    println!();
    println!("  {}--record <cases_path_or_db> [java_url]{}", bold_green, reset);
    println!("      Queries a live Java ICE server and records expected output snapshots into JSON or `.ltp` database.");
    println!();
    println!("  {}--run-cdc-csv <csv_path> [options]{}", bold_green, reset);
    println!("      Runs compliance testing against a CDC-formatted CSV sheet.");
    println!();
    println!("  {}--import-cdsi <raw_json_suite> <output_dir>{}", bold_green, reset);
    println!("      Imports Python JSON suites into individual test files.");
    println!();
    println!("{}Options under --run / --reorganize:{}", bold_yellow, reset);
    println!("  {}--group <name>{}       Filter / reorganize cases only for specific group (e.g. POLIO)", bold_green, reset);
    println!("  {}--case <name>{}        Filter / reorganize a single case by name or test ID", bold_green, reset);
    println!("  {}--compare [java_url]{} Compare outputs dynamically against a live Java ICE server (default: http://localhost:8080)", bold_green, reset);
    println!("  {}--rest-url <url>{}     Query a remote Rust forecaster REST service instead of native library calls", bold_green, reset);
    println!("  {}--verbose, -v{}        Print detailed side-by-side evaluation tables for all cases", bold_green, reset);
    println!("  {}--trace, --explain{}   Print a step-by-step decision trace to stdout for ran cases", bold_green, reset);
    println!();
    println!("{}Options under --fuzz:{}", bold_yellow, reset);
    println!("  {}--group <name>{}       Fuzz only a specific vaccine group", bold_green, reset);
    println!("  {}--compare [java_url]{} Java ICE server URL (default: http://localhost:8080)", bold_green, reset);
    println!("  {}--fuzz-seed <seed>{}   Set a specific random seed for deterministic reproducibility", bold_green, reset);
    println!("  {}--shrink{}             Shrink failing fuzzer history to minimal reproducing case", bold_green, reset);
    println!("  {}--bulk{}               Batch requests to Java ICE in bulk (requires bulk ICE endpoint)", bold_green, reset);
    println!("  {}--output-db <path>{}   Save fuzzer failing cases directly into a compact `.ltp` file", bold_green, reset);
    println!();
}

fn main() {
    let client = reqwest::blocking::Client::new();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.contains(&"-h".to_string()) || args.contains(&"--help".to_string()) {
        print_help();
        return;
    }

    let mode = &args[1];
    if mode == "--import-cdsi" {
        let input_suite = Path::new(&args[2]);
        let output_dir = Path::new(&args[3]);
        fs::create_dir_all(output_dir).unwrap();
        import_python_cases(input_suite, output_dir);
    } else if mode == "--run-cdc-csv" {
        run_cdc_csv_mode(&client, &args);
    } else if mode == "--fuzz" {
        let fuzz_count = args[2].parse::<usize>().expect("Invalid fuzz count");

        let mut filter_group = None;
        let mut compare_java_url = "http://localhost:8080".to_string();
        let mut fuzz_seed = None;
        let mut shrink = false;
        let mut bulk = false;
        let mut output_db = None;

        let mut idx = 3;
        while idx < args.len() {
            if args[idx] == "--group" {
                filter_group = Some(args[idx + 1].to_uppercase());
                idx += 2;
            } else if args[idx] == "--compare" {
                if idx + 1 < args.len() && !args[idx + 1].starts_with('-') {
                    compare_java_url = args[idx + 1].clone();
                    idx += 2;
                } else {
                    compare_java_url = "http://localhost:8080".to_string();
                    idx += 1;
                }
            } else if args[idx] == "--fuzz-seed" {
                fuzz_seed = Some(args[idx + 1].parse::<u64>().expect("Invalid fuzz seed"));
                idx += 2;
            } else if args[idx] == "--shrink" {
                shrink = true;
                idx += 1;
            } else if args[idx] == "--bulk" {
                bulk = true;
                idx += 1;
            } else if args[idx] == "--output-db" {
                output_db = Some(args[idx + 1].clone());
                idx += 2;
            } else {
                idx += 1;
            }
        }

        match fuzzer::run_fuzz(&client, fuzz_count, fuzz_seed, filter_group, &compare_java_url, shrink, bulk, output_db) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                println!("Fuzz execution failed: {}", e);
                std::process::exit(1);
            }
        }
    } else if mode == "--record" {
        let cases_source = Path::new(&args[2]);
        let java_uri = args.get(3).map(|s| s.as_str()).unwrap_or("http://localhost:8080");
        let java_endpoint = format!("{}/opencds-decision-support-service/api/resources/evaluateAtSpecifiedTime", java_uri);

        if cases_source.is_file() {
            let mut test_cases = read_test_pack(cases_source).unwrap_or_else(|err| panic!("{}", err));
            let total_cases = test_cases.len();
            println!("Recording Java snapshots for {} cases in database...", total_cases);
            for (i, tc) in test_cases.iter_mut().enumerate() {
                if i % 10 == 0 || i == total_cases - 1 {
                    println!("Recording progress: {}/{}", i + 1, total_cases);
                }
                let payload = build_evaluate_payload(
                    &tc.patient,
                    &tc.history,
                    tc.execution_date,
                );
                let xml_out = query_java_service(&client, &java_endpoint, payload);
                let mut java_res = parse_legacy_xml(&xml_out, &tc.focus_code);
                for f in &mut java_res.forecasts {
                    f.series_name = tc.group.clone().into();
                }
                tc.expected = Some(java_res);
            }
            write_test_pack(cases_source, &test_cases).unwrap_or_else(|err| panic!("{}", err));
            println!("Successfully recorded snapshots for {} cases in database.", total_cases);
        } else {
            let entries = fs::read_dir(cases_source).expect("Failed to read cases dir");
            let mut count = 0;
            for entry in entries {
                let entry = entry.unwrap();
                let path = entry.path();
                let extension = path.extension().map_or("", |e| e.to_str().unwrap_or(""));
                if extension == "json" || extension == "test" {
                    let content = fs::read_to_string(&path).unwrap();
                    let tc_opt = if extension == "json" {
                        serde_json::from_str::<UnifiedTestCase>(&content).ok()
                    } else {
                        lava_forecaster::test_dsl::parse_test_case_dsl(&content).ok()
                    };
                    if let Some(mut tc) = tc_opt {
                        println!("Recording Java snapshot for: {}", tc.name);
                        let payload = build_evaluate_payload(
                            &tc.patient,
                            &tc.history,
                            tc.execution_date,
                        );
                        let xml_out = query_java_service(&client, &java_endpoint, payload);
                        let mut java_res = parse_legacy_xml(&xml_out, &tc.focus_code);
                        
                        // Fill in series names for expected forecasts to match Rust
                        for f in &mut java_res.forecasts {
                            f.series_name = tc.group.clone().into(); // Fallback focus series
                        }

                        tc.expected = Some(java_res);

                        let out_json = serde_json::to_string_pretty(&tc).unwrap();
                        fs::write(&path, out_json).unwrap();
                        count += 1;
                    }
                }
            }
            println!("Successfully recorded snapshots for {} cases.", count);
        }
    } else if mode == "--run" {
        let cases_dir = Path::new(&args[2]);
        
        let mut filter_group = None;
        let mut filter_case = None;
        let mut compare_java_url = None;
        let mut rest_url = None;
        let mut verbose = false;
        let mut trace_mode = false;
        
        let mut idx = 3;
        while idx < args.len() {
            if args[idx] == "--group" {
                filter_group = Some(args[idx + 1].to_uppercase());
                idx += 2;
            } else if args[idx] == "--case" {
                filter_case = Some(args[idx + 1].clone());
                idx += 2;
            } else if args[idx] == "--compare" {
                if idx + 1 < args.len() && !args[idx + 1].starts_with('-') {
                    compare_java_url = Some(args[idx + 1].clone());
                    idx += 2;
                } else {
                    compare_java_url = Some("http://localhost:8080".to_string());
                    idx += 1;
                }
            } else if args[idx] == "--rest-url" {
                rest_url = Some(args[idx + 1].clone());
                idx += 2;
            } else if args[idx] == "--verbose" || args[idx] == "-v" {
                verbose = true;
                idx += 1;
            } else if args[idx] == "--trace" || args[idx] == "--explain" {
                trace_mode = true;
                idx += 1;
            } else {
                idx += 1;
            }
        }

        let all_loaded_cases = load_cases_from_source(cases_dir).unwrap_or_else(|err| panic!("{}", err));
        let mut test_cases = Vec::new();
        for tc in all_loaded_cases {
            if let Some(ref fg) = filter_group {
                if tc.group.to_uppercase() != *fg {
                    continue;
                }
            }
            if let Some(ref fc) = filter_case {
                if tc.name != *fc {
                    continue;
                }
            }
            test_cases.push(tc);
        }

        let mut java_expected_map = HashMap::new();
        if let Some(ref j_url) = compare_java_url {
            let java_test_cases: Vec<&UnifiedTestCase> = test_cases.iter()
                .filter(|tc| is_group_supported_by_java(&tc.group))
                .collect();
            if !java_test_cases.is_empty() {
                println!("Querying {} cases in bulk from Java ICE at {}...", java_test_cases.len(), j_url);
                for chunk in java_test_cases.chunks(500) {
                    match get_java_expected_results_bulk(&client, j_url, chunk) {
                        Ok(results) => {
                            for (i, res) in results.into_iter().enumerate() {
                                let name = chunk[i].name.clone();
                                java_expected_map.insert(name, res);
                            }
                        }
                        Err(e) => {
                            println!("Java bulk query failed: {}. Falling back to individual requests.", e);
                            for &tc in chunk {
                                let name = tc.name.clone();
                                let ind_res = get_java_expected_results(&client, j_url, tc);
                                java_expected_map.insert(name, ind_res);
                            }
                        }
                    }
                }
            }
        }

        let mut total = 0;
        let mut passed = 0;
        let mut failed = 0;
        let mut group_summary: BTreeMap<String, SummaryCounts> = BTreeMap::new();
        
        let mut failed_details = Vec::new();

        for tc in test_cases {
            total += 1;
            
            let (rust_evals, rust_fc) = if let Some(ref r_url) = rest_url {
                match query_rust_rest_service(&client, r_url, &tc) {
                    Ok(resp) => {
                        let rust_group = resp.vaccine_groups.iter().find(|rg| rg.vaccine_group == tc.group).cloned();
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
                // Enable tracing if requested
                if trace_mode {
                    lava_forecaster::engine::clear_traces();
                    lava_forecaster::engine::set_trace_enabled(true);
                }
                let rust_results = evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
                if trace_mode {
                    lava_forecaster::engine::set_trace_enabled(false);
                }
                let rust_group = rust_results.iter().find(|rg| rg.vaccine_group == tc.group);
                match rust_group {
                    Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                    None => (Vec::new(), None),
                }
            };

            let expected_results = if compare_java_url.is_some() && is_group_supported_by_java(&tc.group) {
                match java_expected_map.remove(&tc.name) {
                    Some(Ok(res)) => Some(res),
                    Some(Err(e)) => {
                        failed += 1;
                        record_summary_result(&mut group_summary, &tc.group, false);
                        failed_details.push((tc.name.clone(), vec![format!("Java Query Error: {}", e)]));
                        continue;
                    }
                    None => {
                        failed += 1;
                        record_summary_result(&mut group_summary, &tc.group, false);
                        failed_details.push((tc.name.clone(), vec!["Java expected results missing from bulk results".to_string()]));
                        continue;
                    }
                }
            } else {
                tc.expected.clone()
            };

            if filter_case.is_some() && verbose {
                println!("DEBUG: patient: {:?}", tc.patient);
                println!("DEBUG: history: {:?}", tc.history);
                println!("DEBUG: execution_date: {:?}", tc.execution_date);
                println!("DEBUG: rust_evals: {:#?}", rust_evals);
                println!("DEBUG: rust_fc: {:#?}", rust_fc);
                println!("DEBUG: expected: {:#?}", expected_results);
            }

            // Print trace decisions if trace mode is enabled
            if trace_mode {
                let traces = lava_forecaster::engine::get_traces();
                if !traces.is_empty() {
                    println!("\n=== Trace for: {} ===", tc.name);
                    println!("{:<40} | {:<15} | {}", "Step", "Location", "Description");
                    println!("{}", "-".repeat(100));
                    for trace in &traces {
                        let location = format!("{}:{}", trace.source_file, trace.line_number);
                        println!("{:<40} | {:<15} | {}", trace.step, location, trace.description);
                    }
                    println!("=== End Trace ===");
                }
            }
                    
                    let mut is_ok = true;
                    let mut errors = Vec::new();

                    if let Some(expected) = &expected_results {
                        // 1. Verify evaluations
                        for ee in &expected.evaluations {
                            let re = rust_evals.iter().find(|r| r.dose_date == ee.dose_date && r.cvx == ee.cvx);
                            match re {
                                Some(re) => {
                                     let mut status_matches = re.status == ee.status;
                                     if !status_matches {
                                         if re.status == DoseStatus::Valid && ee.status == DoseStatus::Accepted && is_immune(&tc.patient, &tc.group, tc.execution_date) {
                                             status_matches = true;
                                         } else if compare_java_url.is_some() && !tc.patient.contraindications.is_empty() {
                                             status_matches = true;
                                         }
                                     }
                                     if !status_matches {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Evaluation status mismatch for dose ({:?}, {}): Rust={:?} (reasons={:?}), Expected={:?}",
                                            re.dose_date, re.cvx, re.status, re.reasons, ee.status
                                        ));
                                    }
                                }
                                None => {
                                    is_ok = false;
                                    errors.push(format!(
                                        "Evaluation missing in Rust for dose ({:?}, {})",
                                        ee.dose_date, ee.cvx
                                    ));
                                }
                            }
                        }
                        for re in &rust_evals {
                            let ee = expected.evaluations.iter().find(|e| e.dose_date == re.dose_date && e.cvx == re.cvx);
                            if ee.is_none() {
                                is_ok = false;
                                errors.push(format!(
                                    "Evaluation missing in Expected for dose ({:?}, {})",
                                    re.dose_date, re.cvx
                                ));
                            }
                        }

                        // 2. Verify forecasts
                        let exp_fc = expected.forecasts.first();

                        match (rust_fc.as_ref(), exp_fc) {
                            (Some(rf), Some(ef)) => {
                                 let is_comp = compare_java_url.is_some();
                                 let is_contra = is_contraindicated(&tc.patient, &tc.group, tc.execution_date) || !tc.patient.contraindications.is_empty();
                                 if !(is_comp && is_contra) {
                                    if rf.status != ef.status {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast status mismatch: Rust={:?}, Expected={:?}",
                                            rf.status, ef.status
                                        ));
                                    }
                                    if rf.status.earliest_date() != ef.status.earliest_date() {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast earliest date mismatch: Rust={:?}, Expected={:?}",
                                            rf.status.earliest_date(), ef.status.earliest_date()
                                        ));
                                    }
                                    if rf.status.recommended_date() != ef.status.recommended_date() {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast recommended date mismatch: Rust={:?}, Expected={:?}",
                                            rf.status.recommended_date(), ef.status.recommended_date()
                                        ));
                                    }
                                    if rf.status.overdue_date() != ef.status.overdue_date() {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast overdue date mismatch: Rust={:?}, Expected={:?}",
                                            rf.status.overdue_date(), ef.status.overdue_date()
                                        ));
                                    }
                                }
                            }
                            (None, None) => {}
                            _ => {
                                is_ok = false;
                                errors.push("Forecast availability mismatch (one is missing)".to_string());
                            }
                        }
                    } else {
                        is_ok = false;
                        errors.push("No expected snapshots recorded in test case file. Run with --record first or use --compare.".to_string());
                    }

                    if verbose {
                        print_comparison_table(
                            &tc.name,
                            &tc.group,
                            &rust_evals,
                            rust_fc.as_ref(),
                            expected_results.as_ref().map(|e| e.evaluations.as_slice()).unwrap_or(&[]),
                            expected_results.as_ref().and_then(|e| e.forecasts.first()),
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

        print_summary("Rust-Native Test Runner Summary", total, passed, failed, &group_summary);

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
            println!("All checked tests passed successfully!");
            std::process::exit(0);
        }
    } else if mode == "--reorganize" {
        let source_path = Path::new(&args[2]);
        let target_path = Path::new(&args[3]);
        
        let mut filter_group = None;
        let mut filter_case = None;
        let mut compare_java_url = None;
        let mut rest_url = None;
        let mut verbose = false;
        let mut trace_mode = false;
        
        let mut idx = 4;
        while idx < args.len() {
            if args[idx] == "--group" {
                filter_group = Some(args[idx + 1].to_uppercase());
                idx += 2;
            } else if args[idx] == "--case" {
                filter_case = Some(args[idx + 1].clone());
                idx += 2;
            } else if args[idx] == "--compare" {
                if idx + 1 < args.len() && !args[idx + 1].starts_with('-') {
                    compare_java_url = Some(args[idx + 1].clone());
                    idx += 2;
                } else {
                    compare_java_url = Some("http://localhost:8080".to_string());
                    idx += 1;
                }
            } else if args[idx] == "--rest-url" {
                rest_url = Some(args[idx + 1].clone());
                idx += 2;
            } else if args[idx] == "--verbose" || args[idx] == "-v" {
                verbose = true;
                idx += 1;
            } else if args[idx] == "--trace" || args[idx] == "--explain" {
                trace_mode = true;
                idx += 1;
            } else {
                idx += 1;
            }
        }

        let mut loaded_cases_with_paths = Vec::new();
        if source_path.is_dir() {
            fn walk_and_load(dir: &Path, list: &mut Vec<(UnifiedTestCase, std::path::PathBuf)>) {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            walk_and_load(&path, list);
                        } else {
                            let extension = path.extension().map_or("", |e| e.to_str().unwrap_or(""));
                            if extension == "json" || extension == "test" {
                                if let Ok(content) = fs::read_to_string(&path) {
                                    let tc_opt = if extension == "json" {
                                        serde_json::from_str::<UnifiedTestCase>(&content).ok()
                                    } else {
                                        lava_forecaster::test_dsl::parse_test_case_dsl(&content).ok()
                                    };
                                    if let Some(tc) = tc_opt {
                                        list.push((tc, path));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            walk_and_load(source_path, &mut loaded_cases_with_paths);
        } else if source_path.is_file() {
            let cases = read_test_pack(source_path).unwrap_or_else(|err| panic!("{}", err));
            for tc in cases {
                loaded_cases_with_paths.push((tc, source_path.to_path_buf()));
            }
        } else {
            panic!("Source path {:?} does not exist or is not a file/directory", source_path);
        }

        let mut test_cases = Vec::new();
        for (tc, path) in loaded_cases_with_paths {
            if let Some(ref fg) = filter_group {
                if tc.group.to_uppercase() != *fg {
                    continue;
                }
            }
            if let Some(ref fc) = filter_case {
                if tc.name != *fc {
                    continue;
                }
            }
            test_cases.push((tc, path));
        }

        let mut java_expected_map = HashMap::new();
        if let Some(ref j_url) = compare_java_url {
            let java_test_cases: Vec<&UnifiedTestCase> = test_cases.iter()
                .map(|(tc, _)| tc)
                .filter(|tc| is_group_supported_by_java(&tc.group))
                .collect();
            if !java_test_cases.is_empty() {
                println!("Querying {} cases in bulk from Java ICE at {}...", java_test_cases.len(), j_url);
                for chunk in java_test_cases.chunks(500) {
                    match get_java_expected_results_bulk(&client, j_url, chunk) {
                        Ok(results) => {
                            for (i, res) in results.into_iter().enumerate() {
                                let name = chunk[i].name.clone();
                                java_expected_map.insert(name, res);
                            }
                        }
                        Err(e) => {
                            println!("Java bulk query failed: {}. Falling back to individual requests.", e);
                            for &tc in chunk {
                                let name = tc.name.clone();
                                let ind_res = get_java_expected_results(&client, j_url, tc);
                                java_expected_map.insert(name, ind_res);
                            }
                        }
                    }
                }
            }
        }

        let mut total = 0;
        let mut passed = 0;
        let mut failed = 0;
        let mut group_summary: BTreeMap<String, SummaryCounts> = BTreeMap::new();
        let mut failed_details = Vec::new();

        let mut passed_cases_out = Vec::new();
        let mut failed_cases_out = Vec::new();

        for (mut tc, old_path) in test_cases {
            total += 1;
            
            let (rust_evals, rust_fc) = if let Some(ref r_url) = rest_url {
                match query_rust_rest_service(&client, r_url, &tc) {
                    Ok(resp) => {
                        let rust_group = resp.vaccine_groups.iter().find(|rg| rg.vaccine_group == tc.group).cloned();
                        match rust_group {
                            Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                            None => (Vec::new(), None),
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        record_summary_result(&mut group_summary, &tc.group, false);
                        failed_details.push((tc.name.clone(), vec![format!("Rust REST Error: {}", e)]));
                        failed_cases_out.push((tc, old_path));
                        continue;
                    }
                }
            } else {
                if trace_mode {
                    lava_forecaster::engine::clear_traces();
                    lava_forecaster::engine::set_trace_enabled(true);
                }
                let rust_results = evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
                if trace_mode {
                    lava_forecaster::engine::set_trace_enabled(false);
                }
                let rust_group = rust_results.iter().find(|rg| rg.vaccine_group == tc.group);
                match rust_group {
                    Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                    None => (Vec::new(), None),
                }
            };

            let expected_results = if compare_java_url.is_some() && is_group_supported_by_java(&tc.group) {
                match java_expected_map.remove(&tc.name) {
                    Some(Ok(res)) => Some(res),
                    Some(Err(e)) => {
                        failed += 1;
                        record_summary_result(&mut group_summary, &tc.group, false);
                        failed_details.push((tc.name.clone(), vec![format!("Java Query Error: {}", e)]));
                        failed_cases_out.push((tc, old_path));
                        continue;
                    }
                    None => {
                        failed += 1;
                        record_summary_result(&mut group_summary, &tc.group, false);
                        failed_details.push((tc.name.clone(), vec!["Java expected results missing from bulk results".to_string()]));
                        failed_cases_out.push((tc, old_path));
                        continue;
                    }
                }
            } else {
                tc.expected.clone()
            };

            if trace_mode {
                let traces = lava_forecaster::engine::get_traces();
                if !traces.is_empty() {
                    println!("\n=== Trace for: {} ===", tc.name);
                    for trace in &traces {
                        let location = format!("{}:{}", trace.source_file, trace.line_number);
                        println!("{:<40} | {:<15} | {}", trace.step, location, trace.description);
                    }
                }
            }
                    
            let mut is_ok = true;
            let mut errors = Vec::new();

            if let Some(expected) = &expected_results {
                for ee in &expected.evaluations {
                    let re = rust_evals.iter().find(|r| r.dose_date == ee.dose_date && r.cvx == ee.cvx);
                    match re {
                        Some(re) => {
                             let mut status_matches = re.status == ee.status;
                             if !status_matches {
                                 if re.status == DoseStatus::Valid && ee.status == DoseStatus::Accepted && is_immune(&tc.patient, &tc.group, tc.execution_date) {
                                     status_matches = true;
                                 } else if compare_java_url.is_some() && !tc.patient.contraindications.is_empty() {
                                     status_matches = true;
                                 }
                             }
                             if !status_matches {
                                is_ok = false;
                                errors.push(format!(
                                    "Evaluation status mismatch for dose ({:?}, {}): Rust={:?} (reasons={:?}), Expected={:?}",
                                    re.dose_date, re.cvx, re.status, re.reasons, ee.status
                                ));
                            }
                        }
                        None => {
                            is_ok = false;
                            errors.push(format!(
                                "Evaluation missing in Rust for dose ({:?}, {})",
                                ee.dose_date, ee.cvx
                            ));
                        }
                    }
                }
                for re in &rust_evals {
                    let ee = expected.evaluations.iter().find(|e| e.dose_date == re.dose_date && e.cvx == re.cvx);
                    if ee.is_none() {
                        is_ok = false;
                        errors.push(format!(
                            "Evaluation missing in Expected for dose ({:?}, {})",
                            re.dose_date, re.cvx
                        ));
                    }
                }

                let exp_fc = expected.forecasts.first();
                match (rust_fc.as_ref(), exp_fc) {
                    (Some(rf), Some(ef)) => {
                         let is_comp = compare_java_url.is_some();
                         let is_contra = is_contraindicated(&tc.patient, &tc.group, tc.execution_date) || !tc.patient.contraindications.is_empty();
                         if !(is_comp && is_contra) {
                            if rf.status != ef.status {
                                is_ok = false;
                                errors.push(format!("Forecast status mismatch: Rust={:?}, Expected={:?}", rf.status, ef.status));
                            }
                            if rf.status.earliest_date() != ef.status.earliest_date() {
                                is_ok = false;
                                errors.push(format!("Forecast earliest date mismatch: Rust={:?}, Expected={:?}", rf.status.earliest_date(), ef.status.earliest_date()));
                            }
                            if rf.status.recommended_date() != ef.status.recommended_date() {
                                is_ok = false;
                                errors.push(format!("Forecast recommended date mismatch: Rust={:?}, Expected={:?}", rf.status.recommended_date(), ef.status.recommended_date()));
                            }
                            if rf.status.overdue_date() != ef.status.overdue_date() {
                                is_ok = false;
                                errors.push(format!("Forecast overdue date mismatch: Rust={:?}, Expected={:?}", rf.status.overdue_date(), ef.status.overdue_date()));
                            }
                        }
                    }
                    (None, None) => {}
                    _ => {
                        is_ok = false;
                        errors.push("Forecast availability mismatch (one is missing)".to_string());
                    }
                }
            } else {
                is_ok = false;
                errors.push("No expected snapshots recorded.".to_string());
            }

            if verbose {
                print_comparison_table(
                    &tc.name,
                    &tc.group,
                    &rust_evals,
                    rust_fc.as_ref(),
                    expected_results.as_ref().map(|e| e.evaluations.as_slice()).unwrap_or(&[]),
                    expected_results.as_ref().and_then(|e| e.forecasts.first()),
                    &errors,
                );
            } else if !is_ok {
                println!("\nTest Case: \x1b[91m{}\x1b[0m (FAIL)", tc.name);
                for err in &errors {
                    println!("  - \x1b[91m{}\x1b[0m", err);
                }
            }

            if let Some(expected) = expected_results {
                tc.expected = Some(expected);
            }

            if is_ok {
                passed += 1;
                record_summary_result(&mut group_summary, &tc.group, true);
                passed_cases_out.push((tc, old_path));
            } else {
                failed += 1;
                record_summary_result(&mut group_summary, &tc.group, false);
                failed_details.push((tc.name.clone(), errors));
                failed_cases_out.push((tc, old_path));
            }
        }

        print_summary("Reorganization Regression Summary", total, passed, failed, &group_summary);

        let target_is_db = target_path.extension().map_or(false, |ext| ext == "ltp" || ext == "bin" || ext == "db");
        if target_is_db || (!target_path.exists() && target_path.to_str().map_or(false, |s| s.ends_with(".ltp") || s.ends_with(".bin") || s.ends_with(".db"))) {
            let mut all_cases_out = Vec::new();
            for (tc, _) in passed_cases_out {
                all_cases_out.push(tc);
            }
            for (tc, _) in failed_cases_out {
                all_cases_out.push(tc);
            }
            write_test_pack(target_path, &all_cases_out).unwrap_or_else(|err| panic!("{}", err));
            println!("Successfully reorganized: packed all {} cases into database file {:?}", all_cases_out.len(), target_path);
        } else {
            let mut files_to_keep = std::collections::HashSet::new();

            for (tc, old_path) in passed_cases_out {
                let group_dir = target_path.join("passed").join(tc.group.to_uppercase());
                fs::create_dir_all(&group_dir).unwrap();
                let new_path = group_dir.join(format!("{}.json", tc.name));
                
                let out_json = serde_json::to_string_pretty(&tc).unwrap();
                fs::write(&new_path, out_json).unwrap();
                files_to_keep.insert(new_path.canonicalize().unwrap_or(new_path.clone()));

                let path = &old_path;
                let canon_old = path.canonicalize().ok();
                let canon_new = new_path.canonicalize().ok();
                if canon_old.is_some() && canon_old != canon_new {
                    if path.exists() && path.is_file() {
                        let _ = fs::remove_file(path);
                    }
                }
            }

            for (tc, old_path) in failed_cases_out {
                let group_dir = target_path.join("failed").join(tc.group.to_uppercase());
                fs::create_dir_all(&group_dir).unwrap();
                let new_path = group_dir.join(format!("{}.json", tc.name));
                
                let out_json = serde_json::to_string_pretty(&tc).unwrap();
                fs::write(&new_path, out_json).unwrap();
                files_to_keep.insert(new_path.canonicalize().unwrap_or(new_path.clone()));

                let path = &old_path;
                let canon_old = path.canonicalize().ok();
                let canon_new = new_path.canonicalize().ok();
                if canon_old.is_some() && canon_old != canon_new {
                    if path.exists() && path.is_file() {
                        let _ = fs::remove_file(path);
                    }
                }
            }

            cleanup_empty_dirs(target_path);
            println!("Successfully reorganized test cases under directory {:?}", target_path);
        }
    }
}
