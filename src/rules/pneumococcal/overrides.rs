use lava_cvx_macro::cvx;
use chrono::NaiveDate;

use crate::date_utils::{SmallVec, compare_elapsed, TimePeriod};
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus};

const CHILD_PCV_CVX: &[u16] = &[cvx!("100"), cvx!("133"), cvx!("177"), cvx!("215"), cvx!("216"), cvx!("109"), cvx!("152")];
const MODERN_PCV_CVX: &[u16] = &[cvx!("133"), cvx!("215"), cvx!("216"), cvx!("327")];
const ADULT_PCV_CVX: &[u16] = &[cvx!("133"), cvx!("215"), cvx!("216"), cvx!("327")];
const ADULT_COMPLETE_PCV_CVX: &[u16] = &[cvx!("216"), cvx!("327")];

fn is_pneumo_cvx(cvx: Cvx) -> bool {
    matches!(cvx.0, cvx!("33") | cvx!("100") | cvx!("109") | cvx!("133") | cvx!("152") | cvx!("177") | cvx!("215") | cvx!("216") | cvx!("327"))
}

fn age_ge(birth: NaiveDate, date: NaiveDate, age: TimePeriod) -> bool {
    compare_elapsed(birth, date, &age) != std::cmp::Ordering::Less
}

fn age_lt(birth: NaiveDate, date: NaiveDate, age: TimePeriod) -> bool {
    compare_elapsed(birth, date, &age) == std::cmp::Ordering::Less
}

fn count_history_before_age(history: &[Dose], birth: NaiveDate, age: TimePeriod) -> usize {
    let cutoff = age.add_to(birth);
    history
        .iter()
        .filter(|d| is_pneumo_cvx(d.cvx) && d.date < cutoff)
        .count()
}

fn has_valid_modern_pcv(valid_doses: &[(NaiveDate, usize)], history: &[Dose]) -> bool {
    valid_doses.iter().any(|(date, _)| {
        history
            .iter()
            .any(|d| d.date == *date && MODERN_PCV_CVX.contains(&d.cvx.0))
    })
}

