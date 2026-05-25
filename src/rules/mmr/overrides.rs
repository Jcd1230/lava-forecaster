use ice_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus};
use crate::rules::helpers::{clamp_date_at_least, age_ge, age_lt};

fn is_live_virus(cvx: Cvx) -> bool {
    const LIVE_VIRUS: &[u16] = &[cvx!("03"), cvx!("04"), cvx!("05"), cvx!("06"), cvx!("07"), cvx!("21"), cvx!("37"), cvx!("38"), cvx!("75"), cvx!("94"), cvx!("105"), cvx!("111"), cvx!("121"), cvx!("125"), cvx!("149"), cvx!("151"), cvx!("183"), cvx!("184"), cvx!("325"), cvx!("333")];
    LIVE_VIRUS.contains(&cvx.0)
}

fn is_mmr_group(cvx: Cvx) -> bool {
    const MMR_CVX: &[u16] = &[cvx!("03"), cvx!("04"), cvx!("05"), cvx!("06"), cvx!("07"), cvx!("38"), cvx!("94")];
    MMR_CVX.contains(&cvx.0)
}

fn has_same_day_mixed_live_virus(history: &[Dose], eval_date: NaiveDate) -> bool {
    let has_mmr_live = history
        .iter()
        .any(|dose| dose.date == eval_date && is_mmr_group(dose.cvx));
    let has_non_mmr_live = history
        .iter()
        .any(|dose| dose.date == eval_date && is_live_virus(dose.cvx) && !is_mmr_group(dose.cvx));

    has_mmr_live && has_non_mmr_live
}

fn get_valid_doses_cvx(ctx: &EvaluationContext) -> Vec<Cvx> {
    let mut cvxs = Vec::new();
    let mut history_idx = 0;
    for &(valid_date, _) in ctx.valid_doses {
        while history_idx < ctx.history.len() && ctx.history[history_idx].date != valid_date {
            history_idx += 1;
        }
        if history_idx < ctx.history.len() {
            cvxs.push(ctx.history[history_idx].cvx.clone());
            history_idx += 1;
        }
    }
    cvxs
}

pub fn mmr_custom_evaluation_hook(
    _series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        let birth_date = ctx.patient.birth_date;

        // 1. Outside Routine Series for Dose 1
        if target_dose_idx == 1 {
            if dose.cvx.0 == cvx!("03") || dose.cvx.0 == cvx!("04") || dose.cvx.0 == cvx!("05") {
                if age_ge(birth_date, dose.date, "6m-4d") && age_lt(birth_date, dose.date, "1y-4d") {
                    *status = DoseStatus::Accepted;
                    reasons.clear();
                    reasons.push(EvaluationReason::OutsideRoutineSeries);
                    return; // Skip further checks if accepted outside routine
                }
            }
        }

        // 2. Adult Dose 2 Booster/Completion
        if target_dose_idx == 2 {
            if age_ge(birth_date, dose.date, "19y") {
                *status = DoseStatus::Accepted;
                reasons.clear();
                reasons.push(EvaluationReason::BoosterDose);
                return; // Skip further checks
            }
        }

        // Redundancy check: if the dose components are already fully satisfied by previous valid doses
        let mut valid_m_count = 0;
        let mut valid_mu_count = 0;
        let mut valid_r_count = 0;
        let valid_cvxs = get_valid_doses_cvx(ctx);
        for cvx in &valid_cvxs {
            if matches!(cvx.0, cvx!("03") | cvx!("04") | cvx!("05") | cvx!("94")) {
                valid_m_count += 1;
            }
            if matches!(cvx.0, cvx!("03") | cvx!("07") | cvx!("38") | cvx!("94")) {
                valid_mu_count += 1;
            }
            if matches!(cvx.0, cvx!("03") | cvx!("04") | cvx!("06") | cvx!("38") | cvx!("94")) {
                valid_r_count += 1;
            }
        }
        let has_m = matches!(dose.cvx.0, cvx!("03") | cvx!("04") | cvx!("05") | cvx!("94"));
        let has_mu = matches!(dose.cvx.0, cvx!("03") | cvx!("07") | cvx!("38") | cvx!("94"));
        let has_r = matches!(dose.cvx.0, cvx!("03") | cvx!("04") | cvx!("06") | cvx!("38") | cvx!("94"));
        let mut redundant = true;
        if has_m && valid_m_count < 2 {
            redundant = false;
        }
        if has_mu && valid_mu_count < 2 {
            redundant = false;
        }
        if has_r && valid_r_count < 2 {
            redundant = false;
        }
        if redundant {
            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::BoosterDose);
            return;
        }

        // 3. Live Virus Conflict
        if is_live_virus(dose.cvx) {
            for prev in ctx.history {
                if prev.date < dose.date && is_live_virus(prev.cvx) {
                    let is_both_mmr = is_mmr_group(dose.cvx) && is_mmr_group(prev.cvx);
                    let required_days = if is_both_mmr {
                        if dose.cvx.0 == cvx!("94") || prev.cvx.0 == cvx!("94") {
                            28
                        } else {
                            24
                        }
                    } else {
                        28
                    };

                    if dose.date < prev.date + chrono::Duration::days(required_days) {
                        *status = DoseStatus::Invalid;
                        if !reasons.contains(&EvaluationReason::TooEarlyLiveVirus) {
                            reasons.push(EvaluationReason::TooEarlyLiveVirus);
                        }
                    }
                }
            }
        }
    }
}

