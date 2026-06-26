use crate::date_utils::{SmallVec, TimePeriod, add_years_unchecked};
use crate::engine::EvaluationContext;
use crate::engine::ValidDoseRef;
use crate::models::{
    Cvx, Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus,
};
use chrono::NaiveDate;
use lava_cvx_macro::cvx;

fn get_vaccine_min_age(cvx: Cvx) -> Option<TimePeriod> {
    match cvx.0 {
        cvx!("114") | cvx!("147") | cvx!("316") => Some(crate::time_period!("9m-4d")),
        cvx!("136") | cvx!("328") => Some(crate::time_period!("2m-4d")),
        cvx!("203") | cvx!("108") | cvx!("32") => Some(crate::time_period!("2y-4d")),
        _ => None,
    }
}

pub fn mcv_completion_condition(ctx: &EvaluationContext) -> bool {
    if ctx.valid_doses.len() == 1 {
        let dose1 = ctx.valid_doses[0];
        if dose1.dose_number == 1 {
            let dose1_date = dose1.date;
            let age_16 = add_years_unchecked(ctx.patient.birth_date, 16);
            let age_19 = add_years_unchecked(ctx.patient.birth_date, 19);
            if dose1_date >= age_16 && dose1_date < age_19 {
                // Confirm no prior administered doses before age 16y
                let count_before_16 = ctx.history.iter().filter(|d| d.date < age_16).count();
                if count_before_16 == 0 {
                    return true;
                }
            }
        }
    }
    false
}

pub fn mcv_custom_evaluation_hook(
    _series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        let birth_date = ctx.patient.birth_date;

        // 1. Dose 1 Evaluation Age Override
        if target_dose_idx == 1 {
            let abs_min_age_10y = add_years_unchecked(birth_date, 10);
            if dose.date < abs_min_age_10y {
                if let Some(min_age_tp) = get_vaccine_min_age(dose.cvx) {
                    let min_age_date = min_age_tp.add_to(birth_date);
                    if dose.date >= min_age_date {
                        *status = DoseStatus::Accepted;
                        if !reasons.contains(&EvaluationReason::BelowMinimumAge) {
                            reasons.push(EvaluationReason::BelowMinimumAge);
                        }
                        if !reasons.contains(&EvaluationReason::OutsideRoutineSeries) {
                            reasons.push(EvaluationReason::OutsideRoutineSeries);
                        }
                    }
                }
            }
        }

        // 2. Late Dose Evaluation (>= 22y)
        let age_22 = add_years_unchecked(birth_date, 22);
        if dose.date >= age_22 {
            let age_19 = add_years_unchecked(birth_date, 19);
            let valid_before_19 = ctx
                .valid_doses
                .iter()
                .filter(|dose| dose.date < age_19)
                .count();
            if valid_before_19 < 2 {
                *status = DoseStatus::Accepted;
                reasons.clear();
                reasons.push(EvaluationReason::AboveRecommendedAgeSeries);
                reasons.push(EvaluationReason::OutsideRoutineSeries);
            }
        }
    }
}

pub fn mcv_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[ValidDoseRef],
    _evaluations: &[crate::models::DoseEvaluation],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    // 1. If patient has completed the series, forecast is Complete / COMPLETE_HIGH_RISK
    if forecast.status == SeriesStatus::Complete {
        forecast.mark_complete(crate::forecast_reasons!["COMPLETE_HIGH_RISK"]);
        return;
    }

    // 2. If patient is >= 19 years and did NOT complete the series before 19 years of age, Recommendation is Conditional/HIGH_RISK
    let age_19 = add_years_unchecked(patient.birth_date, 19);
    if eval_date >= age_19 {
        forecast.mark_conditionally_recommended(crate::forecast_reasons!["HIGH_RISK"]);
        return;
    }

    // 3. Recommend Dose 1 at 16yrs of age if Patient >= 16yrs and < 19yrs of Age with 0 doses
    if valid_doses.is_empty() {
        let age_16 = add_years_unchecked(patient.birth_date, 16);
        if eval_date >= age_16 && eval_date < age_19 {
            forecast.status = forecast.status.with_recommended_date(Some(age_16));
        }
    }
}
