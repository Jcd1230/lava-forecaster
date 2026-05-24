use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast};

pub fn rotavirus_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    let Some(dose) = ctx.current_dose else {
        return;
    };

    // Strict age clamp: Any dose given at age > 8 months is evaluated as Accepted/AboveRecommendedAgeSeries
    let tp_8m = crate::date_utils::TimePeriod::parse("8m").unwrap();
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

    if forecast.status == SeriesStatus::Complete {
        forecast.reasons = vec!["COMPLETE".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    // Strict age clamp: if patient is currently > 8 months, or will be > 8 months as of the recommended date due
    let tp_8m = crate::date_utils::TimePeriod::parse("8m").unwrap();
    let date_8m = tp_8m.add_to(birth);

    let is_currently_gt_8m = eval_date > date_8m;
    let is_rec_gt_8m = forecast.recommended_date.map_or(false, |d| d > date_8m);

    if is_currently_gt_8m || is_rec_gt_8m {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = vec!["TOO_OLD".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    // Initiation check: If patient is >= 105 days and has 0 valid doses
    let tp_105d = crate::date_utils::TimePeriod::parse("105d").unwrap();
    let date_105d = tp_105d.add_to(birth);
    if eval_date >= date_105d && valid_doses.is_empty() {
        forecast.status = SeriesStatus::NotRecommended;
        forecast.reasons = vec!["TOO_OLD_TO_INITIATE".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
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

        if d.cvx.0 == 122 {
            prior_122_count += 1;
        }
    }

    ctx.target_dose_number + prior_122_count
}

pub fn rotavirus_group_selection(
    _patient: &Patient,
    _history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut std::collections::HashMap<String, VaccineGroupForecast>,
) -> String {
    let series_2_dose = "ROTAVIRUS_2_DOSE_SERIES".to_string();
    let series_3_dose = "ROTAVIRUS_3_DOSE_SERIES".to_string();

    let has_2_dose = candidate_forecasts.contains_key(&series_2_dose);
    let has_3_dose = candidate_forecasts.contains_key(&series_3_dose);

    if has_2_dose && has_3_dose {
        // 1. Check if any valid dose of CVX 116, 74, or 122 was administered in either candidate's evaluations.
        let fc_3 = candidate_forecasts.get(&series_3_dose).unwrap();
        let has_valid_3dose_cvx = fc_3.evaluations.iter().any(|e| {
            e.status == DoseStatus::Valid && (e.cvx.0 == 116 || e.cvx.0 == 74 || e.cvx.0 == 122)
        });
        if has_valid_3dose_cvx {
            return series_3_dose;
        }

        // 2. Check if dose 1 is CVX 119 and is valid in 2-dose series candidate evaluations.
        let fc_2 = candidate_forecasts.get(&series_2_dose).unwrap();
        let dose_1_is_valid_rv1 = fc_2.evaluations.iter().any(|e| {
            e.status == DoseStatus::Valid && e.dose_number == Some(1) && e.cvx.0 == 119
        });
        if dose_1_is_valid_rv1 {
            return series_2_dose;
        }
    }

    series_3_dose
}
