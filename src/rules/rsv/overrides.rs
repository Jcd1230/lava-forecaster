use lava_cvx_macro::cvx;
use chrono::{Datelike, NaiveDate};
use crate::date_utils::{SmallVec, add_months_unchecked, add_years_unchecked, compare_elapsed, TimePeriod};
use crate::engine::EvaluationContext;
use crate::models::{Cvx, 
    Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus,
    VaccineGroupForecast,
};

const RSV_SUPPORT_START: (i32, u32, u32) = (2023, 6, 21);
const RSV_INITIAL_INFANT_SEASON_START: (i32, u32, u32) = (2023, 10, 1);

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn age_ge(birth_date: NaiveDate, date_to_check: NaiveDate, age: TimePeriod) -> bool {
    compare_elapsed(birth_date, date_to_check, &age)
        != std::cmp::Ordering::Less
}

fn age_lt(birth_date: NaiveDate, date_to_check: NaiveDate, age: TimePeriod) -> bool {
    compare_elapsed(birth_date, date_to_check, &age)
        == std::cmp::Ordering::Less
}

fn infant_recommendation_date(birth_date: NaiveDate, eval_date: NaiveDate) -> NaiveDate {
    let season_start = if eval_date.month() >= 10 {
        date(eval_date.year(), 10, 1)
    } else if eval_date.month() <= 3 {
        date(eval_date.year() - 1, 10, 1)
    } else {
        date(eval_date.year(), 10, 1)
    };

    season_start.max(birth_date).max(date(
        RSV_INITIAL_INFANT_SEASON_START.0,
        RSV_INITIAL_INFANT_SEASON_START.1,
        RSV_INITIAL_INFANT_SEASON_START.2,
    ))
}

fn availability_cutoff(series_name: &str, cvx: Cvx) -> Option<NaiveDate> {
    match series_name {
        "RSV_ADULT_SERIES" => match cvx.0 {
            cvx!("303") | cvx!("304") | cvx!("305") | cvx!("314") | cvx!("326") => Some(date(2023, 6, 21)),
            _ => None,
        },
        "RSV_INFANT_SERIES" => match cvx.0 {
            cvx!("332") => Some(date(2025, 6, 9)),
            cvx!("304") | cvx!("306") | cvx!("307") | cvx!("315") => Some(date(2023, 8, 3)),
            _ => None,
        },
        _ => None,
    }
}

pub fn rsv_custom_evaluation_hook(
    series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    let Some(dose) = ctx.current_dose else {
        return;
    };

    if let Some(cutoff) = availability_cutoff(series_name, dose.cvx) {
        if dose.date < cutoff {
            *status = DoseStatus::Invalid;
            reasons.clear();
            return;
        }
    }

    if age_ge(ctx.patient.birth_date, dose.date, crate::time_period!("8m")) && age_lt(ctx.patient.birth_date, dose.date, crate::time_period!("50y")) {
        *status = DoseStatus::Accepted;
        reasons.clear();
        reasons.push(EvaluationReason::OutsideRoutineSeries);
    }
}

pub fn rsv_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
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

    let support_start = date(RSV_SUPPORT_START.0, RSV_SUPPORT_START.1, RSV_SUPPORT_START.2);
    if eval_date < support_start {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = crate::reasons!["NOT_SUPPORTED"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if forecast.series_name == "RSV_INFANT_SERIES" {
        if age_ge(patient.birth_date, eval_date, crate::time_period!("8m")) && age_lt(patient.birth_date, eval_date, crate::time_period!("20m")) {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = crate::reasons!["HIGH_RISK"];
            forecast.status = forecast.status.with_earliest_date(None);
            forecast.status = forecast.status.with_recommended_date(None);
            forecast.status = forecast.status.with_overdue_date(None);
            forecast.status = forecast.status.with_latest_date(None);
            return;
        }

        let mut recommendation_date = infant_recommendation_date(patient.birth_date, eval_date);
        if valid_doses.is_empty()
            && !history.is_empty()
            && recommendation_date < eval_date
        {
            recommendation_date = eval_date;
        }
        if age_ge(patient.birth_date, recommendation_date, crate::time_period!("8m"))
            && age_lt(patient.birth_date, recommendation_date, crate::time_period!("20m"))
        {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = crate::reasons!["HIGH_RISK"];
            forecast.status = forecast.status.with_earliest_date(None);
            forecast.status = forecast.status.with_recommended_date(None);
            forecast.status = forecast.status.with_overdue_date(None);
            forecast.status = forecast.status.with_latest_date(None);
            return;
        }

        forecast.status = SeriesStatus::default();
        forecast.reasons = crate::reasons!["NOT_COMPLETE"];
        forecast.status = forecast.status.with_earliest_date(Some(recommendation_date));
        forecast.status = forecast.status.with_recommended_date(Some(recommendation_date));
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if age_ge(patient.birth_date, eval_date, crate::time_period!("50y")) && age_lt(patient.birth_date, eval_date, crate::time_period!("75y")) {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons!["HIGH_RISK"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    let age_75 = add_years_unchecked(patient.birth_date, 75);
    let adult_recommendation_date = age_75.max(date(2024, 6, 26));
    forecast.status = SeriesStatus::default();
    forecast.reasons = crate::reasons!["NOT_COMPLETE"];
    forecast.status = forecast.status.with_earliest_date(Some(adult_recommendation_date));
    forecast.status = forecast.status.with_recommended_date(Some(adult_recommendation_date));
    forecast.status = forecast.status.with_overdue_date(None);
    forecast.status = forecast.status.with_latest_date(None);
}

pub fn rsv_group_selection(
    patient: &Patient,
    _history: &[Dose],
    eval_date: NaiveDate,
    _candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let adult_start = add_months_unchecked(patient.birth_date, 20);
    if eval_date >= adult_start {
        "RSV_ADULT_SERIES"
    } else {
        "RSV_INFANT_SERIES"
    }
}
