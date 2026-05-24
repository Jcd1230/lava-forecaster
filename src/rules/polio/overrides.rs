use chrono::NaiveDate;
use crate::engine::{EvaluationContext, ParameterOverrideRule, ConditionalCompletionRule, RecommendationOverrideRule};
use crate::date_utils::{TimePeriod, compare_elapsed, add_years, add_months};
use crate::models::{Patient, SeriesForecast, Dose, DoseStatus, EvaluationReason, VaccineGroupForecast, DoseEvaluation};
use std::collections::HashMap;

pub fn polio_parameter_overrides() -> Vec<ParameterOverrideRule> {
    vec![
        // Pre-2009 Overrides for POLIO_4_DOSE_SERIES:
        // "If dose 4 administered before 8/7/2009: abs_min_age = 122d, abs_min_interval for dose 3 = 24d"
        ParameterOverrideRule {
            description: "Polio Pre-2009 Dose 4 Absolute Minimums",
            target_dose_number: 4,
            condition: |ctx| {
                if ctx.active_series_name == "POLIO_4_DOSE_SERIES" {
                    if let Some(dose) = ctx.current_dose {
                        return dose.date < NaiveDate::from_ymd_opt(2009, 8, 7).unwrap();
                    }
                }
                false
            },
            override_abs_min_age: Some(TimePeriod::parse("122d").unwrap()),
            override_abs_min_interval_from_dose: Some((3, TimePeriod::parse("24d").unwrap())),
        },
        // Pre-2009 Overrides for POLIO_FRACTIONAL_IPV_SERIES:
        // "If dose 5 administered before 8/7/2009: abs_min_age = 122d, abs_min_interval for dose 4 = 24d"
        ParameterOverrideRule {
            description: "Polio Pre-2009 fIPV Dose 5 Absolute Minimums",
            target_dose_number: 5,
            condition: |ctx| {
                if ctx.active_series_name == "POLIO_FRACTIONAL_IPV_SERIES" {
                    if let Some(dose) = ctx.current_dose {
                        return dose.date < NaiveDate::from_ymd_opt(2009, 8, 7).unwrap();
                    }
                }
                false
            },
            override_abs_min_age: Some(TimePeriod::parse("122d").unwrap()),
            override_abs_min_interval_from_dose: Some((4, TimePeriod::parse("24d").unwrap())),
        }
    ]
}

pub fn polio_completion_rules() -> Vec<ConditionalCompletionRule> {
    vec![
        // Conditional 3-Dose Completion for POLIO_4_DOSE_SERIES:
        // "Complete with 3 doses if 3 prior valid doses, child >= 4y-4d at dose 3, and interval 2 to 3 is >= 6m-4d"
        ConditionalCompletionRule {
            description: "Polio 3-Dose Completion Rule (Dose 3 at >= 4 years)",
            condition: |ctx| {
                if ctx.active_series_name == "POLIO_4_DOSE_SERIES" && ctx.valid_doses.len() >= 3 {
                    let dose_3_date = ctx.valid_doses[2].0;
                    let birth_date = ctx.patient.birth_date;
                    
                    let age_ok = compare_elapsed(
                        birth_date,
                        dose_3_date,
                        &TimePeriod::parse("4y-4d").unwrap(),
                    ) != std::cmp::Ordering::Less;
                    
                    let dose_2_date = ctx.valid_doses[1].0;
                    let interval_ok = compare_elapsed(
                        dose_2_date,
                        dose_3_date,
                        &TimePeriod::parse("6m-4d").unwrap(),
                    ) != std::cmp::Ordering::Less;
                    
                    age_ok && interval_ok
                } else {
                    false
                }
            },
        },
        // Conditional 4-Dose Completion for POLIO_FRACTIONAL_IPV_SERIES:
        // "Complete with 4 doses if 4 prior valid doses, child >= 4y-4d at dose 4, and interval 3 to 4 is >= 6m-4d"
        ConditionalCompletionRule {
            description: "Polio fIPV 4-Dose Completion Rule (Dose 4 at >= 4 years)",
            condition: |ctx| {
                if ctx.active_series_name == "POLIO_FRACTIONAL_IPV_SERIES" && ctx.valid_doses.len() >= 4 {
                    let dose_4_date = ctx.valid_doses[3].0;
                    let birth_date = ctx.patient.birth_date;
                    
                    let age_ok = compare_elapsed(
                        birth_date,
                        dose_4_date,
                        &TimePeriod::parse("4y-4d").unwrap(),
                    ) != std::cmp::Ordering::Less;
                    
                    let dose_3_date = ctx.valid_doses[2].0;
                    let interval_ok = compare_elapsed(
                        dose_3_date,
                        dose_4_date,
                        &TimePeriod::parse("6m-4d").unwrap(),
                    ) != std::cmp::Ordering::Less;
                    
                    age_ok && interval_ok
                } else {
                    false
                }
            },
        }
    ]
}

