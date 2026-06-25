use crate::date_utils::SmallVec;
use crate::engine::CandidateForecastsExt;
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Dose,
    DoseStatus,
    EvaluationReason,
    Patient,
    SeriesForecast,
    SeriesStatus,
    VaccineGroupForecast,
};

const MPOX_1_DOSE_SERIES: &str = "MPOX_1_DOSE_SERIES";
const MPOX_2_DOSE_SERIES: &str = "MPOX_2_DOSE_SERIES";

fn series_primary_dose_count(series_name: &str) -> Option<usize> {
    match series_name {
        MPOX_1_DOSE_SERIES => Some(1),
        MPOX_2_DOSE_SERIES => Some(2),
        _ => None,
    }
}

pub fn mpox_custom_evaluation_hook(
    series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        if dose.cvx.0 == 75 || dose.cvx.0 == 105 {
            let limit = crate::time_period!("1y-4d");
            if crate::date_utils::compare_elapsed(ctx.patient.birth_date, dose.date, &limit).is_lt() {
                *status = DoseStatus::Invalid;
                reasons.clear();
                reasons.push(EvaluationReason::BelowMinimumAge);
                return;
            }
        }
    }

    let Some(primary_dose_count) = series_primary_dose_count(series_name) else {
        return;
    };

    if reasons.contains(&EvaluationReason::VaccineNotPartOfSeries)
        && ctx.valid_doses.len() < primary_dose_count
    {
        *status = DoseStatus::Accepted;
        reasons.clear();
        reasons.push(EvaluationReason::VaccineNotCountedBasedOnMostRecentVaccineGiven);
    }
}

pub fn mpox_custom_extra_dose_hook(
    series_name: &str,
    ctx: &EvaluationContext,
) -> Option<(DoseStatus, SmallVec<[EvaluationReason; 4]>)> {
    let dose = ctx.current_dose?;
    let primary_dose_count = series_primary_dose_count(series_name)?;

    let cvx = dose.cvx.0;

    // The booster dose MUST be the immediately following administered shot in history after the completing valid dose.
    let completing_dose_date = ctx.valid_doses
        .iter()
        .find(|(_, num)| *num == primary_dose_count)
        .map(|(date, _)| *date);

    if let Some(comp_date) = completing_dose_date {
        let comp_idx = ctx.history.iter().position(|d| d.date == comp_date);
        if let Some(idx) = comp_idx {
            // Find the next shot in history
            if idx + 1 < ctx.history.len() {
                let next_shot = &ctx.history[idx + 1];
                if next_shot.date != dose.date {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    // 1. If it's the booster dose (first dose after series is complete)
    if ctx.valid_doses.len() == primary_dose_count
        && ctx.target_dose_number == primary_dose_count + 1
    {
        return Some((DoseStatus::Valid, crate::reasons![EvaluationReason::BoosterDose]));
    }

    // 2. Perform age check for CVX 75 / 105
    if cvx == 75 || cvx == 105 {
        let limit = crate::time_period!("1y-4d");
        if crate::date_utils::compare_elapsed(ctx.patient.birth_date, dose.date, &limit).is_lt() {
            return Some((DoseStatus::Invalid, crate::reasons![EvaluationReason::BelowMinimumAge]));
        }
    }

    // 3. For any subsequent extra doses, if they pass age checks, they are Accepted / ExtraDose
    if ctx.target_dose_number > primary_dose_count + 1 {
        return Some((DoseStatus::Accepted, crate::reasons![EvaluationReason::BoosterDose]));
    }

    None
}

pub fn mpox_custom_forecast_hook(
    _patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    _evaluations: &[crate::models::DoseEvaluation],
    _history: &[Dose],
    _eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.reasons = crate::reasons!["COMPLETE_HIGH_RISK"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        return;
    }

    if valid_doses.is_empty() {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons!["HIGH_RISK"];
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(None);
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
    }
}

pub fn mpox_group_selection(
    _patient: &Patient,
    _history: &[Dose],
    _eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let two_dose = candidate_forecasts.get_forecast(MPOX_2_DOSE_SERIES);
    let one_dose = candidate_forecasts.get_forecast(MPOX_1_DOSE_SERIES);

    let two_dose_complete = two_dose.map_or(false, |f| {
        f.forecasts.iter().any(|sf| sf.series_name == MPOX_2_DOSE_SERIES && sf.status == SeriesStatus::Complete)
    });
    let one_dose_complete = one_dose.map_or(false, |f| {
        f.forecasts.iter().any(|sf| sf.series_name == MPOX_1_DOSE_SERIES && sf.status == SeriesStatus::Complete)
    });

    let d1_1_dose = one_dose.and_then(|f| {
        f.evaluations.iter().find(|e| e.status == DoseStatus::Valid && e.dose_number == Some(1)).map(|e| e.dose_date)
    });
    let d1_2_dose = two_dose.and_then(|f| {
        f.evaluations.iter().find(|e| e.status == DoseStatus::Valid && e.dose_number == Some(1)).map(|e| e.dose_date)
    });

    let comp_1_dose = d1_1_dose;
    let comp_2_dose = two_dose.and_then(|f| {
        f.evaluations.iter().find(|e| e.status == DoseStatus::Valid && e.dose_number == Some(2)).map(|e| e.dose_date)
    });

    if two_dose_complete && one_dose_complete {
        match (comp_1_dose, comp_2_dose) {
            (Some(c1), Some(c2)) => {
                if c1 < c2 {
                    MPOX_1_DOSE_SERIES
                } else if c2 < c1 {
                    MPOX_2_DOSE_SERIES
                } else {
                    match (d1_1_dose, d1_2_dose) {
                        (Some(s1), Some(s2)) => {
                            if s1 < s2 {
                                MPOX_1_DOSE_SERIES
                            } else {
                                MPOX_2_DOSE_SERIES
                            }
                        }
                        _ => MPOX_2_DOSE_SERIES,
                    }
                }
            }
            (Some(_), None) => MPOX_1_DOSE_SERIES,
            (None, Some(_)) => MPOX_2_DOSE_SERIES,
            _ => MPOX_2_DOSE_SERIES,
        }
    } else if two_dose_complete {
        MPOX_2_DOSE_SERIES
    } else if one_dose_complete {
        MPOX_1_DOSE_SERIES
    } else {
        match (d1_1_dose, d1_2_dose) {
            (Some(s1), Some(s2)) => {
                if s1 < s2 {
                    MPOX_1_DOSE_SERIES
                } else {
                    MPOX_2_DOSE_SERIES
                }
            }
            (Some(_), None) => MPOX_1_DOSE_SERIES,
            (None, Some(_)) => MPOX_2_DOSE_SERIES,
            _ => MPOX_2_DOSE_SERIES,
        }
    }
}

pub fn mpox_custom_completion_hook(ctx: &EvaluationContext) -> bool {
    let target_doses = match ctx.active_series_name {
        MPOX_1_DOSE_SERIES => 1,
        MPOX_2_DOSE_SERIES => 2,
        _ => return false,
    };
    ctx.valid_doses.iter().any(|(_, num)| *num == target_doses)
}
