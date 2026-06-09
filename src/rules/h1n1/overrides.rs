use crate::date_utils::SmallVec;
use crate::engine::CandidateForecastsExt;
use crate::engine::EvaluationContext;
use crate::models::{
    Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus, VaccineGroupForecast,
};
use chrono::NaiveDate;
use lava_cvx_macro::cvx;

pub fn h1n1_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
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
    let season_start = NaiveDate::from_ymd_opt(2009, 10, 1).unwrap();
    let season_end = NaiveDate::from_ymd_opt(2010, 6, 30).unwrap();

    if forecast.status == SeriesStatus::Complete {
        forecast.reasons = crate::reasons!["COMPLETE"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if let SeriesStatus::NotComplete {
        earliest_date,
        recommended_date,
        ..
    } = &mut forecast.status
    {
        if let Some(date) = earliest_date {
            if *date < season_start {
                *earliest_date = Some(season_start);
            }
        }
        if let Some(date) = recommended_date {
            if *date < season_start {
                *recommended_date = Some(season_start);
            }
        }

        // Anchor on any previous H1N1 dose (even if invalid) for the 28-day interval
        let last_h1n1_shot = _history
            .iter()
            .filter(|d| {
                matches!(
                    d.cvx.0,
                    cvx!("125") | cvx!("126") | cvx!("127") | cvx!("128")
                )
            })
            .map(|d| d.date)
            .max();

        if let Some(last_shot) = last_h1n1_shot {
            let interval_28d = crate::time_period!("28d").add_to(last_shot);
            if let Some(date) = earliest_date {
                if *date < interval_28d {
                    *earliest_date = Some(interval_28d);
                }
            }
            if let Some(date) = recommended_date {
                if *date < interval_28d {
                    *recommended_date = Some(interval_28d);
                }
            }
        }
    }

    if eval_date > season_end {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = crate::reasons!["VAC_GROUP_NO_LONGER_REC"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if let Some(earliest_date) = forecast.status.earliest_date() {
        if earliest_date > season_end {
            forecast.status = SeriesStatus::NotRecommended;
            forecast.reasons = crate::reasons!["VAC_GROUP_NO_LONGER_REC"];
            forecast.status = forecast.status.with_earliest_date(None);
            forecast.status = forecast.status.with_recommended_date(None);
            forecast.status = forecast.status.with_overdue_date(None);
            forecast.status = forecast.status.with_latest_date(None);
        }
    }
}

pub fn h1n1_custom_extra_dose_hook(
    _series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, SmallVec<[EvaluationReason; 4]>)> {
    if let Some(dose) = ctx.current_dose {
        let start = NaiveDate::from_ymd_opt(2009, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2010, 6, 30).unwrap();

        if dose.date < start || dose.date > end {
            return Some((
                DoseStatus::Invalid,
                crate::reasons![EvaluationReason::OutsideFluVacSeason],
            ));
        }
    }
    None
}

pub fn h1n1_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let age_10y = crate::date_utils::add_years_unchecked(patient.birth_date, 10);

    let last_h1n1_dose = history
        .iter()
        .filter(|d| {
            matches!(
                d.cvx.0,
                cvx!("125") | cvx!("126") | cvx!("127") | cvx!("128")
            )
        })
        .max_by_key(|d| d.date);

    let select_2_dose = match last_h1n1_dose {
        None => eval_date < age_10y,
        Some(last_dose) => {
            let has_ge_2_valid = candidate_forecasts
                .get_forecast("H1N1_2_DOSE_SERIES")
                .map(|f| {
                    f.evaluations
                        .iter()
                        .filter(|e| e.status == DoseStatus::Valid)
                        .count()
                        >= 2
                })
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

    let season_start = NaiveDate::from_ymd_opt(2009, 10, 1).unwrap();
    let season_end = NaiveDate::from_ymd_opt(2010, 6, 30).unwrap();
    let has_zero_h1n1_doses = history
        .iter()
        .filter(|d| {
            matches!(
                d.cvx.0,
                cvx!("125") | cvx!("126") | cvx!("127") | cvx!("128")
            )
        })
        .count()
        == 0;

    if (eval_date < season_start || eval_date > season_end) && has_zero_h1n1_doses {
        if let Some(f) = candidate_forecasts.get_forecast_mut(&selected) {
            f.forecasts.clear();
        }
    }

    selected
}