fn valid_adult_doses<'a>(valid_doses: &[(NaiveDate, usize)], history: &'a [Dose]) -> Vec<&'a Dose> {
    valid_doses
        .iter()
        .filter(|(_, dose_num)| *dose_num >= 6)
        .filter_map(|(date, _)| {
            history
                .iter()
                .find(|d| d.date == *date && is_pneumo_cvx(d.cvx))
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
    let mut target = ctx.target_dose_number;

    if age_ge(birth, ref_date, crate::time_period!("5y")) {
        return target.max(6);
    }

    if age_ge(birth, ref_date, crate::time_period!("24m")) {
        return target.max(4);
    }

    if age_ge(birth, ref_date, crate::time_period!("12m")) && age_lt(birth, ref_date, crate::time_period!("24m")) {
        let before_12m = count_history_before_age(ctx.history, birth, crate::time_period!("12m"));
        if before_12m >= 2 {
            target = target.max(4);
        } else {
            target = target.max(3);
        }
    } else if age_ge(birth, ref_date, crate::time_period!("7m")) && age_lt(birth, ref_date, crate::time_period!("12m")) {
        let before_7m = count_history_before_age(ctx.history, birth, crate::time_period!("7m"));
        if before_7m >= 1 && target == 2 {
            target = 3;
        } else {
            target = target.max(2);
        }
    }

    target
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

        if target_dose_idx > 4 && age_lt(birth, dose.date, crate::time_period!("5y")) {
            let has_no_modern_pcv = !has_valid_modern_pcv(ctx.valid_doses, ctx.history);
            if has_no_modern_pcv && MODERN_PCV_CVX.contains(&dose.cvx.0) {
                if let Some((prev_date, _)) = ctx.valid_doses.last() {
                    if compare_elapsed(*prev_date, dose.date, &crate::time_period!("52d"))
                        == std::cmp::Ordering::Less
                    {
                        *status = DoseStatus::Invalid;
                        reasons.clear();
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    } else {
                        *status = DoseStatus::Valid;
                        reasons.clear();
                    }
                }
                return;
            }

            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::BoosterDose);
            return;
        }

        if target_dose_idx == 4 && age_lt(birth, dose.date, crate::time_period!("1y-4d")) {
            let before_7m = count_history_before_age(ctx.history, birth, crate::time_period!("7m"));
            if before_7m == 0 {
                *status = DoseStatus::Invalid;
                if !reasons.contains(&EvaluationReason::BelowMinimumAgeFinalDose) {
                    reasons.push(EvaluationReason::BelowMinimumAgeFinalDose);
                }
            }
        }

        if *status == DoseStatus::Valid {
            let last_valid_date = ctx.valid_doses.last().map(|(date, _)| *date);
            let last_retry_attempt = ctx
                .history
                .iter()
                .filter(|prior| is_pneumo_cvx(prior.cvx) && prior.date < dose.date)
                .filter(|prior| last_valid_date.is_some_and(|last_valid| prior.date > last_valid))
                .map(|prior| prior.date)
                .max();

            if let Some(last_retry_attempt) = last_retry_attempt {
                if compare_elapsed(last_retry_attempt, dose.date, &crate::time_period!("52d"))
                    == std::cmp::Ordering::Less
                {
                    *status = DoseStatus::Invalid;
                    reasons.clear();
                    reasons.push(EvaluationReason::BelowMinimumInterval);
                }
            }
        }

        return;
    }

    if dose.cvx.0 == cvx!("100") && age_ge(birth, dose.date, crate::time_period!("5y")) {
        *status = if age_ge(birth, dose.date, crate::time_period!("19y")) {
            DoseStatus::Ignored
        } else {
            DoseStatus::Accepted
        };
        reasons.clear();
        reasons.push(EvaluationReason::VaccineNotPartOfSeries);
        return;
    }

    if (dose.cvx.0 == cvx!("109") || dose.cvx.0 == cvx!("152")) && age_ge(birth, dose.date, crate::time_period!("19y")) {
        *status = DoseStatus::Invalid;
        if !reasons.contains(&EvaluationReason::VaccineNotPartOfSeries) {
            reasons.push(EvaluationReason::VaccineNotPartOfSeries);
        }
        return;
    }

    if age_ge(birth, dose.date, crate::time_period!("5y")) && age_lt(birth, dose.date, crate::time_period!("19y")) {
        *status = DoseStatus::Accepted;
        reasons.clear();
        reasons.push(EvaluationReason::OutsideRoutineSeries);
        return;
    }

    if age_ge(birth, dose.date, crate::time_period!("19y"))
        && *status == DoseStatus::Invalid
        && !matches!(dose.cvx.0, cvx!("109") | cvx!("152"))
    {
        *status = DoseStatus::Accepted;
        reasons.clear();
        reasons.push(EvaluationReason::OutsideRoutineSeries);
        return;
    }

    if age_ge(birth, dose.date, crate::time_period!("19y")) && matches!(dose.cvx.0, cvx!("215") | cvx!("216") | cvx!("327")) {
        let adult_valid = valid_adult_doses(ctx.valid_doses, ctx.history);
        let has_prior_pcv13 = adult_valid
            .iter()
            .any(|prior| prior.date < dose.date && prior.cvx.0 == cvx!("133"));
        let has_prior_ppsv23 = adult_valid
            .iter()
            .any(|prior| prior.date < dose.date && prior.cvx.0 == cvx!("33"));
        if has_prior_pcv13 && has_prior_ppsv23 {
            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::OutsideRoutineSeries);
            return;
        }
    }

    if target_dose_idx == 7 {
        let adult_valid = valid_adult_doses(ctx.valid_doses, ctx.history);
        if let Some(first) = adult_valid.first() {
            let allowed = if first.cvx.0 == cvx!("33") || first.cvx.0 == cvx!("216") || first.cvx.0 == cvx!("327") {
                ADULT_PCV_CVX.contains(&dose.cvx.0)
            } else {
                matches!(dose.cvx.0, cvx!("33") | cvx!("216") | cvx!("327"))
            };
            if !allowed {
                *status = DoseStatus::Invalid;
                reasons.clear();
                reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            }
        }
    }

    if target_dose_idx == 8 && !matches!(dose.cvx.0, cvx!("33") | cvx!("216") | cvx!("327")) {
        *status = if dose.cvx.0 == cvx!("133") {
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
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let birth = patient.birth_date;
    let adult_valid = valid_adult_doses(valid_doses, history);
    let has_valid_child_modern_pcv = has_valid_child_modern_pcv(valid_doses, history);
    let child_complete = valid_doses
        .iter()
        .any(|(date, dose_num)| *dose_num == 4 && age_lt(birth, *date, crate::time_period!("5y")));
    let has_adult_pcv13 = adult_valid.iter().any(|d| d.cvx.0 == cvx!("133"));
    let has_adult_modern_pcv = adult_valid
        .iter()
        .any(|d| matches!(d.cvx.0, cvx!("215") | cvx!("216") | cvx!("327")));
    let has_adult_ppsv23 = adult_valid.iter().any(|d| d.cvx.0 == cvx!("33"));

    if adult_valid
        .iter()
        .any(|d| ADULT_COMPLETE_PCV_CVX.contains(&d.cvx.0) && age_ge(birth, d.date, crate::time_period!("19y")))
    {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = crate::reasons!["COMPLETE"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        return;
    }

    let has_adult_pcv15 = adult_valid
        .iter()
        .any(|d| d.cvx.0 == cvx!("215") && age_ge(birth, d.date, crate::time_period!("19y")));
    if has_adult_pcv15 && has_adult_ppsv23 {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = crate::reasons!["COMPLETE"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        return;
    }

    let has_ppsv65 = adult_valid
        .iter()
        .any(|d| d.cvx.0 == cvx!("33") && age_ge(birth, d.date, crate::time_period!("65y")));

    if age_ge(birth, eval_date, crate::time_period!("65y")) && has_adult_pcv13 && has_ppsv65 && !has_adult_modern_pcv {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons!["COMPLETE"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        return;
    }

    if forecast.status == SeriesStatus::Complete {
        return;
    }

    if valid_doses.is_empty()
        && history
            .iter()
            .max_by_key(|d| d.date)
            .map(|d| d.cvx.0)
            == Some(cvx!("33"))
        && age_lt(birth, eval_date, crate::time_period!("2y"))
    {
        let earliest = crate::time_period!("42d").add_to(birth);
        let recommended = crate::time_period!("2m").add_to(birth).max(earliest);
        let overdue = crate::time_period!("3m+4w").add_to(birth).pred_opt();
        forecast.status = forecast.status.with_earliest_date(Some(earliest));
        forecast.status = forecast.status.with_recommended_date(Some(recommended));
        forecast.status = forecast.status.with_overdue_date(overdue);
        return;
    }

    if age_ge(birth, eval_date, crate::time_period!("5y")) && age_lt(birth, eval_date, crate::time_period!("19y")) {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons![if child_complete {
            "COMPLETE_HIGH_RISK"
        } else {
            "HIGH_RISK"
        }];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        return;
    }

    if age_ge(birth, eval_date, crate::time_period!("19y")) && age_lt(birth, eval_date, crate::time_period!("50y")) {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons!["HIGH_RISK"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        return;
    }

    if child_complete
        && age_lt(birth, eval_date, crate::time_period!("5y"))
        && has_valid_child_modern_pcv
    {
        forecast.status = SeriesStatus::Complete;
        forecast.reasons = crate::reasons!["COMPLETE_HIGH_RISK"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        return;
    }

    if age_lt(birth, eval_date, crate::time_period!("5y"))
        && !has_valid_child_modern_pcv
        && matches!(forecast.status, SeriesStatus::NotComplete { .. })
        && forecast.status.recommended_date().is_some()
        && history.last().is_some_and(|dose| {
            is_pneumo_cvx(dose.cvx)
                && (age_ge(birth, dose.date, crate::time_period!("24m")) || child_complete)
                && age_lt(birth, crate::time_period!("8w").add_to(dose.date), crate::time_period!("5y"))
        })
    {
        forecast.status = forecast.status.with_earliest_date(None);
    }

    let last_ppsv_or_unspecified = history
        .iter()
        .filter(|d| d.cvx.0 == cvx!("33") || d.cvx.0 == cvx!("109"))
        .map(|d| d.date)
        .max();
    if valid_doses.iter().map(|(_, n)| *n).max().unwrap_or(1) >= 6 {
        if let Some(last) = last_ppsv_or_unspecified {
            if has_adult_pcv13 || has_adult_pcv15 || has_adult_modern_pcv {
                let date = crate::time_period!("5y").add_to(last);
                if matches!(forecast.status.recommended_date(), Some(current) if date > current) {
                    forecast.status = forecast.status.with_recommended_date(Some(date));
                }
                if matches!(forecast.status.earliest_date(), Some(current) if date > current) {
                    forecast.status = forecast.status.with_earliest_date(Some(date));
                }
            }
        } else if let Some(last_pcv) = history
            .iter()
            .filter(|d| ADULT_PCV_CVX.contains(&d.cvx.0) || d.cvx.0 == cvx!("152"))
            .map(|d| d.date)
            .max()
        {
            let date = crate::time_period!("1y").add_to(last_pcv);
            if matches!(forecast.status.recommended_date(), Some(current) if date > current) {
                forecast.status = forecast.status.with_recommended_date(Some(date));
            }
            if matches!(forecast.status.earliest_date(), Some(current) if date > current) {
                forecast.status = forecast.status.with_earliest_date(Some(date));
            }
        }
    }

    if forecast.status.recommended_date().is_none() {
        let ref_date = if age_ge(birth, eval_date, crate::time_period!("5y")) {
            crate::time_period!("50y").add_to(birth)
        } else if age_ge(birth, eval_date, crate::time_period!("24m")) {
            eval_date
        } else {
            forecast.status.recommended_date().unwrap_or(eval_date)
        };
        forecast.status = forecast.status.with_recommended_date(Some(ref_date));
        forecast.status.earliest_date().get_or_insert(ref_date);
    }
}

pub struct PneumococcalPolicy;

impl crate::engine::EvaluationPolicy for PneumococcalPolicy {
    fn custom_forecast_hook(
        &self,
        patient: &Patient,
        valid_doses: &[(NaiveDate, usize)],
        history: &[Dose],
        eval_date: NaiveDate,
        forecast: &mut SeriesForecast,
    ) {
        pneumococcal_custom_forecast_hook(patient, valid_doses, history, eval_date, forecast)
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

    fn ignore_evaluation_for_maximum_date(&self, patient: &Patient, eval: &crate::models::DoseEvaluation) -> bool {
        eval.cvx.0 == cvx!("33")
            && compare_elapsed(
                patient.birth_date,
                eval.dose_date,
                &crate::time_period!("2y"),
            ) == std::cmp::Ordering::Less
    }
}
