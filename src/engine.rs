use crate::date_utils::{compare_elapsed, SmallVec, TimePeriod};
use crate::models::{
    Cvx, Dose, DoseEvaluation, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus,
    VaccineGroupForecast,
};
use crate::schedule::{CompiledDoseInterval, CompiledSeries};
use crate::set_field;
use chrono::NaiveDate;
use std::cmp::max;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DecisionTrace {
    pub step: &'static str,
    pub description: String,
    pub source_file: &'static str,
    pub line_number: u32,
}

thread_local! {
    pub static TRACE_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    pub static DECISION_TRACES: std::cell::RefCell<Vec<DecisionTrace>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub fn is_trace_enabled() -> bool {
    TRACE_ENABLED.with(|val| val.get())
}

pub fn set_trace_enabled(enabled: bool) {
    TRACE_ENABLED.with(|val| val.set(enabled));
}

pub fn log_decision(
    step: &'static str,
    description: String,
    source_file: &'static str,
    line_number: u32,
) {
    DECISION_TRACES.with(|traces| {
        traces.borrow_mut().push(DecisionTrace {
            step,
            description,
            source_file,
            line_number,
        });
    });
}

pub fn clear_traces() {
    DECISION_TRACES.with(|traces| {
        traces.borrow_mut().clear();
    });
}

pub fn get_traces() -> Vec<DecisionTrace> {
    DECISION_TRACES.with(|traces| traces.borrow().clone())
}

#[macro_export]
macro_rules! trace_decision {
    ($step:expr, $($arg:tt)*) => {
        if $crate::engine::is_trace_enabled() {
            $crate::engine::log_decision($step, format!($($arg)*), file!(), line!());
        }
    };
    ($ctx:expr, $step:expr, $($arg:tt)*) => {
        if $crate::engine::is_trace_enabled() {
            $crate::engine::log_decision($step, format!($($arg)*), file!(), line!());
        }
    };
}

#[allow(dead_code)]
pub struct EvaluationContext<'a> {
    pub patient: &'a Patient,
    pub history: &'a [Dose],
    pub valid_doses: &'a [(NaiveDate, usize)],
    pub current_dose: Option<&'a Dose>,
    pub target_dose_number: usize,
    pub eval_date: NaiveDate,
    pub active_series_name: &'a str,
}

impl<'a> EvaluationContext<'a> {
    pub fn new(
        patient: &'a Patient,
        history: &'a [Dose],
        valid_doses: &'a [(NaiveDate, usize)],
        current_dose: Option<&'a Dose>,
        target_dose_number: usize,
        eval_date: NaiveDate,
        active_series_name: &'a str,
    ) -> Self {
        Self {
            patient,
            history,
            valid_doses,
            current_dose,
            target_dose_number,
            eval_date,
            active_series_name,
        }
    }

    /// Counts the number of valid doses administered before the specified age
    pub fn count_valid_doses_before(&self, tp: TimePeriod) -> usize {
        let cutoff = tp.add_to(self.patient.birth_date);
        self.valid_doses
            .iter()
            .filter(|(date, _)| *date < cutoff)
            .count()
    }

    /// Checks if a specific CVX code was administered but is not in the valid doses
    pub fn has_invalid_cvx(&self, cvx: Cvx) -> bool {
        let has_cvx = self.history.iter().any(|d| d.cvx == cvx);
        let valid_has_cvx = self
            .valid_doses
            .iter()
            .any(|(date, _)| self.history.iter().any(|d| d.cvx == cvx && d.date == *date));
        has_cvx && !valid_has_cvx
    }

    /// Counts how many doses matching any of the provided CVX codes were administered before the specified age
    pub fn count_cvx_before(&self, cvx_list: &[u16], tp: TimePeriod) -> usize {
        let cutoff = tp.add_to(self.patient.birth_date);
        self.history
            .iter()
            .filter(|d| cvx_list.contains(&d.cvx.0) && d.date < cutoff)
            .count()
    }
}

pub type RuleCondition = fn(&EvaluationContext) -> bool;
pub type CustomForecastHook = fn(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    evaluations: &[DoseEvaluation],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
);

pub type CustomSwitchHook = fn(
    current_series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
) -> Option<&'static str>;

pub type CustomEvaluationHook = fn(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
);

pub type CustomDoseNumberHook = fn(series_name: &str, ctx: &EvaluationContext) -> usize;

pub type CustomExtraDoseHook = fn(
    series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, SmallVec<[EvaluationReason; 4]>)>;

pub type CustomCompletionHook = fn(ctx: &EvaluationContext) -> bool;

