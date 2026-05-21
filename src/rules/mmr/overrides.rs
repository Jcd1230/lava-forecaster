use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus};
use crate::date_utils::{TimePeriod, add_years};

fn is_live_virus(cvx: &str) -> bool {
    const LIVE_VIRUS: &[&str] = &[
        "03", "04", "05", "06", "07", "21", "37", "38", "75", "94", "105", "111", "121", "125", "149", "151", "183", "184", "325", "333"
    ];
    LIVE_VIRUS.contains(&cvx)
}

fn is_mmr_group(cvx: &str) -> bool {
    const MMR_CVX: &[&str] = &["03", "04", "05", "06", "07", "38", "94"];
    MMR_CVX.contains(&cvx)
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
            if dose.cvx == "03" || dose.cvx == "04" || dose.cvx == "05" {
                let date_6m_minus_4d = TimePeriod::parse("6m-4d").unwrap().add_to(birth_date);
                let date_1y_minus_4d = TimePeriod::parse("1y-4d").unwrap().add_to(birth_date);
                if dose.date >= date_6m_minus_4d && dose.date < date_1y_minus_4d {
                    *status = DoseStatus::Accepted;
                    reasons.clear();
                    reasons.push(EvaluationReason::OutsideRoutineSeries);
                    return; // Skip further checks if accepted outside routine
                }
            }
        }

        // 2. Adult Dose 2 Booster/Completion
        if target_dose_idx == 2 {
            let age_19 = add_years(birth_date, 19);
            if dose.date >= age_19 {
                *status = DoseStatus::Accepted;
                reasons.clear();
                reasons.push(EvaluationReason::BoosterDose);
                return; // Skip further checks
            }
        }

        // 3. Live Virus Conflict
        if is_live_virus(&dose.cvx) {
            for prev in ctx.history {
                if prev.date < dose.date && is_live_virus(&prev.cvx) {
                    let is_both_mmr = is_mmr_group(&dose.cvx) && is_mmr_group(&prev.cvx);
                    let required_days = if is_both_mmr {
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

pub fn mmr_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let age_19 = add_years(patient.birth_date, 19);
    let pre_1957 = NaiveDate::from_ymd_opt(1957, 1, 1).unwrap();

    // Case 1: Series is Complete (either already completed or adult completion)
    let is_completed = forecast.status == SeriesStatus::Complete;
    let is_adult_complete = !valid_doses.is_empty() && (
        eval_date >= age_19 || forecast.recommended_date.map(|d| d >= age_19).unwrap_or(false)
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
                .filter(|d| is_live_virus(&d.cvx))
                .map(|d| d.date)
                .max();

            if let Some(last_date) = last_live_virus {
                let conflict_free_date = last_date + chrono::Duration::days(28);
                
                if let Some(ref mut earliest) = forecast.earliest_date {
                    if *earliest < conflict_free_date {
                        *earliest = conflict_free_date;
                    }
                }
                if let Some(ref mut recommended) = forecast.recommended_date {
                    if *recommended < conflict_free_date {
                        *recommended = conflict_free_date;
                    }
                }
                if let (Some(earliest), Some(recommended)) = (forecast.earliest_date, forecast.recommended_date.as_mut()) {
                    if *recommended < earliest {
                        *recommended = earliest;
                    }
                }
            }
        }
    }
}
