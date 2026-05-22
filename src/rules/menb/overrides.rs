use crate::date_utils::add_years;
use crate::engine::EvaluationContext;
use crate::models::{
    Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus, VaccineGroupForecast,
};
use chrono::NaiveDate;
use std::collections::HashMap;

const DUPLICATE_POLICY_CHANGE_DATE: (i32, u32, u32) = (2024, 10, 25);

fn is_4c(cvx: &str) -> bool {
    matches!(cvx, "163" | "328")
}

fn is_fhbp(cvx: &str) -> bool {
    matches!(cvx, "162" | "316")
}

fn has_mixed_brand_same_day(dose: &Dose, history: &[Dose]) -> bool {
    history.iter().any(|other| {
        other.date == dose.date
            && ((is_4c(&dose.cvx) && is_fhbp(&other.cvx))
                || (is_fhbp(&dose.cvx) && is_4c(&other.cvx)))
    })
}

pub fn menb_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    let Some(dose) = ctx.current_dose else {
        return;
    };

    let age_10 = add_years(ctx.patient.birth_date, 10);
    if matches!(dose.cvx.as_str(), "162" | "163")
        && dose.date >= age_10
        && reasons.contains(&EvaluationReason::BelowMinimumAge)
    {
        *status = DoseStatus::Accepted;
        reasons.retain(|r| *r != EvaluationReason::BelowMinimumAge);
    }

    let policy_change = NaiveDate::from_ymd_opt(
        DUPLICATE_POLICY_CHANGE_DATE.0,
        DUPLICATE_POLICY_CHANGE_DATE.1,
        DUPLICATE_POLICY_CHANGE_DATE.2,
    )
    .unwrap();
    if dose.date >= policy_change && has_mixed_brand_same_day(dose, ctx.history) {
        *status = DoseStatus::Invalid;
        reasons.clear();
        reasons.push(EvaluationReason::DuplicateShotSameDay);
    }
}

pub fn menb_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.reasons = vec!["COMPLETE".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    if eval_date >= add_years(patient.birth_date, 10) && !valid_doses.is_empty() {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = vec!["CLINICAL_PATIENT_DISCRETION".to_string()];
    }
}

pub fn menb_custom_switch_hook(
    current_series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
) -> Option<&'static str> {
    let dose = ctx.current_dose?;
    if is_4c(&dose.cvx) && current_series_name.starts_with("MEN_BF_HBP") {
        return Some("MEN_B_4_C_2_DOSE_SERIES");
    }
    if is_fhbp(&dose.cvx) && current_series_name.starts_with("MEN_B_4_C") {
        return Some("MEN_BF_HBP_2_DOSE_SERIES");
    }
    None
}

pub fn menb_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut HashMap<String, VaccineGroupForecast>,
) -> String {
    let policy_change = NaiveDate::from_ymd_opt(
        DUPLICATE_POLICY_CHANGE_DATE.0,
        DUPLICATE_POLICY_CHANGE_DATE.1,
        DUPLICATE_POLICY_CHANGE_DATE.2,
    )
    .unwrap();
    if history
        .iter()
        .any(|dose| dose.date < policy_change && has_mixed_brand_same_day(dose, history))
    {
        return "MEN_B_4_C_2_DOSE_SERIES".to_string();
    }

    for name in [
        "MEN_B_4_C_2_DOSE_SERIES",
        "MEN_BF_HBP_2_DOSE_SERIES",
        "MEN_B_4_C_3_DOSE_SERIES",
        "MEN_BF_HBP_3_DOSE_SERIES",
    ] {
        if candidate_forecasts.get(name).is_some_and(|f| {
            f.forecasts
                .iter()
                .any(|fc| fc.status == SeriesStatus::Complete)
        }) {
            return name.to_string();
        }
    }

    let first_menb = history
        .iter()
        .filter(|d| is_4c(&d.cvx) || is_fhbp(&d.cvx))
        .min_by_key(|d| d.date);

    if let Some(dose) = first_menb {
        let age_16 = add_years(patient.birth_date, 16);
        if is_4c(&dose.cvx) {
            return if dose.date >= age_16 {
                "MEN_B_4_C_2_DOSE_SERIES".to_string()
            } else {
                "MEN_B_4_C_3_DOSE_SERIES".to_string()
            };
        }
        if is_fhbp(&dose.cvx) {
            return if dose.date >= age_16 {
                "MEN_BF_HBP_2_DOSE_SERIES".to_string()
            } else {
                "MEN_BF_HBP_3_DOSE_SERIES".to_string()
            };
        }
    }

    if eval_date >= add_years(patient.birth_date, 16) {
        "MEN_B_4_C_2_DOSE_SERIES".to_string()
    } else {
        "MEN_B_4_C_3_DOSE_SERIES".to_string()
    }
}
