use ice_cvx_macro::cvx;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus};
use chrono::NaiveDate;

pub fn yellow_fever_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    _ctx: &EvaluationContext,
    _reasons: &mut Vec<EvaluationReason>,
    _status: &mut DoseStatus,
) {
    // Standard CDSi evaluation is sufficient.
}

pub fn yellow_fever_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = vec![
            "COMPLETE_HIGH_RISK".to_string(),
            "YELLOW_FEVER_LIVE_MIN_INTERVALS_SEE_ACIP".to_string(),
        ];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    let birth = patient.birth_date;
    let tp_6m = crate::time_period!("6m");
    let tp_9m = crate::time_period!("9m");

    let age_6m_date = tp_6m.add_to(birth);
    let age_9m_date = tp_9m.add_to(birth);

    let is_under_6m = eval_date < age_6m_date;
    let is_under_9m = eval_date < age_9m_date;

    if is_under_6m {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = vec!["YELLOW_FEVER_LIVE_MIN_INTERVALS_SEE_ACIP".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
    } else if is_under_9m {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = vec![
            "BELOW_REC_AGE_SERIES".to_string(),
            "HIGH_RISK".to_string(),
            "YELLOW_FEVER_LIVE_MIN_INTERVALS_SEE_ACIP".to_string(),
        ];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
    } else {
        // >= 9 months old and incomplete
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = vec![
            "HIGH_RISK".to_string(),
            "YELLOW_FEVER_LIVE_MIN_INTERVALS_SEE_ACIP".to_string(),
        ];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
    }
}
