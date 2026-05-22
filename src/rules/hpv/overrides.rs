use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast};
use crate::date_utils::{TimePeriod, add_years};
use std::collections::HashMap;

pub fn hpv_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        // 1. Gender-specific restriction: CVX 118 (bivalent) is not licensed for males
        if dose.cvx == "118" && ctx.patient.gender == crate::models::Gender::Male {
            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::VaccineNotLicensedForMales);
            return;
        }

        // 2. Age-based acceptance cap: if >= 46 years and series is not complete, mark Accepted
        let age_46 = add_years(ctx.patient.birth_date, 46);
        if dose.date >= age_46 {
            // Check if series is not complete before this dose
            if ctx.valid_doses.len() < target_dose_idx {
                *status = DoseStatus::Accepted;
                reasons.clear();
                reasons.push(EvaluationReason::AboveRecommendedAgeSeries);
                return;
            }
        }

        // 3. Absolute minimum interval 1->3 for HPV_3_DOSE_SERIES
        if series_name == "HPV_3_DOSE_SERIES" && target_dose_idx == 3 {
            if ctx.valid_doses.len() >= 2 {
                let dose_1_date = ctx.valid_doses[0].0;
                let min_int_1_3 = TimePeriod::parse("5m-4d").unwrap().add_to(dose_1_date);
                if dose.date < min_int_1_3 {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }
    }
}

pub fn hpv_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let age_46 = add_years(patient.birth_date, 46);
    if eval_date >= age_46 {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
    }
}

pub fn hpv_group_selection(
    patient: &Patient,
    _history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut HashMap<String, VaccineGroupForecast>,
) -> String {
    let forecast_2 = candidate_forecasts.get("HPV_2_DOSE_SERIES").unwrap();
    let forecast_3 = candidate_forecasts.get("HPV_3_DOSE_SERIES").unwrap();

    // Find the first valid dose date in either series (will be identical since dose 1 requirements are same)
    let first_valid_dose_date = forecast_2.evaluations.iter()
        .filter(|e| e.status == DoseStatus::Valid)
        .map(|e| e.dose_date)
        .min();

    match first_valid_dose_date {
        None => {
            // No doses administered yet: select based on age on evaluation date
            let age_15 = add_years(patient.birth_date, 15);
            if eval_date < age_15 {
                "HPV_2_DOSE_SERIES".to_string()
            } else {
                "HPV_3_DOSE_SERIES".to_string()
            }
        }
        Some(d1_date) => {
            let age_15_at_d1 = add_years(patient.birth_date, 15);
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
                        "HPV_3_DOSE_SERIES".to_string()
                    } else {
                        "HPV_2_DOSE_SERIES".to_string()
                    }
                } else if has_valid_d2_in_3_dose {
                    // 2-dose is not satisfied, but 3-dose is
                    "HPV_3_DOSE_SERIES".to_string()
                } else {
                    // Only 1 dose, or no valid doses at all
                    "HPV_2_DOSE_SERIES".to_string()
                }
            } else {
                // Initiated at or after age 15 -> must use 3-dose series
                "HPV_3_DOSE_SERIES".to_string()
            }
        }
    }
}
