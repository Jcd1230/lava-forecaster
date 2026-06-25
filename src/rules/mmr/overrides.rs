use crate::date_utils::SmallVec;
use crate::engine::EvaluationContext;
use crate::engine::ValidDoseRef;
use crate::models::{
    Cvx, Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus,
};
use crate::rules::helpers::{age_ge, age_lt, clamp_date_at_least};
use chrono::NaiveDate;
use lava_cvx_macro::cvx;

fn is_live_virus(cvx: Cvx) -> bool {
    const LIVE_VIRUS: &[u16] = &[
        cvx!("03"),
        cvx!("04"),
        cvx!("05"),
        cvx!("06"),
        cvx!("07"),
        cvx!("21"),
        cvx!("37"),
        cvx!("38"),
        cvx!("75"),
        cvx!("94"),
        cvx!("105"),
        cvx!("111"),
        cvx!("121"),
        cvx!("125"),
        cvx!("149"),
        cvx!("151"),
        cvx!("183"),
        cvx!("184"),
        cvx!("325"),
        cvx!("333"),
    ];
    LIVE_VIRUS.contains(&cvx.0)
}

fn is_mmr_group(cvx: Cvx) -> bool {
    const MMR_CVX: &[u16] = &[
        cvx!("03"),
        cvx!("04"),
        cvx!("05"),
        cvx!("06"),
        cvx!("07"),
        cvx!("38"),
        cvx!("94"),
    ];
    MMR_CVX.contains(&cvx.0)
}

fn get_valid_doses_cvx(ctx: &EvaluationContext) -> Vec<Cvx> {
    ctx.valid_doses.iter().map(|dose| dose.cvx).collect()
}

fn count_components(cvxs: &[Cvx]) -> (usize, usize, usize) {
    let mut m = 0;
    let mut mu = 0;
    let mut r = 0;
    for cvx in cvxs {
        let c = cvx.0;
        if matches!(c, 3 | 4 | 5 | 94) {
            m += 1;
        }
        if matches!(c, 3 | 7 | 38 | 94) {
            mu += 1;
        }
        if matches!(c, 3 | 4 | 6 | 38 | 94) {
            r += 1;
        }
    }
    (m, mu, r)
}

fn check_live_virus_conflict(
    dose: &Dose,
    history: &[Dose],
) -> Option<SmallVec<[EvaluationReason; 4]>> {
    if is_live_virus(dose.cvx) {
        let mut reasons = SmallVec::new();
        let mut conflict = false;
        for prev in history {
            if prev.date < dose.date && is_live_virus(prev.cvx) {
                let is_both_mmr = is_mmr_group(dose.cvx) && is_mmr_group(prev.cvx);
                let required_days = if is_both_mmr {
                    let prev_has_varicella = prev.cvx.0 == 94;
                    let custom_rule = dose.cvx.0 == 94 && prev.cvx.0 == 3;
                    if prev_has_varicella || custom_rule {
                        28
                    } else {
                        24
                    }
                } else {
                    28
                };

                if dose.date < prev.date + chrono::Duration::days(required_days) {
                    conflict = true;
                    if !reasons.contains(&EvaluationReason::TooEarlyLiveVirus) {
                        reasons.push(EvaluationReason::TooEarlyLiveVirus);
                    }
                }
            }
        }
        if conflict {
            return Some(reasons);
        }
    }
    None
}

pub fn mmr_custom_evaluation_hook(
    _series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        let birth_date = ctx.patient.birth_date;

        // 1. Live Virus Conflict
        if let Some(conflict_reasons) = check_live_virus_conflict(dose, ctx.history) {
            *status = DoseStatus::Invalid;
            for r in conflict_reasons {
                if !reasons.contains(&r) {
                    reasons.push(r);
                }
            }
            return;
        }

        // 2. Outside Routine Series for Dose 1
        if target_dose_idx == 1 {
            if dose.cvx.0 == 3 || dose.cvx.0 == 4 || dose.cvx.0 == 5 {
                if age_ge(birth_date, dose.date, crate::time_period!("6m-4d"))
                    && age_lt(birth_date, dose.date, crate::time_period!("1y-4d"))
                {
                    *status = DoseStatus::Accepted;
                    reasons.clear();
                    reasons.push(EvaluationReason::OutsideRoutineSeries);
                    return;
                }
            }
        }

        // 3. Adult Dose 2 Booster/Completion
        if target_dose_idx == 2 {
            if age_ge(birth_date, dose.date, crate::time_period!("19y")) {
                *status = DoseStatus::Accepted;
                reasons.clear();
                reasons.push(EvaluationReason::BoosterDose);
                return;
            }
        }

        // Redundancy check
        let valid_cvxs = get_valid_doses_cvx(ctx);
        let (v_m, v_mu, v_r) = count_components(&valid_cvxs);
        let (d_m, d_mu, d_r) = count_components(&[dose.cvx]);

        let mut redundant = true;
        if d_m > 0 && v_m < 2 {
            redundant = false;
        }
        if d_mu > 0 && v_mu < 2 {
            redundant = false;
        }
        if d_r > 0 && v_r < 2 {
            redundant = false;
        }

        if redundant && (d_m > 0 || d_mu > 0 || d_r > 0) {
            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::BoosterDose);
            return;
        }
    }
}

