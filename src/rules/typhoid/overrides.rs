use crate::engine::EvaluationContext;
use crate::models::{Cvx, Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus};
use chrono::NaiveDate;

pub fn typhoid_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        if dose.cvx.0 == 25 {
            let tp_6y_minus_4d = crate::date_utils::TimePeriod::parse("6y-4d").unwrap();
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
        forecast.reasons = vec![
            "COMPLETE_HIGH_RISK".to_string(),
            "TYPHOID_NOT_ROUTINE_SEE_ACIP".to_string(),
        ];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    let birth = patient.birth_date;
    let tp_2y = crate::date_utils::TimePeriod::parse("2y").unwrap();
    let age_2y_date = tp_2y.add_to(birth);
    let is_under_2y = eval_date < age_2y_date;

    if is_under_2y {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = vec!["TYPHOID_NOT_ROUTINE_SEE_ACIP".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
    } else {
        // >= 2 years old and not complete
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = vec![
            "HIGH_RISK".to_string(),
            "TYPHOID_NOT_ROUTINE_SEE_ACIP".to_string(),
        ];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
    }
}
