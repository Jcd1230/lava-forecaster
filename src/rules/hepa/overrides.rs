use crate::date_utils::SmallVec;
use crate::engine::CandidateForecastsExt;
use crate::engine::EvaluationContext;
use crate::engine::ValidDoseRef;
use crate::models::{
    Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus, VaccineGroupForecast,
};
use crate::rules::helpers::{age_ge, clamp_date_at_least, interval_ge, interval_lt};
use chrono::NaiveDate;
use lava_cvx_macro::cvx;

pub struct HepaPolicy;

impl crate::engine::EvaluationPolicy for HepaPolicy {
    fn same_day_priority(
        &self,
        dose: &Dose,
        context: &crate::engine::SameDayPriorityContext,
    ) -> i32 {
        let age_19 = crate::date_utils::add_years_unchecked(context.patient.birth_date, 19);
        if dose.date >= age_19 {
            if dose.cvx.0 == 52 {
                0
            } else {
                1
            }
        } else if dose.cvx.0 == 52 {
            1
        } else {
            0
        }
    }
}

pub fn hepa_custom_switch_hook(
    current_series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
) -> Option<&'static str> {
    if current_series_name != "HEP_A_2_DOSE_CHILD_ADULT_SERIES" {
        return None;
    }
    if let Some(dose) = ctx.current_dose {
        if dose.cvx.0 == cvx!("104") {
            let birth_date = ctx.patient.birth_date;
            if target_dose_idx == 1 {
                if age_ge(birth_date, dose.date, crate::time_period!("19y")) {
                    return Some("HEP_A_ADULT_3_DOSE_SERIES");
                }
            } else if target_dose_idx == 2 {
                if let Some(prev) = ctx.valid_doses.last() {
                    let prev_date = prev.date;
                    // Case A: age at dose 2 >= 19y and interval >= 24d
                    if age_ge(birth_date, dose.date, crate::time_period!("19y"))
                        && interval_ge(prev_date, dose.date, crate::time_period!("24d"))
                    {
                        return Some("HEP_A_ADULT_3_DOSE_SERIES");
                    }

                    // Case B: Dose 1 is CVX 104 at >= 18y-4d, and interval is >= 24d and < 6m-4d
                    if prev.cvx.0 == cvx!("104")
                        && age_ge(birth_date, prev.date, crate::time_period!("18y-4d"))
                        && interval_ge(prev_date, dose.date, crate::time_period!("24d"))
                        && interval_lt(prev_date, dose.date, crate::time_period!("6m-4d"))
                    {
                        return Some("HEP_A_ADULT_3_DOSE_SERIES");
                    }
                }
            }
        }
    }
    None
}