pub fn mmr_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[ValidDoseRef],
    _evaluations: &[crate::models::DoseEvaluation],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let pre_1957 = NaiveDate::from_ymd_opt(1957, 1, 1).unwrap();

    let valid_cvxs: Vec<Cvx> = valid_doses.iter().map(|dose| dose.cvx).collect();
    let (m, mu, r) = count_components(&valid_cvxs);
    let is_completed = m >= 2 && mu >= 2 && r >= 2;

    let is_adult_complete = !valid_doses.is_empty()
        && (age_ge(patient.birth_date, eval_date, crate::time_period!("19y"))
            || forecast
                .status
                .recommended_date()
                .map(|d| age_ge(patient.birth_date, d, crate::time_period!("19y")))
                .unwrap_or(false));

    if is_completed || is_adult_complete || forecast.status == SeriesStatus::Complete {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = crate::reasons!["COMPLETE_HIGH_RISK"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    } else {
        if patient.birth_date < pre_1957 {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            forecast.reasons = crate::reasons!["CONDITIONAL"];
            forecast.status = forecast.status.with_earliest_date(None);
            forecast.status = forecast.status.with_recommended_date(None);
            forecast.status = forecast.status.with_overdue_date(None);
            forecast.status = forecast.status.with_latest_date(None);
        } else {
            let last_live_virus = history
                .iter()
                .filter(|d| is_live_virus(d.cvx))
                .map(|d| d.date)
                .max();

            if let Some(last_date) = last_live_virus {
                let conflict_free_date = last_date + chrono::Duration::days(28);

                if let Some(d) = forecast.status.earliest_date_mut() {
                    clamp_date_at_least(d, conflict_free_date);
                }
                if let Some(d) = forecast.status.recommended_date_mut() {
                    clamp_date_at_least(d, conflict_free_date);
                }

                if let Some(earliest) = forecast.status.earliest_date() {
                    if let Some(d) = forecast.status.recommended_date_mut() {
                        clamp_date_at_least(d, earliest);
                    }
                }
            }
        }
    }
}

pub fn mmr_custom_dose_number_hook(_series_name: &str, ctx: &EvaluationContext) -> usize {
    let valid_cvxs = get_valid_doses_cvx(ctx);
    let (m, mu, r) = count_components(&valid_cvxs);
    std::cmp::min(m, std::cmp::min(mu, r)) + 1
}

pub fn mmr_custom_completion_hook(ctx: &EvaluationContext) -> bool {
    let valid_cvxs = get_valid_doses_cvx(ctx);
    let (m, mu, r) = count_components(&valid_cvxs);
    m >= 2 && mu >= 2 && r >= 2
}

pub struct MmrPolicy;

impl crate::engine::EvaluationPolicy for MmrPolicy {
    fn same_day_priority(
        &self,
        dose: &Dose,
        _context: &crate::engine::SameDayPriorityContext,
    ) -> i32 {
        match dose.cvx.0 {
            94 => 0,
            3 => 1,
            4 | 5 => 2,
            _ => 3,
        }
    }

    fn custom_forecast_hook(
        &self,
        patient: &Patient,
        valid_doses: &[ValidDoseRef],
        _evaluations: &[crate::models::DoseEvaluation],
        history: &[Dose],
        eval_date: NaiveDate,
        forecast: &mut SeriesForecast,
    ) {
        mmr_custom_forecast_hook(
            patient,
            valid_doses,
            _evaluations,
            history,
            eval_date,
            forecast,
        )
    }

    fn custom_evaluation_hook(
        &self,
        series_name: &str,
        target_dose_idx: usize,
        ctx: &EvaluationContext,
        reasons: &mut SmallVec<[EvaluationReason; 4]>,
        status: &mut DoseStatus,
    ) {
        mmr_custom_evaluation_hook(series_name, target_dose_idx, ctx, reasons, status)
    }

    fn custom_dose_number_hook(&self, series_name: &str, ctx: &EvaluationContext) -> Option<usize> {
        Some(mmr_custom_dose_number_hook(series_name, ctx))
    }

    fn custom_completion_hook(&self, ctx: &EvaluationContext) -> Option<bool> {
        Some(mmr_custom_completion_hook(ctx))
    }

    fn custom_extra_dose_hook(
        &self,
        _series_name: &str,
        ctx: &EvaluationContext,
    ) -> Option<(DoseStatus, SmallVec<[EvaluationReason; 4]>)> {
        if let Some(dose) = ctx.current_dose {
            if let Some(conflict_reasons) = check_live_virus_conflict(dose, ctx.history) {
                return Some((DoseStatus::Invalid, conflict_reasons));
            }
        }
        None
    }

    fn adjust_same_day_target_dose_number(
        &self,
        dose: &Dose,
        evaluations: &[crate::models::DoseEvaluation],
    ) -> Option<usize> {
        let (d_m, d_mu, d_r) = count_components(&[dose.cvx]);

        let same_day_match = evaluations.iter().find(|e| {
            if e.dose_date != dose.date {
                return false;
            }
            let (e_m, e_mu, e_r) = count_components(&[e.cvx]);
            !((d_m > 0 && e_m > 0) || (d_mu > 0 && e_mu > 0) || (d_r > 0 && e_r > 0))
        });

        same_day_match.and_then(|e| e.dose_number)
    }

    fn is_same_day_duplicate(&self, dose: &Dose, sorted_history_subset: &[Dose]) -> bool {
        sorted_history_subset.iter().any(|prev_dose| {
            if prev_dose.date != dose.date {
                return false;
            }
            prev_dose.cvx == dose.cvx
                || prev_dose.cvx.0 == 3
                || prev_dose.cvx.0 == 94
                || dose.cvx.0 == 3
                || dose.cvx.0 == 94
        })
    }
}
