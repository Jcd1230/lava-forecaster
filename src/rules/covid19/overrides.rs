use chrono::NaiveDate;
use crate::date_utils::{add_years, compare_elapsed, TimePeriod};
use crate::engine::EvaluationContext;
use crate::models::{
    Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus,
    VaccineGroupForecast,
};

const AUG_2025_SEASON_START: (i32, u32, u32) = (2025, 9, 1);

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn age_ge(birth_date: NaiveDate, date_to_check: NaiveDate, age: &str) -> bool {
    compare_elapsed(birth_date, date_to_check, &TimePeriod::parse(age).unwrap())
        != std::cmp::Ordering::Less
}

fn age_lt(birth_date: NaiveDate, date_to_check: NaiveDate, age: &str) -> bool {
    compare_elapsed(birth_date, date_to_check, &TimePeriod::parse(age).unwrap())
        == std::cmp::Ordering::Less
}

fn season_start() -> NaiveDate {
    date(
        AUG_2025_SEASON_START.0,
        AUG_2025_SEASON_START.1,
        AUG_2025_SEASON_START.2,
    )
}

fn most_recent_non_ignored_prior(current_date: NaiveDate, history: &[Dose]) -> Option<&Dose> {
    history.iter().filter(|dose| dose.date < current_date).max_by_key(|dose| dose.date)
}

fn most_recent_dose(history: &[Dose]) -> Option<&Dose> {
    history.iter().max_by_key(|dose| dose.date)
}

fn has_in_season_dose_before_age(patient: &Patient, history: &[Dose], age: &str) -> bool {
    let cutoff = TimePeriod::parse(age).unwrap().add_to(patient.birth_date);
    history
        .iter()
        .any(|dose| dose.date >= season_start() && dose.date < cutoff)
}

pub fn covid19_custom_evaluation_hook(
    series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    let Some(dose) = ctx.current_dose else {
        return;
    };

    if series_name == "COVID_19_AUG_2025_LT_2_SERIES" && target_dose_idx == 1 {
        if dose.date < TimePeriod::parse("6m-4d").unwrap().add_to(ctx.patient.birth_date) {
            *status = DoseStatus::Invalid;
            if !reasons.contains(&EvaluationReason::BelowMinimumAge) {
                reasons.push(EvaluationReason::BelowMinimumAge);
            }
            return;
        }
    }

    if matches!(series_name, "COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES" | "COVID_19_AUG_2025_GTE_65_SERIES")
        && target_dose_idx == 1
    {
        if let Some(previous_dose) = most_recent_non_ignored_prior(dose.date, ctx.history) {
            let min_interval = if previous_dose.cvx == "313" && dose.cvx == "313" {
                TimePeriod::parse("17d").unwrap()
            } else {
                TimePeriod::parse("8w-4d").unwrap()
            };
            if compare_elapsed(previous_dose.date, dose.date, &min_interval) == std::cmp::Ordering::Less {
                *status = DoseStatus::Invalid;
                reasons.retain(|reason| *reason != EvaluationReason::BelowMinimumAge);
                if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                    reasons.push(EvaluationReason::BelowMinimumInterval);
                }
            }
        }
    }
}

pub fn covid19_custom_forecast_hook(
    patient: &Patient,
    valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    if forecast.status == SeriesStatus::Complete {
        forecast.reasons = vec!["COMPLETE_HIGH_RISK".to_string()];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
        forecast.latest_date = None;
        return;
    }

    if forecast.series_name == "COVID_19_AUG_2025_LT_2_SERIES" {
        let age_6m = TimePeriod::parse("6m").unwrap().add_to(patient.birth_date);
        if valid_doses.is_empty() && history.is_empty() {
            let recommendation_date = eval_date.max(age_6m);
            forecast.status = SeriesStatus::NotComplete;
            forecast.reasons = vec!["NOT_COMPLETE".to_string()];
            forecast.earliest_date = Some(recommendation_date);
            forecast.recommended_date = Some(recommendation_date);
            forecast.overdue_date = None;
            forecast.latest_date = None;
            return;
        }

        if let Some(last_dose) = most_recent_dose(history) {
            let earliest = TimePeriod::parse("28d").unwrap().add_to(last_dose.date);
            let overdue = TimePeriod::parse("8w").unwrap().add_to(last_dose.date).pred_opt();
            forecast.status = SeriesStatus::NotComplete;
            forecast.reasons = vec!["NOT_COMPLETE".to_string()];
            forecast.earliest_date = Some(earliest);
            forecast.recommended_date = Some(earliest);
            forecast.overdue_date = overdue;
            forecast.latest_date = None;
            return;
        }
    }

    if matches!(forecast.series_name.as_str(), "COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES" | "COVID_19_AUG_2025_GTE_65_SERIES") {
        if let Some(last_dose) = most_recent_dose(history) {
            let recommendation_date = TimePeriod::parse("8w").unwrap().add_to(last_dose.date);
            forecast.status = if forecast.series_name == "COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES"
                && age_lt(patient.birth_date, eval_date, "19y")
                && last_dose.date < season_start()
                && !history.iter().any(|dose| dose.date >= season_start())
            {
                SeriesStatus::ConditionallyRecommended
            } else {
                SeriesStatus::NotComplete
            };
            forecast.reasons = vec![if forecast.status == SeriesStatus::ConditionallyRecommended {
                "HIGH_RISK"
            } else {
                "NOT_COMPLETE"
            }
            .to_string()];
            forecast.earliest_date = Some(recommendation_date);
            forecast.recommended_date = Some(recommendation_date);
            forecast.overdue_date = None;
            forecast.latest_date = None;
            return;
        }
    }
}

pub fn covid19_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    _candidate_forecasts: &mut std::collections::HashMap<String, VaccineGroupForecast>,
) -> String {
    if age_lt(patient.birth_date, eval_date, "2y") || has_in_season_dose_before_age(patient, history, "2y") {
        return "COVID_19_AUG_2025_LT_2_SERIES".to_string();
    }

    let age_65 = add_years(patient.birth_date, 65);
    let within_12m_of_65 = season_start() < age_65
        && compare_elapsed(season_start(), age_65, &TimePeriod::parse("12m").unwrap())
            != std::cmp::Ordering::Greater;

    if age_ge(patient.birth_date, eval_date, "65y") {
        return "COVID_19_AUG_2025_GTE_65_SERIES".to_string();
    }

    if history.iter().any(|dose| dose.date >= season_start() && dose.date >= age_65) || within_12m_of_65 {
        return "COVID_19_AUG_2025_GTE_65_SERIES".to_string();
    }

    "COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES".to_string()
}
