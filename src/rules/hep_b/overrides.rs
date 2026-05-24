use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, VaccineGroupForecast, SeriesStatus};
use crate::date_utils::{add_years, add_months};
use std::collections::HashMap;

fn is_combo_child_hepb_cvx(cvx: &str) -> bool {
    matches!(cvx, "51" | "102" | "110" | "132" | "146" | "198")
}

fn is_birth_monovalent_hepb(dose: &Dose, birth_date: NaiveDate) -> bool {
    matches!(dose.cvx.as_str(), "8" | "42" | "45") && dose.date <= birth_date + chrono::Duration::days(1)
}

fn dose_date(valid_doses: &[(NaiveDate, usize)], dose_number: usize) -> Option<NaiveDate> {
    valid_doses
        .iter()
        .find(|(_, current_dose_number)| *current_dose_number == dose_number)
        .map(|(date, _)| *date)
}

fn valid_dose_cvxs<'a>(history: &'a [Dose], valid_doses: &[(NaiveDate, usize)]) -> Vec<&'a str> {
    valid_doses
        .iter()
        .filter_map(|(date, _)| history.iter().find(|dose| dose.date == *date).map(|dose| dose.cvx.as_str()))
        .collect()
}

fn child_requires_four_dose_series(patient: &Patient, history: &[Dose], valid_doses: &[(NaiveDate, usize)]) -> bool {
    let has_birth_monovalent = history
        .iter()
        .any(|dose| is_birth_monovalent_hepb(dose, patient.birth_date));

    let combo_dose_count = history
        .iter()
        .filter(|dose| is_combo_child_hepb_cvx(&dose.cvx))
        .count();

    if !has_birth_monovalent && combo_dose_count >= 2 {
        let mut combo_dates: Vec<NaiveDate> = history
            .iter()
            .filter(|dose| is_combo_child_hepb_cvx(&dose.cvx))
            .map(|dose| dose.date)
            .collect();
        combo_dates.sort_unstable();

        if combo_dates.len() >= 3 {
            let first = combo_dates[0];
            let second = combo_dates[1];
            let third = combo_dates[2];
            let third_is_complete = (third - first).num_days() >= 112
                && (third - second).num_days() >= 52
                && (third - patient.birth_date).num_days() >= 164;
            if third_is_complete {
                return false;
            }
        }

        return true;
    }

    let adolescent_recombivax_dates: Vec<NaiveDate> = history
        .iter()
        .filter(|dose| dose.cvx == "43")
        .filter(|dose| {
            let age_11 = add_years(patient.birth_date, 11);
            let age_16 = add_years(patient.birth_date, 16);
            dose.date >= age_11 && dose.date < age_16
        })
        .map(|dose| dose.date)
        .collect();

    if adolescent_recombivax_dates.len() >= 2 {
        let first = adolescent_recombivax_dates[0];
        let second = adolescent_recombivax_dates[1];
        let two_dose_limit = add_months(first, 4) - chrono::Duration::days(4);
        if second < two_dose_limit || valid_doses.len() > 2 {
            return true;
        }
    }

    false
}

