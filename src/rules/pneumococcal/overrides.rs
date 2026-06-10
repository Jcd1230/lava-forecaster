use crate::rules::helpers::{age_ge, age_lt};
use lava_cvx_macro::cvx;
use chrono::NaiveDate;

use crate::date_utils::{SmallVec, compare_elapsed, TimePeriod};
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus};

const CHILD_PCV_CVX: &[u16] = &[cvx!("100"), cvx!("133"), cvx!("177"), cvx!("215"), cvx!("216"), cvx!("109"), cvx!("152"), cvx!("327")];
const MODERN_PCV_CVX: &[u16] = &[cvx!("133"), cvx!("152"), cvx!("177"), cvx!("215"), cvx!("216")];
const ADULT_COMPLETE_PCV_CVX: &[u16] = &[cvx!("216"), cvx!("327")];

fn is_pcv_cvx(cvx: Cvx) -> bool {
    matches!(cvx.0, cvx!("100") | cvx!("109") | cvx!("133") | cvx!("152") | cvx!("177") | cvx!("215") | cvx!("216") | cvx!("327"))
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

fn count_pcv_history_before_age(history: &[Dose], birth: NaiveDate, age: TimePeriod) -> usize {
    let cutoff = age.add_to(birth);
    history
        .iter()
        .filter(|d| is_pcv_cvx(d.cvx) && d.date < cutoff)
        .count()
}

fn valid_adult_doses<'a>(valid_doses: &[(NaiveDate, usize)], history: &'a [Dose]) -> Vec<&'a Dose> {
    valid_doses
        .iter()
        .filter(|(_, dose_num)| *dose_num >= 6)
        .filter_map(|(date, _)| {
            history
                .iter()
                .find(|d| d.date == *date && (d.cvx.0 == cvx!("33") || is_pcv_cvx(d.cvx)))
        })
        .collect()
}

fn has_valid_child_modern_pcv(valid_doses: &[(NaiveDate, usize)], history: &[Dose]) -> bool {
    valid_doses.iter().any(|(date, dose_num)| {
        *dose_num <= 5
            && history
                .iter()
                .any(|d| d.date == *date && MODERN_PCV_CVX.contains(&d.cvx.0))
    })
}

pub fn pneumococcal_custom_dose_number_hook(_series_name: &str, ctx: &EvaluationContext) -> usize {
    let ref_date = ctx.current_dose.map(|d| d.date).unwrap_or(ctx.eval_date);
    let birth = ctx.patient.birth_date;
    let target = ctx.target_dose_number;

    if age_ge(birth, ref_date, crate::time_period!("5y")) {
        return target.max(6);
    }
    if age_ge(birth, ref_date, crate::time_period!("24m")) {
        return target.max(4);
    }
    if age_ge(birth, ref_date, crate::time_period!("12m")) {
        let pcv_before_12m = count_pcv_history_before_age(ctx.history, birth, crate::time_period!("12m"));
        if pcv_before_12m == 0 { return target.max(4); }
        if pcv_before_12m == 1 { return target.max(5); }
        return target.max(4);
    }
    if age_ge(birth, ref_date, crate::time_period!("7m")) {
        let pcv_before_7m = count_pcv_history_before_age(ctx.history, birth, crate::time_period!("7m"));
        if pcv_before_7m == 0 { return target.max(2); }
    }
    target
}

pub fn pneumococcal_custom_completion_hook(ctx: &EvaluationContext) -> bool {
    let birth = ctx.patient.birth_date;
    let ref_date = ctx.eval_date;
    
    let is_high_risk = ctx.history.iter().any(|d| {
        matches!(d.cvx.0, cvx!("327") | cvx!("216")) && age_lt(birth, d.date, crate::time_period!("19y"))
    });

    if age_ge(birth, ref_date, crate::time_period!("5y")) && age_lt(birth, ref_date, crate::time_period!("19y")) {
        return !is_high_risk;
    }

    let valid_pcv_doses: Vec<NaiveDate> = ctx.valid_doses.iter()
        .filter(|(d, _)| ctx.history.iter().any(|h| h.date == *d && is_pcv_cvx(h.cvx)))
        .map(|(d, _)| *d)
        .collect();
    
    if valid_pcv_doses.is_empty() { return false; }
    
    let first_dose_date = valid_pcv_doses[0];
    let num_doses = valid_pcv_doses.len();
    
    if age_ge(birth, first_dose_date, crate::time_period!("24m")) {
        return num_doses >= 1;
    }
    if age_ge(birth, first_dose_date, crate::time_period!("12m")) {
        return num_doses >= 2;
    }
    if age_ge(birth, first_dose_date, crate::time_period!("7m")) {
        return num_doses >= 3;
    }
    
    num_doses >= 4
}

