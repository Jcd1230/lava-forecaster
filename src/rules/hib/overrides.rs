use crate::engine::CandidateForecastsExt;
use ice_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast};
use crate::date_utils::{TinyVec, TimePeriod, compare_elapsed};

pub fn is_hib_cvx(cvx: Cvx) -> bool {
    const HIB_CVX: &[u16] = &[cvx!("17"), cvx!("22"), cvx!("46"), cvx!("47"), cvx!("48"), cvx!("49"), cvx!("50"), cvx!("51"), cvx!("102"), cvx!("120"), cvx!("132"), cvx!("146"), cvx!("148"), cvx!("170"), cvx!("198")];
    HIB_CVX.contains(&cvx.0)
}

pub fn is_omp_cvx(cvx: Cvx) -> bool {
    const OMP_CVX: &[u16] = &[cvx!("49"), cvx!("51")];
    OMP_CVX.contains(&cvx.0)
}

fn count_valid_doses_before(valid_doses: &[(NaiveDate, usize)], cutoff: NaiveDate) -> usize {
    valid_doses.iter().filter(|(date, _)| *date < cutoff).count()
}

fn effective_dose_number_before(valid_doses: &[(NaiveDate, usize)], cutoff: NaiveDate) -> usize {
    valid_doses
        .iter()
        .filter(|(date, _)| *date < cutoff)
        .map(|(_, dose_number)| *dose_number)
        .max()
        .unwrap_or(0)
}

pub fn hib_custom_dose_number_hook(
    series_name: &str,
    ctx: &EvaluationContext,
) -> usize {
    if series_name != "HIB_4_DOSE_SERIES" {
        return ctx.target_dose_number;
    }

    let birth = ctx.patient.birth_date;
    let ref_date = ctx.current_dose.map(|d| d.date).unwrap_or(ctx.eval_date);
    let mut target_dose_idx = ctx.target_dose_number;

    // Helper closure to check age boundaries
    let is_age_ge = |tp: TimePeriod| -> bool {
        compare_elapsed(birth, ref_date, &tp) != std::cmp::Ordering::Less
    };
    let is_age_lt = |tp: TimePeriod| -> bool {
        compare_elapsed(birth, ref_date, &tp) == std::cmp::Ordering::Less
    };

    let tp_7m = crate::time_period!("7m");
    let tp_12m = crate::time_period!("12m");

    // Forecast-time skip rules only apply when there are no valid doses at all.
    // Once a series has started, Java keeps forecasting from the effective
    // next dose number produced by the evaluated history.
    if ctx.current_dose.is_none() && !ctx.valid_doses.is_empty() {
        return target_dose_idx;
    }

    let count_hib_before_12m = ctx.valid_doses.iter()
        .filter(|(date, _)| compare_elapsed(birth, *date, &tp_12m) == std::cmp::Ordering::Less)
        .count();

    // Check age ranges:
    if is_age_ge(crate::time_period!("15m")) {
        // Skip to 4: Patient over 15 Months
        if target_dose_idx < 4 {
            target_dose_idx = 4;
        }
    } else if is_age_ge(crate::time_period!("12m")) && is_age_lt(crate::time_period!("15m")) {
        // Patient between 12 and 15 Months
        if count_hib_before_12m < 2 {
            if target_dose_idx < 3 {
                target_dose_idx = 3;
            }
        } else if count_hib_before_12m == 2 {
            if target_dose_idx < 4 {
                target_dose_idx = 4;
            }
        }
    } else if is_age_ge(crate::time_period!("12m-28d")) && is_age_lt(crate::time_period!("12m")) {
        // Patient between 12m-28d and 12m
        // Rule: Skip to 3 if patient has received exactly 1 prior dose which was administered < 7m of age
        let prior_hib_count = ctx.valid_doses.len();
        if prior_hib_count == 1 {
            let prior_dose_lt_7m = compare_elapsed(birth, ctx.valid_doses[0].0, &tp_7m) == std::cmp::Ordering::Less;
            if prior_dose_lt_7m {
                if target_dose_idx < 3 {
                    target_dose_idx = 3;
                }
            }
        }
    } else if is_age_ge(crate::time_period!("7m")) && is_age_lt(crate::time_period!("12m")) {
        // Patient between 7 and 12 Months
        if target_dose_idx < 2 {
            target_dose_idx = 2;
        }
    }

    target_dose_idx
}