pub fn polio_recommendation_overrides() -> Vec<RecommendationOverrideRule> {
    vec![
        // Pre-2009 Forecast Recommendations for POLIO_4_DOSE_SERIES:
        // "If evaluation date is before 8/7/2009, min_age of dose 4 is 126d and min_interval for dose 3 is 28d"
        RecommendationOverrideRule {
            description: "Polio Pre-2009 Dose 4 Forecast Overrides",
            target_dose_number: 4,
            condition: |ctx| {
                ctx.active_series_name == "POLIO_4_DOSE_SERIES" && ctx.eval_date < NaiveDate::from_ymd_opt(2009, 8, 7).unwrap()
            },
            override_min_age: Some(TimePeriod::parse("126d").unwrap()),
            override_min_interval: Some(TimePeriod::parse("28d").unwrap()),
        },
        // Pre-2009 Forecast Recommendations for POLIO_FRACTIONAL_IPV_SERIES:
        // "If evaluation date is before 8/7/2009, min_age of dose 5 is 126d and min_interval for dose 4 is 28d"
        RecommendationOverrideRule {
            description: "Polio Pre-2009 fIPV Dose 5 Forecast Overrides",
            target_dose_number: 5,
            condition: |ctx| {
                ctx.active_series_name == "POLIO_FRACTIONAL_IPV_SERIES" && ctx.eval_date < NaiveDate::from_ymd_opt(2009, 8, 7).unwrap()
            },
            override_min_age: Some(TimePeriod::parse("126d").unwrap()),
            override_min_interval: Some(TimePeriod::parse("28d").unwrap()),
        }
    ]
}