pub type GroupSelectionAndPostProcess = fn(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str;

pub trait CandidateForecastsExt {
    fn get_forecast(&self, name: &str) -> Option<&VaccineGroupForecast>;
    fn get_forecast_mut(&mut self, name: &str) -> Option<&mut VaccineGroupForecast>;
    fn contains_forecast(&self, name: &str) -> bool;
}

impl CandidateForecastsExt for [(&'static str, VaccineGroupForecast)] {
    fn get_forecast(&self, name: &str) -> Option<&VaccineGroupForecast> {
        self.iter().find(|(n, _)| *n == name).map(|(_, f)| f)
    }
    fn get_forecast_mut(&mut self, name: &str) -> Option<&mut VaccineGroupForecast> {
        self.iter_mut().find(|(n, _)| *n == name).map(|(_, f)| f)
    }
    fn contains_forecast(&self, name: &str) -> bool {
        self.iter().any(|(n, _)| *n == name)
    }
}

// Primitives for generic override rules
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParameterOverrideRule {
    pub description: &'static str,
    pub target_dose_number: usize,
    pub condition: RuleCondition,
    pub override_abs_min_age: Option<TimePeriod>,
    pub override_abs_min_interval_from_dose: Option<(usize, TimePeriod)>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ConditionalCompletionRule {
    pub description: &'static str,
    pub condition: RuleCondition,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RecommendationOverrideRule {
    pub description: &'static str,
    pub target_dose_number: usize,
    pub condition: RuleCondition,
    pub override_min_age: Option<TimePeriod>,
    pub override_min_interval: Option<TimePeriod>,
}

pub trait EvaluationPolicy: Send + Sync {
    fn custom_forecast_hook(
        &self,
        _patient: &Patient,
        _valid_doses: &[(NaiveDate, usize)],
        _evaluations: &[DoseEvaluation],
        _history: &[Dose],
        _eval_date: NaiveDate,
        _forecast: &mut SeriesForecast,
    ) {
    }

    fn custom_switch_hook(
        &self,
        _current_series_name: &str,
        _target_dose_idx: usize,
        _ctx: &EvaluationContext,
    ) -> Option<&'static str> {
        None
    }

    fn custom_evaluation_hook(
        &self,
        _series_name: &str,
        _target_dose_idx: usize,
        _ctx: &EvaluationContext,
        _reasons: &mut SmallVec<[EvaluationReason; 4]>,
        _status: &mut DoseStatus,
    ) {
    }

    fn custom_dose_number_hook(
        &self,
        _series_name: &str,
        _ctx: &EvaluationContext,
    ) -> Option<usize> {
        None
    }

    fn custom_extra_dose_hook(
        &self,
        _series_name: &str,
        _ctx: &EvaluationContext,
    ) -> Option<(DoseStatus, SmallVec<[EvaluationReason; 4]>)> {
        None
    }

    fn custom_completion_hook(&self, _ctx: &EvaluationContext) -> Option<bool> {
        None
    }

    fn adjust_same_day_target_dose_number(
        &self,
        _dose: &Dose,
        _evaluations: &[crate::models::DoseEvaluation],
    ) -> Option<usize> {
        None
    }

    fn is_same_day_duplicate(&self, _dose: &Dose, _sorted_history_subset: &[Dose]) -> bool {
        _sorted_history_subset
            .iter()
            .any(|prev_dose| prev_dose.date == _dose.date)
    }

    fn ignore_evaluation_for_maximum_date(
        &self,
        _patient: &Patient,
        _eval: &crate::models::DoseEvaluation,
    ) -> bool {
        false
    }
}

pub struct EvaluationEngine<'a> {
    pub series: &'a CompiledSeries,
    pub param_overrides: &'a [ParameterOverrideRule],
    pub completion_rules: &'a [ConditionalCompletionRule],
    pub rec_overrides: &'a [RecommendationOverrideRule],
    pub policy: Option<&'a dyn EvaluationPolicy>,
}

impl<'a> EvaluationEngine<'a> {
    pub fn new(series: &'a CompiledSeries) -> Self {
        EvaluationEngine {
            series,
            param_overrides: &[],
            completion_rules: &[],
            rec_overrides: &[],
            policy: None,
        }
    }

    pub fn evaluate_patient(
        &self,
        patient: &Patient,
        history: &[Dose],
        eval_date: NaiveDate,
        group_series: &'a [CompiledSeries],
    ) -> VaccineGroupForecast {
        // 1. Filter and sort history chronologically (only keep doses relevant to this series or group)
        let mut sorted_history = SmallVec::<[(usize, Dose); 32]>::new();
        for (orig_idx, dose) in history.iter().enumerate() {
            let keep = if group_series.is_empty() {
                self.series
                    .doses
                    .iter()
                    .any(|d_rule| d_rule.allowed_cvx.contains(&dose.cvx.0))
            } else {
                group_series.iter().any(|s| {
                    s.doses
                        .iter()
                        .any(|d_rule| d_rule.allowed_cvx.contains(&dose.cvx.0))
                })
            };
            if keep {
                sorted_history.push((orig_idx, *dose));
            }
        }
        let first_relevant_date = sorted_history.iter().map(|(_, dose)| dose.date).min();

        sorted_history.sort_by(|a, b| {
            if a.1.date != b.1.date {
                a.1.date.cmp(&b.1.date)
            } else {
                let group = &self.series.vaccine_group;
                let dtp_preserve_196_source_order = *group == "DTP"
                    && should_preserve_dtp_196_source_order(first_relevant_date, a.1.date)
                    && (a.1.cvx.0 == 196 || b.1.cvx.0 == 196);
                let prio_a = get_same_day_priority(
                    group,
                    a.1.cvx,
                    patient.birth_date,
                    a.1.date,
                    dtp_preserve_196_source_order,
                );
                let prio_b = get_same_day_priority(
                    group,
                    b.1.cvx,
                    patient.birth_date,
                    b.1.date,
                    dtp_preserve_196_source_order,
                );
                if prio_a != prio_b {
                    prio_a.cmp(&prio_b)
                } else if *group == "DTP" && a.1.cvx == b.1.cvx {
                    let dtp_same_date_has_cvx_115 =
                        history.iter().any(|d| d.date == a.1.date && d.cvx.0 == 115);
                    if a.1.cvx.0 == 198 && !dtp_same_date_has_cvx_115 {
                        a.0.cmp(&b.0)
                    } else {
                        b.0.cmp(&a.0) // Later original index comes first
                    }
                } else if *group == "INFLUENZA" {
                    b.0.cmp(&a.0) // Later original index comes first
                } else {
                    std::cmp::Ordering::Equal
                }
            }
        });

        let sorted_history_doses = sorted_history
            .iter()
            .map(|(_, dose)| *dose)
            .collect::<SmallVec<[Dose; 32]>>();

        let mut evaluations = SmallVec::<[DoseEvaluation; 8]>::new();
        let mut eval_target_dose_numbers = SmallVec::<[usize; 8]>::new();
        let mut evaluation_orig_indices = SmallVec::<[usize; 8]>::new();
        let mut valid_doses = SmallVec::<[(NaiveDate, usize); 32]>::new();
        let mut is_completed = false;
        let mut active_series: &'a CompiledSeries = self.series;

        // 2. Chronological dose evaluation loop
        let mut i = 0;
        while i < sorted_history.len() {
            let dose = &sorted_history[i].1;
            let mut target_dose_idx =
                valid_doses.iter().map(|(_, num)| *num).max().unwrap_or(0) + 1;

            if let Some(valid_override) = dose.is_valid {
                let output_dose_number = std::cmp::min(target_dose_idx, valid_doses.len() + 1);
                let (status, reason) = if valid_override {
                    valid_doses.push((dose.date, target_dose_idx));
                    (DoseStatus::Valid, EvaluationReason::DoseOverrideValid)
                } else {
                    (DoseStatus::Invalid, EvaluationReason::DoseOverrideInvalid)
                };

                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx,
                    status,
                    reasons: crate::reasons![reason],
                    dose_number: Some(output_dose_number),
                    sources: std::collections::HashMap::new(),
                });
                eval_target_dose_numbers.push(target_dose_idx);
                evaluation_orig_indices.push(sorted_history[i].0);

                if target_dose_idx >= active_series.num_doses && valid_override {
                    is_completed = true;
                }

                i += 1;
                continue;
            }

            let is_contraindicated =
                is_dose_contraindicated(patient, dose.cvx, dose.date, &active_series.vaccine_group);
            if is_contraindicated {
                let output_dose_number = std::cmp::min(target_dose_idx, valid_doses.len() + 1);
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx,
                    status: DoseStatus::Invalid,
                    reasons: crate::reasons![EvaluationReason::ContraindicatedVaccine],
                    dose_number: Some(output_dose_number),
                    sources: std::collections::HashMap::new(),
                });
                eval_target_dose_numbers.push(target_dose_idx);
                evaluation_orig_indices.push(sorted_history[i].0);
                i += 1;
                continue;
            }

            if let Some(policy) = self.policy {
                let ctx = EvaluationContext::new(
                    patient,
                    history,
                    &valid_doses,
                    Some(dose),
                    target_dose_idx,
                    eval_date,
                    &active_series.name,
                );
                if let Some(num) = policy.custom_dose_number_hook(&active_series.name, &ctx) {
                    target_dose_idx = num;
                }

                if let Some(num) = policy.adjust_same_day_target_dose_number(dose, &evaluations) {
                    target_dose_idx = num;
                }
            }

            // Same-day duplicate check
            // A dose is a same-day duplicate if another dose on the same day was
            // already evaluated as a competing dose. Doses that were Accepted with
            // VaccineNotLicensedForMales are non-competing (e.g., CVX 118 for males)
            // and should not trigger duplicate detection for subsequent same-day doses.
            let is_duplicate = if i > 0 {
                let history_subset = &sorted_history_doses[0..i];
                let has_same_day = self
                    .policy
                    .map(|p| p.is_same_day_duplicate(dose, history_subset))
                    .unwrap_or_else(|| history_subset.iter().any(|prev| prev.date == dose.date));
                if has_same_day {
                    // Check if any same-day dose was a competing evaluation
                    // (not just Accepted with VaccineNotLicensedForMales)
                    evaluations.iter().any(|e| {
                        e.dose_date == dose.date
                            && e.status != DoseStatus::Invalid
                            && !(e.status == DoseStatus::Accepted
                                && e.reasons
                                    .contains(&EvaluationReason::VaccineNotLicensedForMales))
                            && !e.reasons.contains(&EvaluationReason::MissingAntigen)
                            && !e
                                .reasons
                                .contains(&EvaluationReason::VaccineNotPartOfSeries)
                    })
                } else {
                    false
                }
            } else {
                false
            };
            if is_duplicate {
                let prev_dose_num = evaluations
                    .last()
                    .and_then(|e| e.dose_number)
                    .unwrap_or(target_dose_idx);
                let mut dup_status = DoseStatus::Invalid;
                let mut dup_reasons = crate::reasons![EvaluationReason::DuplicateShotSameDay];
                // Allow custom evaluation hook to override duplicate decision
                // (e.g., HPV CVX 118 for males should be Accepted, not a duplicate)
                if let Some(policy) = self.policy {
                    let ctx = EvaluationContext::new(
                        patient,
                        history,
                        &valid_doses,
                        Some(dose),
                        target_dose_idx,
                        eval_date,
                        &active_series.name,
                    );
                    policy.custom_evaluation_hook(
                        &active_series.name,
                        target_dose_idx,
                        &ctx,
                        &mut dup_reasons,
                        &mut dup_status,
                    );
                }
                let output_dose_number = if dup_status == DoseStatus::Invalid {
                    prev_dose_num
                } else {
                    std::cmp::min(target_dose_idx, valid_doses.len() + 1)
                };
                if dup_status == DoseStatus::Valid {
                    valid_doses.push((dose.date, target_dose_idx));
                }
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx,
                    status: dup_status,
                    reasons: dup_reasons,
                    dose_number: Some(output_dose_number),
                    sources: std::collections::HashMap::new(),
                });
                eval_target_dose_numbers.push(target_dose_idx);
                evaluation_orig_indices.push(sorted_history[i].0);
                i += 1;
                continue;
            }

            // Birthdate boundary check
            if dose.date < patient.birth_date {
                let output_dose_number = std::cmp::min(target_dose_idx, valid_doses.len() + 1);
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx,
                    status: DoseStatus::Invalid,
                    reasons: crate::reasons![EvaluationReason::PriorToDOB],
                    dose_number: Some(output_dose_number),
                    sources: std::collections::HashMap::new(),
                });
                eval_target_dose_numbers.push(target_dose_idx);
                evaluation_orig_indices.push(sorted_history[i].0);
                i += 1;
                continue;
            }

            // Apply custom switch hook if present
            if let Some(policy) = self.policy {
                let ctx = EvaluationContext::new(
                    patient,
                    history,
                    &valid_doses,
                    Some(dose),
                    target_dose_idx,
                    eval_date,
                    &active_series.name,
                );
                if let Some(new_series_name) =
                    policy.custom_switch_hook(&active_series.name, target_dose_idx, &ctx)
                {
                    if let Some(new_series) =
                        group_series.iter().find(|s| s.name == new_series_name)
                    {
                        if new_series.name != active_series.name {
                            active_series = new_series;
                            is_completed = false;
                        }
                    }
                }
            }

            // If patient already completed the series, additional doses are boosters/extra
            if is_completed || target_dose_idx > active_series.num_doses {
                let ctx = EvaluationContext::new(
                    patient,
                    history,
                    &valid_doses,
                    Some(dose),
                    target_dose_idx,
                    eval_date,
                    &active_series.name,
                );
                if let Some(policy) = self.policy {
                    if let Some((status, reasons)) =
                        policy.custom_extra_dose_hook(&active_series.name, &ctx)
                    {
                        let output_dose_number =
                            std::cmp::min(target_dose_idx, valid_doses.len() + 1);
                        if status == DoseStatus::Valid {
                            valid_doses.push((dose.date, target_dose_idx));
                        }
                        evaluations.push(DoseEvaluation {
                            dose_date: dose.date,
                            cvx: dose.cvx,
                            status,
                            reasons,
                            dose_number: Some(output_dose_number),
                            sources: std::collections::HashMap::new(),
                        });
                        eval_target_dose_numbers.push(target_dose_idx);
                        evaluation_orig_indices.push(sorted_history[i].0);
                        i += 1;
                        continue;
                    }
                }

                let output_dose_number = std::cmp::min(target_dose_idx, valid_doses.len() + 1);
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx,
                    status: DoseStatus::Accepted, // Mark accepted as extra dose
                    reasons: crate::reasons![EvaluationReason::BoosterDose],
                    dose_number: Some(output_dose_number),
                    sources: std::collections::HashMap::new(),
                });
                eval_target_dose_numbers.push(target_dose_idx);
                evaluation_orig_indices.push(sorted_history[i].0);
                i += 1;
                continue;
            }

            // Look up dose parameters
            let mut dose_rule = active_series.doses[target_dose_idx - 1].clone();
            let mut applicable_intervals: Vec<CompiledDoseInterval> = if target_dose_idx > 1 {
                let mut intervals: Vec<CompiledDoseInterval> = active_series
                    .intervals
                    .iter()
                    .filter(|int| int.to_dose == target_dose_idx)
                    .cloned()
                    .collect();
                let has_same_dose_prev =
                    evaluations
                        .iter()
                        .zip(&eval_target_dose_numbers)
                        .any(|(e, t_num)| {
                            *t_num == target_dose_idx
                                && !is_eval_ignored(
                                    active_series.vaccine_group,
                                    e.cvx,
                                    e.dose_date,
                                    e.status,
                                    Some(patient.birth_date),
                                )
                        });
                if has_same_dose_prev {
                    if let Some(int) = active_series
                        .intervals
                        .iter()
                        .find(|int| int.to_dose == target_dose_idx)
                    {
                        intervals.push(CompiledDoseInterval {
                            from_dose: target_dose_idx,
                            to_dose: target_dose_idx,
                            absolute_minimum_interval: int.absolute_minimum_interval.clone(),
                            minimum_interval: int.minimum_interval.clone(),
                            earliest_recommended_interval: int
                                .earliest_recommended_interval
                                .clone(),
                            latest_recommended_interval: int.latest_recommended_interval.clone(),
                        });
                    }
                }
                intervals
            } else {
                let has_prev_non_ignored_eval = evaluations.iter().any(|e| {
                    !is_eval_ignored(
                        active_series.vaccine_group,
                        e.cvx,
                        e.dose_date,
                        e.status,
                        Some(patient.birth_date),
                    )
                });
                if has_prev_non_ignored_eval {
                    active_series
                        .intervals
                        .iter()
                        .filter(|int| int.to_dose == 2 && int.from_dose == 1)
                        .map(|int| CompiledDoseInterval {
                            from_dose: 1,
                            to_dose: 1,
                            absolute_minimum_interval: int.absolute_minimum_interval.clone(),
                            minimum_interval: int.minimum_interval.clone(),
                            earliest_recommended_interval: int
                                .earliest_recommended_interval
                                .clone(),
                            latest_recommended_interval: int.latest_recommended_interval.clone(),
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            };

            // Apply parameter overrides (pre-2009 overrides, etc.)
            let ctx = EvaluationContext::new(
                patient,
                history,
                &valid_doses,
                Some(dose),
                target_dose_idx,
                eval_date,
                &active_series.name,
            );

            for rule in self.param_overrides {
                if rule.target_dose_number == target_dose_idx && (rule.condition)(&ctx) {
                    trace_decision!(
                        "parameter_override",
                        "Dose {} parameter override applied: {}",
                        target_dose_idx,
                        rule.description
                    );
                    if let Some(ref over_age) = rule.override_abs_min_age {
                        dose_rule.absolute_minimum_age = Some(over_age.clone());
                    }
                    if let Some((from_dose, ref over_int)) =
                        rule.override_abs_min_interval_from_dose
                    {
                        for int_rule in &mut applicable_intervals {
                            if int_rule.from_dose == from_dose {
                                int_rule.absolute_minimum_interval = Some(over_int.clone());
                            }
                        }
                    }
                }
            }

            let mut reasons = SmallVec::<[EvaluationReason; 4]>::new();
            let mut is_valid = true;

            // Check vaccine code eligibility
            if !dose_rule.allowed_cvx.contains(&dose.cvx.0) {
                is_valid = false;
                reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            }

            // Check minimum age
            if let Some(ref abs_min_age) = dose_rule.absolute_minimum_age {
                let abs_min_age_date = abs_min_age.add_to(patient.birth_date);
                let age_ok = compare_elapsed(patient.birth_date, dose.date, abs_min_age)
                    != std::cmp::Ordering::Less;
                trace_decision!(
                    "age_check",
                    "Dose {} cvx {} age check: date={} birth={} abs_min_age={:?} (date={}) ok={}",
                    target_dose_idx,
                    dose.cvx,
                    dose.date,
                    patient.birth_date,
                    abs_min_age,
                    abs_min_age_date,
                    age_ok
                );
                if !age_ok {
                    is_valid = false;
                    reasons.push(EvaluationReason::BelowMinimumAge);
                }
            }

            // Check minimum interval
            for int_rule in &applicable_intervals {
                if let Some(ref abs_min_int) = int_rule.absolute_minimum_interval {
                    let prev_date = evaluations
                        .iter()
                        .zip(&eval_target_dose_numbers)
                        .filter(|&(e, t_num)| {
                            *t_num == int_rule.from_dose
                                && !is_eval_ignored(
                                    active_series.vaccine_group,
                                    e.cvx,
                                    e.dose_date,
                                    e.status,
                                    Some(patient.birth_date),
                                )
                        })
                        .max_by_key(|&(e, _)| (e.status == DoseStatus::Valid, e.dose_date))
                        .map(|(e, _)| e.dose_date);
                    let retry_anchor_date =
                        if active_series.name == "DTP_3_DOSE_SERIES" && target_dose_idx > 1 {
                            evaluations
                                .iter()
                                .zip(&eval_target_dose_numbers)
                                .filter(|&(e, t_num)| {
                                    *t_num == target_dose_idx
                                        && e.status == DoseStatus::Invalid
                                        && e.dose_date < dose.date
                                        && !is_eval_ignored(
                                            active_series.vaccine_group,
                                            e.cvx,
                                            e.dose_date,
                                            e.status,
                                            Some(patient.birth_date),
                                        )
                                })
                                .map(|(e, _)| e.dose_date)
                                .max()
                        } else {
                            None
                        };
                    let prev_date = match (prev_date, retry_anchor_date) {
                        (Some(prev), Some(retry)) => Some(prev.max(retry)),
                        (Some(prev), None) => Some(prev),
                        (None, Some(retry)) => Some(retry),
                        (None, None) => None,
                    };
                    if let Some(prev_date) = prev_date {
                        let interval_ok = prev_date == dose.date || compare_elapsed(prev_date, dose.date, abs_min_int)
                            != std::cmp::Ordering::Less;
                        trace_decision!("interval_check", "Dose {} cvx {} interval check from dose {}: date={} prev_date={} abs_min_interval={:?} ok={}", target_dose_idx, dose.cvx, int_rule.from_dose, dose.date, prev_date, abs_min_int, interval_ok);
                        if !interval_ok {
                            is_valid = false;
                            if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                                reasons.push(EvaluationReason::BelowMinimumInterval);
                            }
                        }
                    } else {
                        trace_decision!("interval_check", "Dose {} cvx {} interval check from dose {}: no previous non-ignored dose found", target_dose_idx, dose.cvx, int_rule.from_dose);
                    }
                }
            }

            let mut status = if is_valid {
                DoseStatus::Valid
            } else {
                DoseStatus::Invalid
            };

            // Invoke custom_evaluation_hook after checking standard requirements
            if let Some(policy) = self.policy {
                let prev_status = status;
                let prev_reasons_len = reasons.len();
                policy.custom_evaluation_hook(
                    &active_series.name,
                    target_dose_idx,
                    &ctx,
                    &mut reasons,
                    &mut status,
                );
                if status != prev_status || reasons.len() != prev_reasons_len {
                    trace_decision!("custom_evaluation_hook", "Custom evaluation hook triggered for series={}: mutated status from {:?} to {:?} (reasons count: {} -> {})", active_series.name, prev_status, status, prev_reasons_len, reasons.len());
                }
            }

            let output_dose_number = std::cmp::min(target_dose_idx, valid_doses.len() + 1);

            if status == DoseStatus::Valid {
                valid_doses.push((dose.date, target_dose_idx));
            }

            evaluations.push(DoseEvaluation {
                dose_date: dose.date,
                cvx: dose.cvx,
                status,
                reasons,
                dose_number: Some(output_dose_number),
                sources: std::collections::HashMap::new(),
            });
            eval_target_dose_numbers.push(target_dose_idx);
            evaluation_orig_indices.push(sorted_history[i].0);

            let ctx_complete = EvaluationContext::new(
                patient,
                &sorted_history_doses,
                &valid_doses,
                Some(dose),
                target_dose_idx,
                eval_date,
                &active_series.name,
            );

            for rule in self.completion_rules {
                if (rule.condition)(&ctx_complete) {
                    is_completed = true;
                }
            }

            let series_completed = if let Some(res) = self
                .policy
                .and_then(|p| p.custom_completion_hook(&ctx_complete))
            {
                res
            } else {
                valid_doses
                    .iter()
                    .any(|(_, num)| *num == active_series.num_doses)
            };
            if series_completed {
                is_completed = true;
            }

            i += 1;
        }

        let series_completed = if let Some(res) = self.policy.and_then(|p| {
            let ctx_end = EvaluationContext::new(
                patient,
                &sorted_history_doses,
                &valid_doses,
                None,
                0,
                eval_date,
                &active_series.name,
            );
            p.custom_completion_hook(&ctx_end)
        }) {
            res
        } else {
            evaluations.iter().any(|e| {
                (e.status == DoseStatus::Valid
                    || (e.status == DoseStatus::Accepted
                        && !e.reasons.contains(&EvaluationReason::OutsideRoutineSeries)
                        && !e
                            .reasons
                            .contains(&EvaluationReason::VaccineNotLicensedForMales)))
                    && e.dose_number == Some(active_series.num_doses)
            })
        };

        let mut is_completed = is_completed || series_completed;
        if !is_completed
            && is_patient_immune_to_group(patient, active_series.vaccine_group, eval_date)
        {
            is_completed = true;
        }

        let satisfied_count = evaluations
            .iter()
            .filter(|e| {
                e.status == DoseStatus::Valid
                    || (e.status == DoseStatus::Accepted
                        && !e.reasons.contains(&EvaluationReason::OutsideRoutineSeries)
                        && !e
                            .reasons
                            .contains(&EvaluationReason::VaccineNotLicensedForMales))
            })
            .count();

        // 4. Recommendation / Forecasting
        let forecast = self.generate_forecast(
            patient,
            history,
            &valid_doses,
            &evaluations,
            satisfied_count,
            is_completed,
            eval_date,
            active_series,
        );

        let mut forecasts = SmallVec::new();
        forecasts.push(forecast);

        let mut ordered_evaluations = evaluations
            .into_iter()
            .zip(evaluation_orig_indices.into_iter())
            .collect::<SmallVec<[(DoseEvaluation, usize); 8]>>();
        ordered_evaluations.sort_by_key(|(_, orig_idx)| *orig_idx);
        let evaluations = ordered_evaluations
            .into_iter()
            .map(|(evaluation, _)| evaluation)
            .collect::<SmallVec<[DoseEvaluation; 8]>>();

        VaccineGroupForecast {
            vaccine_group: active_series.vaccine_group.into(),
            evaluations,
            forecasts,
            selected_series: Some(active_series.name.into()),
        }
    }

    fn generate_forecast(
        &self,
        patient: &Patient,
        history: &[Dose],
        valid_doses: &[(NaiveDate, usize)],
        evaluations: &[DoseEvaluation],
        _satisfied_count: usize,
        is_completed: bool,
        eval_date: NaiveDate,
        active_series: &CompiledSeries,
    ) -> SeriesForecast {
        if is_group_contraindicated(patient, active_series.vaccine_group, eval_date) {
            let mut f = SeriesForecast {
                series_name: active_series.name.into(),
                status: SeriesStatus::NotRecommended,
                reasons: crate::reasons!["CONTRAINDICATION"],
                sources: std::collections::HashMap::new(),
            };
            trace_decision!(
                "forecast_contraindicated",
                "Group {} is contraindicated",
                active_series.vaccine_group
            );
            if let Some(policy) = self.policy {
                policy.custom_forecast_hook(
                    patient,
                    valid_doses,
                    evaluations,
                    history,
                    eval_date,
                    &mut f,
                );
            }
            return f;
        }

        let mut forecast = if is_completed {
            let mut f = SeriesForecast {
                series_name: active_series.name.into(),
                status: SeriesStatus::Complete,
                reasons: crate::reasons!["COMPLETE"],
                sources: std::collections::HashMap::new(),
            };
            trace_decision!(
                "forecast_complete",
                "Series {} is completed",
                active_series.name
            );
            if let Some(policy) = self.policy {
                policy.custom_forecast_hook(
                    patient,
                    valid_doses,
                    evaluations,
                    history,
                    eval_date,
                    &mut f,
                );
            }
            f
        } else {
            let mut next_dose_idx = valid_doses.iter().map(|(_, num)| *num).max().unwrap_or(0) + 1;
            if let Some(policy) = self.policy {
                let ctx = EvaluationContext::new(
                    patient,
                    history,
                    valid_doses,
                    None,
                    next_dose_idx,
                    eval_date,
                    &active_series.name,
                );
                if let Some(num) = policy.custom_dose_number_hook(&active_series.name, &ctx) {
                    trace_decision!(
                        "forecast_custom_dose_number_hook",
                        "Custom dose number hook for series {} set next dose idx from {} to {}",
                        active_series.name,
                        next_dose_idx,
                        num
                    );
                    next_dose_idx = num;
                }
            }

            if next_dose_idx > active_series.num_doses {
                // If there's a custom completion hook and it says we're not done yet,
                // generate a placeholder forecast and let the custom_forecast_hook refine it.
                let has_completion_hook = self
                    .policy
                    .and_then(|p| {
                        let ctx = EvaluationContext::new(
                            patient,
                            history,
                            valid_doses,
                            None,
                            next_dose_idx,
                            eval_date,
                            &active_series.name,
                        );
                        p.custom_completion_hook(&ctx)
                    })
                    .is_some();

                if has_completion_hook && !is_completed {
                    let mut f = SeriesForecast {
                        series_name: active_series.name.into(),
                        status: SeriesStatus::default(),
                        reasons: crate::reasons!["NOT_COMPLETE"],
                        sources: std::collections::HashMap::new(),
                    };
                    trace_decision!("forecast_not_completed_awaiting_custom_hook", "Next dose idx {} > num_doses {} but custom completion hook exists; delegating to custom forecast hook", next_dose_idx, active_series.num_doses);
                    if let Some(policy) = self.policy {
                        policy.custom_forecast_hook(
                            patient,
                            valid_doses,
                            evaluations,
                            history,
                            eval_date,
                            &mut f,
                        );
                    }
                    return f;
                }
                let mut f = SeriesForecast {
                    series_name: active_series.name.into(),
                    status: SeriesStatus::Complete,
                    reasons: crate::reasons!["COMPLETE"],
                    sources: std::collections::HashMap::new(),
                };
                trace_decision!(
                    "forecast_complete_extra_doses",
                    "Next dose idx {} > num_doses {} without custom completion override",
                    next_dose_idx,
                    active_series.num_doses
                );
                if let Some(policy) = self.policy {
                    policy.custom_forecast_hook(
                        patient,
                        valid_doses,
                        evaluations,
                        history,
                        eval_date,
                        &mut f,
                    );
                }
                f
            } else {
                let mut dose_rule = active_series.doses[next_dose_idx - 1].clone();
                let mut applicable_intervals: Vec<CompiledDoseInterval> = if next_dose_idx > 1 {
                    active_series
                        .intervals
                        .iter()
                        .filter(|int| int.to_dose == next_dose_idx)
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                };

                let ctx = EvaluationContext::new(
                    patient,
                    history,
                    valid_doses,
                    None,
                    next_dose_idx,
                    eval_date,
                    &active_series.name,
                );

                // Apply recommendation overrides
                for rule in self.rec_overrides {
                    if rule.target_dose_number == next_dose_idx && (rule.condition)(&ctx) {
                        trace_decision!(
                            "forecast_recommendation_override",
                            "Dose {} recommendation override applied: {}",
                            next_dose_idx,
                            rule.description
                        );
                        if let Some(ref over_age) = rule.override_min_age {
                            dose_rule.minimum_age = Some(over_age.clone());
                        }
                        if let Some(ref over_int) = rule.override_min_interval {
                            for int_rule in &mut applicable_intervals {
                                int_rule.minimum_interval = Some(over_int.clone());
                            }
                        }
                    }
                }

                // Get the most recent shot date for any dose belonging to this series that is valid or invalid solely due to age/interval
                let last_shot_date = evaluations
                    .iter()
                    .filter(|e| {
                        e.status == DoseStatus::Valid
                            || e.status == DoseStatus::Accepted
                            || (e.status == DoseStatus::Invalid
                                && !e.reasons.is_empty()
                                && e.reasons.iter().all(|r| {
                                    *r == EvaluationReason::BelowMinimumAge
                                        || *r == EvaluationReason::BelowMinimumInterval
                                        || *r == EvaluationReason::TooEarlyLiveVirus
                                        || *r == EvaluationReason::BelowMinimumAgeFinalDose
                                        || *r == EvaluationReason::OutsideFluVacSeason
                                        || *r == EvaluationReason::AboveRecommendedAgeSeries
                                        || *r == EvaluationReason::VaccineNotAllowedInUs
                                        || *r == EvaluationReason::ContraindicatedVaccine
                                }))
                    })
                    .filter(|e| {
                        !self
                            .policy
                            .map(|p| p.ignore_evaluation_for_maximum_date(patient, e))
                            .unwrap_or(false)
                    })
                    .map(|e| e.dose_date)
                    .max();

                // Calculate Earliest Date: max(minimum_age, minimum_interval)
                let min_age_tp = dose_rule.minimum_age.as_ref();
                let min_age_date = min_age_tp.map(|tp| tp.add_to(patient.birth_date));
                trace_decision!(
                    "forecast_dates",
                    "Dose {} minimum age: {:?} (date={:?})",
                    next_dose_idx,
                    min_age_tp,
                    min_age_date
                );

                let get_dose_date = |dose_num: usize| -> Option<NaiveDate> {
                    valid_doses
                        .iter()
                        .find(|(_, num)| *num == dose_num)
                        .map(|(date, _)| *date)
                };

                let min_int_date = applicable_intervals
                    .iter()
                    .filter_map(|int_rule| {
                        int_rule.minimum_interval.as_ref().and_then(|tp| {
                            let date = if int_rule.from_dose == next_dose_idx - 1 {
                                // Sequential interval: measured from the most recent dose (valid or invalid)
                                last_shot_date.map(|d| tp.add_to(d))
                            } else {
                                // Non-sequential interval: measured ONLY from the valid from_dose
                                get_dose_date(int_rule.from_dose).map(|d| tp.add_to(d))
                            };
                            trace_decision!("forecast_dates", "Dose {} minimum interval from dose {}: {:?} (anchor_date={:?}, date={:?})", next_dose_idx, int_rule.from_dose, tp, if int_rule.from_dose == next_dose_idx - 1 { last_shot_date } else { get_dose_date(int_rule.from_dose) }, date);
                            date
                        })
                    })
                    .max();

                let earliest_date = match (min_age_date, min_int_date) {
                    (Some(a), Some(i)) => Some(max(a, i)),
                    (Some(a), None) => Some(a),
                    (None, Some(i)) => Some(i),
                    (None, None) => None,
                };
                trace_decision!(
                    "forecast_dates",
                    "Dose {} earliest date calculated: {:?}",
                    next_dose_idx,
                    earliest_date
                );

                // Calculate Recommended Date
                let rec_age_date = dose_rule
                    .earliest_recommended_age
                    .as_ref()
                    .map(|tp| tp.add_to(patient.birth_date));
                trace_decision!(
                    "forecast_dates",
                    "Dose {} earliest recommended age: {:?} (date={:?})",
                    next_dose_idx,
                    dose_rule.earliest_recommended_age,
                    rec_age_date
                );
                let rec_int_date = applicable_intervals
                    .iter()
                    .filter_map(|int_rule| {
                        int_rule
                            .earliest_recommended_interval
                            .as_ref()
                            .and_then(|tp| {
                                let date = if int_rule.from_dose == next_dose_idx - 1 {
                                    last_shot_date.map(|d| tp.add_to(d))
                                } else {
                                    get_dose_date(int_rule.from_dose).map(|d| tp.add_to(d))
                                };
                                trace_decision!("forecast_dates", "Dose {} earliest recommended interval from dose {}: {:?} (anchor={:?}, date={:?})", next_dose_idx, int_rule.from_dose, tp, if int_rule.from_dose == next_dose_idx - 1 { last_shot_date } else { get_dose_date(int_rule.from_dose) }, date);
                                date
                            })
                    })
                    .max();

                let recommended_date = match (rec_age_date, rec_int_date) {
                    (Some(a), Some(i)) => Some(max(a, i)),
                    (Some(a), None) => Some(a),
                    (None, Some(i)) => Some(i),
                    (None, None) => None,
                };
                trace_decision!(
                    "forecast_dates",
                    "Dose {} recommended date calculated: {:?}",
                    next_dose_idx,
                    recommended_date
                );

                // Calculate Overdue Date
                let overdue_age_date = dose_rule
                    .latest_recommended_age
                    .as_ref()
                    .map(|tp| tp.add_to(patient.birth_date));
                trace_decision!(
                    "forecast_dates",
                    "Dose {} latest recommended age: {:?} (date={:?})",
                    next_dose_idx,
                    dose_rule.latest_recommended_age,
                    overdue_age_date
                );
                let overdue_int_date = applicable_intervals
                    .iter()
                    .filter_map(|int_rule| {
                        int_rule
                            .latest_recommended_interval
                            .as_ref()
                            .and_then(|tp| {
                                let date = if int_rule.from_dose == next_dose_idx - 1 {
                                    last_shot_date.map(|d| tp.add_to(d))
                                } else {
                                    get_dose_date(int_rule.from_dose).map(|d| tp.add_to(d))
                                };
                                trace_decision!("forecast_dates", "Dose {} latest recommended interval from dose {}: {:?} (anchor={:?}, date={:?})", next_dose_idx, int_rule.from_dose, tp, if int_rule.from_dose == next_dose_idx - 1 { last_shot_date } else { get_dose_date(int_rule.from_dose) }, date);
                                date
                            })
                    })
                    .max();

                let overdue_date = if overdue_age_date.is_some() {
                    overdue_age_date
                } else {
                    overdue_int_date
                };
                let overdue_date = overdue_date.and_then(|d| d.pred_opt());
                trace_decision!(
                    "forecast_dates",
                    "Dose {} overdue date calculated (pred_opt): {:?}",
                    next_dose_idx,
                    overdue_date
                );

                let mut f = SeriesForecast {
                    series_name: active_series.name.into(),
                    status: crate::models::SeriesStatus::NotComplete {
                        earliest_date,
                        recommended_date,
                        overdue_date,
                        latest_date: None,
                    },
                    reasons: crate::reasons!["NOT_COMPLETE"],
                    sources: std::collections::HashMap::new(),
                };

                // Apply max age clamp if configured
                if let Some((max_age, status)) = &active_series.max_age_clamp {
                    let clamp_date = max_age.add_to(patient.birth_date);
                    if eval_date >= clamp_date {
                        if !matches!(f.status, SeriesStatus::Complete) {
                            trace_decision!("forecast_max_age_clamp", "Max age clamp triggered: eval_date={:?} >= max_age_date={:?}, shifting status from {:?} to {:?}", eval_date, clamp_date, f.status, status);
                            set_field!(f, status, status.clone());
                            f.status = f.status.with_earliest_date(None);
                            f.status = f.status.with_recommended_date(None);
                            f.status = f.status.with_overdue_date(None);
                            f.status = f.status.with_latest_date(None);
                            f.reasons = crate::reasons!["MAX_AGE_EXCEEDED"];
                        }
                    }
                }

                // Apply custom rules hook (like Polio 2009 reset) if present
                if let Some(policy) = self.policy {
                    let prev_status = f.status.clone();
                    policy.custom_forecast_hook(
                        patient,
                        valid_doses,
                        evaluations,
                        history,
                        eval_date,
                        &mut f,
                    );
                    if f.status != prev_status {
                        trace_decision!(
                            "forecast_custom_forecast_hook",
                            "Custom forecast hook mutated forecast status from {:?} to {:?}",
                            prev_status,
                            f.status
                        );
                    }
                }

                f
            }
        };

        if let crate::models::SeriesStatus::NotComplete {
            ref mut earliest_date,
            ref mut recommended_date,
            ref mut overdue_date,
            ..
        } = forecast.status
        {
            let last_group_date = history.iter().map(|d| d.date).max();
            if let Some(last_date) = last_group_date {
                if let Some(earliest) = earliest_date {
                    if *earliest < last_date {
                        *earliest = last_date;
                    }
                }
                if let Some(recommended) = recommended_date {
                    if *recommended < last_date {
                        *recommended = last_date;
                    }
                }
                if let Some(overdue) = overdue_date {
                    if *overdue < last_date {
                        *overdue = last_date;
                    }
                }
            }

            // Align earliest <= recommended <= overdue
            if let (Some(earliest), Some(recommended)) = (*earliest_date, recommended_date.as_mut())
            {
                if *recommended < earliest {
                    *recommended = earliest;
                }
            }
            if let (Some(recommended), Some(overdue)) = (*recommended_date, overdue_date.as_mut()) {
                if *overdue < recommended {
                    *overdue = recommended;
                }
            }
            if let (Some(earliest), Some(overdue)) = (*earliest_date, overdue_date.as_mut()) {
                if *overdue < earliest {
                    *overdue = earliest;
                }
            }
        }

        forecast
    }
}

