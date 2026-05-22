use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, VaccineGroupForecast, SeriesStatus};
use crate::date_utils::{add_years, add_months};
use std::collections::HashMap;

pub fn hep_b_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        // 1. Max valid age clamps for CVX 42 and CVX 08
        if dose.cvx == "42" || dose.cvx == "08" {
            let age_20y = add_years(ctx.patient.birth_date, 20);
            if dose.date >= age_20y {
                *status = DoseStatus::Invalid;
                if !reasons.contains(&EvaluationReason::InsufficientAntigen) {
                    reasons.push(EvaluationReason::InsufficientAntigen);
                }
            }
        }

        // 2. Twinrix 3-dose absolute minimum interval 1->3 is 6 months - 4 days
        if series_name == "HEP_B_3_DOSE_TWINRIX_SERIES" && target_dose_idx == 3 {
            if ctx.valid_doses.len() >= 1 {
                let dose1_date = ctx.valid_doses[0].0;
                let limit = add_months(dose1_date, 6) - chrono::Duration::days(4);
                if dose.date < limit {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }

        // 3. Twinrix 4-dose Accelerated absolute minimum interval 1->4 is 12 months - 4 days
        if series_name == "HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES" && target_dose_idx == 4 {
            if ctx.valid_doses.len() >= 1 {
                let dose1_date = ctx.valid_doses[0].0;
                let limit = add_months(dose1_date, 12) - chrono::Duration::days(4);
                if dose.date < limit {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }

        // 4. Child/Adolescent 3-dose and 4-dose absolute minimum interval 1->3 and 1->4 is 112 days
        if series_name == "HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES" && target_dose_idx == 3 {
            if ctx.valid_doses.len() >= 1 {
                let dose1_date = ctx.valid_doses[0].0;
                if (dose.date - dose1_date).num_days() < 112 {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }
        if series_name == "HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES" && target_dose_idx == 4 {
            if ctx.valid_doses.len() >= 1 {
                let dose1_date = ctx.valid_doses[0].0;
                if (dose.date - dose1_date).num_days() < 112 {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }

        // 5. Child/Adolescent 4-dose absolute minimum interval 2->4 is 52 days
        if series_name == "HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES" && target_dose_idx == 4 {
            if ctx.valid_doses.len() >= 2 {
                let dose2_date = ctx.valid_doses[1].0;
                if (dose.date - dose2_date).num_days() < 52 {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }

        // 6. Adult 3-dose absolute minimum interval 1->3 is 16w-4d (108 days)
        if series_name == "HEP_B_ADULT_3_DOSE_SERIES" && target_dose_idx == 3 {
            if ctx.valid_doses.len() >= 1 {
                let dose1_date = ctx.valid_doses[0].0;
                if (dose.date - dose1_date).num_days() < 108 {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }

        // 7. Adult "Enough is Enough" Rule
        if series_name == "HEP_B_ADULT_3_DOSE_SERIES" && target_dose_idx == 3 {
            if *status == DoseStatus::Invalid && reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                if ctx.valid_doses.len() >= 2 {
                    let dose2_date = ctx.valid_doses[1].0;
                    if (dose.date - dose2_date).num_days() >= 52 {
                        reasons.retain(|r| *r != EvaluationReason::BelowMinimumInterval);
                        if reasons.is_empty() {
                            *status = DoseStatus::Valid;
                        }
                    }
                }
            }
        }

        // 8. Heplisav-B (CVX 189) interval suppression
        if dose.cvx == "189" {
            let prior_valid_189 = ctx.valid_doses.iter().find(|(v_date, _)| {
                ctx.history.iter().any(|h| h.date == *v_date && h.cvx == "189")
            });
            if let Some((prior_date, _)) = prior_valid_189 {
                let days = (dose.date - *prior_date).num_days();
                if days >= 24 {
                    if *status == DoseStatus::Invalid && reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.retain(|r| *r != EvaluationReason::BelowMinimumInterval);
                        if reasons.is_empty() {
                            *status = DoseStatus::Valid;
                        }
                    }
                }
            }
        }

        // 9. Series switching child 3-dose to 4-dose: Dose 3 validated retroactively
        if series_name == "HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES" && target_dose_idx == 3 {
            if *status == DoseStatus::Invalid {
                let only_allowable_invalid_reasons = reasons.iter().all(|r| {
                    *r == EvaluationReason::BelowMinimumInterval || *r == EvaluationReason::BelowMinimumAge
                });
                if only_allowable_invalid_reasons && !reasons.is_empty() {
                    *status = DoseStatus::Valid;
                    reasons.clear();
                }
            }
        }
    }
}

pub fn hep_b_custom_forecast_hook(
    _patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    _eval_date: NaiveDate,
    _forecast: &mut SeriesForecast,
) {
    // Standard forecasting is sufficient or can be updated post-process
}

pub fn hep_b_adolescent_completion_condition(ctx: &EvaluationContext) -> bool {
    if ctx.active_series_name != "HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES" {
        return false;
    }
    if ctx.valid_doses.len() == 2 {
        let birth_date = ctx.patient.birth_date;
        let dose1_date = ctx.valid_doses[0].0;
        let dose2_date = ctx.valid_doses[1].0;

        let dose1_cvx = ctx.history.iter().find(|d| d.date == dose1_date).map(|d| d.cvx.as_str()).unwrap_or("");
        let dose2_cvx = ctx.history.iter().find(|d| d.date == dose2_date).map(|d| d.cvx.as_str()).unwrap_or("");

        if dose1_cvx == "43" && dose2_cvx == "43" {
            let age_11 = add_years(birth_date, 11);
            let age_16 = add_years(birth_date, 16);
            if dose1_date >= age_11 && dose1_date < age_16 &&
               dose2_date >= age_11 && dose2_date < age_16 {
                let limit = add_months(dose1_date, 4) - chrono::Duration::days(4);
                if dose2_date >= limit {
                    return true;
                }
            }
        }
    }
    false
}

pub fn hep_b_custom_switch_hook(
    current_series_name: &str,
    target_dose_idx: usize,
    _ctx: &EvaluationContext,
) -> Option<&'static str> {
    // If evaluating Dose 4 but we are on Child 3-dose, switch to Child 4-dose!
    if current_series_name == "HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES" && target_dose_idx == 4 {
        return Some("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES");
    }
    None
}

pub fn hep_b_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut HashMap<String, VaccineGroupForecast>,
) -> String {
    // 1. Post-process Heplisav-B (CVX 189) Adult 2-Dose exception
    // Check if there are 2 doses of CVX 189 given at age >= 18y-4d, separated by >= 24 days.
    let age_18y_minus_4d = add_years(patient.birth_date, 18) - chrono::Duration::days(4);
    let cvx189_doses: Vec<&Dose> = history.iter()
        .filter(|d| d.cvx == "189" && d.date >= age_18y_minus_4d)
        .collect();

    let mut heplisav_complete = false;
    let mut pair_indices = None;
    if cvx189_doses.len() >= 2 {
        for j in 0..cvx189_doses.len() {
            for k in (j + 1)..cvx189_doses.len() {
                if (cvx189_doses[k].date - cvx189_doses[j].date).num_days() >= 24 {
                    heplisav_complete = true;
                    pair_indices = Some((cvx189_doses[j].date, cvx189_doses[k].date));
                    break;
                }
            }
            if heplisav_complete {
                break;
            }
        }
    }

    if heplisav_complete {
        let (d1_date, d2_date) = pair_indices.unwrap();
        // Post-process the candidate forecast for HEP_B_ADULT_2_DOSE_SERIES to be Complete and override other doses to Accepted
        if let Some(forecast) = candidate_forecasts.get_mut("HEP_B_ADULT_2_DOSE_SERIES") {
            forecast.evaluations.clear();
            let mut d1_done = false;
            let mut d2_done = false;
            for dose in history {
                if dose.cvx == "189" && dose.date == d1_date && !d1_done {
                    forecast.evaluations.push(crate::models::DoseEvaluation {
                        dose_date: dose.date,
                        cvx: dose.cvx.clone(),
                        status: DoseStatus::Valid,
                        reasons: Vec::new(),
                        dose_number: Some(1),
                    });
                    d1_done = true;
                } else if dose.cvx == "189" && dose.date == d2_date && !d2_done {
                    forecast.evaluations.push(crate::models::DoseEvaluation {
                        dose_date: dose.date,
                        cvx: dose.cvx.clone(),
                        status: DoseStatus::Valid,
                        reasons: Vec::new(),
                        dose_number: Some(2),
                    });
                    d2_done = true;
                } else {
                    forecast.evaluations.push(crate::models::DoseEvaluation {
                        dose_date: dose.date,
                        cvx: dose.cvx.clone(),
                        status: DoseStatus::Accepted,
                        reasons: vec![EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven],
                        dose_number: None,
                    });
                }
            }
            for f in &mut forecast.forecasts {
                f.status = SeriesStatus::Complete;
                f.reasons = vec!["COMPLETE".to_string()];
                f.earliest_date = None;
                f.recommended_date = None;
                f.overdue_date = None;
                f.latest_date = None;
            }
            return "HEP_B_ADULT_2_DOSE_SERIES".to_string();
        }
    }

    // 2. Select series by status or age
    // Order of preference: Complete series first
    let series_priority = [
        "HEP_B_ADULT_2_DOSE_SERIES",
        "HEP_B_3_DOSE_TWINRIX_SERIES",
        "HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES",
        "HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES",
        "HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES",
        "HEP_B_ADULT_3_DOSE_SERIES",
    ];

    for name in &series_priority {
        if let Some(f) = candidate_forecasts.get(*name) {
            if f.forecasts.iter().any(|fc| fc.status == SeriesStatus::Complete) {
                return name.to_string();
            }
        }
    }

    // If none are complete, select based on history content or age
    let has_twinrix = history.iter().any(|d| d.cvx == "104");
    if has_twinrix {
        // Prefer Twinrix
        if candidate_forecasts.contains_key("HEP_B_3_DOSE_TWINRIX_SERIES") {
            return "HEP_B_3_DOSE_TWINRIX_SERIES".to_string();
        }
    }

    let has_cvx189 = history.iter().any(|d| d.cvx == "189");
    if has_cvx189 {
        let first_cvx189_date = history.iter().find(|d| d.cvx == "189").map(|d| d.date).unwrap();
        let age_18 = add_years(patient.birth_date, 18);
        if first_cvx189_date >= age_18 {
            if candidate_forecasts.contains_key("HEP_B_ADULT_2_DOSE_SERIES") {
                return "HEP_B_ADULT_2_DOSE_SERIES".to_string();
            }
        }
    }

    // Default to age-based choice at evaluation date
    let age_19y = add_years(patient.birth_date, 19);
    if eval_date >= age_19y {
        "HEP_B_ADULT_3_DOSE_SERIES".to_string()
    } else {
        // For children/adolescents, check if switched or default to 3-Dose
        if candidate_forecasts.contains_key("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES") {
            let f4 = candidate_forecasts.get("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES").unwrap();
            // If any dose was evaluated as Dose 4, it means we switched!
            if f4.evaluations.iter().any(|e| e.dose_number == Some(4)) {
                return "HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES".to_string();
            }
        }
        "HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES".to_string()
    }
}