pub fn hib_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
) {
    let birth = ctx.patient.birth_date;
    let dose = match ctx.current_dose {
        Some(d) => d,
        None => return,
    };
    let admin_date = dose.date;

    // 1. Booster Check (CVX 50)
    if dose.cvx.0 == cvx!("50") {
        let tp_1y_4d = crate::time_period!("1y-4d");
        let tp_5y = crate::time_period!("5y");
        let age_ge_1y_4d = compare_elapsed(birth, admin_date, &tp_1y_4d) != std::cmp::Ordering::Less;
        let age_ge_5y = compare_elapsed(birth, admin_date, &tp_5y) != std::cmp::Ordering::Less;

        let num_doses = if series_name == "HIB_OMP_SERIES" { 3 } else { 4 };
        let is_final_dose = target_dose_idx == num_doses;
        let has_prior_valid = !ctx.valid_doses.is_empty();

        let allowed = (is_final_dose && has_prior_valid && age_ge_1y_4d) || age_ge_5y;
        if !allowed {
            *status = DoseStatus::Invalid;
            if !reasons.contains(&EvaluationReason::VaccineNotPartOfSeries) {
                reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            }
            return;
        }
    }

    // 2. Age Clamp (>= 5y)
    let tp_5y = crate::time_period!("5y");
    let age_ge_5y = compare_elapsed(birth, admin_date, &tp_5y) != std::cmp::Ordering::Less;
    if age_ge_5y {
        let num_doses = if series_name == "HIB_OMP_SERIES" { 3 } else { 4 };
        let valid_before_5y = ctx.count_valid_doses_before(crate::time_period!("5y"));
        if valid_before_5y < num_doses {
            *status = DoseStatus::Accepted;
            if !reasons.contains(&EvaluationReason::AboveRecommendedAgeSeries) {
                reasons.push(EvaluationReason::AboveRecommendedAgeSeries);
            }
            reasons.retain(|r| *r != EvaluationReason::BelowMinimumAge && *r != EvaluationReason::BelowMinimumInterval);
        }
    }

    // 3. Below Absolute Minimum Age for Final Dose of 4-Dose Series
    if series_name == "HIB_4_DOSE_SERIES" && target_dose_idx == 4 {
        let tp_1y_4d = crate::time_period!("1y-4d");
        let age_lt_1y_4d = compare_elapsed(birth, admin_date, &tp_1y_4d) == std::cmp::Ordering::Less;
        if age_lt_1y_4d {
            let hib_cvx = &[cvx!("17"), cvx!("22"), cvx!("46"), cvx!("47"), cvx!("48"), cvx!("49"), cvx!("50"), cvx!("51"), cvx!("102"), cvx!("120"), cvx!("132"), cvx!("146"), cvx!("148"), cvx!("170"), cvx!("198")];
            let count_before_7m = ctx.count_cvx_before(hib_cvx, crate::time_period!("7m"));
            if count_before_7m == 0 {
                *status = DoseStatus::Invalid;
                if !reasons.contains(&EvaluationReason::BelowMinimumAge) {
                    reasons.push(EvaluationReason::BelowMinimumAge);
                }
            }
        }
    }
}

