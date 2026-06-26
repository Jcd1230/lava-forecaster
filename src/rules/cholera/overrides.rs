use crate::date_utils::SmallVec;
use crate::engine::EvaluationContext;
use crate::engine::ValidDoseRef;
use crate::models::{
    Dose, DoseStatus, EvaluationReason, ForecastReason, Patient, SeriesForecast, SeriesStatus,
};
use chrono::NaiveDate;

pub fn cholera_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    _ctx: &EvaluationContext,
    _reasons: &mut SmallVec<[EvaluationReason; 4]>,
    _status: &mut DoseStatus,
) {
    // Standard CDSi parameters in schedules.rs are sufficient; no custom dose evaluation logic needed.
}

pub fn cholera_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[ValidDoseRef],
    _evaluations: &[crate::models::DoseEvaluation],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.set_reasons(crate::forecast_reasons!["COMPLETE"]);
        forecast.clear_dates();
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
        forecast.mark_not_recommended(crate::forecast_reasons!["CHOLERA_NOT_ROUTINE_SEE_ACIP"]);
    } else if is_under_65y {
        if valid_doses.is_empty() {
            forecast.mark_conditionally_recommended(crate::forecast_reasons![
                "HIGH_RISK",
                "CHOLERA_NOT_ROUTINE_SEE_ACIP",
            ]);
        } else {
            if !forecast
                .reasons
                .iter()
                .any(|r| *r == ForecastReason::CholeraNotRoutineSeeAcip)
            {
                forecast
                    .reasons
                    .push(ForecastReason::CholeraNotRoutineSeeAcip);
            }
        }
    } else {
        // >= 65 years old
        if valid_doses.is_empty() {
            forecast.mark_not_recommended(crate::forecast_reasons![
                "TOO_OLD",
                "CHOLERA_NOT_ROUTINE_SEE_ACIP",
            ]);
        } else {
            if !forecast
                .reasons
                .iter()
                .any(|r| *r == ForecastReason::CholeraNotRoutineSeeAcip)
            {
                forecast
                    .reasons
                    .push(ForecastReason::CholeraNotRoutineSeeAcip);
            }
        }
    }
}
