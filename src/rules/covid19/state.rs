#![allow(dead_code)]

use crate::models::{Dose, DoseEvaluation, DoseStatus, EvaluationReason, Patient};
use crate::rules::covid19::facts::{CovidDoseFact, normalize_covid_history};
use crate::rules::covid19::policy::{CovidSeriesPolicy, CvxRelationship, ForecastAnchorPolicy};
use crate::rules::covid19::products::is_aug2025_current_formulation;
use crate::rules::covid19::seasons::CovidSeason;
use crate::rules::covid19::selection::select_aug2025_policy;
use chrono::NaiveDate;

/// Behavior-neutral read model for COVID parity work.
///
/// This state is intentionally derived from already-computed Rust evaluations plus
/// normalized COVID facts. Forecasting and evaluation still run through the legacy
/// hooks until one policy branch is migrated deliberately.
#[derive(Debug, Clone)]
pub struct CovidEvaluatedState<'a> {
    pub selected_series: &'static CovidSeriesPolicy,
    pub dose_facts: Vec<CovidDoseFact<'a>>,
    pub evaluations: Vec<DoseEvaluation>,
    pub latest_not_ignored_covid_shot: Option<usize>,
    pub latest_valid_current_season_shot: Option<usize>,
    pub latest_invalid_current_season_nonseries_shot: Option<usize>,
    pub latest_invalid_forecast_anchor_shot: Option<usize>,
}

impl<'a> CovidEvaluatedState<'a> {
    pub fn from_aug2025_policy(
        patient: &Patient,
        history: &'a [Dose],
        eval_date: NaiveDate,
        evaluations: &[DoseEvaluation],
    ) -> Self {
        let dose_facts = normalize_covid_history(patient, history);
        let selected_series = select_aug2025_policy(patient, eval_date, &dose_facts);
        Self::from_selected_policy(selected_series, dose_facts, evaluations)
    }

    pub fn from_selected_policy(
        selected_series: &'static CovidSeriesPolicy,
        dose_facts: Vec<CovidDoseFact<'a>>,
        evaluations: &[DoseEvaluation],
    ) -> Self {
        let evaluations = evaluations.to_vec();
        let latest_not_ignored_covid_shot =
            latest_matching_fact(&dose_facts, &evaluations, |fact, eval| {
                fact.supported_by_java_covid && !is_ignored_for_covid_state(eval)
            });
        let latest_valid_current_season_shot =
            latest_matching_fact(&dose_facts, &evaluations, |fact, eval| {
                fact.season == selected_series.season && eval.status == DoseStatus::Valid
            });
        let latest_invalid_current_season_nonseries_shot =
            latest_matching_fact(&dose_facts, &evaluations, |fact, eval| {
                fact.season == selected_series.season
                    && eval.status == DoseStatus::Invalid
                    && fact.relationship_to(selected_series)
                        != CvxRelationship::MemberOfSelectedSeries
            });
        let latest_invalid_forecast_anchor_shot = latest_forecast_anchor_fact(
            selected_series,
            &dose_facts,
            &evaluations,
            latest_invalid_current_season_nonseries_shot,
        );

        Self {
            selected_series,
            dose_facts,
            evaluations,
            latest_not_ignored_covid_shot,
            latest_valid_current_season_shot,
            latest_invalid_current_season_nonseries_shot,
            latest_invalid_forecast_anchor_shot,
        }
    }

    pub fn evaluation_for_fact_index(&self, index: usize) -> Option<&DoseEvaluation> {
        self.dose_facts
            .get(index)
            .and_then(|fact| self.evaluation_for_fact(fact))
    }

    pub fn evaluation_for_fact(&self, fact: &CovidDoseFact<'_>) -> Option<&DoseEvaluation> {
        self.evaluations
            .iter()
            .find(|eval| eval.dose_date == fact.raw.date && eval.cvx == fact.raw.cvx)
    }

    pub fn current_season_dose_count(&self) -> usize {
        self.dose_facts
            .iter()
            .filter(|fact| fact.season == self.selected_series.season)
            .count()
    }

    pub fn supported_dose_count(&self) -> usize {
        self.dose_facts
            .iter()
            .filter(|fact| fact.supported_by_java_covid)
            .count()
    }

    pub fn current_season_valid_dose_count(&self) -> usize {
        self.dose_facts
            .iter()
            .filter(|fact| fact.season == self.selected_series.season)
            .filter(|fact| {
                self.evaluation_for_fact(fact)
                    .map(|eval| eval.status == DoseStatus::Valid)
                    .unwrap_or(false)
            })
            .count()
    }

    pub fn relationship_for_fact(&self, fact: CovidDoseFact<'_>) -> CvxRelationship {
        fact.relationship_to(self.selected_series)
    }

    pub fn is_old_aug2025_product(&self, fact: CovidDoseFact<'_>) -> bool {
        self.selected_series.season == CovidSeason::Aug2025
            && !is_aug2025_current_formulation(fact.raw.cvx)
    }

    pub fn fact_summary_label(&self, index: Option<usize>) -> &'static str {
        match index.and_then(|idx| self.dose_facts.get(idx)) {
            Some(fact) if self.is_old_aug2025_product(*fact) => "old_aug2025_product",
            Some(fact) if !fact.supported_by_java_covid => "unsupported_covid",
            Some(_) => "supported_covid",
            None => "none",
        }
    }
}

fn latest_matching_fact(
    dose_facts: &[CovidDoseFact<'_>],
    evaluations: &[DoseEvaluation],
    predicate: impl Fn(CovidDoseFact<'_>, &DoseEvaluation) -> bool,
) -> Option<usize> {
    dose_facts
        .iter()
        .enumerate()
        .filter_map(|(idx, fact)| {
            evaluations
                .iter()
                .find(|eval| eval.dose_date == fact.raw.date && eval.cvx == fact.raw.cvx)
                .filter(|eval| predicate(*fact, eval))
                .map(|_| (idx, fact.raw.date))
        })
        .max_by_key(|(_, date)| *date)
        .map(|(idx, _)| idx)
}

fn latest_forecast_anchor_fact(
    selected_series: &CovidSeriesPolicy,
    dose_facts: &[CovidDoseFact<'_>],
    evaluations: &[DoseEvaluation],
    latest_invalid_current_season_nonseries_shot: Option<usize>,
) -> Option<usize> {
    match selected_series.forecast.anchor {
        ForecastAnchorPolicy::LastInvalidOldProductPlus56 => {
            latest_invalid_current_season_nonseries_shot.filter(|idx| {
                dose_facts
                    .get(*idx)
                    .map(|fact| !is_aug2025_current_formulation(fact.raw.cvx))
                    .unwrap_or(false)
            })
        }
        ForecastAnchorPolicy::LastInvalidLt2Plus28 => {
            latest_matching_fact(dose_facts, evaluations, |fact, eval| {
                fact.season == selected_series.season && eval.status == DoseStatus::Invalid
            })
        }
        _ => None,
    }
}

fn is_ignored_for_covid_state(eval: &DoseEvaluation) -> bool {
    eval.status == DoseStatus::Invalid
        && (eval.reasons.contains(&EvaluationReason::PriorToDOB)
            || eval
                .reasons
                .contains(&EvaluationReason::DuplicateShotSameDay))
}
