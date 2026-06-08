use std::collections::BTreeMap;
use chrono::NaiveDate;
use lava_forecaster::engine::is_eval_ignored;
use lava_forecaster::models::{DoseEvaluation, DoseStatus, SeriesForecast, Cvx};

#[derive(Debug, Default, Clone)]
pub struct SummaryCounts {
    pub executed: usize,
    pub passed: usize,
    pub failed: usize,
}

pub fn record_summary_result(summary: &mut BTreeMap<String, SummaryCounts>, group: &str, passed: bool) {
    let entry = summary.entry(group.to_string()).or_default();
    entry.executed += 1;
    if passed {
        entry.passed += 1;
    } else {
        entry.failed += 1;
    }
}

pub fn print_summary(title: &str, total: usize, passed: usize, failed: usize, group_summary: &BTreeMap<String, SummaryCounts>) {
    println!("
--- {} ---", title);
    println!("Executed: {}", total);
    println!("Passed  : {}", passed);
    println!("Failed  : {}", failed);
    if total > 0 {
        println!("Pass %  : {:.2}", passed as f64 / total as f64 * 100.0);
    }

    if !group_summary.is_empty() {
        println!("
Per-Group Summary:");
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

pub fn print_comparison_table(
    tc_name: &str,
    group: &str,
    rust_evals: &[DoseEvaluation],
    rust_fc: Option<&SeriesForecast>,
    exp_evals: &[DoseEvaluation],
    exp_fc: Option<&SeriesForecast>,
    errors: &[String],
) {
    if !errors.is_empty() {
        println!("
Test Case: \x1b[91m{}\x1b[0m (FAIL)", tc_name);
        for err in errors {
            println!("{}", err);
        }
    } else {
        println!("
Test Case: \x1b[92m{}\x1b[0m (PASS)", tc_name);
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

    if rust_fc.is_some() || exp_fc.is_some() {
        println!("
Forecasts:");
        println!("{:<15} | {:<45} | {:<22}", "Field", "Rust Forecast", "Expected Forecast");
        println!("{}", "-".repeat(90));

        let rust_forecast = rust_fc.unwrap();
        let expected_forecast = exp_fc.unwrap();

        let fields = ["status", "earliest_date", "recommended_date", "overdue_date", "latest_date"];
        for field in &fields {
            let field_name = *field;
            let rust_val = get_field_val(rust_forecast, field_name);
            let expected_val = get_field_val(expected_forecast, field_name);
            
            if rust_val != expected_val {
                 let source = rust_forecast.sources.get(field_name).map(|s| format!("(Source: {})", s)).unwrap_or_default();
                 println!(
                     "    - \x1b[91m{}\x1b[0m: Rust={:?}, Expected={:?} \x1b[93m{}\x1b[0m",
                     field_name,
                     rust_val,
                     expected_val,
                     source
                 );
            }
        }
    }
    println!();
}

fn get_field_val<'a>(forecast: &'a SeriesForecast, field: &str) -> String {
    match field {
        "status" => format!("{:?}", forecast.status),
        "earliest_date" => forecast.status.earliest_date().map_or("-".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        "recommended_date" => forecast.status.recommended_date().map_or("-".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        "overdue_date" => forecast.status.overdue_date().map_or("-".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        "latest_date" => forecast.status.latest_date().map_or("-".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        _ => "".to_string(),
    }
}
