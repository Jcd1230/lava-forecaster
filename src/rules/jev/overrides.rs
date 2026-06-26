use crate::date_utils::{SmallVec, add_years_unchecked};
use crate::engine::CandidateForecastsExt;
use crate::engine::EvaluationContext;
use crate::engine::ValidDoseRef;
use crate::models::{
    Dose, DoseStatus, EvaluationReason, ForecastReason, Patient, SeriesForecast, SeriesStatus,
    VaccineGroupForecast,
};
use chrono::NaiveDate;

pub fn jev_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        if target_dose_idx == 2 {
            let age_18 = add_years_unchecked(ctx.patient.birth_date, 18);
            let age_66 = add_years_unchecked(ctx.patient.birth_date, 66);

            if series_name == "JEVC_RISK_2_DOSE_SERIES" {
                if dose.date >= age_18 && dose.date < age_66 {
                    if let Some(prev) = ctx.valid_doses.last() {
                        let prev_date = prev.date;
                        let days = (dose.date - prev_date).num_days();
                        if days >= 7 {
                            *status = DoseStatus::Valid;
                            reasons.retain(|r| *r != EvaluationReason::BelowMinimumInterval);
                        }
                    }
                }
            } else if series_name == "JEVC_RISK_2_DOSE_ACCELERATED_SERIES" {
                if dose.date >= age_66 {
                    if let Some(prev) = ctx.valid_doses.last() {
                        let prev_date = prev.date;
                        let days = (dose.date - prev_date).num_days();
                        if days < 24 {
                            *status = DoseStatus::Invalid;
                            if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                                reasons.push(EvaluationReason::BelowMinimumInterval);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn jev_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[ValidDoseRef],
    _evaluations: &[crate::models::DoseEvaluation],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons =
            crate::forecast_reasons!["COMPLETE_HIGH_RISK", "JE_NOT_ROUTINE_ACCEL_18_65_SEE_ACIP",];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if valid_doses.is_empty() {
        let age_2m = crate::time_period!("2m").add_to(patient.birth_date);
        if eval_date < age_2m {
            forecast.status = SeriesStatus::NotRecommended;
            forecast.reasons = crate::forecast_reasons!["JE_NOT_ROUTINE_ACCEL_18_65_SEE_ACIP"];
        } else {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons =
                crate::forecast_reasons!["HIGH_RISK", "JE_NOT_ROUTINE_ACCEL_18_65_SEE_ACIP",];
        }
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if valid_doses.len() == 1 {
        if !forecast
            .reasons
            .iter()
            .any(|r| *r == ForecastReason::JeNotRoutineAccel18To65SeeAcip)
        {
            forecast
                .reasons
                .push(ForecastReason::JeNotRoutineAccel18To65SeeAcip);
        }

        if forecast.series_name == "JEVC_RISK_2_DOSE_ACCELERATED_SERIES" {
            let prev_dose_date = valid_doses[0].date;
            let age_66_minus_7d = crate::time_period!("66y-7d").add_to(patient.birth_date);

            if prev_dose_date >= age_66_minus_7d {
                forecast.status = forecast.status.with_overdue_date(None);
            }

            let age_66 = add_years_unchecked(patient.birth_date, 66);
            if forecast
                .status
                .recommended_date()
                .map(|d| d >= age_66)
                .unwrap_or(false)
            {
                let new_date = prev_dose_date + chrono::Duration::days(28);
                forecast.status = forecast.status.with_earliest_date(Some(new_date));
                forecast.status = forecast.status.with_recommended_date(Some(new_date));
                forecast.status = forecast.status.with_overdue_date(None);
            }
        }
    }
}

pub fn jev_group_selection(
    patient: &Patient,
    _history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let first_valid_dose_date = candidate_forecasts
        .get_forecast("JEVC_RISK_2_DOSE_SERIES")
        .and_then(|f| {
            f.evaluations
                .iter()
                .find(|e| e.status == DoseStatus::Valid && e.dose_number == Some(1))
                .map(|e| e.dose_date)
        });

    let age_at_reference = first_valid_dose_date.unwrap_or(eval_date);

    let tp_18y_minus_4d = crate::time_period!("18y-4d");
    let age_18_minus_4d = tp_18y_minus_4d.add_to(patient.birth_date);
    let age_66 = add_years_unchecked(patient.birth_date, 66);

    let select_accelerated = age_at_reference >= age_18_minus_4d && age_at_reference < age_66;

    if select_accelerated {
        "JEVC_RISK_2_DOSE_ACCELERATED_SERIES"
    } else {
        "JEVC_RISK_2_DOSE_SERIES"
    }
}
