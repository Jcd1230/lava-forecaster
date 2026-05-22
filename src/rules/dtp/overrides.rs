use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, VaccineGroupForecast};
use crate::date_utils::{add_years, add_months};
use std::collections::HashMap;

pub fn is_pertussis_vaccine(cvx: &str) -> bool {
    // DT/Td (tetanus/diphtheria only, no pertussis) CVX codes
    const DT_TD_CVX: &[&str] = &["09", "28", "113", "138", "139", "195", "196"];
    !DT_TD_CVX.contains(&cvx)
}

pub fn dtp_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    _ctx: &EvaluationContext,
    _reasons: &mut Vec<EvaluationReason>,
    _status: &mut DoseStatus,
) {
    // Optional/Placeholder
}

pub fn dtp_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    _eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if (forecast.series_name == "DTP_5_DOSE_SERIES" || forecast.series_name == "DTP_3_DOSE_SERIES")
        && forecast.status == crate::models::SeriesStatus::Complete
    {
        let age_10 = add_years(patient.birth_date, 10);
        let has_adolescent_tdap = history.iter().any(|dose| {
            dose.date >= age_10 && (dose.cvx == "115" || dose.cvx == "198")
        });
        if !has_adolescent_tdap {
            let is_booster_needed = if forecast.series_name == "DTP_3_DOSE_SERIES" {
                let age_7 = add_years(patient.birth_date, 7);
                history.iter().any(|dose| {
                    dose.date >= age_7 && dose.date < age_10 && is_pertussis_vaccine(&dose.cvx)
                })
            } else {
                true
            };
            
            if is_booster_needed {
                forecast.status = crate::models::SeriesStatus::NotComplete;
                forecast.reasons = vec!["NOT_COMPLETE".to_string()];
                
                let exception_occurred = if forecast.series_name == "DTP_5_DOSE_SERIES" {
                    let age_4y_minus_4d = add_years(patient.birth_date, 4) - chrono::Duration::days(4);
                    let has_pertussis_at_least_4y = history.iter().any(|dose| {
                        dose.date >= age_4y_minus_4d && is_pertussis_vaccine(&dose.cvx)
                    });
                    
                    let age_7y = add_years(patient.birth_date, 7);
                    let num_pertussis_under_7 = history.iter().filter(|dose| {
                        dose.date < age_7y && is_pertussis_vaccine(&dose.cvx)
                    }).count();
                    
                    !has_pertussis_at_least_4y || num_pertussis_under_7 < 4
                } else {
                    false
                };
                
                if exception_occurred {
                    let rec_date = add_years(patient.birth_date, 7);
                    forecast.earliest_date = Some(rec_date);
                    forecast.recommended_date = Some(rec_date);
                    forecast.overdue_date = Some(rec_date);
                } else {
                    let early_date = crate::date_utils::TimePeriod::parse("11y").unwrap().add_to(patient.birth_date);
                    forecast.earliest_date = Some(early_date);
                    forecast.recommended_date = Some(early_date);
                    
                    let overdue_date = crate::date_utils::TimePeriod::parse("13y+4w").unwrap().add_to(patient.birth_date);
                    forecast.overdue_date = overdue_date.pred_opt();
                }
            }
        }
    }
}

// Conditional Completion Rule 1: DTP 3-Dose Adult Series completion check
// If valid_doses.len() >= 3 and at least one of those valid doses is pertussis-containing
pub fn dtp_3_dose_completion_condition(ctx: &EvaluationContext) -> bool {
    if ctx.active_series_name != "DTP_3_DOSE_SERIES" {
        return false;
    }
    if ctx.valid_doses.len() >= 3 {
        ctx.valid_doses.iter().any(|(v_date, _)| {
            ctx.history.iter().any(|h_dose| h_dose.date == *v_date && is_pertussis_vaccine(&h_dose.cvx))
        })
    } else {
        false
    }
}

// Conditional Completion Rule 2: DTP 5-Dose Exception 1 (3-Dose Completion)
// Complete with 3 doses if patient age >= 7 years, first valid dose was given at >= 12 months, and at least one valid dose was given at >= 4 years.
pub fn dtp_5_dose_exception_1_condition(ctx: &EvaluationContext) -> bool {
    if ctx.active_series_name != "DTP_5_DOSE_SERIES" {
        return false;
    }
    if ctx.valid_doses.len() >= 3 {
        let birth_date = ctx.patient.birth_date;
        let is_at_least_7 = ctx.eval_date >= add_years(birth_date, 7);
        let first_valid_dose_at_least_12m = ctx.valid_doses[0].0 >= add_months(birth_date, 12);
        let any_valid_dose_at_least_4y = ctx.valid_doses.iter().any(|(v_date, _)| {
            *v_date >= add_years(birth_date, 4)
        });
        
        is_at_least_7 && first_valid_dose_at_least_12m && any_valid_dose_at_least_4y
    } else {
        false
    }
}

// Conditional Completion Rule 3: DTP 5-Dose Exception 2 (4-Dose Completion)
// Complete with 4 doses if 4th dose was administered at >= 4 years, and interval between valid dose 3 and 4 is >= 6 months minus 4 days.
pub fn dtp_5_dose_exception_2_condition(ctx: &EvaluationContext) -> bool {
    if ctx.active_series_name != "DTP_5_DOSE_SERIES" {
        return false;
    }
    if ctx.valid_doses.len() >= 4 {
        let birth_date = ctx.patient.birth_date;
        let dose3_date = ctx.valid_doses[2].0;
        let dose4_date = ctx.valid_doses[3].0;
        
        let dose4_at_least_4y = dose4_date >= add_years(birth_date, 4);
        let interval_ok = dose4_date >= add_months(dose3_date, 6) - chrono::Duration::days(4);
        
        dose4_at_least_4y && interval_ok
    } else {
        false
    }
}

pub fn dtp_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    _candidate_forecasts: &mut HashMap<String, VaccineGroupForecast>,
) -> String {
    let age_7 = add_years(patient.birth_date, 7);
    
    // Check if any dose on record is before age 7
    let has_dose_before_7 = history.iter().any(|dose| dose.date < age_7);
    
    // Patient must be >= 7 years of age
    let is_at_least_7 = eval_date >= age_7;
    
    if is_at_least_7 && !has_dose_before_7 {
        "DTP_3_DOSE_SERIES".to_string()
    } else {
        "DTP_5_DOSE_SERIES".to_string()
    }
}
