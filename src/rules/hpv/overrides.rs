use crate::engine::CandidateForecastsExt;
use ice_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast};
use crate::date_utils::{TinyVec, TimePeriod, add_years_unchecked};

fn is_hpv_cvx(cvx: Cvx) -> bool {
    matches!(cvx.0, cvx!("62") | cvx!("118") | cvx!("137") | cvx!("165"))
}

fn add_interval(date: NaiveDate, interval: TimePeriod) -> NaiveDate {
    interval.add_to(date)
}

fn latest_boundary(date: NaiveDate, interval: TimePeriod) -> NaiveDate {
    add_interval(date, interval) - chrono::Duration::days(1)
}

pub fn hpv_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        // 1. Gender-specific restriction: CVX 118 (bivalent) is not licensed for males
        if dose.cvx.0 == cvx!("118") && ctx.patient.gender == crate::models::Gender::Male {
            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::VaccineNotLicensedForMales);
            return;
        }

        // 2. Age-based acceptance cap: if >= 46 years and series is not complete, mark Accepted
        let age_46 = add_years_unchecked(ctx.patient.birth_date, 46);
        if dose.date >= age_46 {
            // Check if series is not complete before this dose
            if ctx.valid_doses.len() < target_dose_idx {
                *status = DoseStatus::Accepted;
                reasons.clear();
                reasons.push(EvaluationReason::AboveRecommendedAgeSeries);
                reasons.push(EvaluationReason::OutsideRoutineSeries);
                return;
            }
        }

        // 3. Absolute minimum interval 1->3 for HPV_3_DOSE_SERIES
        // For shots before 12/16/2016: abs_min 1->3 is 16w-4d (108 days)
        // For shots on or after 12/16/2016: abs_min 1->3 is 5m-4d (~147 days)
        if series_name == "HPV_3_DOSE_SERIES" && target_dose_idx == 3 {
            if ctx.valid_doses.len() >= 2 {
                let dose_1_date = ctx.valid_doses[0].0;
                let dose_2_date = ctx.valid_doses[1].0;
                let cutoff_2016 = NaiveDate::from_ymd_opt(2016, 12, 16).unwrap();
                let abs_min_1_3 = if dose.date < cutoff_2016 {
                    crate::time_period!("16w-4d") // Pre-2016: 108 days
                } else {
                    crate::time_period!("5m-4d")  // Post-2016: ~147 days
                };
                let min_int_1_3 = abs_min_1_3.add_to(dose_1_date);
                let min_int_2_3 = crate::time_period!("80d").add_to(dose_2_date);
                if dose.date < min_int_1_3 || dose.date < min_int_2_3 {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                } else if reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                    reasons.retain(|reason| *reason != EvaluationReason::BelowMinimumInterval);
                    *status = DoseStatus::Valid;
                }
            }
        }
    }
}

