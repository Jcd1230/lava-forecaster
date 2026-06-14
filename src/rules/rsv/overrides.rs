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

    // Absolute maximum age checks for RSV mAb:
    // CVX 306, 307, 315: absolute maximum age is 24m (2 years)
    // CVX 332: absolute maximum age is 8m
    let is_rsv_mab_invalid = match dose.cvx.0 {
        cvx!("306") | cvx!("307") | cvx!("315") => {
            age_ge(ctx.patient.birth_date, dose.date, crate::time_period!("24m"))
        }
        cvx!("332") => {
            age_ge(ctx.patient.birth_date, dose.date, crate::time_period!("8m"))
        }
        _ => false,
    };
    if is_rsv_mab_invalid {
        *status = DoseStatus::Invalid;
        reasons.clear();
        return;
    }

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

fn get_rsv_same_day_priority(cvx: Cvx) -> i32 {
    match cvx.0 {
        304 | 314 | 315 => 1,
        _ => 0,
    }
}

fn is_dose_valid_or_accepted_for_series(
    series_name: &str,
    patient: &Patient,
    history: &[Dose],
    dose_idx: usize,
) -> bool {
    let limit = add_months_unchecked(patient.birth_date, 20);
    let dose = &history[dose_idx];

    // Check if the dose matches the series boundary:
    if series_name == "RSV_INFANT_SERIES" && dose.date >= limit {
        return false;
    }
    if series_name == "RSV_ADULT_SERIES" && dose.date < limit {
        return false;
    }
    if series_name == "RSV_ADULT_SERIES" && !age_ge(patient.birth_date, dose.date, crate::time_period!("50y")) {
        return false;
    }

    // 1. Same-day priority check:
    let same_day_doses: Vec<(usize, &Dose)> = history.iter().enumerate()
        .filter(|(_, d)| d.date == dose.date)
        .collect();
    if same_day_doses.len() > 1 {
        let mut best_idx = same_day_doses[0].0;
        let mut best_prio = get_rsv_same_day_priority(same_day_doses[0].1.cvx);
        for &(idx, d) in same_day_doses.iter().skip(1) {
            let prio = get_rsv_same_day_priority(d.cvx);
            if prio < best_prio {
                best_prio = prio;
                best_idx = idx;
            }
        }
        if dose_idx != best_idx {
            return false;
        }
    }

    // 2. Absolute maximum age checks:
    let is_rsv_mab_invalid = match dose.cvx.0 {
        cvx!("306") | cvx!("307") | cvx!("315") => {
            age_ge(patient.birth_date, dose.date, crate::time_period!("24m"))
        }
        cvx!("332") => {
            age_ge(patient.birth_date, dose.date, crate::time_period!("8m"))
        }
        _ => false,
    };
    if is_rsv_mab_invalid {
        return false;
    }

    // 3. Availability cutoff check:
    if let Some(cutoff) = availability_cutoff(series_name, dose.cvx) {
        if dose.date < cutoff {
            return false;
        }
    }

    // 4. Allowed vaccine check:
    let is_allowed = match series_name {
        "RSV_INFANT_SERIES" => matches!(dose.cvx.0, cvx!("304") | cvx!("306") | cvx!("307") | cvx!("315") | cvx!("332")),
        "RSV_ADULT_SERIES" => matches!(dose.cvx.0, cvx!("303") | cvx!("304") | cvx!("305") | cvx!("314") | cvx!("326")),
        _ => false,
    };
    if !is_allowed {
        return false;
    }

    true
}

pub fn rsv_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _evaluations: &[crate::models::DoseEvaluation],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
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

    let is_complete = forecast.status == SeriesStatus::Complete || (0..history.len()).any(|idx| {
        is_dose_valid_or_accepted_for_series(&forecast.series_name, patient, history, idx)
    });

    if is_complete {
        if forecast.series_name == "RSV_INFANT_SERIES" {
            if age_lt(patient.birth_date, eval_date, crate::time_period!("20m")) {
                forecast.status = SeriesStatus::Complete;
                forecast.reasons = crate::reasons!["COMPLETE_HIGH_RISK"];
            } else {
                forecast.status = SeriesStatus::Complete;
                forecast.reasons = crate::reasons!["COMPLETE"];
            }
        } else {
            forecast.status = SeriesStatus::Complete;
            forecast.reasons = crate::reasons!["COMPLETE"];
        }
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
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let limit = add_months_unchecked(patient.birth_date, 20);

    // Find the infant and adult forecasts in candidate_forecasts
    let mut infant_idx = None;
    let mut adult_idx = None;
    for (idx, (name, _)) in candidate_forecasts.iter().enumerate() {
        if *name == "RSV_INFANT_SERIES" {
            infant_idx = Some(idx);
        } else if *name == "RSV_ADULT_SERIES" {
            adult_idx = Some(idx);
        }
    }

    // If we have both, merge their evaluations based on the patient's age at the time of each dose.
    if let (Some(inf_idx), Some(ad_idx)) = (infant_idx, adult_idx) {
        let inf_evals = candidate_forecasts[inf_idx].1.evaluations.clone();
        let ad_evals = candidate_forecasts[ad_idx].1.evaluations.clone();

        let mut merged_evals = crate::date_utils::SmallVec::new();
        for i in 0..inf_evals.len() {
            let inf_eval = &inf_evals[i];
            if inf_eval.dose_date < limit {
                merged_evals.push(inf_eval.clone());
            } else if i < ad_evals.len() {
                merged_evals.push(ad_evals[i].clone());
            }
        }

        candidate_forecasts[inf_idx].1.evaluations = merged_evals.clone();
        candidate_forecasts[ad_idx].1.evaluations = merged_evals;
    }

    if eval_date < limit {
        "RSV_INFANT_SERIES"
    } else {
        "RSV_ADULT_SERIES"
    }
}

pub fn rsv_custom_extra_dose_hook(
    series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, SmallVec<[EvaluationReason; 4]>)> {
    let mut status = DoseStatus::Accepted;
    let mut reasons = crate::reasons![EvaluationReason::BoosterDose];
    rsv_custom_evaluation_hook(series_name, 0, ctx, &mut reasons, &mut status);
    Some((status, reasons))
}