fn get_same_day_priority(
    group: &str,
    cvx: Cvx,
    birth_date: NaiveDate,
    dose_date: NaiveDate,
    dtp_preserve_196_source_order: bool,
) -> i32 {
    let cvx_code = cvx.0;
    match group {
        "MMR" => match cvx_code {
            94 => 0,
            3 => 1,
            4 | 5 => 2,
            _ => 3,
        },
        "POLIO" => match cvx_code {
            2 | 182 => 1,
            _ => 0,
        },
        "HEP_A" => {
            let age_19 = crate::date_utils::add_years_unchecked(birth_date, 19);
            if dose_date >= age_19 {
                if cvx_code == 52 {
                    0
                } else {
                    1
                }
            } else {
                if cvx_code == 52 {
                    1
                } else {
                    0
                }
            }
        }
        "VARICELLA" => match cvx_code {
            94 => 0,
            21 => 1,
            _ => 2,
        },
        "DTP" => match cvx_code {
            196 if dtp_preserve_196_source_order => 0, // observed no-prior-dose exception
            196 => 1,
            9 | 28 | 113 | 138 | 139 | 195 => 1, // DT/Td
            _ => 0,                              // Pertussis-containing DTP/DTaP/Tdap
        },
        "HEP_B" => match cvx_code {
            104 | 110 | 146 => 0,
            _ => 1,
        },
        "MENB" => {
            let policy_change = NaiveDate::from_ymd_opt(2024, 10, 25).unwrap();
            if dose_date < policy_change {
                match cvx_code {
                    163 | 328 => 0,
                    162 | 316 => 1,
                    _ => 2,
                }
            } else {
                0
            }
        }
        "ROTAVIRUS" => {
            let policy_change = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
            if dose_date >= policy_change {
                match cvx_code {
                    119 => 1,
                    74 => 2,
                    _ => 0,
                }
            } else {
                match cvx_code {
                    74 => 0,
                    119 => 2,
                    _ => 1,
                }
            }
        }
        "MPOX" => match cvx_code {
            206 => 0,
            75 | 105 => 1,
            325 => 2,
            _ => 3,
        },
        "HPV" => match cvx_code {
            118 => 0, // Cervarix (female-only) — lowest priority
            62 => 1,  // Gardasil
            137 => 2, // Gardasil (Shire)
            165 => 3, // Gardasil 9 — highest priority
            _ => 0,
        },
        "INFLUENZA" => {
            let is_disallowed = matches!(cvx_code, 194 | 200 | 201 | 202 | 231 | 331 | 337);
            let is_nos = matches!(cvx_code, 88 | 151);
            if is_disallowed {
                100
            } else if is_nos {
                90
            } else {
                20
            }
        }
        "RSV" => match cvx_code {
            304 | 314 | 315 => 1,
            _ => 0,
        },
        _ => 0,
    }
}