pub fn hepa_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        if series_name == "HEP_A_ADULT_3_DOSE_SERIES" && target_dose_idx == 3 {
            if ctx.valid_doses.len() >= 2 {
                let dose_1_date = ctx.valid_doses[0].date;
                if interval_lt(dose_1_date, dose.date, crate::time_period!("6m-4d")) {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        } else if series_name == "HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES" && target_dose_idx == 4 {
            if ctx.valid_doses.len() >= 3 {
                let dose_1_date = ctx.valid_doses[0].date;
                if interval_lt(dose_1_date, dose.date, crate::time_period!("12m-4d")) {
                    *status = DoseStatus::Invalid;
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        } else if series_name == "HEP_A_2_DOSE_CHILD_ADULT_SERIES" && target_dose_idx == 2 {
            if let Some(dose_1) = ctx.valid_doses.first() {
                let dose_1_date = dose_1.date;
                if interval_ge(dose_1_date, dose.date, crate::time_period!("6m-4d")) {
                    if let Some(pos) = reasons
                        .iter()
                        .position(|r| *r == EvaluationReason::BelowMinimumInterval)
                    {
                        reasons.remove(pos);
                    }
                    let has_other_invalid = reasons.iter().any(|r| {
                        *r == EvaluationReason::BelowMinimumAge
                            || *r == EvaluationReason::VaccineNotPartOfSeries
                    });
                    if !has_other_invalid {
                        *status = DoseStatus::Valid;
                    }
                }
            }
        }
    }
}

pub fn hepa_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[ValidDoseRef],
    _evaluations: &[crate::models::DoseEvaluation],
    _history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if valid_doses.is_empty() {
        if age_ge(patient.birth_date, eval_date, crate::time_period!("19y")) {
            forecast.status = SeriesStatus::ConditionallyRecommended;
            let age_2 = crate::date_utils::add_years_unchecked(patient.birth_date, 2);
            forecast.status = forecast.status.with_recommended_date(Some(age_2));
            forecast.status = forecast.status.with_earliest_date(Some(age_2));
            forecast.status = forecast.status.with_overdue_date(None);
        }
    } else {
        if forecast.series_name == "HEP_A_ADULT_3_DOSE_SERIES" && valid_doses.len() == 2 {
            let dose_1_date = valid_doses[0].date;
            let rec_date = crate::date_utils::add_months_unchecked(dose_1_date, 6);
            if let Some(d) = forecast.status.earliest_date_mut() {
                clamp_date_at_least(d, rec_date);
            }
            if let Some(d) = forecast.status.recommended_date_mut() {
                clamp_date_at_least(d, rec_date);
            }
        } else if forecast.series_name == "HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES"
            && valid_doses.len() == 3
        {
            let dose_1_date = valid_doses[0].date;
            let rec_date = crate::date_utils::add_months_unchecked(dose_1_date, 12);
            if let Some(d) = forecast.status.earliest_date_mut() {
                clamp_date_at_least(d, rec_date);
            }
            if let Some(d) = forecast.status.recommended_date_mut() {
                clamp_date_at_least(d, rec_date);
            }
        }
    }
}

fn is_complete(forecast: &VaccineGroupForecast) -> bool {
    forecast
        .forecasts
        .first()
        .map(|f| f.status == SeriesStatus::Complete)
        .unwrap_or(false)
}

fn last_valid_dose_date(forecast: &VaccineGroupForecast) -> Option<NaiveDate> {
    forecast
        .evaluations
        .iter()
        .filter(|e| e.status == DoseStatus::Valid)
        .last()
        .map(|e| e.dose_date)
}

fn num_valid_doses(forecast: &VaccineGroupForecast) -> usize {
    forecast
        .evaluations
        .iter()
        .filter(|e| e.status == DoseStatus::Valid)
        .count()
}

pub fn hepa_group_selection(
    patient: &Patient,
    _history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let mut selected = "HEP_A_2_DOSE_CHILD_ADULT_SERIES";

    let forecast_2 = candidate_forecasts
        .get_forecast("HEP_A_2_DOSE_CHILD_ADULT_SERIES")
        .unwrap();
    let first_valid_dose_2 = forecast_2
        .evaluations
        .iter()
        .filter(|e| e.status == DoseStatus::Valid)
        .min_by_key(|e| e.dose_date);

    if let Some(fd2) = first_valid_dose_2 {
        let birth_date = patient.birth_date;

        if age_ge(birth_date, fd2.dose_date, crate::time_period!("18y-4d")) {
            // Patient >= 18y-4d at Dose 1. Default initially is 2-dose.

            // Check Override 1: Select 4-dose Accelerated Twinrix series
            let forecast_4 = candidate_forecasts
                .get_forecast("HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES")
                .unwrap();
            let valid_doses_4: Vec<&crate::models::DoseEvaluation> = forecast_4
                .evaluations
                .iter()
                .filter(|e| e.status == DoseStatus::Valid)
                .collect();

            let mut twinrix_selected = false;
            if valid_doses_4.len() >= 2 {
                let d1 = valid_doses_4[0];
                let d2 = valid_doses_4[1];

                // No prior valid doses before d1 (this is implicitly true because d1 is index 0)
                if d1.cvx.0 == cvx!("104")
                    && d2.cvx.0 == cvx!("104")
                    && interval_ge(d1.dose_date, d2.dose_date, crate::time_period!("7d"))
                    && interval_lt(d1.dose_date, d2.dose_date, crate::time_period!("24d"))
                {
                    selected = "HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES";
                    twinrix_selected = true;
                }
            }

            // Check Override 2: Select 3-dose series (if Twinrix accelerated not selected)
            if !twinrix_selected {
                let forecast_3 = candidate_forecasts
                    .get_forecast("HEP_A_ADULT_3_DOSE_SERIES")
                    .unwrap();
                let valid_doses_3: Vec<&crate::models::DoseEvaluation> = forecast_3
                    .evaluations
                    .iter()
                    .filter(|e| e.status == DoseStatus::Valid)
                    .collect();

                if !valid_doses_3.is_empty() {
                    let d1 = valid_doses_3[0];
                    if d1.cvx.0 == cvx!("104") {
                        if valid_doses_3.len() == 1 {
                            selected = "HEP_A_ADULT_3_DOSE_SERIES";
                        } else {
                            let d2 = valid_doses_3[1];

                            if (d1.cvx.0 == cvx!("104") || d2.cvx.0 == cvx!("104"))
                                && interval_ge(
                                    d1.dose_date,
                                    d2.dose_date,
                                    crate::time_period!("24d"),
                                )
                                && interval_lt(
                                    d1.dose_date,
                                    d2.dose_date,
                                    crate::time_period!("6m-4d"),
                                )
                            {
                                selected = "HEP_A_ADULT_3_DOSE_SERIES";
                            }
                        }
                    }
                }
            }

            // Check Override 3: If 4-dose is selected, check if we revert to 3-dose
            if selected == "HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES" {
                let forecast_3 = candidate_forecasts
                    .get_forecast("HEP_A_ADULT_3_DOSE_SERIES")
                    .unwrap();
                let forecast_4 = candidate_forecasts
                    .get_forecast("HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES")
                    .unwrap();
                let v3 = num_valid_doses(forecast_3);
                let v4 = num_valid_doses(forecast_4);

                if v3 == 3 && v4 < 4 {
                    selected = "HEP_A_ADULT_3_DOSE_SERIES";
                } else if v4 < 4 {
                    if let Some(latest_twinrix_date) = last_valid_dose_date(forecast_4) {
                        let has_later_3dose_valid = forecast_3.evaluations.iter().any(|e| {
                            e.status == DoseStatus::Valid && e.dose_date > latest_twinrix_date
                        });
                        if has_later_3dose_valid && (3 - v3) < (4 - v4) {
                            selected = "HEP_A_ADULT_3_DOSE_SERIES";
                        }
                    }
                }
            }

            // Check Override 4: Revert to 2-dose complete series if complete and either the selected is not, or 2-dose completed before it
            if selected == "HEP_A_ADULT_3_DOSE_SERIES"
                || selected == "HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES"
            {
                let forecast_2 = candidate_forecasts
                    .get_forecast("HEP_A_2_DOSE_CHILD_ADULT_SERIES")
                    .unwrap();
                if is_complete(forecast_2) {
                    let forecast_sel = candidate_forecasts.get_forecast(&selected).unwrap();
                    if !is_complete(forecast_sel) {
                        selected = "HEP_A_2_DOSE_CHILD_ADULT_SERIES";
                    } else {
                        let date_2 = last_valid_dose_date(forecast_2).unwrap();
                        let date_sel = last_valid_dose_date(forecast_sel).unwrap();
                        if date_2 < date_sel {
                            selected = "HEP_A_2_DOSE_CHILD_ADULT_SERIES";
                        }
                    }
                }
            }
        }
    }

    // Post-Process status modifications
    if selected == "HEP_A_ADULT_3_DOSE_SERIES" {
        // Find doses Valid in 4-dose, and if they are Invalid in 3-dose, mark Accepted
        let valid_dates_4: Vec<NaiveDate> = candidate_forecasts
            .get_forecast("HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES")
            .unwrap()
            .evaluations
            .iter()
            .filter(|e| e.status == DoseStatus::Valid)
            .map(|e| e.dose_date)
            .collect();

        if let Some(forecast_3) = candidate_forecasts.get_forecast_mut("HEP_A_ADULT_3_DOSE_SERIES")
        {
            for eval in forecast_3.evaluations.iter_mut() {
                if eval.status == DoseStatus::Invalid && valid_dates_4.contains(&eval.dose_date) {
                    eval.status = DoseStatus::Accepted;
                    eval.reasons = crate::reasons![
                        EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven
                    ];
                }
            }
        }
    } else if selected == "HEP_A_2_DOSE_CHILD_ADULT_SERIES" {
        // Only Twinrix-valid alternate-series doses are accepted when the 2-dose series stays selected.
        let mut valid_dates_other = Vec::new();
        if let Some(f3) = candidate_forecasts.get_forecast("HEP_A_ADULT_3_DOSE_SERIES") {
            for e in f3.evaluations.iter() {
                if e.status == DoseStatus::Valid && e.cvx.0 == cvx!("104") {
                    valid_dates_other.push(e.dose_date);
                }
            }
        }
        if let Some(f4) =
            candidate_forecasts.get_forecast("HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES")
        {
            for e in f4.evaluations.iter() {
                if e.status == DoseStatus::Valid && e.cvx.0 == cvx!("104") {
                    valid_dates_other.push(e.dose_date);
                }
            }
        }

        if let Some(forecast_2) =
            candidate_forecasts.get_forecast_mut("HEP_A_2_DOSE_CHILD_ADULT_SERIES")
        {
            for eval in forecast_2.evaluations.iter_mut() {
                if eval.status == DoseStatus::Invalid && valid_dates_other.contains(&eval.dose_date)
                {
                    eval.status = DoseStatus::Accepted;
                    eval.reasons = crate::reasons![
                        EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven
                    ];
                }
            }
        }
    }

    selected
}