pub fn polio_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.series_name != "POLIO_4_DOSE_SERIES" && forecast.series_name != "POLIO_FRACTIONAL_IPV_SERIES" {
        return;
    }

    let aug_7_2009 = NaiveDate::from_ymd_opt(2009, 8, 7).unwrap();
    
    // 2009 Forecast Date Reset Hook
    if eval_date >= aug_7_2009 {
        if let Some(ref mut recommended) = forecast.recommended_date {
            if *recommended < aug_7_2009 {
                *recommended = aug_7_2009;
            }
        }
        if let Some(ref mut earliest) = forecast.earliest_date {
            if *earliest < aug_7_2009 {
                *earliest = aug_7_2009;
            }
        }
    } else {
        // Pre-2009 forecast override undo logic
        if let Some(earliest) = forecast.earliest_date {
            if earliest > aug_7_2009 {
                let target_dose = valid_doses.len() + 1;
                let is_final_dose = (forecast.series_name == "POLIO_4_DOSE_SERIES" && target_dose == 4)
                    || (forecast.series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose == 5);
                if is_final_dose {
                    let birth = patient.birth_date;
                    let age_4y = add_years(birth, 4);
                    let mut new_earliest = age_4y;
                    if let Some((last_dose_date, _)) = valid_doses.last() {
                        let interval_6m = add_months(*last_dose_date, 6);
                        new_earliest = new_earliest.max(interval_6m);
                    }
                    forecast.earliest_date = Some(new_earliest);
                    forecast.recommended_date = Some(new_earliest);
                }
            }
        }
    }

    // >= 4y shift rule
    // If the next dose to recommend is the second-to-last dose (Dose 3 for 4-dose, Dose 4 for 5-dose)
    // and patient is >= 4 years of age (or will be at the recommended date), recommend next dose at 6 months interval from last shot.
    let target_dose_number = valid_doses.len() + 1;
    let is_second_to_last = (forecast.series_name == "POLIO_4_DOSE_SERIES" && target_dose_number == 3)
        || (forecast.series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose_number == 4);

    if is_second_to_last && !valid_doses.is_empty() {
        let birth = patient.birth_date;
        let age_4y = add_years(birth, 4);
        let is_at_least_4y = eval_date >= age_4y || forecast.recommended_date.map(|r| r >= age_4y).unwrap_or(false);
        
        if is_at_least_4y {
            let (last_dose_date, _) = valid_doses.last().unwrap();
            let min_interval_date = add_months(*last_dose_date, 6);
            
            let mut earliest = forecast.earliest_date.unwrap_or(min_interval_date).max(min_interval_date);
            let mut recommended = forecast.recommended_date.unwrap_or(min_interval_date).max(min_interval_date);
            earliest = earliest.max(age_4y);
            recommended = recommended.max(age_4y);
            
            forecast.earliest_date = Some(earliest);
            forecast.recommended_date = Some(recommended);
            
            if let Some(ref mut overdue) = forecast.overdue_date {
                if *overdue < recommended {
                    *overdue = recommended;
                }
            }
        }
    }

    // Interval calculation from last administered polio shot (excluding ignored ones)
    let last_non_ignored_dose = history.iter()
        .filter(|d| {
            let is_cvx_178_179 = d.cvx == "178" || d.cvx == "179";
            let is_cvx_02_182_after_2016 = (d.cvx == "02" || d.cvx == "182") && d.date >= NaiveDate::from_ymd_opt(2016, 4, 1).unwrap();
            !(is_cvx_178_179 || is_cvx_02_182_after_2016)
        })
        .last();

    if let Some(last_dose) = last_non_ignored_dose {
        let target_dose = valid_doses.len() + 1;
        let should_apply_interval = if target_dose > 1 {
            true
        } else {
            let abs_min_age_dose_1 = TimePeriod::parse("38d").unwrap().add_to(patient.birth_date);
            last_dose.date >= abs_min_age_dose_1
        };

        if should_apply_interval {
            let min_interval = if (forecast.series_name == "POLIO_4_DOSE_SERIES" && target_dose == 4)
                || (forecast.series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose == 5) {
                TimePeriod::parse("6m").unwrap()
            } else {
                TimePeriod::parse("28d").unwrap()
            };
            
            let min_interval_date = min_interval.add_to(last_dose.date);
            
            if let Some(ref mut earliest) = forecast.earliest_date {
                if *earliest < min_interval_date {
                    *earliest = min_interval_date;
                }
            }
            if let Some(ref mut recommended) = forecast.recommended_date {
                if *recommended < min_interval_date {
                    *recommended = min_interval_date;
                }
            }
            if let Some(ref mut overdue) = forecast.overdue_date {
                if *overdue < min_interval_date {
                    *overdue = min_interval_date;
                }
            }
        }
    }
}