fn should_preserve_dtp_196_source_order(
    first_relevant_date: Option<NaiveDate>,
    dose_date: NaiveDate,
) -> bool {
    first_relevant_date == Some(dose_date)
}

fn is_patient_immune_to_group(
    patient: &crate::models::Patient,
    group: &str,
    eval_date: NaiveDate,
) -> bool {
    let group_lower = group.to_lowercase();
    patient.immunities.iter().any(|imm| {
        let imm_disease_lower = imm.disease.to_lowercase();
        let matches_group = if group_lower.contains("hepb")
            || group_lower.contains("hep_b")
            || group_lower.contains("hep b")
            || group_lower.contains("hepatitis b")
        {
            imm_disease_lower.contains("hepb")
                || imm_disease_lower.contains("hep_b")
                || imm_disease_lower.contains("hep b")
                || imm_disease_lower.contains("hepatitis b")
        } else if group_lower.contains("varicella") {
            imm_disease_lower.contains("varicella") || imm_disease_lower.contains("chickenpox")
        } else if group_lower.contains("measles") {
            imm_disease_lower.contains("measles") || imm_disease_lower.contains("rubeola")
        } else if group_lower.contains("mumps") {
            imm_disease_lower.contains("mumps")
        } else if group_lower.contains("rubella") {
            imm_disease_lower.contains("rubella") || imm_disease_lower.contains("german measles")
        } else if group_lower.contains("mmr") {
            imm_disease_lower.contains("mmr")
                || imm_disease_lower.contains("measles")
                || imm_disease_lower.contains("mumps")
                || imm_disease_lower.contains("rubella")
        } else {
            imm_disease_lower == group_lower
        };

        matches_group && eval_date >= imm.date
    })
}

