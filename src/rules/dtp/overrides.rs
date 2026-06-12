use crate::date_utils::{add_months_unchecked, add_years_unchecked, SmallVec};
use crate::engine::EvaluationContext;
use crate::models::{
    Cvx, Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, VaccineGroupForecast,
};
use chrono::NaiveDate;
use lava_cvx_macro::cvx;

pub fn is_pertussis_vaccine(cvx: Cvx) -> bool {
    // DT/Td (tetanus/diphtheria only, no pertussis) CVX codes
    const DT_TD_CVX: &[u16] = &[
        cvx!("09"),
        cvx!("28"),
        cvx!("113"),
        cvx!("138"),
        cvx!("139"),
        cvx!("195"),
        cvx!("196"),
    ];
    !DT_TD_CVX.contains(&cvx.0)
}

fn is_adolescent_tdap_completed(ctx: &EvaluationContext) -> bool {
    ctx.valid_doses.iter().any(|(v_date, dose_number)| {
        let post_primary = match ctx.active_series_name {
            "DTP_3_DOSE_SERIES" => *dose_number > 3,
            "DTP_5_DOSE_SERIES" => *dose_number >= 5,
            _ => false,
        };
        let contains_pertussis = ctx
            .history
            .iter()
            .any(|d| d.date == *v_date && is_pertussis_vaccine(d.cvx));
        let age_ge_7 = *v_date >= add_years_unchecked(ctx.patient.birth_date, 7);
        post_primary && contains_pertussis && age_ge_7
    })
}

fn get_last_pertussis_date_before(ctx: &EvaluationContext, date: NaiveDate) -> Option<NaiveDate> {
    ctx.history
        .iter()
        .filter(|d| d.date < date && is_pertussis_vaccine(d.cvx))
        .map(|d| d.date)
        .max()
}

