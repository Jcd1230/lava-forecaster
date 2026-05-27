use crate::date_utils::TinyVec;
use ice_cvx_macro::cvx;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus};
use chrono::NaiveDate;

pub fn typhoid_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        if dose.cvx.0 == cvx!("25") {
            let tp_6y_minus_4d = crate::time_period!("6y-4d");
            let abs_min_date = tp_6y_minus_4d.add_to(ctx.patient.birth_date);
            if dose.date < abs_min_date {
                *status = DoseStatus::Invalid;
                if !reasons.contains(&EvaluationReason::BelowMinimumAge) {
                    reasons.push(EvaluationReason::BelowMinimumAge);
                }
            }
        }
    }
}

pub fn typhoid_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = crate::reasons![
            "COMPLETE_HIGH_RISK",
            "TYPHOID_NOT_ROUTINE_SEE_ACIP",
        ];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    let birth = patient.birth_date;
    let tp_2y = crate::time_period!("2y");
    let age_2y_date = tp_2y.add_to(birth);
    let is_under_2y = eval_date < age_2y_date;

    if is_under_2y {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = crate::reasons!["TYPHOID_NOT_ROUTINE_SEE_ACIP"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    } else {
        // >= 2 years old and not complete
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons![
            "HIGH_RISK",
            "TYPHOID_NOT_ROUTINE_SEE_ACIP",
        ];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    }
}