pub fn polio_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if series_name != "POLIO_4_DOSE_SERIES" && series_name != "POLIO_FRACTIONAL_IPV_SERIES" {
        return;
    }

    if let Some(dose) = ctx.current_dose {
        let is_cvx_178_179 = dose.cvx == "178" || dose.cvx == "179";
        let is_cvx_02_182_after_2016 = (dose.cvx == "02" || dose.cvx == "182") && dose.date >= NaiveDate::from_ymd_opt(2016, 4, 1).unwrap();
        
        if is_cvx_178_179 || is_cvx_02_182_after_2016 {
            *status = DoseStatus::Invalid;
            reasons.clear();
            reasons.push(EvaluationReason::MissingAntigen);
            return;
        }

        // Final Dose below minimum age but absolute minimum interval is met override (>= 2009-08-07)
        let is_final_dose = (series_name == "POLIO_4_DOSE_SERIES" && target_dose_idx == 4)
            || (series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose_idx == 5);
            
        if is_final_dose && dose.date >= NaiveDate::from_ymd_opt(2009, 8, 7).unwrap() {
            let age_4y_minus_4d = add_years(ctx.patient.birth_date, 4) - chrono::Duration::days(4);
            if dose.date < age_4y_minus_4d {
                if let Some((prev_date, _)) = ctx.valid_doses.last() {
                    let interval_ok = compare_elapsed(
                        *prev_date,
                        dose.date,
                        &TimePeriod::parse("6m-4d").unwrap(),
                    ) != std::cmp::Ordering::Less;
                    
                    if interval_ok {
                        *status = DoseStatus::Accepted;
                        reasons.clear();
                        reasons.push(EvaluationReason::BelowMinimumAgeFinalDose);
                        reasons.push(EvaluationReason::OutsideRoutineSeries); // Avoids completing the series
                    }
                }
            }
        }
    }
}

pub fn polio_custom_extra_dose_hook(
    series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, Vec<EvaluationReason>)> {
    if series_name != "POLIO_4_DOSE_SERIES" && series_name != "POLIO_FRACTIONAL_IPV_SERIES" {
        return None;
    }

    let dose = ctx.current_dose?;
    
    let is_cvx_178_179 = dose.cvx == "178" || dose.cvx == "179";
    let is_cvx_02_182_after_2016 = (dose.cvx == "02" || dose.cvx == "182") && dose.date >= NaiveDate::from_ymd_opt(2016, 4, 1).unwrap();
    
    if is_cvx_178_179 || is_cvx_02_182_after_2016 {
        return Some((DoseStatus::Invalid, vec![EvaluationReason::MissingAntigen]));
    }

    let birth_date = ctx.patient.birth_date;
    let age_18 = add_years(birth_date, 18);
    if dose.date >= age_18 {
        if ctx.target_dose_number == ctx.valid_doses.len() + 1 {
            return Some((DoseStatus::Valid, vec![EvaluationReason::BoosterDose]));
        }
    }

    Some((DoseStatus::Accepted, vec![EvaluationReason::BoosterDose]))
}

pub fn polio_group_selection(
    _patient: &Patient,
    _history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut HashMap<String, VaccineGroupForecast>,
) -> String {
    if let Some(fipv_forecast) = candidate_forecasts.get("POLIO_FRACTIONAL_IPV_SERIES") {
        let mut valid_doses: Vec<&DoseEvaluation> = fipv_forecast.evaluations.iter()
            .filter(|e| e.status == DoseStatus::Valid)
            .collect();
        valid_doses.sort_by_key(|e| e.dose_date);
        
        if valid_doses.len() >= 2 {
            if valid_doses[0].cvx == "324" && valid_doses[1].cvx == "324" {
                let second_fipv_date = valid_doses[1].dose_date;
                let mut has_other_valid_before = false;
                if let Some(four_dose_forecast) = candidate_forecasts.get("POLIO_4_DOSE_SERIES") {
                    for e in &four_dose_forecast.evaluations {
                        if e.status == DoseStatus::Valid && e.dose_date < second_fipv_date && e.cvx != "324" {
                            has_other_valid_before = true;
                            break;
                        }
                    }
                }
                if !has_other_valid_before {
                    return "POLIO_FRACTIONAL_IPV_SERIES".to_string();
                }
            }
        }
    }
    "POLIO_4_DOSE_SERIES".to_string()
}
