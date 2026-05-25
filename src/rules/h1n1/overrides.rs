use crate::date_utils::TinyVec;
use crate::engine::CandidateForecastsExt;
use ice_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast};

pub fn h1n1_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        let start = NaiveDate::from_ymd_opt(2009, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2010, 6, 30).unwrap();
        
        if dose.date < start || dose.date > end {
            *status = DoseStatus::Invalid;
            reasons.retain(|r| *r != EvaluationReason::VaccineNotPartOfSeries);
            if !reasons.contains(&EvaluationReason::OutsideFluVacSeason) {
                reasons.push(EvaluationReason::OutsideFluVacSeason);
            }
        }
    }
}

pub fn h1n1_custom_forecast_hook(
    _patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let season_end = NaiveDate::from_ymd_opt(2010, 6, 30).unwrap();

    if forecast.status == SeriesStatus::Complete {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = crate::reasons!["COMPLETE"];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    if eval_date > season_end {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = crate::reasons!["VAC_GROUP_NO_LONGER_REC"];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    if let Some(rec_date) = forecast.recommended_date {
        if rec_date > season_end {
            forecast.status = SeriesStatus::NotRecommended;
            forecast.reasons = crate::reasons!["VAC_GROUP_NO_LONGER_REC"];
            forecast.earliest_date = None;
            forecast.recommended_date = None;
            forecast.overdue_date = None;
            forecast.latest_date = None;
        }
    }
}

pub fn h1n1_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let age_10y = crate::date_utils::add_years(patient.birth_date, 10);
    
    let last_h1n1_dose = history.iter()
        .filter(|d| matches!(d.cvx.0, cvx!("125") | cvx!("126") | cvx!("127") | cvx!("128")))
        .max_by_key(|d| d.date);

    let select_2_dose = match last_h1n1_dose {
        None => {
            eval_date < age_10y
        }
        Some(last_dose) => {
            let has_ge_2_valid = candidate_forecasts.get_forecast("H1N1_2_DOSE_SERIES")
                .map(|f| f.evaluations.iter().filter(|e| e.status == DoseStatus::Valid).count() >= 2)
                .unwrap_or(false);
            
            if has_ge_2_valid {
                true
            } else {
                last_dose.date < age_10y
            }
        }
    };

    let selected = if select_2_dose {
        "H1N1_2_DOSE_SERIES"
    } else {
        "H1N1_1_DOSE_SERIES"
    };

    let season_end = NaiveDate::from_ymd_opt(2010, 6, 30).unwrap();
    let has_zero_h1n1_doses = history.iter()
        .filter(|d| matches!(d.cvx.0, cvx!("125") | cvx!("126") | cvx!("127") | cvx!("128")))
        .count() == 0;

    if eval_date > season_end && has_zero_h1n1_doses {
        if let Some(f) = candidate_forecasts.get_forecast_mut(&selected) {
            f.forecasts.clear();
        }
    }

    selected
}
