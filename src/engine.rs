use chrono::NaiveDate;
use crate::models::{
    Dose, DoseEvaluation, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus,
    Patient, VaccineGroupForecast
};
use crate::schedule::CompiledSeries;
use crate::date_utils::{TimePeriod, compare_elapsed};
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
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
);

pub type CustomDoseNumberHook = fn(
    series_name: &str,
    ctx: &EvaluationContext,
) -> usize;

pub type GroupSelectionAndPostProcess = fn(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut std::collections::HashMap<String, VaccineGroupForecast>,
) -> String;

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

pub struct EvaluationEngine {
    pub series: CompiledSeries,
    pub param_overrides: Vec<ParameterOverrideRule>,
    pub completion_rules: Vec<ConditionalCompletionRule>,
    pub rec_overrides: Vec<RecommendationOverrideRule>,
    pub custom_forecast_hook: Option<CustomForecastHook>,
    pub custom_switch_hook: Option<CustomSwitchHook>,
    pub custom_evaluation_hook: Option<CustomEvaluationHook>,
    pub custom_dose_number_hook: Option<CustomDoseNumberHook>,
}

impl EvaluationEngine {
    pub fn new(series: CompiledSeries) -> Self {
        EvaluationEngine {
            series,
            param_overrides: Vec::new(),
            completion_rules: Vec::new(),
            rec_overrides: Vec::new(),
            custom_forecast_hook: None,
            custom_switch_hook: None,
            custom_evaluation_hook: None,
            custom_dose_number_hook: None,
        }
    }

    pub fn evaluate_patient(
        &self,
        patient: &Patient,
        history: &[Dose],
        eval_date: NaiveDate,
        group_series: &[CompiledSeries],
    ) -> VaccineGroupForecast {
        // 1. Filter and sort history chronologically (only keep doses relevant to this series or group)
        let mut sorted_history: Vec<Dose> = history.iter()
            .filter(|dose| {
                if group_series.is_empty() {
                    self.series.doses.iter().any(|d_rule| d_rule.allowed_cvx.contains(&dose.cvx))
                } else {
                    group_series.iter().any(|s| {
                        s.doses.iter().any(|d_rule| d_rule.allowed_cvx.contains(&dose.cvx))
                    })
                }
            })
            .cloned()
            .collect();
        sorted_history.sort_by(|a, b| {
            if a.date != b.date {
                a.date.cmp(&b.date)
            } else {
                let group = &self.series.vaccine_group;
                let prio_a = get_same_day_priority(group, &a.cvx, patient.birth_date, a.date);
                let prio_b = get_same_day_priority(group, &b.cvx, patient.birth_date, b.date);
                prio_a.cmp(&prio_b)
            }
        });

        let mut evaluations: Vec<DoseEvaluation> = Vec::new();
        let mut valid_doses: Vec<(NaiveDate, usize)> = Vec::new(); // (date, dose_number_in_series)
        let mut is_completed = false;
        let mut active_series = self.series.clone();

        // 2. Chronological dose evaluation loop
        let mut i = 0;
        while i < sorted_history.len() {
            let dose = &sorted_history[i];
            let mut target_dose_idx = valid_doses.iter().map(|(_, num)| *num).max().unwrap_or(0) + 1;
            if let Some(hook) = self.custom_dose_number_hook {
                let ctx = EvaluationContext::new(
                    patient,
                    history,
                    &valid_doses,
                    Some(dose),
                    target_dose_idx,
                    eval_date,
                    &active_series.name,
                );
                target_dose_idx = (hook)(&active_series.name, &ctx);
            }

            // Same-day duplicate check
            let is_duplicate = i > 0 && sorted_history[i - 1].date == dose.date;
            if is_duplicate {
                let prev_dose_num = evaluations.last().and_then(|e| e.dose_number).unwrap_or(target_dose_idx);
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx.clone(),
                    status: DoseStatus::Invalid,
                    reasons: vec![EvaluationReason::DuplicateShotSameDay],
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
                    cvx: dose.cvx.clone(),
                    status: DoseStatus::Invalid,
                    reasons: vec![EvaluationReason::PriorToDOB],
                    dose_number: Some(output_dose_number),
                });
                i += 1;
                continue;
            }

