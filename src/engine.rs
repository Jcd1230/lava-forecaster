use crate::date_utils::{compare_elapsed, TimePeriod, TinyVec};
use crate::models::{
    Cvx, Dose, DoseEvaluation, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus,
    VaccineGroupForecast,
};
use crate::schedule::{CompiledDoseInterval, CompiledSeries};
use chrono::NaiveDate;
use std::cmp::max;

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
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
);

pub type CustomDoseNumberHook = fn(series_name: &str, ctx: &EvaluationContext) -> usize;

pub type CustomExtraDoseHook = fn(
    series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, TinyVec<EvaluationReason, 4>)>;

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
        _history: &[Dose],
        _eval_date: NaiveDate,
        _forecast: &mut SeriesForecast,
    ) {}

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
        _reasons: &mut TinyVec<EvaluationReason, 4>,
        _status: &mut DoseStatus,
    ) {}

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
    ) -> Option<(DoseStatus, TinyVec<EvaluationReason, 4>)> {
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
        _sorted_history_subset.iter().any(|prev_dose| prev_dose.date == _dose.date)
    }

    fn ignore_evaluation_for_maximum_date(&self, _patient: &Patient, _eval: &crate::models::DoseEvaluation) -> bool {
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
        let mut sorted_history = TinyVec::<Dose, 32>::new();
        for dose in history {
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
                sorted_history.push(*dose);
            }
        }
        sorted_history.sort_by(|a, b| {
            if a.date != b.date {
                a.date.cmp(&b.date)
            } else {
                let group = &self.series.vaccine_group;
                let prio_a = get_same_day_priority(group, a.cvx, patient.birth_date, a.date);
                let prio_b = get_same_day_priority(group, b.cvx, patient.birth_date, b.date);
                prio_a.cmp(&prio_b)
            }
        });

        let mut evaluations = TinyVec::<DoseEvaluation, 8>::new();
        let mut valid_doses = TinyVec::<(NaiveDate, usize), 32>::new();
        let mut is_completed = false;
        let mut active_series: &'a CompiledSeries = self.series;

        // 2. Chronological dose evaluation loop
        let mut i = 0;
        while i < sorted_history.len() {
            let dose = &sorted_history[i];
            let mut target_dose_idx =
                valid_doses.iter().map(|(_, num)| *num).max().unwrap_or(0) + 1;
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
            let is_duplicate = if i > 0 {
                let history_subset = &sorted_history[0..i];
                self.policy
                    .map(|p| p.is_same_day_duplicate(dose, history_subset))
                    .unwrap_or_else(|| history_subset.iter().any(|prev| prev.date == dose.date))
            } else {
                false
            };
            if is_duplicate {
                let prev_dose_num = evaluations
                    .last()
                    .and_then(|e| e.dose_number)
                    .unwrap_or(target_dose_idx);
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx,
                    status: DoseStatus::Invalid,
                    reasons: crate::reasons![EvaluationReason::DuplicateShotSameDay],
                    dose_number: Some(prev_dose_num),
                });
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
                });
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
                    if let Some((status, reasons)) = policy.custom_extra_dose_hook(&active_series.name, &ctx) {
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
                        });
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
                });
                i += 1;
                continue;
            }

            // Look up dose parameters
            let mut dose_rule = active_series.doses[target_dose_idx - 1].clone();
            let mut applicable_intervals: Vec<CompiledDoseInterval> = if target_dose_idx > 1 {
                active_series
                    .intervals
                    .iter()
                    .filter(|int| int.to_dose == target_dose_idx)
                    .cloned()
                    .collect()
            } else {
                Vec::new()
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

            let mut reasons = TinyVec::<EvaluationReason, 4>::new();
            let mut is_valid = true;

            // Check vaccine code eligibility
            if !dose_rule.allowed_cvx.contains(&dose.cvx.0) {
                is_valid = false;
                reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            }

            // Check minimum age
            if let Some(ref abs_min_age) = dose_rule.absolute_minimum_age {
                if compare_elapsed(patient.birth_date, dose.date, abs_min_age)
                    == std::cmp::Ordering::Less
                {
                    is_valid = false;
                    reasons.push(EvaluationReason::BelowMinimumAge);
                }
            }

            // Check minimum interval
            for int_rule in &applicable_intervals {
                if let Some(ref abs_min_int) = int_rule.absolute_minimum_interval {
                    let prev_date = valid_doses
                        .iter()
                        .find(|(_, num)| *num == int_rule.from_dose)
                        .map(|(d, _)| *d);
                    if let Some(prev_date) = prev_date {
                        if compare_elapsed(prev_date, dose.date, abs_min_int)
                            == std::cmp::Ordering::Less
                        {
                            is_valid = false;
                            if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                                reasons.push(EvaluationReason::BelowMinimumInterval);
                            }
                        }
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
                policy.custom_evaluation_hook(
                    &active_series.name,
                    target_dose_idx,
                    &ctx,
                    &mut reasons,
                    &mut status,
                );
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
            });

            let ctx_complete = EvaluationContext::new(
                patient,
                &sorted_history,
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

            let series_completed = if let Some(res) = self.policy.and_then(|p| p.custom_completion_hook(&ctx_complete)) {
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
                &sorted_history,
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

        let is_completed = is_completed || series_completed;

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

        let mut forecasts = TinyVec::new();
        forecasts.push(forecast);

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
        let forecast = if is_completed {
            let mut f = SeriesForecast {
                series_name: active_series.name.into(),
                
                status: SeriesStatus::Complete,
                reasons: crate::reasons!["COMPLETE"],
            };
            if let Some(policy) = self.policy {
                policy.custom_forecast_hook(patient, valid_doses, history, eval_date, &mut f);
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
                    next_dose_idx = num;
                }
            }

            if next_dose_idx > active_series.num_doses {
                // If there's a custom completion hook and it says we're not done yet,
                // generate a placeholder forecast and let the custom_forecast_hook refine it.
                let has_completion_hook = self.policy.and_then(|p| {
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
                }).is_some();

                if has_completion_hook && !is_completed {
                    let mut f = SeriesForecast {
                        series_name: active_series.name.into(),
                        
                        status: SeriesStatus::default(),
                        reasons: crate::reasons!["NOT_COMPLETE"],
                    };
                    if let Some(policy) = self.policy {
                        policy.custom_forecast_hook(patient, valid_doses, history, eval_date, &mut f);
                    }
                    return f;
                }
                let mut f = SeriesForecast {
                    series_name: active_series.name.into(),
                    
                    status: SeriesStatus::Complete,
                    reasons: crate::reasons!["COMPLETE"],
                };
                if let Some(policy) = self.policy {
                    policy.custom_forecast_hook(patient, valid_doses, history, eval_date, &mut f);
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
                            || (e.status == DoseStatus::Invalid
                                && !e.reasons.is_empty()
                                && e.reasons.iter().all(|r| {
                                    *r == EvaluationReason::BelowMinimumAge
                                        || *r == EvaluationReason::BelowMinimumInterval
                                        || *r == EvaluationReason::TooEarlyLiveVirus
                                }))
                    })
                    .filter(|e| {
                        !self.policy.map(|p| p.ignore_evaluation_for_maximum_date(patient, e)).unwrap_or(false)
                    })
                    .map(|e| e.dose_date)
                    .max();

                // Calculate Earliest Date: max(minimum_age, minimum_interval)
                let min_age_tp = dose_rule.minimum_age.as_ref();
                let min_age_date = min_age_tp.map(|tp| tp.add_to(patient.birth_date));

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
                            if int_rule.from_dose == next_dose_idx - 1 {
                                // Sequential interval: measured from the most recent dose (valid or invalid)
                                last_shot_date.map(|d| tp.add_to(d))
                            } else {
                                // Non-sequential interval: measured ONLY from the valid from_dose
                                get_dose_date(int_rule.from_dose).map(|d| tp.add_to(d))
                            }
                        })
                    })
                    .max();

                let earliest_date = match (min_age_date, min_int_date) {
                    (Some(a), Some(i)) => Some(max(a, i)),
                    (Some(a), None) => Some(a),
                    (None, Some(i)) => Some(i),
                    (None, None) => None,
                };

                // Calculate Recommended Date
                let rec_age_date = dose_rule
                    .earliest_recommended_age
                    .as_ref()
                    .map(|tp| tp.add_to(patient.birth_date));
                let rec_int_date = applicable_intervals
                    .iter()
                    .filter_map(|int_rule| {
                        int_rule
                            .earliest_recommended_interval
                            .as_ref()
                            .and_then(|tp| {
                                if int_rule.from_dose == next_dose_idx - 1 {
                                    last_shot_date.map(|d| tp.add_to(d))
                                } else {
                                    get_dose_date(int_rule.from_dose).map(|d| tp.add_to(d))
                                }
                            })
                    })
                    .max();

                let recommended_date = match (rec_age_date, rec_int_date) {
                    (Some(a), Some(i)) => Some(max(a, i)),
                    (Some(a), None) => Some(a),
                    (None, Some(i)) => Some(i),
                    (None, None) => None,
                };

                // Calculate Overdue Date
                let overdue_age_date = dose_rule
                    .latest_recommended_age
                    .as_ref()
                    .map(|tp| tp.add_to(patient.birth_date));
                let overdue_int_date = applicable_intervals
                    .iter()
                    .filter_map(|int_rule| {
                        int_rule
                            .latest_recommended_interval
                            .as_ref()
                            .and_then(|tp| {
                                if int_rule.from_dose == next_dose_idx - 1 {
                                    last_shot_date.map(|d| tp.add_to(d))
                                } else {
                                    get_dose_date(int_rule.from_dose).map(|d| tp.add_to(d))
                                }
                            })
                    })
                    .max();

                let overdue_date = if overdue_age_date.is_some() {
                    overdue_age_date
                } else {
                    overdue_int_date
                };
                let overdue_date = overdue_date.and_then(|d| d.pred_opt());

                let mut f = SeriesForecast {
                    series_name: active_series.name.into(),
                    status: crate::models::SeriesStatus::NotComplete { earliest_date, recommended_date, overdue_date, latest_date: None },
                    reasons: crate::reasons!["NOT_COMPLETE"],
                };

                // Apply max age clamp if configured
                if let Some((max_age, status)) = &active_series.max_age_clamp {
                    if eval_date >= max_age.add_to(patient.birth_date) {
                        if !matches!(f.status, SeriesStatus::Complete) {
                            f.status = status.clone();
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
                    policy.custom_forecast_hook(patient, valid_doses, history, eval_date, &mut f);
                }

                f
            }
        };

        let last_group_date = history.iter().map(|d| d.date).max();
        if let Some(last_date) = last_group_date {
            if let Some(ref mut earliest) = forecast.status.earliest_date() {
                if *earliest < last_date {
                    *earliest = last_date;
                }
            }
            if let Some(ref mut recommended) = forecast.status.recommended_date() {
                if *recommended < last_date {
                    *recommended = last_date;
                }
            }
            if let Some(ref mut overdue) = forecast.status.overdue_date() {
                if *overdue < last_date {
                    *overdue = last_date;
                }
            }
        }

        // Align earliest <= recommended <= overdue
        if let (Some(earliest), Some(recommended)) =
            (forecast.status.earliest_date(), forecast.status.recommended_date().as_mut())
        {
            if *recommended < earliest {
                *recommended = earliest;
            }
        }
        if let (Some(recommended), Some(overdue)) =
            (forecast.status.recommended_date(), forecast.status.overdue_date().as_mut())
        {
            if *overdue < recommended {
                *overdue = recommended;
            }
        }
        if let (Some(earliest), Some(overdue)) =
            (forecast.status.earliest_date(), forecast.status.overdue_date().as_mut())
        {
            if *overdue < earliest {
                *overdue = earliest;
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
            115 | 198 => 1,                            // Tdap
            9 | 28 | 113 | 138 | 139 | 195 | 196 => 2, // DT/Td
            _ => 0,                                    // DTaP/DTP combo/single
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
        _ => 0,
    }
}
