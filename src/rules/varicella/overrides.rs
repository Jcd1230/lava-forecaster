use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus};
use crate::date_utils::{add_years};

fn is_live_virus(cvx: &str) -> bool {
    const LIVE_VIRUS: &[&str] = &[
        "03", "04", "05", "06", "07", "21", "37", "38", "75", "94", "105", "111", "121", "125", "149", "151", "183", "184", "325", "333"
    ];
    LIVE_VIRUS.contains(&cvx)
}

fn is_varicella_group(cvx: &str) -> bool {
    cvx == "21" || cvx == "94"
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
            let age_13 = add_years(birth_date, 13);
            if dose.date >= age_13 {
                if let Some((prev_date, _)) = ctx.valid_doses.last() {
                    let interval_days = (dose.date - *prev_date).num_days();
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
        if is_live_virus(&dose.cvx) {
            for prev in ctx.history {
                if prev.date < dose.date && is_live_virus(&prev.cvx) {
                    let is_both_varicella = is_varicella_group(&dose.cvx) && is_varicella_group(&prev.cvx);
                    let required_days = if is_both_varicella {
                        if dose.cvx == "94" || prev.cvx == "94" {
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
        forecast.reasons = vec!["CONDITIONAL".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    // 2. Patient age >= 13 years interval overrides
    if forecast.status != SeriesStatus::Complete && valid_doses.len() == 1 {
        let age_13 = add_years(patient.birth_date, 13);
        if eval_date >= age_13 {
            let dose1_date = valid_doses[0].0;
            let override_date = dose1_date + chrono::Duration::days(28);

            forecast.earliest_date = Some(override_date);
            forecast.recommended_date = Some(override_date);
        }
    }

    // 3. Live Virus Forecast Spacing
    if forecast.status != SeriesStatus::Complete {
        let last_live_virus = history.iter()
            .filter(|d| is_live_virus(&d.cvx))
            .map(|d| d.date)
            .max();

        if let Some(last_date) = last_live_virus {
            let conflict_free_date = last_date + chrono::Duration::days(28);

            if let Some(ref mut earliest) = forecast.earliest_date {
                if *earliest < conflict_free_date {
                    *earliest = conflict_free_date;
                }
            } else {
                forecast.earliest_date = Some(conflict_free_date);
            }

            if let Some(ref mut recommended) = forecast.recommended_date {
                if *recommended < conflict_free_date {
                    *recommended = conflict_free_date;
                }
            } else {
                forecast.recommended_date = Some(conflict_free_date);
            }

            if let (Some(earliest), Some(recommended)) = (forecast.earliest_date, forecast.recommended_date.as_mut()) {
                if *recommended < earliest {
                    *recommended = earliest;
                }
            }
        }
    }
}