pub fn hib_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let birth = patient.birth_date;
    let age_5y = crate::time_period!("5y").add_to(birth);

    if eval_date >= age_5y {
        let doses_required = if forecast.series_name == "HIB_OMP_SERIES" { 3 } else { 4 };
        let effective_before_5y = effective_dose_number_before(valid_doses, age_5y);
        if effective_before_5y < doses_required {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.status = forecast.status.with_earliest_date(None);
            forecast.status = forecast.status.with_recommended_date(None);
            forecast.status = forecast.status.with_overdue_date(None);
            forecast.status = forecast.status.with_latest_date(None);
            return;
        }
    }



    // Removed the manual >= 5 years age check since it is now
    // handled declaratively by active_series.max_age_clamp.

    // Recommended Date Overrides for HIB_4_DOSE_SERIES.
    // These Java rules act on the series' current effective target dose.
    // Once a catch-up dose has already satisfied the skipped target, forecasting
    // should fall back to the generic next-dose timing.
    if forecast.series_name == "HIB_4_DOSE_SERIES" && forecast.status != SeriesStatus::Complete && valid_doses.is_empty() {
        let tp_7m = crate::time_period!("7m");
        let tp_12m = crate::time_period!("12m");
        let tp_15m = crate::time_period!("15m");

        let date_7m = tp_7m.add_to(birth);
        let date_12m = tp_12m.add_to(birth);
        let date_15m = tp_15m.add_to(birth);
        let eval_ge_7m = eval_date >= date_7m;
        let eval_lt_12m = eval_date < date_12m;
        let eval_ge_12m = eval_date >= date_12m;
        let eval_lt_15m = eval_date < date_15m;
        let eval_ge_15m = eval_date >= date_15m;
        let eval_lt_5y = eval_date < age_5y;

        let next_target_dose = if valid_doses.is_empty() {
            if eval_ge_15m && eval_lt_5y {
                4
            } else if eval_ge_12m && eval_lt_15m {
                3
            } else if eval_ge_7m && eval_lt_12m {
                2
            } else {
                1
            }
        } else {
            valid_doses.iter().map(|(_, dose_number)| *dose_number).max().unwrap_or(0) + 1
        };

        let count_valid_before_7m = count_valid_doses_before(valid_doses, date_7m);
        let count_valid_before_12m = count_valid_doses_before(valid_doses, date_12m);
        let effective_before_15m = effective_dose_number_before(valid_doses, date_15m);

        if next_target_dose == 2 && eval_ge_7m && eval_lt_12m {
            if count_valid_before_7m == 0 {
                forecast.status = forecast.status.with_recommended_date(Some(date_7m));
                forecast.status = forecast.status.with_earliest_date(Some(date_7m));
                forecast.status = forecast.status.with_overdue_date(Some(date_7m));
            }
        } else if next_target_dose == 3 && eval_ge_12m && eval_lt_15m {
            if count_valid_before_12m < 2 {
                forecast.status = forecast.status.with_recommended_date(Some(date_12m));
                forecast.status = forecast.status.with_earliest_date(Some(date_12m));
            }
        } else if next_target_dose == 4 && eval_ge_12m && eval_lt_15m {
            if count_valid_before_12m == 2 {
                forecast.status = forecast.status.with_recommended_date(Some(date_12m));
                forecast.status = forecast.status.with_earliest_date(Some(date_12m));
            }
        } else if next_target_dose == 4 && eval_ge_15m && eval_lt_5y {
            if effective_before_15m < 4 {
                forecast.status = forecast.status.with_recommended_date(Some(date_15m));
                forecast.status = forecast.status.with_earliest_date(Some(date_15m));
            }
        }
    }

    // Space by 28-day repeat interval from the last invalid dose in history,
    // unless that last dose was invalid due to BelowMinimumAge.
    let last_dose = history.iter().max_by_key(|d| d.date);
    if let Some(ld) = last_dose {
        let is_valid = valid_doses.iter().any(|(date, _)| *date == ld.date);
        if !is_valid {
            let target_dose_num = valid_doses.len() + 1;
            let abs_min_age = match forecast.series_name.as_ref() {
                "HIB_OMP_SERIES" => match target_dose_num {
                    1 => Some(crate::time_period!("38d")),
                    2 => Some(crate::time_period!("66d")),
                    3 => Some(crate::time_period!("1y-4d")),
                    _ => None,
                },
                _ => match target_dose_num {
                    1 => Some(crate::time_period!("38d")),
                    2 => Some(crate::time_period!("66d")),
                    3 => Some(crate::time_period!("94d")),
                    4 => Some(crate::time_period!("1y-4d")),
                    _ => None,
                },
            };
            let is_below_min_age = if let Some(tp) = abs_min_age {
                compare_elapsed(birth, ld.date, &tp) == std::cmp::Ordering::Less
            } else {
                false
            };

            if !is_below_min_age {
                let repeat_date = ld.date + chrono::Duration::days(28);
                if let Some(ref mut earliest) = forecast.status.earliest_date() {
                    if *earliest < repeat_date {
                        *earliest = repeat_date;
                    }
                }
                if let Some(ref mut recommended) = forecast.status.recommended_date() {
                    if *recommended < repeat_date {
                        *recommended = repeat_date;
                    }
                }
                if let Some(ref mut overdue) = forecast.status.overdue_date() {
                    if *overdue < repeat_date {
                        *overdue = repeat_date;
                    }
                }
            }
        }
    }
}