fn is_group_contraindicated(
    patient: &crate::models::Patient,
    group: &str,
    eval_date: NaiveDate,
) -> bool {
    let group_lower = group.to_lowercase();
    patient.contraindications.iter().any(|c| {
        if c.cvx.is_some() {
            return false;
        }
        let target_lower = c.target.to_lowercase();
        let matches_group = if group_lower.contains("dtp")
            || group_lower.contains("dtap")
            || group_lower.contains("dt")
            || group_lower.contains("tdap")
            || group_lower.contains("td")
            || group_lower.contains("diphtheria")
            || group_lower.contains("tetanus")
            || group_lower.contains("pertussis")
        {
            target_lower.contains("dtp")
                || target_lower.contains("dtap")
                || target_lower.contains("dt")
                || target_lower.contains("tdap")
                || target_lower.contains("td")
                || target_lower.contains("diphtheria")
                || target_lower.contains("tetanus")
                || target_lower.contains("pertussis")
        } else {
            target_lower == group_lower
        };
        let active = eval_date >= c.date && c.valid_until.map_or(true, |until| eval_date < until);
        matches_group && active
    })
}

fn is_dose_contraindicated(
    patient: &crate::models::Patient,
    cvx: Cvx,
    date: NaiveDate,
    group: &str,
) -> bool {
    let group_lower = group.to_lowercase();
    patient.contraindications.iter().any(|c| {
        let active = date >= c.date && c.valid_until.map_or(true, |until| date < until);
        if !active {
            return false;
        }
        if let Some(c_cvx) = c.cvx {
            if c_cvx == cvx {
                return true;
            }
        } else {
            let target_lower = c.target.to_lowercase();
            let matches_group = if group_lower.contains("dtp")
                || group_lower.contains("dtap")
                || group_lower.contains("dt")
                || group_lower.contains("tdap")
                || group_lower.contains("td")
                || group_lower.contains("diphtheria")
                || group_lower.contains("tetanus")
                || group_lower.contains("pertussis")
            {
                target_lower.contains("dtp")
                    || target_lower.contains("dtap")
                    || target_lower.contains("dt")
                    || target_lower.contains("tdap")
                    || target_lower.contains("td")
                    || target_lower.contains("diphtheria")
                    || target_lower.contains("tetanus")
                    || target_lower.contains("pertussis")
            } else {
                target_lower == group_lower
            };
            if matches_group {
                return true;
            }
        }
        false
    })
}

