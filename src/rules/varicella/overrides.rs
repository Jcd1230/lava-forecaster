use ice_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Cvx, Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus};
use crate::rules::helpers::{clamp_date_at_least, interval_days_between, age_ge};

fn is_live_virus(cvx: Cvx) -> bool {
    const LIVE_VIRUS: &[u16] = &[cvx!("03"), cvx!("04"), cvx!("05"), cvx!("06"), cvx!("07"), cvx!("21"), cvx!("37"), cvx!("38"), cvx!("75"), cvx!("94"), cvx!("105"), cvx!("111"), cvx!("121"), cvx!("125"), cvx!("149"), cvx!("151"), cvx!("183"), cvx!("184"), cvx!("325"), cvx!("333")];
    LIVE_VIRUS.contains(&cvx.0)
}

fn is_varicella_group(cvx: Cvx) -> bool {
    cvx.0 == cvx!("21") || cvx.0 == cvx!("94")
}

fn is_old_zoster(cvx: Cvx) -> bool {
    cvx.0 == cvx!("121") || cvx.0 == cvx!("188")
}

pub fn varicella_custom_evaluation_hook(
    _series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        let birth_date = ctx.patient.birth_date;

        // 1. Absolute Minimum Interval 1->2 Override if administered at >= 13 years of age
        if target_dose_idx == 2 {
            if age_ge(birth_date, dose.date, "13y") {
                if let Some((prev_date, _)) = ctx.valid_doses.last() {
                    let interval_days = interval_days_between(*prev_date, dose.date);
                    if interval_days >= 24 {
                        reasons.retain(|r| *r != EvaluationReason::BelowMinimumInterval);
                        if reasons.is_empty() {
                            *status = DoseStatus::Valid;
                        }
                    }
                }
            }
        }

        // 2. Live Virus Conflict
        if is_live_virus(dose.cvx) {
            for prev in ctx.history {
                if prev.date < dose.date && is_live_virus(prev.cvx) {
                    let is_both_varicella = is_varicella_group(dose.cvx) && is_varicella_group(prev.cvx);
                    let required_days = if is_both_varicella {
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

pub fn varicella_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let pre_1980 = NaiveDate::from_ymd_opt(1980, 1, 1).unwrap();

    // 1. Patient born prior to 1980 rule
    if forecast.status != SeriesStatus::Complete && patient.birth_date < pre_1980 {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = vec!["CONDITIONAL".into()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    if forecast.status != SeriesStatus::Complete
        && valid_doses.is_empty()
        && !history.is_empty()
        && history.iter().all(|dose| is_old_zoster(dose.cvx))
    {
        forecast.earliest_date = Some(eval_date);
        forecast.recommended_date = Some(eval_date);
        forecast.overdue_date = Some(eval_date);
        return;
    }

    // 2. Patient age >= 13 years interval overrides
    if forecast.status != SeriesStatus::Complete && valid_doses.len() == 1 {
        if age_ge(patient.birth_date, eval_date, "13y") {
            let dose1_date = valid_doses[0].0;
            let override_date = dose1_date + chrono::Duration::days(28);

            forecast.earliest_date = Some(override_date);
            forecast.recommended_date = Some(override_date);
        }
    }

    // 3. Live Virus Forecast Spacing
    if forecast.status != SeriesStatus::Complete {
        let last_live_virus = history.iter()
            .filter(|d| is_varicella_group(d.cvx))
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