pub fn hib_custom_switch_hook(
    current_series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
) -> Option<&'static str> {
    if current_series_name != "HIB_4_DOSE_SERIES" {
        return None;
    }

    let birth = ctx.patient.birth_date;
    let tp_7m = crate::time_period!("7m");
    let tp_12m = crate::time_period!("12m");

    if let Some(dose) = ctx.current_dose {
        if is_omp_cvx(dose.cvx) {
            // Case 1: First dose, < 7m, is OMP
            if ctx.valid_doses.is_empty() {
                if compare_elapsed(birth, dose.date, &tp_7m) == std::cmp::Ordering::Less {
                    return Some("HIB_OMP_SERIES");
                }
            }
            // Case 2: Second dose, < 12m, is OMP, and the first valid dose was OMP given < 7m
            if ctx.valid_doses.len() == 1 {
                let first_valid = ctx.valid_doses[0];
                let first_dose = ctx.history.iter().find(|d| d.date == first_valid.0);
                if let Some(fd) = first_dose {
                    if is_omp_cvx(fd.cvx) && compare_elapsed(birth, fd.date, &tp_7m) == std::cmp::Ordering::Less {
                        if compare_elapsed(birth, dose.date, &tp_12m) == std::cmp::Ordering::Less {
                            return Some("HIB_OMP_SERIES");
                        }
                    }
                }
            }
        }
    }

    None
}

fn is_series_complete(forecast: &VaccineGroupForecast) -> bool {
    forecast.forecasts.first().map(|f| f.status == SeriesStatus::Complete).unwrap_or(false)
}

fn matches_omp_criteria_from_eval(patient: &Patient, forecast: &VaccineGroupForecast) -> bool {
    let birth = patient.birth_date;
    let tp_7m = crate::time_period!("7m");
    let tp_12m = crate::time_period!("12m");

    let valid_omp_doses: Vec<&crate::models::DoseEvaluation> = forecast.evaluations.iter()
        .filter(|e| e.status == DoseStatus::Valid && is_omp_cvx(e.cvx))
        .collect();

    let total_hib_evals = forecast.evaluations.len();
    if total_hib_evals == 1 && valid_omp_doses.len() == 1 {
        let e = valid_omp_doses[0];
        if compare_elapsed(birth, e.dose_date, &tp_7m) == std::cmp::Ordering::Less {
            return true;
        }
    }

    if valid_omp_doses.len() >= 2 {
        let e1 = valid_omp_doses[0];
        let e2 = valid_omp_doses[1];
        let e1_lt_7m = compare_elapsed(birth, e1.dose_date, &tp_7m) == std::cmp::Ordering::Less;
        let e2_lt_12m = compare_elapsed(birth, e2.dose_date, &tp_12m) == std::cmp::Ordering::Less;
        if e1_lt_7m && e2_lt_12m {
            return true;
        }
    }

    false
}

pub fn hib_group_selection(
    patient: &Patient,
    _history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let omp_name = "HIB_OMP_SERIES";
    let four_dose_name = "HIB_4_DOSE_SERIES";

    let omp_exists = candidate_forecasts.contains_forecast(&omp_name);
    let four_dose_exists = candidate_forecasts.contains_forecast(&four_dose_name);

    if omp_exists && four_dose_exists {
        let omp_fc = candidate_forecasts.get_forecast(&omp_name).unwrap();
        let four_dose_fc = candidate_forecasts.get_forecast(&four_dose_name).unwrap();

        let default_selection = if matches_omp_criteria_from_eval(patient, omp_fc) {
            &omp_name
        } else {
            &four_dose_name
        };

        let selected_complete = if default_selection == &omp_name {
            is_series_complete(omp_fc)
        } else {
            is_series_complete(four_dose_fc)
        };
        let other_complete = if default_selection == &omp_name {
            is_series_complete(four_dose_fc)
        } else {
            is_series_complete(omp_fc)
        };

        if !selected_complete && other_complete {
            return if default_selection == &omp_name {
                four_dose_name
            } else {
                omp_name
            };
        }

        return default_selection;
    }

    four_dose_name
}
