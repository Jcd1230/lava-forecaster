use crate::date_utils::SmallVec;
use crate::engine::CandidateForecastsExt;
use lava_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast};

pub fn rotavirus_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    let Some(dose) = ctx.current_dose else {
        return;
    };

    // Strict age clamp: Any dose given at age > 8 months is evaluated as Accepted/AboveRecommendedAgeSeries
    let tp_8m = crate::time_period!("8m");
    let is_age_gt_8m = crate::date_utils::compare_elapsed(ctx.patient.birth_date, dose.date, &tp_8m) == std::cmp::Ordering::Greater;

    if is_age_gt_8m {
        *status = DoseStatus::Accepted;
        reasons.clear();
        reasons.push(EvaluationReason::AboveRecommendedAgeSeries);
    }
}

pub fn rotavirus_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let birth = patient.birth_date;
    let tp_8m = crate::time_period!("8m");
    let date_8m = tp_8m.add_to(birth);

    if forecast.status == SeriesStatus::Complete {
        let num_required = if forecast.series_name == "ROTAVIRUS_2_DOSE_SERIES" { 2 } else { 3 };
        let actually_complete = valid_doses.iter().any(|(_, num)| *num == num_required);

        if !actually_complete && eval_date > date_8m {
            forecast.status = SeriesStatus::NotRecommended;
            forecast.reasons = crate::reasons!["TOO_OLD"];
            forecast.status = forecast.status.with_earliest_date(None);
            forecast.status = forecast.status.with_recommended_date(None);
            forecast.status = forecast.status.with_overdue_date(None);
            forecast.status = forecast.status.with_latest_date(None);
            return;
        }

        forecast.reasons = crate::reasons!["COMPLETE"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    // Strict age clamp: if patient is currently > 8 months, or will be > 8 months as of the recommended date due
    let tp_8m = crate::time_period!("8m");
    let date_8m = tp_8m.add_to(birth);

    let is_currently_gt_8m = eval_date > date_8m;
    let is_rec_gt_8m = forecast.status.recommended_date().map_or(false, |d| d > date_8m);

    if is_currently_gt_8m || is_rec_gt_8m {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = crate::reasons!["TOO_OLD"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    // Initiation check: If patient is >= 105 days and has 0 valid doses
    let tp_105d = crate::time_period!("105d");
    let date_105d = tp_105d.add_to(birth);
    if eval_date >= date_105d && valid_doses.is_empty() {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = crate::reasons!["TOO_OLD_TO_INITIATE"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }
}

pub fn rotavirus_custom_dose_number_hook(
    series_name: &str,
    ctx: &EvaluationContext,
) -> usize {
    if series_name != "ROTAVIRUS_2_DOSE_SERIES" {
        return ctx.target_dose_number;
    }

    let birth = ctx.patient.birth_date;
    let current_date = ctx.current_dose.map(|d| d.date).unwrap_or(ctx.eval_date);

    // Count how many prior CVX 122 doses exist in history (before the current dose date)
    // that are not prior to DOB and not same-day duplicates.
    let mut prior_122_count = 0;
    let mut seen_dates = std::collections::HashSet::new();

    for d in ctx.history {
        if d.date >= current_date {
            break;
        }
        if d.date < birth {
            continue;
        }
        // Same-day duplicate check: only count the first dose of a given day
        if seen_dates.contains(&d.date) {
            continue;
        }
        seen_dates.insert(d.date);

        if d.cvx.0 == cvx!("122") {
            prior_122_count += 1;
        }
    }

    ctx.target_dose_number + prior_122_count
}

pub fn rotavirus_group_selection(
    _patient: &Patient,
    _history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let series_2_dose = "ROTAVIRUS_2_DOSE_SERIES";
    let series_3_dose = "ROTAVIRUS_3_DOSE_SERIES";

    let has_2_dose = candidate_forecasts.contains_forecast(&series_2_dose);
    let has_3_dose = candidate_forecasts.contains_forecast(&series_3_dose);

    if has_2_dose && has_3_dose {
        // 1. Check if any valid dose of CVX 116, 74, or 122 was administered in either candidate's evaluations.
        let fc_3 = candidate_forecasts.get_forecast(&series_3_dose).unwrap();
        let has_valid_3dose_cvx = fc_3.evaluations.iter().any(|e| {
            e.status == DoseStatus::Valid && (e.cvx.0 == cvx!("116") || e.cvx.0 == cvx!("74") || e.cvx.0 == cvx!("122"))
        });
        if has_valid_3dose_cvx {
            return series_3_dose;
        }

        // 2. Check if dose 1 is CVX 119 and is valid in 2-dose series candidate evaluations.
        let fc_2 = candidate_forecasts.get_forecast(&series_2_dose).unwrap();
        let dose_1_is_valid_rv1 = fc_2.evaluations.iter().any(|e| {
            e.status == DoseStatus::Valid && e.dose_number == Some(1) && e.cvx.0 == cvx!("119")
        });
        if dose_1_is_valid_rv1 {
            return series_2_dose;
        }
    }

    series_3_dose
}
