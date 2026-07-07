#![allow(dead_code)]

use crate::models::{DoseEvaluation, Patient, SeriesForecast};
use crate::rules::covid19::facts::{CovidAgeFacts, CovidDoseFact};
use crate::rules::covid19::policy::{CovidSeriesPolicy, CvxRelationship};
use crate::rules::covid19::products::CovidProductFamily;
use crate::rules::covid19::state::CovidEvaluatedState;
use chrono::NaiveDate;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct CovidTraceDose {
    pub date: chrono::NaiveDate,
    pub cvx: crate::models::Cvx,
    pub season: &'static str,
    pub product_family: &'static str,
    pub relationship: CvxRelationship,
    pub supported_by_java_covid: bool,
    pub age_at_dose: CovidAgeFacts,
    pub age_at_season_start: CovidAgeFacts,
    pub evaluation: Option<DoseEvaluation>,
}

#[derive(Debug, Clone)]
pub struct CovidTrace {
    pub selected_series: &'static CovidSeriesPolicy,
    pub patient_age_at_eval: CovidAgeFacts,
    pub doses: Vec<CovidTraceDose>,
    pub forecast: Option<SeriesForecast>,
}

impl CovidTrace {
    pub fn from_policy_inputs(
        patient: &Patient,
        eval_date: NaiveDate,
        selected_series: &'static CovidSeriesPolicy,
        facts: &[CovidDoseFact<'_>],
        evaluations: &[DoseEvaluation],
        forecast: Option<SeriesForecast>,
    ) -> Self {
        let state =
            CovidEvaluatedState::from_selected_policy(selected_series, facts.to_vec(), evaluations);
        Self::from_state(patient, eval_date, &state, forecast)
    }

    pub fn from_state(
        patient: &Patient,
        eval_date: NaiveDate,
        state: &CovidEvaluatedState<'_>,
        forecast: Option<SeriesForecast>,
    ) -> Self {
        let doses = state
            .dose_facts
            .iter()
            .map(|fact| CovidTraceDose {
                date: fact.raw.date,
                cvx: fact.raw.cvx,
                season: fact.season.ice_key(),
                product_family: product_family_name(fact.product.family),
                relationship: state.relationship_for_fact(*fact),
                supported_by_java_covid: fact.supported_by_java_covid,
                age_at_dose: fact.age_at_dose,
                age_at_season_start: fact.age_at_season_start,
                evaluation: state.evaluation_for_fact(fact).cloned(),
            })
            .collect();

        Self {
            selected_series: state.selected_series,
            patient_age_at_eval: CovidAgeFacts::at(patient, eval_date),
            doses,
            forecast,
        }
    }

    pub fn format(&self) -> String {
        let mut out = String::new();
        let selected = self.selected_series;

        let _ = writeln!(out, "selected_policy = {:?}", selected.id);
        let _ = writeln!(out, "selected_policy_ice_name = {}", selected.ice_name);
        let _ = writeln!(out, "trace_selector = Aug2025SelectionHelper");
        let _ = writeln!(out, "policy.season = {}", selected.season.ice_key());
        let _ = writeln!(out, "policy.age_band = {:?}", selected.age_band);
        let _ = writeln!(out, "policy.product_family = {:?}", selected.product_family);
        let _ = writeln!(out, "policy.max_valid_doses = {}", selected.max_valid_doses);
        let _ = writeln!(out, "policy.cvx_members = {:?}", selected.cvx_members);
        let _ = writeln!(out, "policy.selection = {:?}", selected.selection);
        let _ = writeln!(out, "policy.dose_identity = {:?}", selected.dose_identity);
        let _ = writeln!(out, "policy.overflow = {:?}", selected.overflow);
        let _ = writeln!(
            out,
            "policy.interval_anchor = {:?}",
            selected.intervals.anchor
        );
        let _ = writeln!(
            out,
            "policy.forecast_anchor = {:?}",
            selected.forecast.anchor
        );
        write_source_refs(
            &mut out,
            "policy.sources.series_selection",
            selected.sources.series_selection,
        );
        write_source_refs(
            &mut out,
            "policy.sources.evaluation",
            selected.sources.evaluation,
        );
        write_source_refs(
            &mut out,
            "policy.sources.recommendation",
            selected.sources.recommendation,
        );
        write_source_refs(&mut out, "policy.sources.yaml", selected.sources.yaml);
        write_age_facts(&mut out, "patient_age_at_eval", self.patient_age_at_eval);

        if self.doses.is_empty() {
            let _ = writeln!(out, "\nDose facts: none");
        } else {
            for dose in &self.doses {
                let _ = writeln!(out, "\nDose {} CVX {}:", dose.date, dose.cvx);
                let _ = writeln!(out, "  season = {}", dose.season);
                let _ = writeln!(out, "  product_family = {}", dose.product_family);
                let _ = writeln!(
                    out,
                    "  relationship_to_selected_series = {:?}",
                    dose.relationship
                );
                let _ = writeln!(
                    out,
                    "  supported_by_java_covid = {}",
                    dose.supported_by_java_covid
                );
                write_age_facts(&mut out, "  age_at_dose", dose.age_at_dose);
                write_age_facts(&mut out, "  age_at_season_start", dose.age_at_season_start);
                if let Some(evaluation) = &dose.evaluation {
                    let reasons = evaluation
                        .reasons
                        .iter()
                        .map(|reason| reason.as_str())
                        .collect::<Vec<_>>();
                    let _ = writeln!(out, "  evaluation.status = {:?}", evaluation.status);
                    let _ = writeln!(
                        out,
                        "  evaluation.target_dose = {:?}",
                        evaluation.dose_number
                    );
                    let _ = writeln!(out, "  evaluation.reasons = {:?}", reasons);
                } else {
                    let _ = writeln!(
                        out,
                        "  evaluation = <missing from selected Rust COVID group>"
                    );
                }
            }
        }

        let _ = writeln!(out, "\nForecast:");
        if let Some(forecast) = &self.forecast {
            let _ = writeln!(out, "  status = {:?}", forecast.status);
            let _ = writeln!(
                out,
                "  earliest_date = {:?}",
                forecast.status.earliest_date()
            );
            let _ = writeln!(
                out,
                "  recommended_date = {:?}",
                forecast.status.recommended_date()
            );
            let _ = writeln!(out, "  overdue_date = {:?}", forecast.status.overdue_date());
            let _ = writeln!(out, "  latest_date = {:?}", forecast.status.latest_date());
            let _ = writeln!(
                out,
                "  forecast_anchor_policy = {:?}",
                selected.forecast.anchor
            );
            let _ = writeln!(
                out,
                "  invalid_old_product_retry_days = {:?}",
                selected.forecast.invalid_old_product_retry_days
            );
            let _ = writeln!(
                out,
                "  invalid_lt2_retry_days = {:?}",
                selected.forecast.invalid_lt2_retry_days
            );
        } else {
            let _ = writeln!(out, "  <missing from selected Rust COVID group>");
        }

        out
    }
}

/// Build a compact, behavior-neutral Aug 2025 COVID policy trace for one case.
///
/// This intentionally uses the new COVID policy/fact scaffolding only for debugging output;
/// production COVID evaluation and forecasting still come from `overrides.rs` and `schedules.rs`.
pub fn format_aug2025_policy_trace(
    patient: &Patient,
    history: &[crate::models::Dose],
    eval_date: NaiveDate,
    evaluations: &[DoseEvaluation],
    forecast: Option<&SeriesForecast>,
) -> String {
    let state = CovidEvaluatedState::from_aug2025_policy(patient, history, eval_date, evaluations);
    CovidTrace::from_state(patient, eval_date, &state, forecast.cloned()).format()
}
fn write_source_refs(out: &mut String, label: &str, refs: &[&str]) {
    if !refs.is_empty() {
        let _ = writeln!(out, "{} = {:?}", label, refs);
    }
}

fn write_age_facts(out: &mut String, label: &str, facts: CovidAgeFacts) {
    let _ = writeln!(
        out,
        "{}.at_least_6_months = {}",
        label, facts.at_least_6_months
    );
    let _ = writeln!(out, "{}.under_2_years = {}", label, facts.under_2_years);
    let _ = writeln!(out, "{}.under_5_years = {}", label, facts.under_5_years);
    let _ = writeln!(out, "{}.under_12_years = {}", label, facts.under_12_years);
    let _ = writeln!(
        out,
        "{}.at_least_65_years = {}",
        label, facts.at_least_65_years
    );
}

fn product_family_name(family: CovidProductFamily) -> &'static str {
    match family {
        CovidProductFamily::PfizerPediatric => "PfizerPediatric",
        CovidProductFamily::PfizerAdult => "PfizerAdult",
        CovidProductFamily::ModernaPediatric => "ModernaPediatric",
        CovidProductFamily::ModernaAdult => "ModernaAdult",
        CovidProductFamily::Novavax => "Novavax",
        CovidProductFamily::Janssen => "Janssen",
        CovidProductFamily::OldMonovalent => "OldMonovalent",
        CovidProductFamily::OldBivalent => "OldBivalent",
        CovidProductFamily::Unspecified => "Unspecified",
        CovidProductFamily::OtherSupported => "OtherSupported",
        CovidProductFamily::Unsupported => "Unsupported",
    }
}