pub fn pneumococcal_custom_evaluation_hook(
    _series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    let dose = match ctx.current_dose {
        Some(dose) => dose,
        None => return,
    };
    let birth = ctx.patient.birth_date;

    if target_dose_idx <= 5 {
        if dose.cvx.0 == cvx!("33") {
            if age_lt(birth, dose.date, crate::time_period!("2y")) {
                *status = DoseStatus::Invalid;
                reasons.clear();
                reasons.push(EvaluationReason::BelowMinimumAge);
            } else {
                *status = DoseStatus::Accepted;
                reasons.clear();
                reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            }
            return;
        }

        if !CHILD_PCV_CVX.contains(&dose.cvx.0) {
            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            return;
        }

        if target_dose_idx >= 5 && age_lt(birth, dose.date, crate::time_period!("5y")) {
            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::BoosterDose);
            return;
        }

        if target_dose_idx == 4 && age_lt(birth, dose.date, crate::time_period!("1y-4d")) {
            let pcv_before_7m = count_pcv_history_before_age(ctx.history, birth, crate::time_period!("7m"));
            if pcv_before_7m == 0 {
                *status = DoseStatus::Invalid;
                if !reasons.contains(&EvaluationReason::BelowMinimumAgeFinalDose) {
                    reasons.push(EvaluationReason::BelowMinimumAgeFinalDose);
                }
            }
        }
    }

    if target_dose_idx == 8 && !matches!(dose.cvx.0, cvx!("33") | cvx!("216") | cvx!("327")) {
        *status = if dose.cvx.0 == cvx!("133") || dose.cvx.0 == cvx!("152") {
            DoseStatus::Accepted
        } else {
            DoseStatus::Invalid
        };
        reasons.clear();
        reasons.push(if *status == DoseStatus::Accepted {
            EvaluationReason::OutsideRoutineSeries
        } else {
            EvaluationReason::VaccineNotPartOfSeries
        });
    }
}