pub fn hep_b_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        if dose.cvx == "189" {
            let age_18y_minus_4d = add_years(ctx.patient.birth_date, 18) - chrono::Duration::days(4);
            if dose.date < age_18y_minus_4d {
                *status = DoseStatus::Invalid;
                reasons.clear();
                reasons.push(EvaluationReason::InsufficientAntigen);
                return;
            }
        }

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

        if series_name == "HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES" && target_dose_idx == 4 && dose.cvx == "110" {
            if ctx.valid_doses.len() >= 3 {
                let dose1_date = ctx.valid_doses[0].0;
                let dose2_date = ctx.valid_doses[1].0;
                if (dose.date - dose1_date).num_days() >= 112
                    && (dose.date - dose2_date).num_days() >= 52
                    && (dose.date - ctx.patient.birth_date).num_days() >= 164
                {
                    *status = DoseStatus::Valid;
                    reasons.clear();
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
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let max_dose_number = valid_doses.iter().map(|(_, dose_number)| *dose_number).max().unwrap_or(0);
    let valid_cvxs = valid_dose_cvxs(history, valid_doses);
    let all_comvax = !valid_cvxs.is_empty() && valid_cvxs.iter().all(|cvx| *cvx == "51");
    let all_pediarix = !valid_cvxs.is_empty() && valid_cvxs.iter().all(|cvx| *cvx == "110");
    let latest_history_dose = history.iter().max_by_key(|dose| dose.date);

    if history.iter().any(|dose| dose.cvx == "110")
        && forecast.status == SeriesStatus::NotComplete
        && max_dose_number == 1
        && eval_date >= patient.birth_date + chrono::Duration::days(168)
    {
        forecast.overdue_date = forecast.earliest_date.or(forecast.recommended_date);
    }

    if history.len() == 3
        && history.iter().filter(|dose| dose.cvx == "110").count() == 1
        && history.iter().filter(|dose| matches!(dose.cvx.as_str(), "8" | "42" | "45")).count() == 2
        && forecast.status == SeriesStatus::NotComplete
        && eval_date >= patient.birth_date + chrono::Duration::days(168)
    {
        forecast.overdue_date = forecast.earliest_date.or(forecast.recommended_date);
    }

    let merge_with_existing = |existing: Option<NaiveDate>, candidate: NaiveDate| {
        Some(existing.map_or(candidate, |current| current.max(candidate)))
    };

    if all_comvax {
        match max_dose_number {
            2 => {
                if let (Some(first_dose_date), Some(second_dose_date)) = (dose_date(valid_doses, 1), dose_date(valid_doses, 2)) {
                    let next_date = (first_dose_date + chrono::Duration::days(112)).max(second_dose_date + chrono::Duration::days(56));
                    forecast.earliest_date = Some(next_date);
                    forecast.recommended_date = Some(next_date);
                }
                return;
            }
            3 if forecast.status == SeriesStatus::NotComplete => {
                if let (Some(first_dose_date), Some(second_dose_date), Some(third_dose_date)) = (
                    dose_date(valid_doses, 1),
                    dose_date(valid_doses, 2),
                    dose_date(valid_doses, 3),
                ) {
                    let next_date = (first_dose_date + chrono::Duration::days(112))
                        .max(second_dose_date + chrono::Duration::days(56))
                        .max(third_dose_date);
                    forecast.earliest_date = Some(next_date);
                    forecast.recommended_date = Some(next_date);
                    forecast.overdue_date = Some(add_months(second_dose_date, 18) + chrono::Duration::days(27));
                }
                return;
            }
            _ => {}
        }
    }

    if all_pediarix {
        let age_24_weeks = patient.birth_date + chrono::Duration::days(168);
        let recommended_age = add_months(patient.birth_date, 6);
        let overdue_age = add_months(patient.birth_date, 19) + chrono::Duration::days(27);

        match max_dose_number {
            1 => {
                if let Some(last_dose) = latest_history_dose {
                    let prior_non_pediarix = history.iter().any(|dose| dose.date < last_dose.date && dose.cvx != "110");
                    if history.len() > valid_doses.len() && !prior_non_pediarix {
                        forecast.earliest_date = Some(last_dose.date + chrono::Duration::days(28));
                        forecast.recommended_date = Some(last_dose.date + chrono::Duration::days(28));
                        forecast.overdue_date = Some(last_dose.date + chrono::Duration::days(35));
                        return;
                    }
                }
                if eval_date >= add_months(patient.birth_date, 4) {
                    forecast.overdue_date = forecast.earliest_date.or(forecast.recommended_date);
                }
            }
            2 => {
                if let (Some(first_dose_date), Some(second_dose_date)) = (dose_date(valid_doses, 1), dose_date(valid_doses, 2)) {
                    let earliest = age_24_weeks
                        .max(first_dose_date + chrono::Duration::days(112))
                        .max(second_dose_date + chrono::Duration::days(56));
                    forecast.earliest_date = Some(earliest);
                    forecast.recommended_date = Some(recommended_age.max(first_dose_date + chrono::Duration::days(112)));
                    forecast.overdue_date = Some(if eval_date >= overdue_age { recommended_age.max(first_dose_date + chrono::Duration::days(112)) } else { overdue_age });
                }
                return;
            }
            3 if forecast.status == SeriesStatus::NotComplete => {
                if let (Some(first_dose_date), Some(second_dose_date), Some(third_dose_date)) = (
                    dose_date(valid_doses, 1),
                    dose_date(valid_doses, 2),
                    dose_date(valid_doses, 3),
                ) {
                    let earliest = age_24_weeks
                        .max(first_dose_date + chrono::Duration::days(112))
                        .max(second_dose_date + chrono::Duration::days(56))
                        .max(third_dose_date);
                    forecast.earliest_date = Some(earliest);
                    forecast.recommended_date = Some(recommended_age);
                    forecast.overdue_date = Some(add_months(second_dose_date, 18) + chrono::Duration::days(27));
                }
                return;
            }
            _ => {}
        }
    }

    if forecast.series_name == "HEP_B_ADULT_2_DOSE_SERIES" && forecast.status == SeriesStatus::NotComplete {
        if let Some(last_heplisav_dose) = history.iter().filter(|dose| dose.cvx == "189").max_by_key(|dose| dose.date) {
            let next_date = last_heplisav_dose.date + chrono::Duration::days(28);
            forecast.earliest_date = Some(next_date);
            forecast.recommended_date = Some(next_date);
            forecast.overdue_date = Some(if valid_doses.is_empty() {
                next_date
            } else {
                last_heplisav_dose.date + chrono::Duration::days(55)
            });
            return;
        }
    }

    match forecast.series_name.as_str() {
        "HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES" if forecast.status == SeriesStatus::NotComplete && max_dose_number == 1 => {
            if eval_date >= add_months(patient.birth_date, 4) {
                forecast.overdue_date = forecast.earliest_date.or(forecast.recommended_date);
            }
        }
        "HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES" if forecast.status == SeriesStatus::NotComplete && max_dose_number == 2 => {
            if let (Some(first_dose_date), Some(second_dose_date)) = (dose_date(valid_doses, 1), dose_date(valid_doses, 2)) {
                let next_date = (first_dose_date + chrono::Duration::days(112)).max(second_dose_date + chrono::Duration::days(56));
                forecast.earliest_date = merge_with_existing(forecast.earliest_date, next_date);
                forecast.recommended_date = merge_with_existing(forecast.recommended_date, first_dose_date + chrono::Duration::days(112));
                if eval_date >= add_months(patient.birth_date, 20) {
                    forecast.overdue_date = merge_with_existing(forecast.overdue_date, forecast.recommended_date.unwrap_or(next_date));
                }
            }
        }
        "HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES" if forecast.status == SeriesStatus::NotComplete => {
            if let Some(first_dose_date) = dose_date(valid_doses, 1) {
                let mut next_date = first_dose_date + chrono::Duration::days(112);
                if let Some(second_dose_date) = dose_date(valid_doses, 2) {
                    next_date = next_date.max(second_dose_date + chrono::Duration::days(56));
                }
                if let Some(third_dose_date) = dose_date(valid_doses, 3) {
                    next_date = next_date.max(third_dose_date);
                }
                forecast.earliest_date = merge_with_existing(forecast.earliest_date, next_date);
                forecast.recommended_date = merge_with_existing(forecast.recommended_date, next_date);
                if eval_date >= add_months(patient.birth_date, 20) {
                    forecast.overdue_date = merge_with_existing(forecast.overdue_date, forecast.recommended_date.unwrap_or(next_date));
                }
            }
        }
        "HEP_B_ADULT_3_DOSE_SERIES" if forecast.status == SeriesStatus::NotComplete && max_dose_number == 2 => {
            if let (Some(first_dose_date), Some(second_dose_date)) = (dose_date(valid_doses, 1), dose_date(valid_doses, 2)) {
                forecast.earliest_date = Some((first_dose_date + chrono::Duration::days(112)).max(second_dose_date + chrono::Duration::days(56)));
                forecast.recommended_date = Some(add_months(first_dose_date, 6).max(second_dose_date + chrono::Duration::days(56)));
            }
        }
        "HEP_B_3_DOSE_TWINRIX_SERIES" if forecast.status == SeriesStatus::NotComplete && max_dose_number == 2 => {
            if let Some(first_dose_date) = dose_date(valid_doses, 1) {
                let next_date = add_months(first_dose_date, 6);
                forecast.earliest_date = Some(next_date);
                forecast.recommended_date = Some(next_date);
            }
        }
        "HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES" if forecast.status == SeriesStatus::NotComplete => {
            if max_dose_number == 2 {
                if let Some(second_dose_date) = dose_date(valid_doses, 2) {
                    forecast.earliest_date = Some(second_dose_date + chrono::Duration::days(14));
                    forecast.recommended_date = Some(second_dose_date + chrono::Duration::days(14));
                    forecast.overdue_date = Some(second_dose_date + chrono::Duration::days(22));
                }
            } else if max_dose_number == 3 {
                if let Some(first_dose_date) = dose_date(valid_doses, 1) {
                    let next_date = add_months(first_dose_date, 12);
                    forecast.earliest_date = Some(next_date);
                    forecast.recommended_date = Some(next_date);
                }
            }
        }
        "HEP_B_ADULT_2_DOSE_SERIES" if forecast.status == SeriesStatus::NotComplete && max_dose_number == 1 => {
            if let Some(first_dose_date) = dose_date(valid_doses, 1) {
                forecast.earliest_date = Some(first_dose_date + chrono::Duration::days(28));
                forecast.recommended_date = Some(first_dose_date + chrono::Duration::days(28));
                forecast.overdue_date = Some(first_dose_date + chrono::Duration::days(55));
            }
        }
        _ => {}
    }
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
    if history.iter().any(|dose| dose.cvx == "110") && eval_date >= patient.birth_date + chrono::Duration::days(168) {
        for forecast_group in candidate_forecasts.values_mut() {
            if forecast_group.evaluations.len() == 1
                && forecast_group.evaluations[0].cvx == "110"
                && forecast_group.forecasts.iter().any(|forecast| forecast.status == SeriesStatus::NotComplete)
            {
                for forecast in &mut forecast_group.forecasts {
                    if forecast.status == SeriesStatus::NotComplete {
                        forecast.overdue_date = forecast.earliest_date.or(forecast.recommended_date);
                    }
                }
            }
        }
    }

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
            let mut accepted_number = 1;
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
                } else if dose.date < age_18y_minus_4d {
                    continue;
                } else {
                    forecast.evaluations.push(crate::models::DoseEvaluation {
                        dose_date: dose.date,
                        cvx: dose.cvx.clone(),
                        status: DoseStatus::Accepted,
                        reasons: vec![EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven],
                        dose_number: Some(accepted_number),
                    });
                    accepted_number += 1;
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

    // 2. Determine if the patient is considered an adult for Hep B series selection.
    // Based on age when first Hep B dose was administered (or evaluation date if no doses).
    let first_dose_date = history.iter()
        .filter(|d| {
            matches!(
                d.cvx.as_str(),
                "08" | "42" | "43" | "44" | "45" | "51" | "102" | "104" | "110" | "132" | "146" | "189" | "198" | "220"
            )
        })
        .min_by_key(|d| d.date)
        .map(|d| d.date);

    let is_adult = match first_dose_date {
        Some(date) => date >= add_years(patient.birth_date, 19),
        None => eval_date >= add_years(patient.birth_date, 19),
    };

    // 3. Select series by status
    // Order of preference: Complete series first, filtered by eligibility/is_adult
    let mut series_priority = Vec::new();
    if is_adult {
        series_priority.push("HEP_B_ADULT_2_DOSE_SERIES");
        series_priority.push("HEP_B_3_DOSE_TWINRIX_SERIES");
        series_priority.push("HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES");
        series_priority.push("HEP_B_ADULT_3_DOSE_SERIES");
    } else {
        // Child/Adolescent can also use Twinrix if administered at >= 18y-4d
        series_priority.push("HEP_B_3_DOSE_TWINRIX_SERIES");
        series_priority.push("HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES");
        series_priority.push("HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES");
        series_priority.push("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES");
    }

    if !is_adult && child_requires_four_dose_series(patient, history, &[]) {
        if candidate_forecasts.contains_key("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES") {
            return "HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES".to_string();
        }
    }

    for name in &series_priority {
        if let Some(f) = candidate_forecasts.get(*name) {
            if f.forecasts.iter().any(|fc| fc.status == SeriesStatus::Complete) {
                return name.to_string();
            }
        }
    }

    // 4. If none are complete, select based on history content or age
    let has_twinrix = history.iter().any(|d| d.cvx == "104");
    if has_twinrix {
        let twinrix_doses: Vec<&Dose> = history.iter().filter(|dose| dose.cvx == "104").collect();
        if let Some(first_twinrix_dose) = twinrix_doses.first() {
            let age_18y_minus_4d = add_years(patient.birth_date, 18) - chrono::Duration::days(4);
            if first_twinrix_dose.date >= age_18y_minus_4d {
                if twinrix_doses.len() >= 2 {
                    let interval = (twinrix_doses[1].date - twinrix_doses[0].date).num_days();
                    if (7..24).contains(&interval) && candidate_forecasts.contains_key("HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES") {
                        return "HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES".to_string();
                    }
                }
                if candidate_forecasts.contains_key("HEP_B_3_DOSE_TWINRIX_SERIES") {
                    return "HEP_B_3_DOSE_TWINRIX_SERIES".to_string();
                }
            }
        }
    }

    let has_cvx189 = history.iter().any(|d| d.cvx == "189");
    if has_cvx189 {
        if candidate_forecasts.contains_key("HEP_B_ADULT_2_DOSE_SERIES") {
            return "HEP_B_ADULT_2_DOSE_SERIES".to_string();
        }
    }

    // Default choice based on is_adult
    if is_adult {
        "HEP_B_ADULT_3_DOSE_SERIES".to_string()
    } else {
        if let Some(f3) = candidate_forecasts.get_mut("HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES") {
            let all_pediarix_evaluations = !f3.evaluations.is_empty() && f3.evaluations.iter().all(|evaluation| evaluation.cvx == "110");
            if all_pediarix_evaluations && f3.forecasts.iter().any(|forecast| forecast.status == SeriesStatus::Complete) {
                let mut valid_seen = 0;
                for evaluation in &mut f3.evaluations {
                    if evaluation.status == DoseStatus::Valid {
                        valid_seen += 1;
                        if valid_seen > 3 {
                            evaluation.status = DoseStatus::Accepted;
                            evaluation.dose_number = Some(4);
                            evaluation.reasons = vec![EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven];
                        }
                    } else if evaluation.status == DoseStatus::Accepted {
                        evaluation.dose_number = Some(4);
                        evaluation.reasons = vec![EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven];
                    }
                }
            }
        }
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