pub fn dtp_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    if series_name != "DTP_5_DOSE_SERIES" && series_name != "DTP_3_DOSE_SERIES" {
        return;
    }
    if let Some(dose) = ctx.current_dose {
        let birth_date = ctx.patient.birth_date;
        let age_7_minus_4d = add_years_unchecked(birth_date, 7) - chrono::Duration::days(4);

        if dose.date >= age_7_minus_4d
            && reasons.contains(&EvaluationReason::DuplicateShotSameDay)
            && is_td_family(dose.cvx)
        {
            if let Some((extra_status, extra_reasons)) = dtp_custom_extra_dose_hook(series_name, ctx)
            {
                *status = extra_status;
                *reasons = extra_reasons;
                return;
            }
        }

        let is_adolescent_tdap = dose.cvx.0 == cvx!("115");

        if dose.date < age_7_minus_4d {
            if is_adolescent_tdap && series_name == "DTP_5_DOSE_SERIES" && target_dose_idx <= 3 {
                *status = DoseStatus::Invalid;
                reasons.retain(|r| *r != EvaluationReason::BelowMinimumAge);
                if !reasons.contains(&EvaluationReason::InsufficientAntigen) {
                    reasons.push(EvaluationReason::InsufficientAntigen);
                }
            } else if is_td_min_age_invalid(dose.cvx) {
                *status = DoseStatus::Invalid;
                reasons.retain(|r| *r != EvaluationReason::BelowMinimumAge);
                if !reasons.contains(&EvaluationReason::BelowMinimumAge) {
                    reasons.push(EvaluationReason::BelowMinimumAge);
                }
            }
        } else if dose.cvx.0 == cvx!("113") {
            if let Some(prev_pertussis_date) = get_last_pertussis_date_before(ctx, dose.date) {
                if dose.date < prev_pertussis_date + chrono::Duration::days(28) {
                    *status = DoseStatus::Invalid;
                    reasons.retain(|r| *r != EvaluationReason::BelowMinimumAge);
                    if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                        reasons.push(EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }
    }
}

fn is_td_family(cvx: Cvx) -> bool {
    cvx.0 == cvx!("09")
        || cvx.0 == cvx!("28")
        || cvx.0 == cvx!("113")
        || cvx.0 == cvx!("138")
        || cvx.0 == cvx!("139")
        || cvx.0 == cvx!("195")
        || cvx.0 == cvx!("196")
}

fn is_td_min_age_invalid(cvx: Cvx) -> bool {
    cvx.0 == cvx!("09")
        || cvx.0 == cvx!("113")
        || cvx.0 == cvx!("138")
        || cvx.0 == cvx!("139")
        || cvx.0 == cvx!("196")
}

fn is_first_adult_td_anchor_cvx(cvx: Cvx) -> bool {
    cvx.0 == cvx!("138") || cvx.0 == cvx!("139")
}

fn has_prior_post_primary_td_family(ctx: &EvaluationContext, series_name: &str) -> bool {
    let primary_doses = match series_name {
        "DTP_3_DOSE_SERIES" => 3,
        "DTP_5_DOSE_SERIES" => 5,
        _ => return false,
    };
    let current_date = match ctx.current_dose {
        Some(dose) => dose.date,
        None => return false,
    };
    let last_primary_date = ctx
        .valid_doses
        .iter()
        .filter(|(_, dose_number)| *dose_number <= primary_doses)
        .map(|(date, _)| *date)
        .max();

    let has_valid_post_primary_td = ctx.valid_doses.iter().any(|(date, dose_number)| {
        *dose_number > primary_doses
            && ctx
                .history
                .iter()
                .any(|d| d.date == *date && is_td_family(d.cvx))
    });
    if has_valid_post_primary_td {
        return true;
    }

    match last_primary_date {
        Some(primary_date) => ctx
            .history
            .iter()
            .any(|d| d.date > primary_date && d.date < current_date && is_td_family(d.cvx)),
        None => false,
    }
}

pub fn dtp_custom_dose_number_hook(series_name: &str, ctx: &EvaluationContext) -> usize {
    let mut next_dose = ctx.target_dose_number;
    if series_name == "DTP_5_DOSE_SERIES" {
        let ref_date = ctx.current_dose.map(|d| d.date).unwrap_or(ctx.eval_date);
        let age_ge_7 = ref_date >= add_years_unchecked(ctx.patient.birth_date, 7);
        if age_ge_7 && ctx.valid_doses.len() >= 2 {
            let birth_date = ctx.patient.birth_date;
            let first_valid_dose_at_least_12m =
                ctx.valid_doses[0].0 >= add_months_unchecked(birth_date, 12);
            let any_valid_dose_at_least_4y = ctx
                .valid_doses
                .iter()
                .any(|(v_date, _)| *v_date >= add_years_unchecked(birth_date, 4));
            if first_valid_dose_at_least_12m && any_valid_dose_at_least_4y {
                if next_dose == 3 {
                    next_dose = 4;
                }
            }
        }
    }
    next_dose
}

pub fn dtp_custom_extra_dose_hook(
    series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, SmallVec<[EvaluationReason; 4]>)> {
    if series_name != "DTP_5_DOSE_SERIES" && series_name != "DTP_3_DOSE_SERIES" {
        return None;
    }
    let dose = ctx.current_dose?;
    let birth_date = ctx.patient.birth_date;

    let age_ge_10 = dose.date >= add_years_unchecked(birth_date, 10);
    let t_completed = is_adolescent_tdap_completed(ctx);
    if series_name == "DTP_3_DOSE_SERIES" {
        if age_ge_10 && (is_pertussis_vaccine(dose.cvx) || t_completed) {
            return Some((DoseStatus::Valid, SmallVec::new()));
        }
        if age_ge_10
            && is_first_adult_td_anchor_cvx(dose.cvx)
            && !has_prior_post_primary_td_family(ctx, series_name)
        {
            return Some((DoseStatus::Valid, SmallVec::new()));
        }
        return Some((
            DoseStatus::Accepted,
            crate::reasons![EvaluationReason::BoosterDose],
        ));
    }

    let age_ge_7 = dose.date >= add_years_unchecked(birth_date, 7);
    if ctx.target_dose_number < 5 || (ctx.target_dose_number == 5 && age_ge_7) {
        return Some((DoseStatus::Valid, SmallVec::new()));
    }

    if t_completed {
        // Any subsequent dose is valid as a recurring decennial booster
        return Some((DoseStatus::Valid, SmallVec::new()));
    }

    if age_ge_10 && is_pertussis_vaccine(dose.cvx) {
        return Some((DoseStatus::Valid, SmallVec::new()));
    }

    let is_tdap = dose.cvx.0 == cvx!("115") || dose.cvx.0 == cvx!("198");

    if is_tdap && age_ge_7 {
        if age_ge_10 {
            if let Some(prev_p_date) = get_last_pertussis_date_before(ctx, dose.date) {
                if dose.date < prev_p_date + chrono::Duration::days(28) {
                    return Some((
                        DoseStatus::Accepted,
                        crate::reasons![EvaluationReason::BoosterDose],
                    ));
                }
            }
            return Some((DoseStatus::Valid, SmallVec::new()));
        } else {
            let has_prior_tdap_ge_7 = ctx.valid_doses.iter().any(|(v_date, _)| {
                let is_prior_tdap = ctx.history.iter().any(|d| {
                    d.date == *v_date && (d.cvx.0 == cvx!("115") || d.cvx.0 == cvx!("198"))
                });
                let prior_ge_7 = *v_date >= add_years_unchecked(birth_date, 7);
                is_prior_tdap && prior_ge_7
            });
            if !has_prior_tdap_ge_7 {
                if let Some(prev_p_date) = get_last_pertussis_date_before(ctx, dose.date) {
                    if dose.date < prev_p_date + chrono::Duration::days(28) {
                        return Some((
                            DoseStatus::Accepted,
                            crate::reasons![EvaluationReason::BoosterDose],
                        ));
                    }
                }
                return Some((DoseStatus::Valid, SmallVec::new()));
            }
        }
    }

    Some((
        DoseStatus::Accepted,
        crate::reasons![EvaluationReason::BoosterDose],
    ))
}

pub fn dtp_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    evaluations: &[crate::models::DoseEvaluation],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.series_name != "DTP_5_DOSE_SERIES" && forecast.series_name != "DTP_3_DOSE_SERIES" {
        return;
    }

    // Check if the adolescent Tdap booster has been completed in valid doses.
    let has_valid_tdap_ge_10 = valid_doses.iter().any(|(v_date, _)| {
        let is_tdap = history
            .iter()
            .any(|d| d.date == *v_date && (d.cvx.0 == cvx!("115") || d.cvx.0 == cvx!("198")));
        let age_ge_10 = *v_date >= add_years_unchecked(patient.birth_date, 10);
        is_tdap && age_ge_10
    });
    let has_valid_dtp_ge_10 = valid_doses.iter().any(|(v_date, _)| {
        let is_dtp = history
            .iter()
            .any(|d| d.date == *v_date && is_pertussis_vaccine(d.cvx));
        let age_ge_10 = *v_date >= add_years_unchecked(patient.birth_date, 10);
        is_dtp && age_ge_10
    });

    let has_valid_pertussis_dose = valid_doses.iter().any(|(v_date, _)| {
        history
            .iter()
            .any(|d| d.date == *v_date && is_pertussis_vaccine(d.cvx))
    });
    let has_any_pertussis_history = history.iter().any(|d| is_pertussis_vaccine(d.cvx));

    if forecast.status == crate::models::SeriesStatus::Complete {
        forecast.status = crate::models::SeriesStatus::default();
        forecast.reasons = crate::reasons!["NOT_COMPLETE"];

        if has_valid_tdap_ge_10 || has_valid_dtp_ge_10 {
            // Decennial booster needed
            let last_valid_date = valid_doses
                .iter()
                .map(|(d, _)| *d)
                .max()
                .unwrap_or(eval_date);
            let earliest = add_years_unchecked(last_valid_date, 5);
            let recommended = add_years_unchecked(last_valid_date, 10);
            let overdue = add_years_unchecked(last_valid_date, 10) + chrono::Duration::days(28)
                - chrono::Duration::days(1);

            forecast.status = forecast.status.with_earliest_date(Some(earliest));
            forecast.status = forecast.status.with_recommended_date(Some(recommended));
            forecast.status = forecast.status.with_overdue_date(Some(overdue));
        } else {
            if forecast.series_name == "DTP_5_DOSE_SERIES" {
                if let Some((last_valid_date, _)) = valid_doses.last() {
                    let age_7 = add_years_unchecked(patient.birth_date, 7);
                    let age_10 = add_years_unchecked(patient.birth_date, 10);
                    let last_valid_td_family = history
                        .iter()
                        .any(|d| d.date == *last_valid_date && !is_pertussis_vaccine(d.cvx));

                    if last_valid_td_family
                        && *last_valid_date >= age_7
                        && *last_valid_date < age_10
                    {
                        forecast.status =
                            forecast.status.with_earliest_date(Some(*last_valid_date));
                        forecast.status = forecast
                            .status
                            .with_recommended_date(Some(*last_valid_date));
                        forecast.status = forecast.status.with_overdue_date(Some(*last_valid_date));
                        return;
                    }
                }
            }

            // Adolescent Tdap booster needed
            let mut earliest = add_years_unchecked(patient.birth_date, 11);
            let mut recommended = add_years_unchecked(patient.birth_date, 11);
            let mut overdue = add_years_unchecked(patient.birth_date, 13)
                + chrono::Duration::days(28)
                - chrono::Duration::days(1);

            let mut exception_occurred = if forecast.series_name == "DTP_5_DOSE_SERIES" {
                let age_4y_minus_4d =
                    add_years_unchecked(patient.birth_date, 4) - chrono::Duration::days(4);
                let has_pertussis_ge_4y_minus_4d = valid_doses.iter().any(|(v_date, _)| {
                    *v_date >= age_4y_minus_4d
                        && history
                            .iter()
                            .any(|d| d.date == *v_date && is_pertussis_vaccine(d.cvx))
                });
                let age_7y = add_years_unchecked(patient.birth_date, 7);
                let pertussis_under_7_count = valid_doses
                    .iter()
                    .filter(|(v_date, _)| {
                        *v_date < age_7y
                            && history
                                .iter()
                                .any(|d| d.date == *v_date && is_pertussis_vaccine(d.cvx))
                    })
                    .count();
                !has_pertussis_ge_4y_minus_4d || pertussis_under_7_count < 4
            } else {
                false
            };

            if exception_occurred {
                let has_valid_dose_ge_7 = valid_doses
                    .iter()
                    .any(|(v_date, _)| *v_date >= add_years_unchecked(patient.birth_date, 7));
                if has_valid_dose_ge_7 {
                    exception_occurred = false;
                }
            }

            if exception_occurred {
                let age_7 = add_years_unchecked(patient.birth_date, 7);
                earliest = age_7;
                recommended = age_7;
                overdue = age_7;
            }

            // Check 6-month interval from last pertussis shot
            let last_pertussis_date = history
                .iter()
                .filter(|d| is_pertussis_vaccine(d.cvx))
                .map(|d| d.date)
                .max();
            if let Some(lp_date) = last_pertussis_date {
                let min_interval_date = add_months_unchecked(lp_date, 6);
                earliest = earliest.max(min_interval_date);
                recommended = recommended.max(min_interval_date);
            }

            forecast.status = forecast.status.with_earliest_date(Some(earliest));
            forecast.status = forecast.status.with_recommended_date(Some(recommended));
            forecast.status = forecast.status.with_overdue_date(Some(overdue));
        }
    } else {
        // Series is not complete
        if forecast.series_name == "DTP_3_DOSE_SERIES"
            && valid_doses.len() >= 3
            && !has_valid_pertussis_dose
            && has_any_pertussis_history
        {
            let mut earliest = add_years_unchecked(patient.birth_date, 11);
            let mut recommended = add_years_unchecked(patient.birth_date, 11);
            let overdue = add_years_unchecked(patient.birth_date, 13)
                + chrono::Duration::days(28)
                - chrono::Duration::days(1);

            if let Some(lp_date) = history
                .iter()
                .filter(|d| is_pertussis_vaccine(d.cvx))
                .map(|d| d.date)
                .max()
            {
                let min_interval_date = add_months_unchecked(lp_date, 6);
                earliest = earliest.max(min_interval_date);
                recommended = recommended.max(min_interval_date);
            }

            forecast.status = forecast.status.with_earliest_date(Some(earliest));
            forecast.status = forecast.status.with_recommended_date(Some(recommended));
            forecast.status = forecast.status.with_overdue_date(Some(overdue));
            return;
        }

        let age_7 = add_years_unchecked(patient.birth_date, 7);
        const ALLOWED_CVX: &[u16] = &[
            cvx!("01"),
            cvx!("20"),
            cvx!("106"),
            cvx!("107"),
            cvx!("22"),
            cvx!("50"),
            cvx!("102"),
            cvx!("110"),
            cvx!("120"),
            cvx!("130"),
            cvx!("132"),
            cvx!("146"),
            cvx!("115"),
            cvx!("28"),
            cvx!("09"),
            cvx!("138"),
            cvx!("139"),
            cvx!("113"),
            cvx!("170"),
            cvx!("195"),
            cvx!("196"),
            cvx!("198"),
        ];
        let history_count = evaluations
            .iter()
            .filter(|e| ALLOWED_CVX.contains(&e.cvx.0))
            .filter(|e| {
                !(e.reasons.contains(&EvaluationReason::DuplicateShotSameDay)
                    && is_pertussis_vaccine(e.cvx))
            })
            .count();
        let six_by_seven = eval_date < age_7 && history_count >= 6;

        if six_by_seven {
            forecast.status = forecast.status.with_earliest_date(Some(age_7));
            forecast.status = forecast.status.with_recommended_date(Some(age_7));
            forecast.status = forecast.status.with_overdue_date(Some(age_7));
            return;
        } else if eval_date >= age_7 {
            let earliest = forecast.status.earliest_date().unwrap_or(age_7).max(age_7);
            let recommended = forecast
                .status
                .recommended_date()
                .unwrap_or(age_7)
                .max(age_7);
            let overdue = forecast.status.overdue_date().unwrap_or(age_7).max(age_7);
            forecast.status = forecast.status.with_earliest_date(Some(earliest));
            forecast.status = forecast.status.with_recommended_date(Some(recommended));
            forecast.status = forecast.status.with_overdue_date(Some(overdue));
            return;
        }

        if forecast.series_name == "DTP_5_DOSE_SERIES" {
            let next_dose_idx = valid_doses.len() + 1;
            if let Some(overdue_age) = dtp_5_overdue_age_date(patient.birth_date, next_dose_idx) {
                let recommended = forecast.status.recommended_date().unwrap_or(overdue_age);
                forecast.status = forecast
                    .status
                    .with_overdue_date(Some(overdue_age.max(recommended)));
            }
        }

        // If the last shot in history was ignored, adjust forecast dates accordingly
        let last_ignored_shot = history.last().and_then(|d| {
            let age_7_minus_4d = age_7 - chrono::Duration::days(4);
            let is_tdap = d.cvx.0 == cvx!("115") || d.cvx.0 == cvx!("198");
            let is_td = d.cvx.0 == cvx!("09")
                || d.cvx.0 == cvx!("113")
                || d.cvx.0 == cvx!("138")
                || d.cvx.0 == cvx!("139")
                || d.cvx.0 == cvx!("196");
            if d.date < age_7_minus_4d {
                if (is_tdap && valid_doses.len() <= 2) || is_td {
                    return Some(d);
                }
            }
            None
        });

        if let Some(ignored_shot) = last_ignored_shot {
            if let Some((earliest, recommended, overdue)) =
                get_ignored_adjustments(patient, history, valid_doses, forecast, ignored_shot)
            {
                forecast.status = forecast.status.with_earliest_date(earliest);
                forecast.status = forecast.status.with_recommended_date(recommended);
                forecast.status = forecast.status.with_overdue_date(overdue);
            }
        }
    }
}

fn add_days(date: NaiveDate, days: i64) -> NaiveDate {
    date + chrono::Duration::days(days)
}

fn dtp_5_overdue_age_date(birth: NaiveDate, next_dose_idx: usize) -> Option<NaiveDate> {
    match next_dose_idx {
        2 => Some(
            add_months_unchecked(birth, 5) + chrono::Duration::days(28) - chrono::Duration::days(1),
        ),
        3 => Some(
            add_months_unchecked(birth, 7) + chrono::Duration::days(28) - chrono::Duration::days(1),
        ),
        4 => Some(
            add_months_unchecked(birth, 19) + chrono::Duration::days(28)
                - chrono::Duration::days(1),
        ),
        5 => Some(add_years_unchecked(birth, 7) - chrono::Duration::days(1)),
        _ => None,
    }
}

fn get_ignored_adjustments(
    patient: &Patient,
    history: &[Dose],
    valid_doses: &[(NaiveDate, usize)],
    forecast: &SeriesForecast,
    ignored_shot: &Dose,
) -> Option<(Option<NaiveDate>, Option<NaiveDate>, Option<NaiveDate>)> {
    if valid_doses.is_empty() {
        return None;
    }
    let (prev_date, _) = valid_doses.last()?;
    let birth = patient.birth_date;
    let next_dose_idx = valid_doses.len() + 1;
    let ignored_date_has_cvx_198 = history
        .iter()
        .any(|d| d.date == ignored_shot.date && d.cvx.0 == cvx!("198"));
    let clamp_ignored_earliest_to_recommended = next_dose_idx == 4
        && ignored_shot.cvx.0 != cvx!("198")
        && (ignored_shot.date != *prev_date || ignored_date_has_cvx_198);
    let interval_anchor = if ignored_shot.cvx.0 == cvx!("198")
        || (ignored_shot.date > *prev_date && ignored_date_has_cvx_198)
    {
        ignored_shot.date
    } else {
        *prev_date
    };

    if forecast.series_name == "DTP_5_DOSE_SERIES" {
        match next_dose_idx {
            2 => {
                let min_age = add_days(birth, 70);
                let min_int = add_days(interval_anchor, 28);
                let rec_age = add_months_unchecked(birth, 4);
                let rec_int = add_days(interval_anchor, 28);
                let overdue_age = dtp_5_overdue_age_date(birth, next_dose_idx)?;

                let recommended = rec_age.max(rec_int);
                let mut earliest = min_age.max(min_int);
                if clamp_ignored_earliest_to_recommended {
                    earliest = earliest.max(recommended);
                }
                let overdue = overdue_age.max(recommended);
                Some((Some(earliest), Some(recommended), Some(overdue)))
            }
            3 => {
                let min_age = add_days(birth, 98);
                let min_int = add_days(interval_anchor, 28);
                let rec_age = add_months_unchecked(birth, 6);
                let rec_int = add_days(interval_anchor, 28);
                let overdue_age = dtp_5_overdue_age_date(birth, next_dose_idx)?;

                let recommended = rec_age.max(rec_int);
                let mut earliest = min_age.max(min_int);
                if clamp_ignored_earliest_to_recommended {
                    earliest = earliest.max(recommended);
                }
                let overdue = overdue_age.max(recommended);
                Some((Some(earliest), Some(recommended), Some(overdue)))
            }
            4 => {
                let min_age = add_months_unchecked(birth, 15);
                let min_int = add_months_unchecked(interval_anchor, 4);
                let rec_age = add_months_unchecked(birth, 15);
                let rec_int = add_months_unchecked(interval_anchor, 6);
                let overdue_age = dtp_5_overdue_age_date(birth, next_dose_idx)?;

                let recommended = rec_age.max(rec_int);
                let mut earliest = min_age.max(min_int);
                if clamp_ignored_earliest_to_recommended {
                    earliest = earliest.max(recommended);
                }
                let overdue = overdue_age.max(recommended);
                Some((Some(earliest), Some(recommended), Some(overdue)))
            }
            5 => {
                let min_age = add_years_unchecked(birth, 4);
                let min_int = add_months_unchecked(interval_anchor, 6);
                let rec_age = add_years_unchecked(birth, 4);
                let rec_int = add_months_unchecked(interval_anchor, 6);
                let overdue_age = add_years_unchecked(birth, 7);

                let recommended = rec_age.max(rec_int);
                let mut earliest = min_age.max(min_int);
                if clamp_ignored_earliest_to_recommended {
                    earliest = earliest.max(recommended);
                }
                let overdue = overdue_age.max(recommended);
                Some((Some(earliest), Some(recommended), Some(overdue)))
            }
            _ => None,
        }
    } else {
        None
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
            ctx.history
                .iter()
                .any(|h_dose| h_dose.date == *v_date && is_pertussis_vaccine(h_dose.cvx))
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
        let is_at_least_7 = ctx.eval_date >= add_years_unchecked(birth_date, 7);
        let first_valid_dose_at_least_12m =
            ctx.valid_doses[0].0 >= add_months_unchecked(birth_date, 12);
        let any_valid_dose_at_least_4y = ctx
            .valid_doses
            .iter()
            .any(|(v_date, _)| *v_date >= add_years_unchecked(birth_date, 4));

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

        let dose4_at_least_4y = dose4_date >= add_years_unchecked(birth_date, 4);
        let interval_ok =
            dose4_date >= add_months_unchecked(dose3_date, 6) - chrono::Duration::days(4);

        dose4_at_least_4y && interval_ok
    } else {
        false
    }
}

pub fn dtp_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    _candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let age_7 = add_years_unchecked(patient.birth_date, 7);
    let age_7_minus_4d = age_7 - chrono::Duration::days(4);

    // Java checks for target doses, not arbitrary raw DTP administrations. A
    // very early Td-family dose that will be invalid for child-series DTP does
    // not block adult-series selection; near-age-7 doses use minimum-age grace.
    let has_child_series_target_dose = history
        .iter()
        .any(|dose| dose.date < age_7_minus_4d && !is_td_min_age_invalid(dose.cvx));

    // Patient must be >= 7 years of age
    let is_at_least_7 = eval_date >= age_7;

    if is_at_least_7 && !has_child_series_target_dose {
        "DTP_3_DOSE_SERIES"
    } else {
        "DTP_5_DOSE_SERIES"
    }
}
