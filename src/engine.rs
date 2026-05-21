use chrono::NaiveDate;
use crate::models::{
    Dose, DoseEvaluation, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus,
    Patient, VaccineGroupForecast
};
use crate::schedule::CompiledSeries;
use crate::date_utils::{TimePeriod, compare_elapsed};
use std::cmp::max;

// Primitives for generic override rules
#[derive(Debug, Clone)]
pub struct ParameterOverrideRule {
    pub condition_date_before: Option<NaiveDate>,
    pub target_dose_number: usize,
    pub override_abs_min_age: Option<TimePeriod>,
    pub override_abs_min_interval_from_dose: Option<(usize, TimePeriod)>,
}

#[derive(Debug, Clone)]
pub struct ConditionalCompletionRule {
    pub required_valid_doses: usize,
    pub min_age_at_last_dose: TimePeriod,
    pub min_interval_last_to_prev: TimePeriod,
}

#[derive(Debug, Clone)]
pub struct RecommendationOverrideRule {
    pub condition_eval_date_before: Option<NaiveDate>,
    pub target_dose_number: usize,
    pub override_min_age: Option<TimePeriod>,
    pub override_min_interval: Option<TimePeriod>,
}

pub struct EvaluationEngine {
    pub series: CompiledSeries,
    pub param_overrides: Vec<ParameterOverrideRule>,
    pub completion_rules: Vec<ConditionalCompletionRule>,
    pub rec_overrides: Vec<RecommendationOverrideRule>,
}

impl EvaluationEngine {
    pub fn new(series: CompiledSeries) -> Self {
        EvaluationEngine {
            series,
            param_overrides: Vec::new(),
            completion_rules: Vec::new(),
            rec_overrides: Vec::new(),
        }
    }

    pub fn evaluate_patient(
        &self,
        patient: &Patient,
        history: &[Dose],
        eval_date: NaiveDate,
    ) -> VaccineGroupForecast {
        // 1. Sort and copy history chronologically
        let mut sorted_history = history.to_vec();
        sorted_history.sort_by_key(|d| d.date);

        let mut evaluations = Vec::new();
        let mut valid_doses: Vec<(NaiveDate, usize)> = Vec::new(); // (date, dose_number_in_series)
        let mut is_completed = false;

        // 2. Chronological dose evaluation loop
        let mut i = 0;
        while i < sorted_history.len() {
            let dose = &sorted_history[i];
            
            // Same-day duplicate check
            let is_duplicate = i > 0 && sorted_history[i - 1].date == dose.date;
            if is_duplicate {
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx.clone(),
                    status: DoseStatus::Invalid,
                    reasons: vec![EvaluationReason::DuplicateShotSameDay],
                    dose_number: None,
                });
                i += 1;
                continue;
            }

