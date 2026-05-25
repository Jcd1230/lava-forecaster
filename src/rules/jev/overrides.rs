use ice_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast};
use crate::date_utils::{add_years, TimePeriod};

pub fn jev_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        if target_dose_idx == 2 {
            let age_18 = add_years(ctx.patient.birth_date, 18);
            let age_66 = add_years(ctx.patient.birth_date, 66);

            if series_name == "JEVC_RISK_2_DOSE_SERIES" {
                if dose.date >= age_18 && dose.date < age_66 {
                    if let Some((prev_date, _)) = ctx.valid_doses.last() {
                        let days = (dose.date - *prev_date).num_days();
                        if days >= 7 {
                            *status = DoseStatus::Valid;
                            reasons.retain(|r| *r != EvaluationReason::BelowMinimumInterval);
                        }
                    }
                }
            } else if series_name == "JEVC_RISK_2_DOSE_ACCELERATED_SERIES" {
                if dose.date >= age_66 {
                    if let Some((prev_date, _)) = ctx.valid_doses.last() {
                        let days = (dose.date - *prev_date).num_days();
                        if days < 24 {
                            *status = DoseStatus::Invalid;
                            if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                                reasons.push(EvaluationReason::BelowMinimumInterval);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn jev_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = vec![
            "COMPLETE_HIGH_RISK".into(),
            "JE_NOT_ROUTINE_ACCEL_18_65_SEE_ACIP".into(),
        ];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    if valid_doses.is_empty() {
        let age_2m = crate::time_period!("2m").add_to(patient.birth_date);
        if eval_date < age_2m {
            forecast.status = SeriesStatus::NotRecommended;
            forecast.reasons = vec!["JE_NOT_ROUTINE_ACCEL_18_65_SEE_ACIP".into()];
        } else {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = vec![
                "HIGH_RISK".into(),
                "JE_NOT_ROUTINE_ACCEL_18_65_SEE_ACIP".into(),
            ];
        }
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    if valid_doses.len() == 1 {
        if !forecast.reasons.contains(&"JE_NOT_ROUTINE_ACCEL_18_65_SEE_ACIP".into()) {
            forecast.reasons.push("JE_NOT_ROUTINE_ACCEL_18_65_SEE_ACIP".into());
        }

        if forecast.series_name == "JEVC_RISK_2_DOSE_ACCELERATED_SERIES" {
            let prev_dose_date = valid_doses[0].0;
            let age_66_minus_7d = crate::time_period!("66y-7d").add_to(patient.birth_date);
            
            if prev_dose_date >= age_66_minus_7d {
                forecast.overdue_date = None;
            }

            let age_66 = add_years(patient.birth_date, 66);
            if forecast.recommended_date.map(|d| d >= age_66).unwrap_or(false) {
                let new_date = prev_dose_date + chrono::Duration::days(28);
                forecast.earliest_date = Some(new_date);
                forecast.recommended_date = Some(new_date);
                forecast.overdue_date = None;
            }
        }
    }
}

pub fn jev_group_selection(
    patient: &Patient,
    _history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut std::collections::HashMap<String, VaccineGroupForecast>,
) -> String {
    let first_valid_dose_date = candidate_forecasts.get("JEVC_RISK_2_DOSE_SERIES")
        .and_then(|f| {
            f.evaluations.iter()
                .find(|e| e.status == DoseStatus::Valid && e.dose_number == Some(1))
                .map(|e| e.dose_date)
        });

    let age_at_reference = first_valid_dose_date.unwrap_or(eval_date);
    
    let tp_18y_minus_4d = crate::time_period!("18y-4d");
    let age_18_minus_4d = tp_18y_minus_4d.add_to(patient.birth_date);
    let age_66 = add_years(patient.birth_date, 66);

    let select_accelerated = age_at_reference >= age_18_minus_4d && age_at_reference < age_66;

    if select_accelerated {
        "JEVC_RISK_2_DOSE_ACCELERATED_SERIES".into()
    } else {
        "JEVC_RISK_2_DOSE_SERIES".into()
    }
}