pub fn mmr_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let pre_1957 = NaiveDate::from_ymd_opt(1957, 1, 1).unwrap();

    // Case 1: Series is Complete (either already completed or adult completion)
    let is_completed = forecast.status == SeriesStatus::Complete;
    let is_adult_complete = !valid_doses.is_empty() && (
        age_ge(patient.birth_date, eval_date, "19y")
        || forecast.recommended_date.map(|d| age_ge(patient.birth_date, d, "19y")).unwrap_or(false)
    );

    if is_completed || is_adult_complete {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = vec!["COMPLETE_HIGH_RISK".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
    } else {
        // Case 2: Not complete. Check if born prior to 1957
        if patient.birth_date < pre_1957 {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = vec!["CONDITIONAL".to_string()];
            forecast.earliest_date = None;
            forecast.recommended_date = None;
            forecast.overdue_date = None;
            forecast.latest_date = None;
        } else {
            // Case 3: Adjust earliest and recommended dates based on live virus conflict in history
            let last_live_virus = history.iter()
                .filter(|d| is_live_virus(d.cvx))
                .map(|d| d.date)
                .max();

            if let Some(last_date) = last_live_virus {
                let conflict_free_date = last_date + chrono::Duration::days(28);
                
                clamp_date_at_least(&mut forecast.earliest_date, conflict_free_date);
                clamp_date_at_least(&mut forecast.recommended_date, conflict_free_date);

                if let Some(earliest) = forecast.earliest_date {
                    if forecast.recommended_date.is_some() {
                        clamp_date_at_least(&mut forecast.recommended_date, earliest);
                    }
                }
            }

            if has_same_day_mixed_live_virus(history, eval_date) {
                forecast.earliest_date = Some(eval_date);
                forecast.recommended_date = Some(eval_date);
            }
        }
    }
}

pub fn mmr_custom_dose_number_hook(
    _series_name: &str,
    ctx: &EvaluationContext,
) -> usize {
    let mut m1 = false;
    let mut mu1 = false;
    let mut r1 = false;

    let mut m2 = false;
    let mut mu2 = false;
    let mut r2 = false;

    let valid_cvxs = get_valid_doses_cvx(ctx);
    for cvx in &valid_cvxs {
        let has_m = matches!(cvx.0, cvx!("03") | cvx!("04") | cvx!("05") | cvx!("94"));
        let has_mu = matches!(cvx.0, cvx!("03") | cvx!("07") | cvx!("38") | cvx!("94"));
        let has_r = matches!(cvx.0, cvx!("03") | cvx!("04") | cvx!("06") | cvx!("38") | cvx!("94"));

        let covers_new_d1 = (has_m && !m1) || (has_mu && !mu1) || (has_r && !r1);
        if covers_new_d1 {
            m1 = m1 || has_m;
            mu1 = mu1 || has_mu;
            r1 = r1 || has_r;
        } else {
            m2 = m2 || has_m;
            mu2 = mu2 || has_mu;
            r2 = r2 || has_r;
        }
    }

    if let Some(current_dose) = ctx.current_dose {
        let has_m = matches!(current_dose.cvx.0, cvx!("03") | cvx!("04") | cvx!("05") | cvx!("94"));
        let has_mu = matches!(current_dose.cvx.0, cvx!("03") | cvx!("07") | cvx!("38") | cvx!("94"));
        let has_r = matches!(current_dose.cvx.0, cvx!("03") | cvx!("04") | cvx!("06") | cvx!("38") | cvx!("94"));

        let covers_new_d1 = (has_m && !m1) || (has_mu && !mu1) || (has_r && !r1);
        if covers_new_d1 {
            1
        } else {
            2
        }
    } else {
        // Forecasting mode (current_dose is None)
        let d1_satisfied = m1 && mu1 && r1;
        if !d1_satisfied {
            1
        } else {
            let d2_satisfied = m2 && mu2 && r2;
            if !d2_satisfied {
                2
            } else {
                3
            }
        }
    }
}

pub fn mmr_custom_completion_hook(ctx: &EvaluationContext) -> bool {
    let mut m1 = false;
    let mut mu1 = false;
    let mut r1 = false;

    let mut m2 = false;
    let mut mu2 = false;
    let mut r2 = false;

    let valid_cvxs = get_valid_doses_cvx(ctx);
    for cvx in &valid_cvxs {
        let has_m = matches!(cvx.0, cvx!("03") | cvx!("04") | cvx!("05") | cvx!("94"));
        let has_mu = matches!(cvx.0, cvx!("03") | cvx!("07") | cvx!("38") | cvx!("94"));
        let has_r = matches!(cvx.0, cvx!("03") | cvx!("04") | cvx!("06") | cvx!("38") | cvx!("94"));

        let covers_new_d1 = (has_m && !m1) || (has_mu && !mu1) || (has_r && !r1);
        if covers_new_d1 {
            m1 = m1 || has_m;
            mu1 = mu1 || has_mu;
            r1 = r1 || has_r;
        } else {
            m2 = m2 || has_m;
            mu2 = mu2 || has_mu;
            r2 = r2 || has_r;
        }
    }

    let d1_satisfied = m1 && mu1 && r1;
    let d2_satisfied = m2 && mu2 && r2;
    d1_satisfied && d2_satisfied
}