pub fn pneumococcal_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _evaluations: &[crate::models::DoseEvaluation],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let birth = patient.birth_date;
    let adult_valid = valid_adult_doses(valid_doses, history);
    let has_valid_child_modern_pcv = has_valid_child_modern_pcv(valid_doses, history);
    
    let valid_pcv_doses: Vec<NaiveDate> = valid_doses.iter()
        .filter(|(d, _)| history.iter().any(|h| h.date == *d && is_pcv_cvx(h.cvx)))
        .map(|(d, _)| *d)
        .collect();
    
    let pcv_complete = if valid_pcv_doses.is_empty() { false } else {
        let first_date = valid_pcv_doses[0];
        let n = valid_pcv_doses.len();
        if age_ge(birth, first_date, crate::time_period!("24m")) { n >= 1 }
        else if age_ge(birth, first_date, crate::time_period!("12m")) { n >= 2 }
        else if age_ge(birth, first_date, crate::time_period!("7m")) { n >= 3 }
        else { n >= 4 }
    };

    if adult_valid.iter().any(|d| ADULT_COMPLETE_PCV_CVX.contains(&d.cvx.0) && age_ge(birth, d.date, crate::time_period!("19y"))) {
        forecast.status = SeriesStatus::Complete;
        return;
    }

    if age_ge(birth, eval_date, crate::time_period!("19y")) && age_lt(birth, eval_date, crate::time_period!("50y")) {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        return;
    }

    if age_ge(birth, eval_date, crate::time_period!("2y-4d")) && age_lt(birth, eval_date, crate::time_period!("19y")) {
        let is_high_risk = history.iter().any(|d| matches!(d.cvx.0, cvx!("327") | cvx!("216")));
        if pcv_complete && is_high_risk {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            let date_2y = crate::time_period!("2y").add_to(birth);
            let last_pcv = history.iter().filter(|d| is_pcv_cvx(d.cvx)).map(|d| d.date).max();
            let mut earliest = date_2y;
            if let Some(lp) = last_pcv {
                let spacing = lp + chrono::Duration::days(56);
                if spacing > earliest { earliest = spacing; }
            }
            forecast.status = forecast.status.with_earliest_date(Some(earliest));
            forecast.status = forecast.status.with_recommended_date(Some(earliest));
            forecast.status = forecast.status.with_overdue_date(Some(earliest));
            return;
        }
    }

    if pcv_complete && age_lt(birth, eval_date, crate::time_period!("5y")) && has_valid_child_modern_pcv {
        forecast.status = SeriesStatus::Complete;
        return;
    }

    if forecast.status == SeriesStatus::Complete { return; }

    if forecast.status.recommended_date().is_none() {
        let ref_date = if age_ge(birth, eval_date, crate::time_period!("24m")) { crate::time_period!("2y").add_to(birth) } 
        else if age_ge(birth, eval_date, crate::time_period!("12m")) { crate::time_period!("1y").add_to(birth) } 
        else if age_ge(birth, eval_date, crate::time_period!("7m")) { crate::time_period!("7m").add_to(birth) } 
        else { eval_date };
        forecast.status = forecast.status.with_recommended_date(Some(ref_date));
    }

    if let Some(recommended) = forecast.status.recommended_date() {
        if forecast.status.earliest_date().map_or(true, |e| e > recommended) {
            forecast.status = forecast.status.with_earliest_date(Some(recommended));
        }
    }
    
    // Final Clamp and Alignment for children
    if age_lt(birth, eval_date, crate::time_period!("19y")) {
        let date_2y = crate::time_period!("2y").add_to(birth);
        let date_1y = crate::time_period!("1y").add_to(birth);
        let date_7m = crate::time_period!("7m").add_to(birth);
        
        if valid_pcv_doses.is_empty() {
            if age_ge(birth, eval_date, crate::time_period!("24m")) { clamp_forecast(forecast, date_2y); } 
            else if age_ge(birth, eval_date, crate::time_period!("12m")) { clamp_forecast(forecast, date_1y); } 
            else if age_ge(birth, eval_date, crate::time_period!("7m")) { clamp_forecast(forecast, date_7m); }
        }
        
        if let Some(rec) = forecast.status.recommended_date() {
            forecast.status = forecast.status.with_overdue_date(Some(rec));
        }
    }
}

fn clamp_forecast(forecast: &mut SeriesForecast, date: NaiveDate) {
    forecast.status = forecast.status.with_earliest_date(Some(date));
    forecast.status = forecast.status.with_recommended_date(Some(date));
    forecast.status = forecast.status.with_overdue_date(Some(date));
}

pub struct PneumococcalPolicy;

impl crate::engine::EvaluationPolicy for PneumococcalPolicy {
    fn custom_forecast_hook(
        &self,
        patient: &Patient,
        valid_doses: &[(NaiveDate, usize)],
    _evaluations: &[crate::models::DoseEvaluation],
        history: &[Dose],
        eval_date: NaiveDate,
        forecast: &mut SeriesForecast,
    ) {
        pneumococcal_custom_forecast_hook(patient, valid_doses, _evaluations, history, eval_date, forecast)
    }

    fn custom_evaluation_hook(
        &self,
        series_name: &str,
        target_dose_idx: usize,
        ctx: &EvaluationContext,
        reasons: &mut SmallVec<[EvaluationReason; 4]>,
        status: &mut DoseStatus,
    ) {
        pneumococcal_custom_evaluation_hook(series_name, target_dose_idx, ctx, reasons, status)
    }

    fn custom_dose_number_hook(
        &self,
        series_name: &str,
        ctx: &EvaluationContext,
    ) -> Option<usize> {
        Some(pneumococcal_custom_dose_number_hook(series_name, ctx))
    }

    fn custom_completion_hook(&self, ctx: &EvaluationContext) -> Option<bool> {
        Some(pneumococcal_custom_completion_hook(ctx))
    }

    fn ignore_evaluation_for_maximum_date(&self, patient: &Patient, eval: &crate::models::DoseEvaluation) -> bool {
        eval.cvx.0 == cvx!("33")
            && compare_elapsed(
                patient.birth_date,
                eval.dose_date,
                &crate::time_period!("2y"),
            ) == std::cmp::Ordering::Less
    }
}
