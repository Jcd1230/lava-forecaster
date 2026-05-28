use crate::engine::CandidateForecastsExt;
use ice_cvx_macro::cvx;
use crate::date_utils::{TinyVec, add_years_unchecked, TimePeriod};
use crate::engine::EvaluationContext;
use crate::models::{Cvx, 
    Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus, VaccineGroupForecast,
};
use chrono::NaiveDate;
use std::collections::HashMap;

const DUPLICATE_POLICY_CHANGE_DATE: (i32, u32, u32) = (2024, 10, 25);

fn policy_change_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(
        DUPLICATE_POLICY_CHANGE_DATE.0,
        DUPLICATE_POLICY_CHANGE_DATE.1,
        DUPLICATE_POLICY_CHANGE_DATE.2,
    )
    .unwrap()
}

fn is_4c(cvx: Cvx) -> bool {
    matches!(cvx.0, cvx!("163") | cvx!("328"))
}

fn is_fhbp(cvx: Cvx) -> bool {
    matches!(cvx.0, cvx!("162") | cvx!("316"))
}

fn is_4c_series(series_name: &str) -> bool {
    series_name.starts_with("MEN_B_4_C")
}

fn is_fhbp_series(series_name: &str) -> bool {
    series_name.starts_with("MEN_BF_HBP")
}

fn is_opposite_family(series_name: &str, cvx: Cvx) -> bool {
    (is_4c_series(series_name) && is_fhbp(cvx)) || (is_fhbp_series(series_name) && is_4c(cvx))
}

fn valid_count(forecast: &VaccineGroupForecast) -> usize {
    forecast
        .evaluations
        .iter()
        .filter(|evaluation| evaluation.status == DoseStatus::Valid)
        .count()
}

fn is_complete(forecast: &VaccineGroupForecast) -> bool {
    forecast
        .forecasts
        .first()
        .is_some_and(|series| series.status == SeriesStatus::Complete)
}

fn choose_series(
    two_dose_name: &'static str,
    three_dose_name: &'static str,
    candidate_forecasts: &[(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let Some(two_dose) = candidate_forecasts.get_forecast(two_dose_name) else {
        return three_dose_name;
    };
    let Some(three_dose) = candidate_forecasts.get_forecast(three_dose_name) else {
        return two_dose_name;
    };

    let two_complete = is_complete(two_dose);
    let three_complete = is_complete(three_dose);

    if two_complete != three_complete {
        return if three_complete {
            three_dose_name
        } else {
            two_dose_name
        };
    }

    let two_valid = valid_count(two_dose);
    let three_valid = valid_count(three_dose);

    if three_valid > two_valid {
        three_dose_name
    } else {
        two_dose_name
    }
}

fn latest_family(history: &[Dose]) -> Option<&'static str> {
    history
        .iter()
        .filter(|dose| is_4c(dose.cvx) || is_fhbp(dose.cvx))
        .max_by_key(|dose| dose.date)
        .map(|dose| if is_4c(dose.cvx) { "4C" } else { "FHBP" })
}

fn has_mixed_brand_same_day(dose: &Dose, history: &[Dose]) -> bool {
    history.iter().any(|other| {
        other.date == dose.date
            && ((is_4c(dose.cvx) && is_fhbp(other.cvx))
                || (is_fhbp(dose.cvx) && is_4c(other.cvx)))
    })
}

pub fn menb_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
) {
    let Some(dose) = ctx.current_dose else {
        return;
    };

    let age_10 = add_years_unchecked(ctx.patient.birth_date, 10);
    if matches!(dose.cvx.0, cvx!("162") | cvx!("163"))
        && dose.date >= age_10
        && reasons.contains(&EvaluationReason::BelowMinimumAge)
    {
        *status = DoseStatus::Accepted;
        reasons.retain(|r| *r != EvaluationReason::BelowMinimumAge);
    }

    let policy_change = policy_change_date();
    if dose.date >= policy_change && has_mixed_brand_same_day(dose, ctx.history) {
        *status = DoseStatus::Invalid;
        reasons.clear();
        reasons.push(EvaluationReason::DuplicateShotSameDay);
        return;
    }

    if is_opposite_family(series_name, dose.cvx) {
        *status = DoseStatus::Accepted;
        reasons.clear();
        reasons.push(EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven);
        reasons.push(EvaluationReason::OutsideRoutineSeries);
        return;
    }

    if series_name == "MEN_B_4_C_3_DOSE_SERIES" && dose.date < policy_change {
        *status = DoseStatus::Invalid;
        reasons.clear();
        reasons.push(EvaluationReason::VaccineNotPartOfSeries);
        return;
    }

    if series_name == "MEN_B_4_C_2_DOSE_SERIES" && target_dose_idx == 2 && is_4c(dose.cvx) {
        if let Some((dose1_date, _)) = ctx.valid_doses.first() {
            let threshold = if dose.date >= policy_change {
                crate::time_period!("6m-4d").add_to(*dose1_date)
            } else {
                crate::time_period!("1m-4d").add_to(*dose1_date)
            };

            if dose.date < threshold {
                *status = DoseStatus::Invalid;
                if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                    reasons.push(EvaluationReason::BelowMinimumInterval);
                }
            } else {
                *status = DoseStatus::Valid;
                reasons.retain(|reason| *reason != EvaluationReason::BelowMinimumInterval);
            }
        }
        return;
    }

    if matches!(series_name, "MEN_B_4_C_3_DOSE_SERIES" | "MEN_BF_HBP_3_DOSE_SERIES")
        && target_dose_idx == 3
    {
        if let Some((dose1_date, _)) = ctx.valid_doses.first() {
            let threshold = crate::time_period!("6m-4d").add_to(*dose1_date);
            if dose.date >= threshold {
                *status = DoseStatus::Valid;
                reasons.retain(|reason| *reason != EvaluationReason::BelowMinimumInterval);
            }
        }
    }
}