pub fn hpv_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        return;
    }

    let age_15 = add_years_unchecked(patient.birth_date, 15);
    let age_27 = add_years_unchecked(patient.birth_date, 27);
    let age_46 = add_years_unchecked(patient.birth_date, 46);

    let has_hpv_history = _history.iter().any(|dose| is_hpv_cvx(dose.cvx));

    if !has_hpv_history && eval_date >= age_27 && eval_date < age_46 {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        return;
    }

    if eval_date >= age_46 {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        return;
    }

    if valid_doses.is_empty() {
        if eval_date >= age_15 {
            forecast.status = forecast.status.with_overdue_date(Some(age_15 - chrono::Duration::days(1)));
        }
        return;
    }

    let first_valid_date = valid_doses[0].0;
    let started_at_or_after_15 = first_valid_date >= age_15;
    let hpv_history_dates: Vec<NaiveDate> = _history
        .iter()
        .filter(|dose| is_hpv_cvx(dose.cvx))
        .map(|dose| dose.date)
        .collect();
    let latest_hpv_dose_date = hpv_history_dates
        .iter()
        .copied()
        .max()
        .unwrap_or(first_valid_date);
    let hpv_history_after_first_valid = hpv_history_dates
        .iter()
        .filter(|&&date| date > first_valid_date)
        .count();
    let valid_doses_after_first_valid = valid_doses.len().saturating_sub(1);
    let has_extra_hpv_history_after_first_valid =
        hpv_history_after_first_valid > valid_doses_after_first_valid;

    if valid_doses.len() == 1 && started_at_or_after_15 {
        forecast.status = forecast.status.with_overdue_date(Some(latest_boundary(latest_hpv_dose_date, crate::time_period!("16w"))));
        return;
    }

    if valid_doses.len() == 1 && !started_at_or_after_15 && has_extra_hpv_history_after_first_valid {
        forecast.status = forecast.status.with_earliest_date(Some(add_interval(first_valid_date, crate::time_period!("5m"))));
        forecast.status = forecast.status.with_recommended_date(Some(add_interval(first_valid_date, crate::time_period!("6m"))));
        forecast.status = forecast.status.with_overdue_date(Some(latest_boundary(first_valid_date, crate::time_period!("13m+4w"))));
        return;
    }

    if valid_doses.len() >= 2 {
        let second_valid_date = valid_doses[1].0;
        let earliest_from_dose_1 = add_interval(first_valid_date, crate::time_period!("5m"));
        let earliest_from_latest = add_interval(latest_hpv_dose_date, crate::time_period!("12w"));
        let earliest = earliest_from_dose_1.max(earliest_from_latest);
        let recommended_from_dose_1 = add_interval(first_valid_date, crate::time_period!("6m"));
        let dose_1_to_2_meets_late_series_threshold =
            second_valid_date >= add_interval(first_valid_date, crate::time_period!("5m-4d"));

        forecast.status = forecast.status.with_earliest_date(Some(earliest));
        forecast.status = forecast.status.with_recommended_date(Some(if has_extra_hpv_history_after_first_valid || dose_1_to_2_meets_late_series_threshold {
            earliest
        } else {
            recommended_from_dose_1.max(earliest)
        }));
        forecast.status = forecast.status.with_overdue_date(Some(if started_at_or_after_15 {
            if has_extra_hpv_history_after_first_valid || !dose_1_to_2_meets_late_series_threshold {
                latest_boundary(first_valid_date, crate::time_period!("7m+4w"))
            } else {
                earliest
            }
        } else {
            latest_boundary(first_valid_date, crate::time_period!("13m+4w"))
        }));
    }
}

pub fn hpv_group_selection(
    patient: &Patient,
    _history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let forecast_2 = candidate_forecasts.get_forecast("HPV_2_DOSE_SERIES").unwrap();
    let forecast_3 = candidate_forecasts.get_forecast("HPV_3_DOSE_SERIES").unwrap();

    // Find the first valid dose date in either series (will be identical since dose 1 requirements are same)
    let first_valid_dose_date = forecast_2.evaluations.iter()
        .filter(|e| e.status == DoseStatus::Valid)
        .map(|e| e.dose_date)
        .min();

    match first_valid_dose_date {
        None => {
            // No doses administered yet: select based on age on evaluation date
            let age_15 = add_years_unchecked(patient.birth_date, 15);
            if eval_date < age_15 {
                "HPV_2_DOSE_SERIES"
            } else {
                "HPV_3_DOSE_SERIES"
            }
        }
        Some(d1_date) => {
            let age_15_at_d1 = add_years_unchecked(patient.birth_date, 15);
            if d1_date < age_15_at_d1 {
                // Initiated before age 15 -> eligible for 2-dose series
                let has_valid_d2_in_2_dose = forecast_2.evaluations.iter()
                    .any(|e| e.status == DoseStatus::Valid && e.dose_number == Some(2));
                
                let has_valid_d2_in_3_dose = forecast_3.evaluations.iter()
                    .any(|e| e.status == DoseStatus::Valid && e.dose_number == Some(2));

                if has_valid_d2_in_2_dose {
                    // Check if there is a valid Dose 2 in 3-dose series that was administered earlier
                    let d2_2dose_date = forecast_2.evaluations.iter()
                        .find(|e| e.status == DoseStatus::Valid && e.dose_number == Some(2))
                        .unwrap().dose_date;

                    let earlier_d2_3dose = forecast_3.evaluations.iter()
                        .any(|e| e.status == DoseStatus::Valid && e.dose_number == Some(2) && e.dose_date < d2_2dose_date);

                    if earlier_d2_3dose {
                        "HPV_3_DOSE_SERIES"
                    } else {
                        "HPV_2_DOSE_SERIES"
                    }
                } else if has_valid_d2_in_3_dose {
                    // 2-dose is not satisfied, but 3-dose is
                    "HPV_3_DOSE_SERIES"
                } else {
                    // Only 1 dose, or no valid doses at all
                    "HPV_2_DOSE_SERIES"
                }
            } else {
                // Initiated at or after age 15 -> must use 3-dose series
                "HPV_3_DOSE_SERIES"
            }
        }
    }
}
