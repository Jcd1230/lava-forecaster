use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus};

pub fn cholera_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    _ctx: &EvaluationContext,
    _reasons: &mut Vec<EvaluationReason>,
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
        forecast.reasons = vec!["COMPLETE".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    let birth = patient.birth_date;
    let tp_2y = crate::date_utils::TimePeriod::parse("2y").unwrap();
    let tp_65y = crate::date_utils::TimePeriod::parse("65y").unwrap();

    let age_2y_date = tp_2y.add_to(birth);
    let age_65y_date = tp_65y.add_to(birth);

    let is_under_2y = eval_date < age_2y_date;
    let is_under_65y = eval_date < age_65y_date;

    if is_under_2y {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = vec!["CHOLERA_NOT_ROUTINE_SEE_ACIP".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
    } else if is_under_65y {
        if valid_doses.is_empty() {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = vec![
                "HIGH_RISK".to_string(),
                "CHOLERA_NOT_ROUTINE_SEE_ACIP".to_string(),
            ];
            forecast.earliest_date = None;
            forecast.recommended_date = None;
            forecast.overdue_date = None;
            forecast.latest_date = None;
        } else {
            if !forecast.reasons.contains(&"CHOLERA_NOT_ROUTINE_SEE_ACIP".to_string()) {
                forecast.reasons.push("CHOLERA_NOT_ROUTINE_SEE_ACIP".to_string());
            }
        }
    } else {
        // >= 65 years old
        if valid_doses.is_empty() {
            forecast.status = SeriesStatus::NotRecommended;
            forecast.reasons = vec![
                "TOO_OLD".to_string(),
                "CHOLERA_NOT_ROUTINE_SEE_ACIP".to_string(),
            ];
            forecast.earliest_date = None;
            forecast.recommended_date = None;
            forecast.overdue_date = None;
            forecast.latest_date = None;
        } else {
            if !forecast.reasons.contains(&"CHOLERA_NOT_ROUTINE_SEE_ACIP".to_string()) {
                forecast.reasons.push("CHOLERA_NOT_ROUTINE_SEE_ACIP".to_string());
            }
        }
    }
}