pub fn is_eval_ignored(
    group: &str,
    cvx: Cvx,
    date: NaiveDate,
    status: DoseStatus,
    birth_date: Option<NaiveDate>,
) -> bool {
    if status == DoseStatus::Ignored || status == DoseStatus::Accepted {
        return true;
    }
    if group == "POLIO" {
        let is_cvx_178_179 = cvx.0 == 178 || cvx.0 == 179;
        let is_cvx_182_after_2016 =
            cvx.0 == 182 && date >= NaiveDate::from_ymd_opt(2016, 4, 1).unwrap();
        if is_cvx_178_179 || is_cvx_182_after_2016 {
            return true;
        }
    }
    if group == "DTP" {
        // Td (CVX 09, 113, 138, 139, 196) under 7 years - 4 days
        let is_td = cvx.0 == 9 || cvx.0 == 113 || cvx.0 == 138 || cvx.0 == 139 || cvx.0 == 196;
        // Tdap CVX 115 under 7 years - 4 days. CVX 198 can be a valid
        // child-series DTaP-containing dose and should still anchor intervals.
        let is_tdap = cvx.0 == 115;
        if is_td || is_tdap {
            if let Some(birth) = birth_date {
                let age_7_minus_4d =
                    crate::date_utils::add_years_unchecked(birth, 7) - chrono::Duration::days(4);
                if date < age_7_minus_4d {
                    return true;
                }
            }
        }
    }
    false
}
