use crate::date_utils::TinyVec;
use crate::engine::CandidateForecastsExt;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Dose,
    DoseStatus,
    EvaluationReason,
    Patient,
    SeriesForecast,
    SeriesStatus,
    VaccineGroupForecast,
};

const MPOX_1_DOSE_SERIES: &str = "MPOX_1_DOSE_SERIES";
const MPOX_2_DOSE_SERIES: &str = "MPOX_2_DOSE_SERIES";

fn series_primary_dose_count(series_name: &str) -> Option<usize> {
    match series_name {
        MPOX_1_DOSE_SERIES => Some(1),
        MPOX_2_DOSE_SERIES => Some(2),
        _ => None,
    }
}

fn first_valid_dose_date(forecast: &VaccineGroupForecast, dose_number: usize) -> Option<NaiveDate> {
    forecast
        .evaluations
        .iter()
        .find(|evaluation| {
            evaluation.status == DoseStatus::Valid && evaluation.dose_number == Some(dose_number)
        })
        .map(|evaluation| evaluation.dose_date)
}

fn completion_date(series_name: &str, forecast: &VaccineGroupForecast) -> Option<NaiveDate> {
    first_valid_dose_date(forecast, series_primary_dose_count(series_name)?)
}

pub fn mpox_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
) {
    let Some(primary_dose_count) = series_primary_dose_count(series_name) else {
        return;
    };

    if reasons.contains(&EvaluationReason::VaccineNotPartOfSeries)
        && target_dose_idx > 1
        && ctx.valid_doses.len() < primary_dose_count
    {
        *status = DoseStatus::Accepted;
        reasons.clear();
        reasons.push(EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven);
    }
}

pub fn mpox_custom_extra_dose_hook(
    series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, TinyVec<EvaluationReason, 4>)> {
    let primary_dose_count = series_primary_dose_count(series_name)?;
    if ctx.valid_doses.len() == primary_dose_count
        && ctx.target_dose_number == primary_dose_count + 1
    {
        return Some((DoseStatus::Valid, crate::reasons![EvaluationReason::BoosterDose]));
    }

    None
}

pub fn mpox_custom_forecast_hook(
    _patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    _eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.reasons = crate::reasons!["COMPLETE_HIGH_RISK"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if valid_doses.is_empty() {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons!["HIGH_RISK"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    }
}

pub fn mpox_group_selection(
    _patient: &Patient,
    _history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let two_dose = candidate_forecasts.get_forecast(MPOX_2_DOSE_SERIES);
    let one_dose = candidate_forecasts.get_forecast(MPOX_1_DOSE_SERIES);

    let two_dose_first = two_dose.and_then(|forecast| first_valid_dose_date(forecast, 1));
    let one_dose_first = one_dose.and_then(|forecast| first_valid_dose_date(forecast, 1));

    let mut selected = if let Some(two_date) = two_dose_first {
        if one_dose_first.is_none_or(|one_date| one_date >= two_date) {
            MPOX_2_DOSE_SERIES
        } else {
            MPOX_1_DOSE_SERIES
        }
    } else if one_dose_first.is_some() {
        MPOX_1_DOSE_SERIES
    } else {
        MPOX_2_DOSE_SERIES
    };

    let selected_complete_date = candidate_forecasts
        .get_forecast(selected)
        .and_then(|forecast| completion_date(selected, forecast));

    let other_name = if selected == MPOX_2_DOSE_SERIES {
        MPOX_1_DOSE_SERIES
    } else {
        MPOX_2_DOSE_SERIES
    };
    let other_complete_date = candidate_forecasts
        .get_forecast(other_name)
        .and_then(|forecast| completion_date(other_name, forecast));

    if let Some(other_date) = other_complete_date {
        if selected_complete_date.is_none_or(|selected_date| other_date < selected_date) {
            selected = other_name;
        }
    }

    selected
}
