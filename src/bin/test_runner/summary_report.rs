use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::NaiveDate;
use lava_forecaster::models::{
    DoseEvaluation, DoseStatus, ExpectedResults, SeriesForecast, UnifiedTestCase,
};
use lava_forecaster::rules::covid19::state::CovidEvaluatedState;
use serde::{Deserialize, Serialize};

use crate::eval_match::pair_evaluations_by_occurrence;
use crate::java_client::{is_contraindicated, is_immune};

#[derive(Debug, Serialize, Deserialize)]
pub struct RunSummaryReport {
    pub input: String,
    pub group_filter: Option<String>,
    pub case_filter: Option<String>,
    pub compare: bool,
    pub rest: bool,
    pub executed: usize,
    pub passed: usize,
    pub failed: usize,
    pub cases: Vec<CaseSummaryReport>,
}

impl RunSummaryReport {
    pub fn new(
        input: &Path,
        group_filter: Option<String>,
        case_filter: Option<String>,
        compare: bool,
        rest: bool,
    ) -> Self {
        Self {
            input: input.display().to_string(),
            group_filter,
            case_filter,
            compare,
            rest,
            executed: 0,
            passed: 0,
            failed: 0,
            cases: Vec::new(),
        }
    }

    pub fn record_case(&mut self, case: CaseSummaryReport) {
        self.executed += 1;
        if case.passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.cases.push(case);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaseSummaryReport {
    pub name: String,
    pub group: String,
    pub passed: bool,
    pub dose_count: usize,
    pub cvx_codes: Vec<u16>,
    pub has_same_day_doses: bool,
    pub evaluation_mismatches: Vec<EvaluationMismatchReport>,
    pub forecast_mismatches: Vec<ForecastMismatchReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub covid19_semantics: Option<Covid19SemanticReport>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationMismatchReport {
    pub dose_date: Option<NaiveDate>,
    pub cvx: Option<u16>,
    pub field: String,
    pub rust: Option<String>,
    pub expected: Option<String>,
    pub transition: Option<String>,
    pub same_day_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForecastMismatchReport {
    pub field: String,
    pub rust: Option<String>,
    pub expected: Option<String>,
    pub delta_days: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Covid19SemanticReport {
    pub selected_policy: String,
    pub selected_policy_ice_name: String,
    pub policy_season: String,
    pub policy_age_band: String,
    pub policy_interval_anchor: String,
    pub policy_forecast_anchor: String,
    pub current_season_dose_count: usize,
    pub supported_dose_count: usize,
    pub dose_facts: Vec<Covid19DoseFactReport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Covid19DoseFactReport {
    pub date: NaiveDate,
    pub cvx: u16,
    pub season: String,
    pub current_season: bool,
    pub product_family: String,
    pub relationship_to_selected_series: String,
    pub supported_by_java_covid: bool,
    pub age_at_dose_under_2_years: bool,
    pub age_at_dose_under_5_years: bool,
    pub age_at_dose_at_least_65_years: bool,
    pub rust_status: Option<String>,
    pub rust_target_dose: Option<usize>,
}

pub fn build_case_summary(
    tc: &UnifiedTestCase,
    passed: bool,
    rust_evals: &[DoseEvaluation],
    rust_fc: Option<&SeriesForecast>,
    expected: Option<&ExpectedResults>,
    errors: &[String],
    compare_java: bool,
) -> CaseSummaryReport {
    let mut cvx_codes = tc.history.iter().map(|dose| dose.cvx.0).collect::<Vec<_>>();
    cvx_codes.sort_unstable();
    cvx_codes.dedup();

    let same_day_counts = same_day_counts(tc);
    let has_same_day_doses = same_day_counts.values().any(|count| *count > 1);

    let (evaluation_mismatches, forecast_mismatches) = if passed {
        (Vec::new(), Vec::new())
    } else if let Some(expected) = expected {
        (
            collect_evaluation_mismatches(
                tc,
                rust_evals,
                &expected.evaluations,
                &same_day_counts,
                compare_java,
            ),
            collect_forecast_mismatches(tc, rust_fc, expected.forecasts.first(), compare_java),
        )
    } else {
        (Vec::new(), Vec::new())
    };

    CaseSummaryReport {
        name: tc.name.clone(),
        group: tc.group.clone(),
        passed,
        dose_count: tc.history.len(),
        cvx_codes,
        has_same_day_doses,
        evaluation_mismatches,
        forecast_mismatches,
        covid19_semantics: build_covid19_semantics(tc, rust_evals),
        errors: errors.to_vec(),
    }
}

fn build_covid19_semantics(
    tc: &UnifiedTestCase,
    rust_evals: &[DoseEvaluation],
) -> Option<Covid19SemanticReport> {
    if !tc.group.eq_ignore_ascii_case("COVID19") {
        return None;
    }

    let state = CovidEvaluatedState::from_aug2025_policy(
        &tc.patient,
        &tc.history,
        tc.execution_date,
        rust_evals,
    );
    let selected_policy = state.selected_series;
    let dose_facts = state
        .dose_facts
        .iter()
        .map(|fact| {
            let evaluation = state.evaluation_for_fact(fact);
            Covid19DoseFactReport {
                date: fact.raw.date,
                cvx: fact.raw.cvx.0,
                season: fact.season.ice_key().to_string(),
                current_season: fact.season == selected_policy.season,
                product_family: format!("{:?}", fact.product.family),
                relationship_to_selected_series: format!(
                    "{:?}",
                    state.relationship_for_fact(*fact)
                ),
                supported_by_java_covid: fact.supported_by_java_covid,
                age_at_dose_under_2_years: fact.age_at_dose.under_2_years,
                age_at_dose_under_5_years: fact.age_at_dose.under_5_years,
                age_at_dose_at_least_65_years: fact.age_at_dose.at_least_65_years,
                rust_status: evaluation.map(|eval| format!("{:?}", eval.status)),
                rust_target_dose: evaluation.and_then(|eval| eval.dose_number),
            }
        })
        .collect();

    Some(Covid19SemanticReport {
        selected_policy: format!("{:?}", selected_policy.id),
        selected_policy_ice_name: selected_policy.ice_name.to_string(),
        policy_season: selected_policy.season.ice_key().to_string(),
        policy_age_band: format!("{:?}", selected_policy.age_band),
        policy_interval_anchor: format!("{:?}", selected_policy.intervals.anchor),
        policy_forecast_anchor: format!("{:?}", selected_policy.forecast.anchor),
        current_season_dose_count: state.current_season_dose_count(),
        supported_dose_count: state.supported_dose_count(),
        dose_facts,
    })
}

pub fn write_run_summary(path: &Path, report: &RunSummaryReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("Failed to create summary directory {:?}: {}", parent, err)
            })?;
        }
    }
    let json = serde_json::to_string_pretty(report)
        .map_err(|err| format!("Failed to serialize summary report: {}", err))?;
    fs::write(path, json).map_err(|err| format!("Failed to write summary {:?}: {}", path, err))
}

pub fn summarize_file(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read summary {:?}: {}", path, err))?;
    let report: RunSummaryReport = serde_json::from_str(&content)
        .map_err(|err| format!("Failed to parse summary {:?}: {}", path, err))?;
    print_summary_report(&report);
    Ok(())
}

fn collect_evaluation_mismatches(
    tc: &UnifiedTestCase,
    rust_evals: &[DoseEvaluation],
    expected_evals: &[DoseEvaluation],
    same_day_counts: &BTreeMap<NaiveDate, usize>,
    compare_java: bool,
) -> Vec<EvaluationMismatchReport> {
    let mut mismatches = Vec::new();
    for ((date, cvx), rust, expected) in pair_evaluations_by_occurrence(rust_evals, expected_evals)
    {
        match (rust, expected) {
            (Some(rust), Some(expected)) => {
                let ignored_status_mismatch = rust.status != expected.status
                    && ((rust.status == DoseStatus::Valid
                        && expected.status == DoseStatus::Accepted
                        && is_immune(&tc.patient, &tc.group, tc.execution_date))
                        || (compare_java && !tc.patient.contraindications.is_empty()));
                if rust.status != expected.status && !ignored_status_mismatch {
                    let rust_status = format!("{:?}", rust.status);
                    let expected_status = format!("{:?}", expected.status);
                    mismatches.push(EvaluationMismatchReport {
                        dose_date: Some(date),
                        cvx: Some(cvx),
                        field: "status".to_string(),
                        transition: Some(format!("{} -> {}", rust_status, expected_status)),
                        rust: Some(rust_status),
                        expected: Some(expected_status),
                        same_day_count: *same_day_counts.get(&date).unwrap_or(&1),
                    });
                }
                if expected.dose_number.is_some() && rust.dose_number != expected.dose_number {
                    let rust_dose = rust.dose_number.map(|n| n.to_string());
                    let expected_dose = expected.dose_number.map(|n| n.to_string());
                    mismatches.push(EvaluationMismatchReport {
                        dose_date: Some(date),
                        cvx: Some(cvx),
                        field: "dose_number".to_string(),
                        transition: None,
                        rust: rust_dose,
                        expected: expected_dose,
                        same_day_count: *same_day_counts.get(&date).unwrap_or(&1),
                    });
                }
            }
            (None, Some(expected)) => {
                mismatches.push(EvaluationMismatchReport {
                    dose_date: Some(date),
                    cvx: Some(cvx),
                    field: "missing_in_rust".to_string(),
                    transition: None,
                    rust: None,
                    expected: Some(format!("{:?}", expected.status)),
                    same_day_count: *same_day_counts.get(&date).unwrap_or(&1),
                });
            }
            (Some(rust), None) => {
                mismatches.push(EvaluationMismatchReport {
                    dose_date: Some(date),
                    cvx: Some(cvx),
                    field: "missing_in_expected".to_string(),
                    transition: None,
                    rust: Some(format!("{:?}", rust.status)),
                    expected: None,
                    same_day_count: *same_day_counts.get(&date).unwrap_or(&1),
                });
            }
            (None, None) => {}
        }
    }
    mismatches
}

fn collect_forecast_mismatches(
    tc: &UnifiedTestCase,
    rust_fc: Option<&SeriesForecast>,
    expected_fc: Option<&SeriesForecast>,
    compare_java: bool,
) -> Vec<ForecastMismatchReport> {
    let mut mismatches = Vec::new();
    match (rust_fc, expected_fc) {
        (Some(rust), Some(expected)) => {
            let skip_forecast = compare_java
                && (is_contraindicated(&tc.patient, &tc.group, tc.execution_date)
                    || !tc.patient.contraindications.is_empty());
            if skip_forecast {
                return mismatches;
            }

            let rust_status = format!("{:?}", rust.status);
            let expected_status = format!("{:?}", expected.status);
            if rust_status != expected_status {
                mismatches.push(ForecastMismatchReport {
                    field: "status".to_string(),
                    rust: Some(rust_status),
                    expected: Some(expected_status),
                    delta_days: None,
                });
            }
            collect_forecast_date_mismatch(
                &mut mismatches,
                "earliest_date",
                rust.status.earliest_date(),
                expected.status.earliest_date(),
            );
            collect_forecast_date_mismatch(
                &mut mismatches,
                "recommended_date",
                rust.status.recommended_date(),
                expected.status.recommended_date(),
            );
            collect_forecast_date_mismatch(
                &mut mismatches,
                "overdue_date",
                rust.status.overdue_date(),
                expected.status.overdue_date(),
            );
            collect_forecast_date_mismatch(
                &mut mismatches,
                "latest_date",
                rust.status.latest_date(),
                expected.status.latest_date(),
            );
        }
        (None, Some(expected)) => {
            mismatches.push(ForecastMismatchReport {
                field: "availability".to_string(),
                rust: None,
                expected: Some(format!("{:?}", expected.status)),
                delta_days: None,
            });
        }
        (Some(rust), None) => {
            mismatches.push(ForecastMismatchReport {
                field: "availability".to_string(),
                rust: Some(format!("{:?}", rust.status)),
                expected: None,
                delta_days: None,
            });
        }
        (None, None) => {}
    }
    mismatches
}

fn collect_forecast_date_mismatch(
    mismatches: &mut Vec<ForecastMismatchReport>,
    field: &str,
    rust: Option<NaiveDate>,
    expected: Option<NaiveDate>,
) {
    if rust == expected {
        return;
    }
    mismatches.push(ForecastMismatchReport {
        field: field.to_string(),
        rust: rust.map(|date| date.to_string()),
        expected: expected.map(|date| date.to_string()),
        delta_days: match (rust, expected) {
            (Some(rust), Some(expected)) => Some(expected.signed_duration_since(rust).num_days()),
            _ => None,
        },
    });
}

fn same_day_counts(tc: &UnifiedTestCase) -> BTreeMap<NaiveDate, usize> {
    let mut counts = BTreeMap::new();
    for dose in &tc.history {
        *counts.entry(dose.date).or_default() += 1;
    }
    counts
}

fn print_summary_report(report: &RunSummaryReport) {
    println!("--- Structured Failure Summary ---");
    println!("Input   : {}", report.input);
    if let Some(group) = &report.group_filter {
        println!("Group   : {}", group);
    }
    if let Some(case) = &report.case_filter {
        println!("Case    : {}", case);
    }
    println!("Executed: {}", report.executed);
    println!("Passed  : {}", report.passed);
    println!("Failed  : {}", report.failed);

    let failed_cases = report
        .cases
        .iter()
        .filter(|case| !case.passed)
        .collect::<Vec<_>>();

    if failed_cases.is_empty() {
        println!("All cases passed.");
        return;
    }

    let mut group_counts = BTreeMap::new();
    let mut case_shapes = BTreeMap::new();
    let mut status_transitions = BTreeMap::new();
    let mut cvx_transitions = BTreeMap::new();
    let mut forecast_fields = BTreeMap::new();
    let mut forecast_deltas = BTreeMap::new();
    let mut covid_selected_policies = BTreeMap::new();
    let mut covid_policy_case_shapes = BTreeMap::new();
    let mut covid_forecast_anchors = BTreeMap::new();
    let mut covid_dose_relationships = BTreeMap::new();
    let mut covid_eval_transitions_by_policy = BTreeMap::new();
    let mut covid_forecast_deltas_by_policy = BTreeMap::new();
    let mut same_day_failed_cases = 0usize;
    let mut same_day_eval_mismatches = 0usize;

    for case in &failed_cases {
        *group_counts.entry(case.group.clone()).or_insert(0usize) += 1;
        if case.has_same_day_doses {
            same_day_failed_cases += 1;
        }

        let has_eval = !case.evaluation_mismatches.is_empty();
        let has_forecast = !case.forecast_mismatches.is_empty();
        let shape = match (has_eval, has_forecast) {
            (true, true) => "eval + forecast",
            (true, false) => "eval only",
            (false, true) => "forecast only",
            (false, false) => "other",
        };
        *case_shapes.entry(shape.to_string()).or_insert(0usize) += 1;

        if let Some(covid) = &case.covid19_semantics {
            *covid_selected_policies
                .entry(covid.selected_policy.clone())
                .or_insert(0usize) += 1;
            *covid_policy_case_shapes
                .entry(format!("{} / {}", covid.selected_policy, shape))
                .or_insert(0usize) += 1;
            *covid_forecast_anchors
                .entry(format!(
                    "{} / {}",
                    covid.selected_policy, covid.policy_forecast_anchor
                ))
                .or_insert(0usize) += 1;
            for fact in &covid.dose_facts {
                *covid_dose_relationships
                    .entry(format!(
                        "{} / {} / CVX {} / {} / {}",
                        covid.selected_policy,
                        fact.relationship_to_selected_series,
                        fact.cvx,
                        fact.product_family,
                        fact.season
                    ))
                    .or_insert(0usize) += 1;
            }
        }

        for mismatch in &case.evaluation_mismatches {
            if mismatch.same_day_count > 1 {
                same_day_eval_mismatches += 1;
            }
            if mismatch.field == "status" {
                if let Some(transition) = &mismatch.transition {
                    *status_transitions
                        .entry(transition.clone())
                        .or_insert(0usize) += 1;
                    if let Some(cvx) = mismatch.cvx {
                        *cvx_transitions
                            .entry(format!("CVX {} {}", cvx, transition))
                            .or_insert(0usize) += 1;
                    }
                    if let Some(covid) = &case.covid19_semantics {
                        *covid_eval_transitions_by_policy
                            .entry(format!("{} / {}", covid.selected_policy, transition))
                            .or_insert(0usize) += 1;
                    }
                }
            }
        }

        for mismatch in &case.forecast_mismatches {
            *forecast_fields
                .entry(mismatch.field.clone())
                .or_insert(0usize) += 1;
            if let Some(delta) = mismatch.delta_days {
                let formatted_delta = format_delta(delta);
                *forecast_deltas
                    .entry(formatted_delta.clone())
                    .or_insert(0usize) += 1;
                if let Some(covid) = &case.covid19_semantics {
                    *covid_forecast_deltas_by_policy
                        .entry(format!(
                            "{} / {} / {}",
                            covid.selected_policy, mismatch.field, formatted_delta
                        ))
                        .or_insert(0usize) += 1;
                }
            }
        }
    }

    print_count_section("Failed groups", &group_counts);
    print_count_section("Case shapes", &case_shapes);
    print_count_section("Evaluation status transitions", &status_transitions);
    print_count_section("CVX status transitions", &cvx_transitions);
    print_count_section("Forecast fields", &forecast_fields);
    print_count_section("Forecast date deltas", &forecast_deltas);
    print_count_section("COVID selected policies", &covid_selected_policies);
    print_count_section("COVID policy case shapes", &covid_policy_case_shapes);
    print_count_section("COVID forecast anchors", &covid_forecast_anchors);
    print_count_section(
        "COVID dose relationship occurrences",
        &covid_dose_relationships,
    );
    print_count_section(
        "COVID evaluation transitions by policy",
        &covid_eval_transitions_by_policy,
    );
    print_count_section(
        "COVID forecast deltas by policy",
        &covid_forecast_deltas_by_policy,
    );

    println!();
    println!("Same-day:");
    println!(
        "  failed cases with same-day doses: {}",
        same_day_failed_cases
    );
    println!(
        "  evaluation mismatches on same-day dates: {}",
        same_day_eval_mismatches
    );
}

fn print_count_section(title: &str, counts: &BTreeMap<String, usize>) {
    if counts.is_empty() {
        return;
    }

    println!();
    println!("{}:", title);
    for (label, count) in sorted_counts(counts) {
        println!("  {}: {}", label, count);
    }
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(&String, usize)> {
    let mut rows = counts
        .iter()
        .map(|(label, count)| (label, *count))
        .collect::<Vec<_>>();
    rows.sort_by(|(left_label, left_count), (right_label, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_label.cmp(right_label))
    });
    rows
}

fn format_delta(delta: i64) -> String {
    match delta {
        1 => "+1 day".to_string(),
        -1 => "-1 day".to_string(),
        value if value > 0 => format!("+{} days", value),
        value => format!("{} days", value),
    }
}
