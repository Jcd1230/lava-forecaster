use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast};
use crate::rules::helpers::{clamp_date_at_least, interval_days_between};

/// CVX codes for old live zoster vaccines (Zostavax and variants).
/// These are not valid doses for the recombinant series but are tracked as Accepted.
fn is_old_zoster(cvx: &str) -> bool {
    cvx == "121" || cvx == "188"
}

/// CVX code for recombinant zoster vaccine (Shingrix).
fn is_shingrix(cvx: &str) -> bool {
    cvx == "187"
}

/// CVX code for adult varicella (for live-virus forecast spacing).
fn is_adult_varicella(cvx: &str) -> bool {
    cvx == "21"
}

pub fn zoster_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        // Rule 1: CVX 121/188 (old live zoster) → always Accepted/VaccineNotPartOfSeries
        if is_old_zoster(&dose.cvx) {
            *status = DoseStatus::Accepted;
            reasons.clear();
            reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            reasons.push(EvaluationReason::OutsideRoutineSeries);
            return;
        }

        // Rule 2: CVX 187 given < 52 days after a prior CVX 121 or CVX 188 → Invalid/BelowMinimumInterval
        // (or on the same day as a CVX 121/188)
        if is_shingrix(&dose.cvx) {
            for prior in ctx.history {
                if is_old_zoster(&prior.cvx) && prior.date <= dose.date {
                    let gap = interval_days_between(prior.date, dose.date);
                    if gap < 52 {
                        *status = DoseStatus::Invalid;
                        if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                            reasons.push(EvaluationReason::BelowMinimumInterval);
                        }
                        // Remove VaccineNotPartOfSeries if it was set by standard check
                        reasons.retain(|r| *r != EvaluationReason::VaccineNotPartOfSeries);
                    }
                }
            }
        }
    }
}

pub fn zoster_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    _eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    use crate::models::SeriesStatus;
    if forecast.status == SeriesStatus::Complete {
        return;
    }

    // Age 50 clamp: The entire Zoster series is recommended starting at age 50.
    let age_50 = crate::date_utils::add_years(patient.birth_date, 50);

    clamp_date_at_least(&mut forecast.earliest_date, age_50);
    clamp_date_at_least(&mut forecast.recommended_date, age_50);

    // Live-virus spacing rule:
    // If the last CVX 121/188 (old zoster) was administered, forecast dates must be at least
    // 8 weeks (56 days) after the last dose.
    // Also applies if the last CVX 21 (adult varicella) was given.
    let last_live_zoster = history.iter()
        .filter(|d| is_old_zoster(&d.cvx))
        .map(|d| d.date)
        .max();

    let last_adult_varicella = history.iter()
        .filter(|d| is_adult_varicella(&d.cvx))
        .map(|d| d.date)
        .max();

    let live_virus_date = match (last_live_zoster, last_adult_varicella) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    if let Some(last_date) = live_virus_date {
        let spacing_date = last_date + chrono::Duration::days(56); // 8 weeks

        clamp_date_at_least(&mut forecast.earliest_date, spacing_date);
        clamp_date_at_least(&mut forecast.recommended_date, spacing_date);
    }

    // If a recombinant zoster dose was attempted too soon after live zoster, the failed
    // CVX 187 still anchors the next recombinant-dose forecast interval.
    let last_invalid_shingrix_after_live = history
        .iter()
        .filter(|dose| is_shingrix(&dose.cvx))
        .filter(|dose| {
            history.iter().any(|prior| {
                is_old_zoster(&prior.cvx)
                    && prior.date <= dose.date
                    && interval_days_between(prior.date, dose.date) < 52
            })
        })
        .map(|dose| dose.date)
        .max();

    if let Some(last_invalid_date) = last_invalid_shingrix_after_live {
        let earliest_date = last_invalid_date + chrono::Duration::days(28);
        let recommended_date = last_invalid_date + chrono::Duration::days(56);

        clamp_date_at_least(&mut forecast.earliest_date, earliest_date);
        clamp_date_at_least(&mut forecast.recommended_date, recommended_date);
    }

    // Ensure recommended >= earliest after all adjustments
    if let (Some(earliest), Some(recommended)) = (forecast.earliest_date, forecast.recommended_date.as_mut()) {
        if *recommended < earliest {
            *recommended = earliest;
        }
    }

    // If the patient has ever received old live zoster (CVX 121 or 188), there is no overdue date for the Shingrix dose.
    let has_old_zoster = history.iter().any(|d| is_old_zoster(&d.cvx));
    if has_old_zoster && valid_doses.is_empty() {
        forecast.overdue_date = None;
    }
}