            // Birthdate boundary check
            if dose.date < patient.birth_date {
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx.clone(),
                    status: DoseStatus::Invalid,
                    reasons: vec![EvaluationReason::PriorToDOB],
                    dose_number: None,
                });
                i += 1;
                continue;
            }

            // Target dose calculation
            let target_dose_idx = valid_doses.len() + 1;
            
            // If patient already completed the series, additional doses are boosters/extra
            if target_dose_idx > self.series.num_doses {
                evaluations.push(DoseEvaluation {
                    dose_date: dose.date,
                    cvx: dose.cvx.clone(),
                    status: DoseStatus::Accepted, // Mark accepted as extra dose
                    reasons: vec![EvaluationReason::BoosterDose],
                    dose_number: None,
                });
                i += 1;
                continue;
            }

            // Look up dose parameters
            let mut dose_rule = self.series.doses[target_dose_idx - 1].clone();
            let mut interval_rule = if target_dose_idx > 1 {
                self.series.intervals.iter()
                    .find(|int| int.to_dose == target_dose_idx)
                    .cloned()
            } else {
                None
            };

            // Apply parameter overrides (pre-2009 overrides, etc.)
            for rule in &self.param_overrides {
                let matches_date = rule.condition_date_before.map_or(true, |d| dose.date < d);
                if matches_date && rule.target_dose_number == target_dose_idx {
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

            // Determine status
            let status = if is_valid {
                valid_doses.push((dose.date, target_dose_idx));
                DoseStatus::Valid
            } else {
                DoseStatus::Invalid
            };

            evaluations.push(DoseEvaluation {
                dose_date: dose.date,
                cvx: dose.cvx.clone(),
                status,
                reasons,
                dose_number: Some(target_dose_idx),
            });

            for rule in &self.completion_rules {
                if valid_doses.len() == rule.required_valid_doses {
                    let last_dose_date = valid_doses.last().unwrap().0;
                    let prev_dose_date = valid_doses[valid_doses.len() - 2].0;

                    let age_ok = compare_elapsed(patient.birth_date, last_dose_date, &rule.min_age_at_last_dose) != std::cmp::Ordering::Less;
                    let interval_ok = compare_elapsed(prev_dose_date, last_dose_date, &rule.min_interval_last_to_prev) != std::cmp::Ordering::Less;

                    if age_ok && interval_ok {
                        is_completed = true;
                    }
                }
            }

            if valid_doses.len() == self.series.num_doses {
                is_completed = true;
            }

            i += 1;
        }

        // 4. Recommendation / Forecasting
        let forecast = self.generate_forecast(
            patient,
            &valid_doses,
            is_completed,
            eval_date,
        );

        VaccineGroupForecast {
            vaccine_group: self.series.vaccine_group.clone(),
            evaluations,
            forecasts: vec![forecast],
            selected_series: Some(self.series.name.clone()),
        }
    }

    fn generate_forecast(
        &self,
        patient: &Patient,
        valid_doses: &[(NaiveDate, usize)],
        is_completed: bool,
        eval_date: NaiveDate,
    ) -> SeriesForecast {
        if is_completed {
            return SeriesForecast {
                series_name: self.series.name.clone(),
                earliest_date: None,
                recommended_date: None,
                overdue_date: None,
                latest_date: None,
                status: SeriesStatus::Complete,
                reasons: vec!["COMPLETE".to_string()],
            };
        }

        let next_dose_idx = valid_doses.len() + 1;
        let mut dose_rule = self.series.doses[next_dose_idx - 1].clone();
        let mut interval_rule = if next_dose_idx > 1 {
            self.series.intervals.iter()
                .find(|int| int.to_dose == next_dose_idx)
                .cloned()
        } else {
            None
        };

        // Apply recommendation overrides
        for rule in &self.rec_overrides {
            let matches_eval = rule.condition_eval_date_before.map_or(true, |d| eval_date < d);
            if matches_eval && rule.target_dose_number == next_dose_idx {
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

        // Calculate Earliest Date: max(min_age, min_interval_from_prev)
        let min_age_date = dose_rule.minimum_age.as_ref()
            .map(|tp| tp.add_to(patient.birth_date));
        
        let min_int_date = if let Some(ref int_rule) = interval_rule {
            int_rule.minimum_interval.as_ref().and_then(|tp| {
                valid_doses.last().map(|(prev_date, _)| tp.add_to(*prev_date))
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
                valid_doses.last().map(|(prev_date, _)| tp.add_to(*prev_date))
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
                valid_doses.last().map(|(prev_date, _)| tp.add_to(*prev_date))
            })
        } else {
            None
        };

        let overdue_date = match (overdue_age_date, overdue_int_date) {
            (Some(a), Some(i)) => Some(max(a, i)),
            (Some(a), None) => Some(a),
            (None, Some(i)) => Some(i),
            (None, None) => None,
        };

        // For Polio 2009 reset logic:
        // "If the execution date is before 8/7/2009 and the calculated Earliest Date for Dose 4 is after 8/7/2009, reset back to 4y and 6m"
        // In the interest of keeping code generic, we can perform standard evaluation first, and allow the vaccine module to adjust.
        // Let's implement this as a post-calculation hook if needed, or simply handle it here:
        let mut final_earliest = earliest_date;
        let mut final_recommended = recommended_date;

        if self.series.name == "POLIO_4_DOSE_SERIES" && next_dose_idx == 4 && eval_date < NaiveDate::from_ymd_opt(2009, 8, 7).unwrap() {
            if let Some(e_date) = final_earliest {
                if e_date >= NaiveDate::from_ymd_opt(2009, 8, 7).unwrap() {
                    // Reset to standard 4y age and 6m interval
                    let std_age = TimePeriod::parse("4y").unwrap().add_to(patient.birth_date);
                    let std_int = TimePeriod::parse("6m").unwrap().add_to(valid_doses.last().unwrap().0);
                    final_earliest = Some(max(std_age, std_int));
                    final_recommended = final_earliest;
                }
            }
        }

        SeriesForecast {
            series_name: self.series.name.clone(),
            earliest_date: final_earliest,
            recommended_date: final_recommended,
            overdue_date,
            latest_date: None,
            status: SeriesStatus::NotComplete,
            reasons: vec!["NOT_COMPLETE".to_string()],
        }
    }
}