pub fn menb_custom_forecast_hook(
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

    if valid_doses.is_empty() {
        let age_10 = add_years_unchecked(patient.birth_date, 10);
        if eval_date < age_10 {
            forecast.status = SeriesStatus::NotRecommended;
            forecast.reasons = crate::reasons!["BELOW_MINIMUM_AGE_HIGH_RISK_SERIES"];
        } else if eval_date < add_years_unchecked(patient.birth_date, 16) {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = crate::reasons!["HIGH_RISK"];
        } else if eval_date < add_years_unchecked(patient.birth_date, 24) {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = crate::reasons!["CLINICAL_PATIENT_DISCRETION"];
        } else {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = crate::reasons!["HIGH_RISK"];
        }
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if forecast.series_name == "MEN_B_4_C_2_DOSE_SERIES" {
        if let Some((dose1_date, _)) = valid_doses.first() {
            if *dose1_date >= policy_change_date() {
                let anchor = crate::time_period!("6m").add_to(*dose1_date);
                forecast.status = forecast.status.with_earliest_date(Some(forecast.status.earliest_date().map_or(anchor, |date| date.max(anchor))));
                forecast.status = forecast.status.with_recommended_date(Some(forecast.status.recommended_date().map_or(anchor, |date| date.max(anchor))));
            } else {
                let last_4c_shot = history
                    .iter()
                    .filter(|dose| is_4c(dose.cvx))
                    .map(|dose| dose.date)
                    .max()
                    .unwrap_or(*dose1_date);
                let anchor = crate::time_period!("1m").add_to(last_4c_shot);
                forecast.status = forecast.status.with_earliest_date(Some(anchor));
                forecast.status = forecast.status.with_recommended_date(Some(anchor));
            }
        }
    }

    if matches!(forecast.series_name.as_ref(), "MEN_B_4_C_3_DOSE_SERIES" | "MEN_BF_HBP_3_DOSE_SERIES") {
        if let Some((dose1_date, _)) = valid_doses.first() {
            let anchor = crate::time_period!("6m").add_to(*dose1_date);
            forecast.status = forecast.status.with_earliest_date(Some(forecast.status.earliest_date().map_or(anchor, |date| date.max(anchor))));
            forecast.status = forecast.status.with_recommended_date(Some(forecast.status.recommended_date().map_or(anchor, |date| date.max(anchor))));
        }
    }

    forecast.status = forecast.status.with_overdue_date(None);
}

pub fn menb_custom_switch_hook(
    current_series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
) -> Option<&'static str> {
    let dose = ctx.current_dose?;
    if is_4c(dose.cvx) && current_series_name.starts_with("MEN_BF_HBP") {
        return Some("MEN_B_4_C_2_DOSE_SERIES");
    }
    if is_fhbp(dose.cvx) && current_series_name.starts_with("MEN_B_4_C") {
        return Some("MEN_BF_HBP_2_DOSE_SERIES");
    }
    None
}

pub fn menb_group_selection(
    _patient: &Patient,
    history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let policy_change = policy_change_date();
    if history
        .iter()
        .any(|dose| dose.date < policy_change && has_mixed_brand_same_day(dose, history))
    {
        return "MEN_B_4_C_2_DOSE_SERIES";
    }

    match latest_family(history) {
        Some("FHBP") => choose_series(
            "MEN_BF_HBP_2_DOSE_SERIES",
            "MEN_BF_HBP_3_DOSE_SERIES",
            candidate_forecasts,
        ),
        Some("4C") => choose_series(
            "MEN_B_4_C_2_DOSE_SERIES",
            "MEN_B_4_C_3_DOSE_SERIES",
            candidate_forecasts,
        ),
        _ => "MEN_B_4_C_2_DOSE_SERIES",
    }
}
