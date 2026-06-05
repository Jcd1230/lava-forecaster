use crate::engine::CandidateForecastsExt;
use lava_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::{EvaluationContext, ParameterOverrideRule, RecommendationOverrideRule};
use crate::date_utils::{SmallVec, compare_elapsed, add_years_unchecked, add_months_unchecked};
use crate::models::{Patient, SeriesForecast, Dose, DoseStatus, EvaluationReason, VaccineGroupForecast, DoseEvaluation};

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
            override_abs_min_age: Some(crate::time_period!("122d")),
            override_abs_min_interval_from_dose: Some((3, crate::time_period!("24d"))),
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
            override_abs_min_age: Some(crate::time_period!("122d")),
            override_abs_min_interval_from_dose: Some((4, crate::time_period!("24d"))),
        }
    ]
}

pub fn polio_completion_rules() -> Vec<crate::engine::ConditionalCompletionRule> {
    vec![]
}

/// Custom completion hook for Polio series.
///
/// Replicates the Drools completion rules:
///
/// **4-dose series** (POLIO_4_DOSE_SERIES):
/// - Complete with 3 doses if dose 3 is at or after 4y-4d **and** interval 2→3 >= 6m-4d
/// - Complete with 4 doses if dose 4 was given **before 2009-08-07** OR at/after 4y-4d
///   (A dose 4 given after 2009 but before 4y-4d is Valid but does NOT complete the series)
///
/// **fIPV series** (POLIO_FRACTIONAL_IPV_SERIES):
/// - Complete with 4 doses if dose 4 is at or after 4y-4d **and** interval 3→4 >= 6m-4d
/// - Complete with 5 doses if dose 5 was given **before 2009-08-07** OR at/after 4y-4d
pub fn polio_custom_completion_hook(ctx: &crate::engine::EvaluationContext) -> bool {
    let aug_7_2009 = NaiveDate::from_ymd_opt(2009, 8, 7).unwrap();
    let birth = ctx.patient.birth_date;
    let age_4y_minus_4d = add_years_unchecked(birth, 4) - chrono::Duration::days(4);

    match ctx.active_series_name {
        "POLIO_4_DOSE_SERIES" => {
            let valid = ctx.valid_doses;

            // 3-dose early completion: dose 3 at >= 4y-4d with interval 2->3 >= 6m-4d
            if valid.len() >= 3 {
                let d3 = valid[2].0;
                let d2 = valid[1].0;
                let age_ok = d3 >= age_4y_minus_4d;
                let interval_ok = compare_elapsed(d2, d3, &crate::time_period!("6m-4d"))
                    != std::cmp::Ordering::Less;
                if age_ok && interval_ok {
                    return true;
                }
            }

            // 4-dose completion: dose 4 must be before 2009-08-07 OR at/after 4y-4d
            if valid.len() >= 4 && valid[3].1 == 4 {
                let d4 = valid[3].0;
                if d4 < aug_7_2009 || d4 >= age_4y_minus_4d {
                    return true;
                }

                let completion_dose = valid.iter().find(|(_, dose_number)| *dose_number == 5);
                if d4 >= aug_7_2009 && d4 < age_4y_minus_4d {
                    if let Some((completion_date, _)) = completion_dose {
                        if *completion_date >= age_4y_minus_4d {
                            return true;
                        }
                    }
                }
            }

            false
        }
        "POLIO_FRACTIONAL_IPV_SERIES" => {
            let valid = ctx.valid_doses;

            // 4-dose early completion: dose 4 at >= 4y-4d with interval 3->4 >= 6m-4d
            if valid.len() >= 4 {
                let d4 = valid[3].0;
                let d3 = valid[2].0;
                let age_ok = d4 >= age_4y_minus_4d;
                let interval_ok = compare_elapsed(d3, d4, &crate::time_period!("6m-4d"))
                    != std::cmp::Ordering::Less;
                if age_ok && interval_ok {
                    return true;
                }
            }

            // 5-dose completion: dose 5 must be before 2009-08-07 OR at/after 4y-4d
            if valid.len() >= 5 && valid[4].1 == 5 {
                let d5 = valid[4].0;
                if d5 < aug_7_2009 || d5 >= age_4y_minus_4d {
                    return true;
                }

                let completion_dose = valid.iter().find(|(_, dose_number)| *dose_number == 6);
                if d5 >= aug_7_2009 && d5 < age_4y_minus_4d {
                    if let Some((completion_date, _)) = completion_dose {
                        if *completion_date >= age_4y_minus_4d {
                            return true;
                        }
                    }
                }
            }

            false
        }
        _ => false,
    }
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
            override_min_age: Some(crate::time_period!("126d")),
            override_min_interval: Some(crate::time_period!("28d")),
        },
        // Pre-2009 Forecast Recommendations for POLIO_FRACTIONAL_IPV_SERIES:
        // "If evaluation date is before 8/7/2009, min_age of dose 5 is 126d and min_interval for dose 4 is 28d"
        RecommendationOverrideRule {
            description: "Polio Pre-2009 fIPV Dose 5 Forecast Overrides",
            target_dose_number: 5,
            condition: |ctx| {
                ctx.active_series_name == "POLIO_FRACTIONAL_IPV_SERIES" && ctx.eval_date < NaiveDate::from_ymd_opt(2009, 8, 7).unwrap()
            },
            override_min_age: Some(crate::time_period!("126d")),
            override_min_interval: Some(crate::time_period!("28d")),
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
        if let Some(Some(recommended)) = forecast.status.recommended_date_mut() {
            if *recommended < aug_7_2009 {
                *recommended = aug_7_2009;
            }
        }
        if let Some(Some(earliest)) = forecast.status.earliest_date_mut() {
            if *earliest < aug_7_2009 {
                *earliest = aug_7_2009;
            }
        }
    } else {
        // Pre-2009 forecast override undo logic
        if let Some(earliest) = forecast.status.earliest_date() {
            if earliest > aug_7_2009 {
                let target_dose = valid_doses.len() + 1;
                let is_final_dose = (forecast.series_name == "POLIO_4_DOSE_SERIES" && target_dose == 4)
                    || (forecast.series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose == 5);
                if is_final_dose {
                    let birth = patient.birth_date;
                    let age_4y = add_years_unchecked(birth, 4);
                    let mut new_earliest = age_4y;
                    if let Some((last_dose_date, _)) = valid_doses.last() {
                        let interval_6m = add_months_unchecked(*last_dose_date, 6);
                        new_earliest = new_earliest.max(interval_6m);
                    }
                    forecast.status = forecast.status.with_earliest_date(Some(new_earliest));
                    forecast.status = forecast.status.with_recommended_date(Some(new_earliest));
                }
            }
        }
    }

    // == 4y-before-complete case ==
    // When all series doses are valid but dose 4 (or 5 for fIPV) was given before age 4y-4d,
    // the series isn't complete — forecast the next dose at age 4y.
    let target_dose_number = valid_doses.len() + 1;
    let is_final_awaiting_age = (forecast.series_name == "POLIO_4_DOSE_SERIES" && target_dose_number == 5)
        || (forecast.series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose_number == 6);

    if is_final_awaiting_age && matches!(forecast.status, crate::models::SeriesStatus::NotComplete { .. }) {
        let birth = patient.birth_date;
        let age_4y = add_years_unchecked(birth, 4);
        // Overdue at 7y+4w-1d (Java treats the latest_recommended_age boundary as exclusive)
        let age_7y_4w_minus_1d = add_years_unchecked(birth, 7) + chrono::Duration::weeks(4) - chrono::Duration::days(1);
        // Use max of age_4y and 6m from the last non-OPV administered shot (including invalid shots)
        let last_non_opv = history.iter()
            .filter(|d| {
                let is_opv = d.cvx.0 == cvx!("178") || d.cvx.0 == cvx!("179")
                    || ((d.cvx.0 == cvx!("02") || d.cvx.0 == cvx!("182")) && d.date >= NaiveDate::from_ymd_opt(2016, 4, 1).unwrap());
                !is_opv
            })
            .last();
        let mut earliest = age_4y;
        if let Some(last_dose) = last_non_opv {
            let interval_6m = add_months_unchecked(last_dose.date, 6);
            earliest = earliest.max(interval_6m);
        }
        forecast.status = forecast.status.with_earliest_date(Some(earliest));
        forecast.status = forecast.status.with_recommended_date(Some(earliest));
        forecast.status = forecast.status.with_overdue_date(Some(age_7y_4w_minus_1d));
        return;
    }

    // >= 4y shift rule
    // If the next dose to recommend is the second-to-last dose (Dose 3 for 4-dose, Dose 4 for 5-dose)
    // and patient is >= 4 years of age (or will be at the recommended date), recommend next dose at 6 months interval from last shot.
    let is_second_to_last = (forecast.series_name == "POLIO_4_DOSE_SERIES" && target_dose_number == 3)
        || (forecast.series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose_number == 4);

    if is_second_to_last && !valid_doses.is_empty() {
        let birth = patient.birth_date;
        let age_4y = add_years_unchecked(birth, 4);
        let is_at_least_4y = eval_date >= age_4y || forecast.status.recommended_date().map(|r| r >= age_4y).unwrap_or(false);
        
        if is_at_least_4y {
            let (last_dose_date, _) = valid_doses.last().unwrap();
            let min_interval_date = add_months_unchecked(*last_dose_date, 6);
            
            let mut earliest = forecast.status.earliest_date().unwrap_or(min_interval_date).max(min_interval_date);
            let mut recommended = forecast.status.recommended_date().unwrap_or(min_interval_date).max(min_interval_date);
            earliest = earliest.max(age_4y);
            recommended = recommended.max(age_4y);
            
            forecast.status = forecast.status.with_earliest_date(Some(earliest));
            forecast.status = forecast.status.with_recommended_date(Some(recommended));
            
            if let Some(ref mut overdue) = forecast.status.overdue_date() {
                if *overdue < recommended {
                    *overdue = recommended;
                }
            }
        }
    }

    // Interval calculation from last administered polio shot (excluding ignored ones)
    let last_non_ignored_dose = history.iter()
        .filter(|d| {
            let is_cvx_178_179 = d.cvx.0 == cvx!("178") || d.cvx.0 == cvx!("179");
            let is_cvx_02_182_after_2016 = (d.cvx.0 == cvx!("02") || d.cvx.0 == cvx!("182")) && d.date >= NaiveDate::from_ymd_opt(2016, 4, 1).unwrap();
            !(is_cvx_178_179 || is_cvx_02_182_after_2016)
        })
        .last();

    if let Some(last_dose) = last_non_ignored_dose {
        let target_dose = valid_doses.len() + 1;
        let should_apply_interval = if target_dose > 1 {
            true
        } else {
            let abs_min_age_dose_1 = crate::time_period!("38d").add_to(patient.birth_date);
            last_dose.date >= abs_min_age_dose_1
        };

        if should_apply_interval {
            let min_interval = if (forecast.series_name == "POLIO_4_DOSE_SERIES" && target_dose == 4)
                || (forecast.series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose == 5) {
                crate::time_period!("6m")
            } else {
                crate::time_period!("28d")
            };
            
            let min_interval_date = min_interval.add_to(last_dose.date);
            
            if let Some(Some(earliest)) = forecast.status.earliest_date_mut() {
                if *earliest < min_interval_date {
                    *earliest = min_interval_date;
                }
            }
            if let Some(Some(recommended)) = forecast.status.recommended_date_mut() {
                if *recommended < min_interval_date {
                    *recommended = min_interval_date;
                }
            }
            if let Some(Some(overdue)) = forecast.status.overdue_date_mut() {
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
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    if series_name != "POLIO_4_DOSE_SERIES" && series_name != "POLIO_FRACTIONAL_IPV_SERIES" {
        return;
    }

    if let Some(dose) = ctx.current_dose {
        let is_cvx_178_179 = dose.cvx.0 == cvx!("178") || dose.cvx.0 == cvx!("179");
        let is_cvx_02_182_after_2016 = (dose.cvx.0 == cvx!("02") || dose.cvx.0 == cvx!("182")) && dose.date >= NaiveDate::from_ymd_opt(2016, 4, 1).unwrap();
        
        if is_cvx_178_179 || is_cvx_02_182_after_2016 {
            *status = DoseStatus::Invalid;
            reasons.clear();
            reasons.push(EvaluationReason::MissingAntigen);
            return;
        }

        // Drools rule: "Mark target dose 4 (or 5 for fIPV) administered >= 8/7/2009 and before
        // 4y-4d as VALID" — this also sets abs_min_interval from previous dose to 0d
        // and abs_min_age to the dose-3 (or dose-4 for fIPV) abs_min_age (94d).
        // Effectively: if the shot is before 4y-4d and post-2009, it's Valid as long as
        // the patient meets the dose-3 abs_min_age (94 days old).
        let is_final_dose = (series_name == "POLIO_4_DOSE_SERIES" && target_dose_idx == 4)
            || (series_name == "POLIO_FRACTIONAL_IPV_SERIES" && target_dose_idx == 5);
            
        if is_final_dose && dose.date >= NaiveDate::from_ymd_opt(2009, 8, 7).unwrap() {
            let age_4y_minus_4d = add_years_unchecked(ctx.patient.birth_date, 4) - chrono::Duration::days(4);
            if dose.date < age_4y_minus_4d {
                // Dose-3 abs_min_age is 94d (from POLIO_4_DOSE_SERIES schedule)
                let dose3_abs_min_age_ok = (dose.date - ctx.patient.birth_date).num_days() >= 94;
                if dose3_abs_min_age_ok {
                    // Mark as Valid — interval check is overridden to 0d per Drools rule.
                    // The series is NOT complete because dose 4 before 4y-4d requires a 5th dose.
                    *status = DoseStatus::Valid;
                    reasons.clear();
                }
            }
        }
    }
}

pub fn polio_custom_extra_dose_hook(
    series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, SmallVec<[EvaluationReason; 4]>)> {
    if series_name != "POLIO_4_DOSE_SERIES" && series_name != "POLIO_FRACTIONAL_IPV_SERIES" {
        return None;
    }

    let dose = ctx.current_dose?;
    
    let is_cvx_178_179 = dose.cvx.0 == cvx!("178") || dose.cvx.0 == cvx!("179");
    let is_cvx_02_182_after_2016 = (dose.cvx.0 == cvx!("02") || dose.cvx.0 == cvx!("182")) && dose.date >= NaiveDate::from_ymd_opt(2016, 4, 1).unwrap();
    
    if is_cvx_178_179 || is_cvx_02_182_after_2016 {
        return Some((DoseStatus::Invalid, crate::reasons![EvaluationReason::MissingAntigen]));
    }

    let birth_date = ctx.patient.birth_date;
    let age_4y_minus_4d = add_years_unchecked(birth_date, 4) - chrono::Duration::days(4);
    let aug_7_2009 = NaiveDate::from_ymd_opt(2009, 8, 7).unwrap();

    // Handle the "awaiting-completion" dose: when all required doses are valid but the
    // last final dose was given before 4y-4d (series not complete), the next shot is the
    // required completion dose. It must meet 4y-4d age requirement.
    let awaiting_completion = if series_name == "POLIO_4_DOSE_SERIES" {
        ctx.valid_doses.len() == 4
            && ctx.valid_doses.last().map(|(d, _)| *d >= aug_7_2009 && *d < age_4y_minus_4d).unwrap_or(false)
    } else if series_name == "POLIO_FRACTIONAL_IPV_SERIES" {
        ctx.valid_doses.len() == 5
            && ctx.valid_doses.last().map(|(d, _)| *d >= aug_7_2009 && *d < age_4y_minus_4d).unwrap_or(false)
    } else {
        false
    };

    if awaiting_completion {
        if dose.date >= age_4y_minus_4d {
            // This completes the series — Valid
            return Some((DoseStatus::Valid, crate::reasons![]));
        } else {
            // Required dose, but still before minimum age
            return Some((DoseStatus::Invalid, crate::reasons![EvaluationReason::BelowMinimumAge]));
        }
    }

    let age_18 = add_years_unchecked(birth_date, 18);
    if dose.date >= age_18 {
        if ctx.target_dose_number == ctx.valid_doses.len() + 1 {
            return Some((DoseStatus::Valid, crate::reasons![EvaluationReason::BoosterDose]));
        }
    }

    Some((DoseStatus::Accepted, crate::reasons![EvaluationReason::BoosterDose]))
}

pub fn polio_group_selection(
    _patient: &Patient,
    _history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    if let Some(fipv_forecast) = candidate_forecasts.get_forecast("POLIO_FRACTIONAL_IPV_SERIES") {
        let mut valid_doses: Vec<&DoseEvaluation> = fipv_forecast.evaluations.iter()
            .filter(|e| e.status == DoseStatus::Valid)
            .collect();
        valid_doses.sort_by_key(|e| e.dose_date);
        
        if valid_doses.len() >= 2 {
            if valid_doses[0].cvx.0 == cvx!("324") && valid_doses[1].cvx.0 == cvx!("324") {
                let second_fipv_date = valid_doses[1].dose_date;
                let mut has_other_valid_before = false;
                if let Some(four_dose_forecast) = candidate_forecasts.get_forecast("POLIO_4_DOSE_SERIES") {
                    for e in four_dose_forecast.evaluations.iter() {
                        if e.status == DoseStatus::Valid && e.dose_date < second_fipv_date && e.cvx.0 != cvx!("324") {
                            has_other_valid_before = true;
                            break;
                        }
                    }
                }
                if !has_other_valid_before {
                    return "POLIO_FRACTIONAL_IPV_SERIES";
                }
            }
        }
    }
    "POLIO_4_DOSE_SERIES"
}
