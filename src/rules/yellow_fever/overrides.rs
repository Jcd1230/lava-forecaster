use crate::date_utils::SmallVec;
use crate::engine::EvaluationContext;
use crate::models::{Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus};
use chrono::NaiveDate;

pub fn yellow_fever_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    _ctx: &EvaluationContext,
    _reasons: &mut SmallVec<[EvaluationReason; 4]>,
    _status: &mut DoseStatus,
) {
    // Standard CDSi evaluation is sufficient.
}

pub fn yellow_fever_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _evaluations: &[crate::models::DoseEvaluation],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = crate::reasons![
            "COMPLETE_HIGH_RISK",
            "YELLOW_FEVER_LIVE_MIN_INTERVALS_SEE_ACIP",
        ];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
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
        forecast.reasons = crate::reasons!["YELLOW_FEVER_LIVE_MIN_INTERVALS_SEE_ACIP"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    } else if is_under_9m {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons![
            "BELOW_REC_AGE_SERIES",
            "HIGH_RISK",
            "YELLOW_FEVER_LIVE_MIN_INTERVALS_SEE_ACIP",
        ];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    } else {
        // >= 9 months old and incomplete
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons![
            "HIGH_RISK",
            "YELLOW_FEVER_LIVE_MIN_INTERVALS_SEE_ACIP",
        ];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    }
}
