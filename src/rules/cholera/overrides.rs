use crate::date_utils::TinyVec;
use ice_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus};

pub fn cholera_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    _ctx: &EvaluationContext,
    _reasons: &mut TinyVec<EvaluationReason, 4>,
    _status: &mut DoseStatus,
) {
    // Standard CDSi parameters in schedules.rs are sufficient; no custom dose evaluation logic needed.
}

pub fn cholera_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.reasons = crate::reasons!["COMPLETE"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    let birth = patient.birth_date;
    let tp_2y = crate::time_period!("2y");
    let tp_65y = crate::time_period!("65y");

    let age_2y_date = tp_2y.add_to(birth);
    let age_65y_date = tp_65y.add_to(birth);

    let is_under_2y = eval_date < age_2y_date;
    let is_under_65y = eval_date < age_65y_date;

    if is_under_2y {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = crate::reasons!["CHOLERA_NOT_ROUTINE_SEE_ACIP"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    } else if is_under_65y {
        if valid_doses.is_empty() {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = crate::reasons![
                "HIGH_RISK",
                "CHOLERA_NOT_ROUTINE_SEE_ACIP",
            ];
            forecast.status = forecast.status.with_earliest_date(None);
            forecast.status = forecast.status.with_recommended_date(None);
            forecast.status = forecast.status.with_overdue_date(None);
            forecast.status = forecast.status.with_latest_date(None);
        } else {
            if !forecast.reasons.iter().any(|r| r == "CHOLERA_NOT_ROUTINE_SEE_ACIP") {
                forecast.reasons.push("CHOLERA_NOT_ROUTINE_SEE_ACIP".into());
            }
        }
    } else {
        // >= 65 years old
        if valid_doses.is_empty() {
            forecast.status = SeriesStatus::NotRecommended;
            forecast.reasons = crate::reasons![
                "TOO_OLD",
                "CHOLERA_NOT_ROUTINE_SEE_ACIP",
            ];
            forecast.status = forecast.status.with_earliest_date(None);
            forecast.status = forecast.status.with_recommended_date(None);
            forecast.status = forecast.status.with_overdue_date(None);
            forecast.status = forecast.status.with_latest_date(None);
        } else {
            if !forecast.reasons.iter().any(|r| r == "CHOLERA_NOT_ROUTINE_SEE_ACIP") {
                forecast.reasons.push("CHOLERA_NOT_ROUTINE_SEE_ACIP".into());
            }
        }
    }
}