            // Apply custom switch hook if present
            if let Some(switch_hook) = self.custom_switch_hook {
                let ctx = EvaluationContext::new(
                    patient,
                    history,
                    &valid_doses,
                    Some(dose),
                    target_dose_idx,
                    eval_date,
                    &active_series.name,
                );
                if let Some(new_series_name) = (switch_hook)(&active_series.name, target_dose_idx, &ctx) {
                    if let Some(new_series) = group_series.iter().find(|s| s.name == new_series_name) {
                        active_series = new_series.clone();
                    }
                }
            }

            // If patient already completed the series, additional doses are boosters/extra
            if is_completed || target_dose_idx > active_series.num_doses {
                let output_dose_number = std::cmp::min(target_dose_idx, valid_doses.len() + 1);
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx.clone(),
                    status: DoseStatus::Accepted, // Mark accepted as extra dose
                    reasons: vec![EvaluationReason::BoosterDose],
                    dose_number: Some(output_dose_number),
                });
                i += 1;
                continue;
            }

            // Look up dose parameters
            let mut dose_rule = active_series.doses[target_dose_idx - 1].clone();
            let mut interval_rule = if target_dose_idx > 1 {
                active_series.intervals.iter()
                    .find(|int| int.to_dose == target_dose_idx)
                    .cloned()
            } else {
                None
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

            for rule in &self.param_overrides {
                if rule.target_dose_number == target_dose_idx && (rule.condition)(&ctx) {
                    if let Some(ref over_age) = rule.override_abs_min_age {
                        dose_rule.absolute_minimum_age = Some(over_age.clone());
                    }
                    if let Some((from_dose, ref over_int)) = rule.override_abs_min_interval_from_dose {
                        if let Some(ref mut int_rule) = interval_rule {
                            if int_rule.from_dose == from_dose {
                                int_rule.absolute_minimum_interval = Some(over_int.clone());
                            }
                        }
                    }
                }
            }

            let mut reasons = Vec::new();
            let mut is_valid = true;

            // Check vaccine code eligibility
            if !dose_rule.allowed_cvx.contains(&dose.cvx) {
                is_valid = false;
                reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            }

            // Check minimum age
            if let Some(ref abs_min_age) = dose_rule.absolute_minimum_age {
                if compare_elapsed(patient.birth_date, dose.date, abs_min_age) == std::cmp::Ordering::Less {
                    is_valid = false;
                    reasons.push(EvaluationReason::BelowMinimumAge);
                }
            }

            // Check minimum interval
            if let Some(ref int_rule) = interval_rule {
                if let Some(ref abs_min_int) = int_rule.absolute_minimum_interval {
                    if let Some((prev_date, _)) = valid_doses.last() {
                        if compare_elapsed(*prev_date, dose.date, abs_min_int) == std::cmp::Ordering::Less {
                            is_valid = false;
                            reasons.push(EvaluationReason::BelowMinimumInterval);
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
            if let Some(eval_hook) = self.custom_evaluation_hook {
                (eval_hook)(&active_series.name, target_dose_idx, &ctx, &mut reasons, &mut status);
            }

            let output_dose_number = std::cmp::min(target_dose_idx, valid_doses.len() + 1);

            if status == DoseStatus::Valid {
                valid_doses.push((dose.date, target_dose_idx));
            }

            evaluations.push(DoseEvaluation {
                dose_date: dose.date,
                cvx: dose.cvx.clone(),
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

            for rule in &self.completion_rules {
                if (rule.condition)(&ctx_complete) {
                    is_completed = true;
                }
            }

            if valid_doses.len() == active_series.num_doses {
                is_completed = true;
            }

            i += 1;
        }

        let satisfied_count = evaluations.iter()
            .filter(|e| {
                e.status == DoseStatus::Valid || 
                (e.status == DoseStatus::Accepted && 
                 !e.reasons.contains(&EvaluationReason::OutsideRoutineSeries) &&
                 !e.reasons.contains(&EvaluationReason::VaccineNotLicensedForMales))
            })
            .count();

        let is_completed = is_completed || satisfied_count >= active_series.num_doses;

        // 4. Recommendation / Forecasting
        let forecast = self.generate_forecast(
            patient,
            history,
            &valid_doses,
            satisfied_count,
            is_completed,
            eval_date,
            &active_series,
        );

        VaccineGroupForecast {
            vaccine_group: active_series.vaccine_group.clone(),
            evaluations,
            forecasts: vec![forecast],
            selected_series: Some(active_series.name.clone()),
        }
    }

    fn generate_forecast(
        &self,
        patient: &Patient,
        history: &[Dose],
        valid_doses: &[(NaiveDate, usize)],
        satisfied_count: usize,
        is_completed: bool,
        eval_date: NaiveDate,
        active_series: &CompiledSeries,
    ) -> SeriesForecast {
        if is_completed {
            let mut forecast = SeriesForecast {
                series_name: active_series.name.clone(),
                earliest_date: None,
                recommended_date: None,
                overdue_date: None,
                latest_date: None,
                status: SeriesStatus::Complete,
                reasons: vec!["COMPLETE".to_string()],
            };
            if let Some(hook) = self.custom_forecast_hook {
                (hook)(patient, valid_doses, history, eval_date, &mut forecast);
            }
            return forecast;
        }

        let mut next_dose_idx = valid_doses.iter().map(|(_, num)| *num).max().unwrap_or(0) + 1;
        if let Some(hook) = self.custom_dose_number_hook {
            let ctx = EvaluationContext::new(
                patient,
                history,
                valid_doses,
                None,
                next_dose_idx,
                eval_date,
                &active_series.name,
            );
            next_dose_idx = (hook)(&active_series.name, &ctx);
        }

        if next_dose_idx > active_series.num_doses {
            let mut forecast = SeriesForecast {
                series_name: active_series.name.clone(),
                earliest_date: None,
                recommended_date: None,
                overdue_date: None,
                latest_date: None,
                status: SeriesStatus::Complete,
                reasons: vec!["COMPLETE".to_string()],
            };
            if let Some(hook) = self.custom_forecast_hook {
                (hook)(patient, valid_doses, history, eval_date, &mut forecast);
            }
            return forecast;
        }

        let mut dose_rule = active_series.doses[next_dose_idx - 1].clone();
        let mut interval_rule = if next_dose_idx > 1 {
            active_series.intervals.iter()
                .find(|int| int.to_dose == next_dose_idx)
                .cloned()
        } else {
            None
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
        for rule in &self.rec_overrides {
            if rule.target_dose_number == next_dose_idx && (rule.condition)(&ctx) {
                if let Some(ref over_age) = rule.override_min_age {
                    dose_rule.minimum_age = Some(over_age.clone());
                }
                if let Some(ref over_int) = rule.override_min_interval {
                    if let Some(ref mut int_rule) = interval_rule {
                        int_rule.minimum_interval = Some(over_int.clone());
                    }
                }
            }
        }

        // Get the most recent shot date for any dose belonging to this series
        let last_shot_date = history.iter()
            .filter(|d| active_series.doses.iter().any(|d_rule| d_rule.allowed_cvx.contains(&d.cvx)))
            .map(|d| d.date)
            .max();

        // Calculate Earliest Date: max(minimum_age, minimum_interval)
        let min_age_tp = dose_rule.minimum_age.as_ref();
        let min_age_date = min_age_tp.map(|tp| tp.add_to(patient.birth_date));
        
        let min_int_date = if let Some(ref int_rule) = interval_rule {
            int_rule.minimum_interval.as_ref().and_then(|tp| {
                last_shot_date.map(|prev_date| tp.add_to(prev_date))
            })
        } else {
            None
        };

        let earliest_date = match (min_age_date, min_int_date) {
            (Some(a), Some(i)) => Some(max(a, i)),
            (Some(a), None) => Some(a),
            (None, Some(i)) => Some(i),
            (None, None) => None,
        };

        // Calculate Recommended Date
        let rec_age_date = dose_rule.earliest_recommended_age.as_ref()
            .map(|tp| tp.add_to(patient.birth_date));
        let rec_int_date = if let Some(ref int_rule) = interval_rule {
            int_rule.earliest_recommended_interval.as_ref().and_then(|tp| {
                last_shot_date.map(|prev_date| tp.add_to(prev_date))
            })
        } else {
            None
        };

        let recommended_date = match (rec_age_date, rec_int_date) {
            (Some(a), Some(i)) => Some(max(a, i)),
            (Some(a), None) => Some(a),
            (None, Some(i)) => Some(i),
            (None, None) => None,
        };

        // Calculate Overdue Date
        let overdue_age_date = dose_rule.latest_recommended_age.as_ref()
            .map(|tp| tp.add_to(patient.birth_date));
        let overdue_int_date = if let Some(ref int_rule) = interval_rule {
            int_rule.latest_recommended_interval.as_ref().and_then(|tp| {
                last_shot_date.map(|prev_date| tp.add_to(prev_date))
            })
        } else {
            None
        };

        let overdue_date = if overdue_age_date.is_some() {
            overdue_age_date
        } else {
            overdue_int_date
        };
        let overdue_date = overdue_date.and_then(|d| d.pred_opt());

        let mut forecast = SeriesForecast {
            series_name: active_series.name.clone(),
            earliest_date,
            recommended_date,
            overdue_date,
            latest_date: None,
            status: SeriesStatus::NotComplete,
            reasons: vec!["NOT_COMPLETE".to_string()],
        };

        // Apply custom rules hook (like Polio 2009 reset) if present
        if let Some(hook) = self.custom_forecast_hook {
            (hook)(patient, valid_doses, history, eval_date, &mut forecast);
        }

        if let Some(last_date) = last_shot_date {
            if let Some(ref mut earliest) = forecast.earliest_date {
                if *earliest < last_date {
                    *earliest = last_date;
                }
            }
            if let Some(ref mut recommended) = forecast.recommended_date {
                if *recommended < last_date {
                    *recommended = last_date;
                }
            }
            if let Some(ref mut overdue) = forecast.overdue_date {
                if *overdue < last_date {
                    *overdue = last_date;
                }
            }
        }

        // Align earliest <= recommended <= overdue
        if let (Some(earliest), Some(recommended)) = (forecast.earliest_date, forecast.recommended_date.as_mut()) {
            if *recommended < earliest {
                *recommended = earliest;
            }
        }
        if let (Some(recommended), Some(overdue)) = (forecast.recommended_date, forecast.overdue_date.as_mut()) {
            if *overdue < recommended {
                *overdue = recommended;
            }
        }
        if let (Some(earliest), Some(overdue)) = (forecast.earliest_date, forecast.overdue_date.as_mut()) {
            if *overdue < earliest {
                *overdue = earliest;
            }
        }

        forecast
    }
}

fn get_same_day_priority(group: &str, cvx: &str, birth_date: NaiveDate, dose_date: NaiveDate) -> i32 {
    match group {
        "MMR" => match cvx {
            "94" => 0,
            "03" => 1,
            "04" | "05" => 2,
            _ => 3,
        },
        "POLIO" => match cvx {
            "02" | "182" => 1,
            _ => 0,
        },
        "HEP_A" => {
            let age_19 = crate::date_utils::add_years(birth_date, 19);
            if dose_date >= age_19 {
                if cvx == "52" { 0 } else { 1 }
            } else {
                if cvx == "52" { 1 } else { 0 }
            }
        }
        "VARICELLA" => match cvx {
            "94" => 0,
            "21" => 1,
            _ => 2,
        },
        "DTP" => match cvx {
            "115" | "198" => 1, // Tdap
            "09" | "28" | "113" | "138" | "139" | "195" | "196" => 2, // DT/Td
            _ => 0, // DTaP/DTP combo/single
        },
        "HEP_B" => match cvx {
            "104" | "110" | "146" => 0,
            _ => 1,
        },
        _ => 0,
    }
}

