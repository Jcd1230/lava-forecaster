use crate::date_utils::TinyVec;
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
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        let birth_date = ctx.patient.birth_date;

        // 1. Outside Routine Series for Dose 1
        if target_dose_idx == 1 {
            if dose.cvx.0 == cvx!("03") || dose.cvx.0 == cvx!("04") || dose.cvx.0 == cvx!("05") {
                if age_ge(birth_date, dose.date, crate::time_period!("6m-4d")) && age_lt(birth_date, dose.date, crate::time_period!("1y-4d")) {
                    *status = DoseStatus::Accepted;
                    reasons.clear();
                    reasons.push(EvaluationReason::OutsideRoutineSeries);
                    return; // Skip further checks if accepted outside routine
                }
            }
        }

        // 2. Adult Dose 2 Booster/Completion
        if target_dose_idx == 2 {
            if age_ge(birth_date, dose.date, crate::time_period!("19y")) {
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
        age_ge(patient.birth_date, eval_date, crate::time_period!("19y"))
        || forecast.recommended_date.map(|d| age_ge(patient.birth_date, d, crate::time_period!("19y"))).unwrap_or(false)
    );

    if is_completed || is_adult_complete {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = crate::reasons!["COMPLETE_HIGH_RISK"];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
    } else {
        // Case 2: Not complete. Check if born prior to 1957
        if patient.birth_date < pre_1957 {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = crate::reasons!["CONDITIONAL"];
            forecast.earliest_date = None;
            forecast.recommended_date = None;
            forecast.overdue_date = None;
            forecast.latest_date = None;
        } else {
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

pub struct MmrPolicy;

impl crate::engine::EvaluationPolicy for MmrPolicy {
    fn custom_forecast_hook(
        &self,
        patient: &Patient,
        valid_doses: &[(NaiveDate, usize)],
        history: &[Dose],
        eval_date: NaiveDate,
        forecast: &mut SeriesForecast,
    ) {
        mmr_custom_forecast_hook(patient, valid_doses, history, eval_date, forecast)
    }

    fn custom_evaluation_hook(
        &self,
        series_name: &str,
        target_dose_idx: usize,
        ctx: &EvaluationContext,
        reasons: &mut TinyVec<EvaluationReason, 4>,
        status: &mut DoseStatus,
    ) {
        mmr_custom_evaluation_hook(series_name, target_dose_idx, ctx, reasons, status)
    }

    fn custom_dose_number_hook(
        &self,
        series_name: &str,
        ctx: &EvaluationContext,
    ) -> Option<usize> {
        Some(mmr_custom_dose_number_hook(series_name, ctx))
    }

    fn custom_completion_hook(&self, ctx: &EvaluationContext) -> Option<bool> {
        Some(mmr_custom_completion_hook(ctx))
    }

    fn adjust_same_day_target_dose_number(
        &self,
        dose: &Dose,
        evaluations: &[crate::models::DoseEvaluation],
    ) -> Option<usize> {
        let has_m = |c: Cvx| matches!(c.0, Cvx::MMR | Cvx::MEASLES_RUBELLA | Cvx::MEASLES | Cvx::MMRV);
        let has_mu = |c: Cvx| matches!(c.0, Cvx::MMR | Cvx::MUMPS | Cvx::RUBELLA_MUMPS | Cvx::MMRV);
        let has_r = |c: Cvx| matches!(c.0, Cvx::MMR | Cvx::MEASLES_RUBELLA | Cvx::RUBELLA | Cvx::RUBELLA_MUMPS | Cvx::MMRV);

        let cur_cvx = dose.cvx;
        let same_day_match = evaluations.iter().find(|e| {
            e.dose_date == dose.date
                && !((has_m(e.cvx) && has_m(cur_cvx))
                    || (has_mu(e.cvx) && has_mu(cur_cvx))
                    || (has_r(e.cvx) && has_r(cur_cvx)))
        });
        
        if let Some(prev_eval) = same_day_match {
            prev_eval.dose_number
        } else {
            None
        }
    }

    fn is_same_day_duplicate(&self, dose: &Dose, sorted_history_subset: &[Dose]) -> bool {
        let cur_cvx = dose.cvx;
        sorted_history_subset.iter().any(|prev_dose| {
            prev_dose.date == dose.date && {
                let prev_cvx = prev_dose.cvx;
                let has_m = |c: Cvx| matches!(c.0, Cvx::MMR | Cvx::MEASLES_RUBELLA | Cvx::MEASLES | Cvx::MMRV);
                let has_mu = |c: Cvx| matches!(c.0, Cvx::MMR | Cvx::MUMPS | Cvx::RUBELLA_MUMPS | Cvx::MMRV);
                let has_r = |c: Cvx| matches!(c.0, Cvx::MMR | Cvx::MEASLES_RUBELLA | Cvx::RUBELLA | Cvx::RUBELLA_MUMPS | Cvx::MMRV);
                (has_m(prev_cvx) && has_m(cur_cvx))
                    || (has_mu(prev_cvx) && has_mu(cur_cvx))
                    || (has_r(prev_cvx) && has_r(cur_cvx))
            }
        })
    }
}
