use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast};
use crate::date_utils::{TimePeriod, compare_elapsed};

pub fn is_hib_cvx(cvx: &str) -> bool {
    const HIB_CVX: &[&str] = &[
        "17", "22", "46", "47", "48", "49", "50", "51", "102", "120", "132", "146", "148", "170", "198"
    ];
    HIB_CVX.contains(&cvx)
}

pub fn is_omp_cvx(cvx: &str) -> bool {
    const OMP_CVX: &[&str] = &["49", "51"];
    OMP_CVX.contains(&cvx)
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
    let is_age_ge = |age_str: &str| -> bool {
        let tp = TimePeriod::parse(age_str).unwrap();
        compare_elapsed(birth, ref_date, &tp) != std::cmp::Ordering::Less
    };
    let is_age_lt = |age_str: &str| -> bool {
        let tp = TimePeriod::parse(age_str).unwrap();
        compare_elapsed(birth, ref_date, &tp) == std::cmp::Ordering::Less
    };

    // Calculate number of Hib doses in overall history administered before 12m of age
    let count_hib_before_12m = ctx.history.iter()
        .filter(|d| {
            is_hib_cvx(&d.cvx) && {
                let tp_12m = TimePeriod::parse("12m").unwrap();
                compare_elapsed(birth, d.date, &tp_12m) == std::cmp::Ordering::Less
            }
        })
        .count();

    // Check age ranges:
    if is_age_ge("15m") {
        // Skip to 4: Patient over 15 Months
        if target_dose_idx < 4 {
            target_dose_idx = 4;
        }
    } else if is_age_ge("12m") && is_age_lt("15m") {
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
    } else if is_age_ge("12m-28d") && is_age_lt("12m") {
        // Patient between 12m-28d and 12m
        // Rule: Skip to 3 if patient has received exactly 1 prior dose which was administered < 7m of age
        let prior_hib_count = ctx.history.iter()
            .filter(|d| is_hib_cvx(&d.cvx) && d.date < ref_date)
            .count();
        if prior_hib_count == 1 {
            let prior_dose = ctx.history.iter().find(|d| is_hib_cvx(&d.cvx) && d.date < ref_date).unwrap();
            let tp_7m = TimePeriod::parse("7m").unwrap();
            let prior_dose_lt_7m = compare_elapsed(birth, prior_dose.date, &tp_7m) == std::cmp::Ordering::Less;
            if prior_dose_lt_7m {
                if target_dose_idx < 3 {
                    target_dose_idx = 3;
                }
            }
        }
    } else if is_age_ge("7m") && is_age_lt("12m") {
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
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    let birth = ctx.patient.birth_date;
    let dose = match ctx.current_dose {
        Some(d) => d,
        None => return,
    };
    let admin_date = dose.date;

    // 1. Booster Check (CVX 50)
    if dose.cvx == "50" {
        let tp_1y_4d = TimePeriod::parse("1y-4d").unwrap();
        let tp_5y = TimePeriod::parse("5y").unwrap();
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
    let tp_5y = TimePeriod::parse("5y").unwrap();
    let age_ge_5y = compare_elapsed(birth, admin_date, &tp_5y) != std::cmp::Ordering::Less;
    if age_ge_5y {
        let num_doses = if series_name == "HIB_OMP_SERIES" { 3 } else { 4 };
        let valid_before_5y = ctx.valid_doses.iter()
            .filter(|(date, _)| *date < tp_5y.add_to(birth))
            .count();
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
        let tp_1y_4d = TimePeriod::parse("1y-4d").unwrap();
        let age_lt_1y_4d = compare_elapsed(birth, admin_date, &tp_1y_4d) == std::cmp::Ordering::Less;
        if age_lt_1y_4d {
            let tp_7m = TimePeriod::parse("7m").unwrap();
            let count_before_7m = ctx.history.iter()
                .filter(|d| is_hib_cvx(&d.cvx) && d.date < tp_7m.add_to(birth))
                .count();
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

    // Check if patient received CVX 50 and there are no valid doses
    let has_cvx50 = history.iter().any(|d| d.cvx == "50");
    if valid_doses.is_empty() && has_cvx50 {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }


    // Check if patient is >= 5 years of age at evaluation date
    let age_5y = TimePeriod::parse("5y").unwrap().add_to(birth);
    let is_eval_ge_5y = eval_date >= age_5y;

    if is_eval_ge_5y {
        let num_doses = if forecast.series_name == "HIB_OMP_SERIES" { 3 } else { 4 };
        let valid_before_5y = valid_doses.iter()
            .filter(|(date, _)| *date < age_5y)
            .count();

        if valid_before_5y < num_doses {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = vec!["HIGH_RISK".to_string()];
            forecast.earliest_date = None;
            forecast.recommended_date = None;
            forecast.overdue_date = None;
            forecast.latest_date = None;
            return;
        }
    }

    // Recommended Date Overrides for HIB_4_DOSE_SERIES
    if forecast.series_name == "HIB_4_DOSE_SERIES" && forecast.status != SeriesStatus::Complete {
        let tp_7m = TimePeriod::parse("7m").unwrap();
        let tp_12m = TimePeriod::parse("12m").unwrap();
        let tp_15m = TimePeriod::parse("15m").unwrap();

        let date_7m = tp_7m.add_to(birth);
        let date_12m = tp_12m.add_to(birth);
        let date_15m = tp_15m.add_to(birth);

        let eval_ge_7m = eval_date >= date_7m;
        let eval_lt_12m = eval_date < date_12m;
        let eval_ge_12m = eval_date >= date_12m;
        let eval_lt_15m = eval_date < date_15m;
        let eval_ge_15m = eval_date >= date_15m;
        let eval_lt_5y = eval_date < age_5y;

        if eval_ge_7m && eval_lt_12m {
            let count_before_7m = history.iter()
                .filter(|d| is_hib_cvx(&d.cvx) && d.date < date_7m)
                .count();
            if count_before_7m == 0 {
                forecast.recommended_date = Some(date_7m);
                forecast.earliest_date = Some(date_7m);
            }
        } else if eval_ge_12m && eval_lt_15m {
            let count_before_12m = history.iter()
                .filter(|d| is_hib_cvx(&d.cvx) && d.date < date_12m)
                .count();
            if count_before_12m < 2 {
                forecast.recommended_date = Some(date_12m);
                forecast.earliest_date = Some(date_12m);
            } else if count_before_12m == 2 {
                forecast.recommended_date = Some(date_12m);
                forecast.earliest_date = Some(date_12m);
            }
        } else if eval_ge_15m && eval_lt_5y {
            let count_before_15m = history.iter()
                .filter(|d| is_hib_cvx(&d.cvx) && d.date < date_15m)
                .count();
            if count_before_15m < 4 {
                forecast.recommended_date = Some(date_15m);
                forecast.earliest_date = Some(date_15m);
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
    let tp_7m = TimePeriod::parse("7m").unwrap();
    let tp_12m = TimePeriod::parse("12m").unwrap();

    if let Some(dose) = ctx.current_dose {
        if is_omp_cvx(&dose.cvx) {
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
                    if is_omp_cvx(&fd.cvx) && compare_elapsed(birth, fd.date, &tp_7m) == std::cmp::Ordering::Less {
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
    let tp_7m = TimePeriod::parse("7m").unwrap();
    let tp_12m = TimePeriod::parse("12m").unwrap();

    let valid_omp_doses: Vec<&crate::models::DoseEvaluation> = forecast.evaluations.iter()
        .filter(|e| e.status == DoseStatus::Valid && is_omp_cvx(&e.cvx))
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
    candidate_forecasts: &mut std::collections::HashMap<String, VaccineGroupForecast>,
) -> String {
    let omp_name = "HIB_OMP_SERIES".to_string();
    let four_dose_name = "HIB_4_DOSE_SERIES".to_string();

    let omp_exists = candidate_forecasts.contains_key(&omp_name);
    let four_dose_exists = candidate_forecasts.contains_key(&four_dose_name);

    if omp_exists && four_dose_exists {
        let omp_complete = is_series_complete(candidate_forecasts.get(&omp_name).unwrap());
        let four_dose_complete = is_series_complete(candidate_forecasts.get(&four_dose_name).unwrap());

        if omp_complete && !four_dose_complete {
            return omp_name;
        }
        if four_dose_complete && !omp_complete {
            return four_dose_name;
        }

        let omp_fc = candidate_forecasts.get(&omp_name).unwrap();
        if matches_omp_criteria_from_eval(patient, omp_fc) {
            return omp_name;
        }
    }

    four_dose_name
}
