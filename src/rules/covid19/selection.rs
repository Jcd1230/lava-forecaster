#![allow(dead_code)]

use crate::date_utils::compare_elapsed;
use crate::models::Patient;
use crate::rules::covid19::facts::CovidDoseFact;
use crate::rules::covid19::policy::{CovidSeriesId, CovidSeriesPolicy};
use crate::rules::covid19::seasons::CovidSeason;
use crate::rules::covid19::series::policy_by_id;
use chrono::NaiveDate;

/// Select the Aug 2025 policy using the same high-level branches Java ICE uses in
/// `SeriesSelection.drl`. This helper is not yet wired into production behavior;
/// it exists so future parity work can migrate selection logic one branch at a time.
pub fn select_aug2025_policy(
    patient: &Patient,
    eval_date: NaiveDate,
    facts: &[CovidDoseFact<'_>],
) -> &'static CovidSeriesPolicy {
    if should_select_aug2025_lt2(patient, eval_date, facts) {
        return policy_by_id(CovidSeriesId::Aug2025Lt2).expect("Aug2025 LT2 policy missing");
    }

    if should_select_aug2025_gte65(patient, eval_date, facts) {
        return policy_by_id(CovidSeriesId::Aug2025Age65Plus)
            .expect("Aug2025 65+ policy missing");
    }

    policy_by_id(CovidSeriesId::Aug2025Age2To64).expect("Aug2025 2-64 policy missing")
}

pub fn should_select_aug2025_lt2(
    patient: &Patient,
    eval_date: NaiveDate,
    facts: &[CovidDoseFact<'_>],
) -> bool {
    age_lt(patient, eval_date, crate::time_period!("2y"))
        || facts.iter().any(|fact| {
            fact.season == CovidSeason::Aug2025
                && fact.supported_by_java_covid
                && fact.age_at_dose.under_2_years
        })
}

pub fn should_select_aug2025_gte65(
    patient: &Patient,
    eval_date: NaiveDate,
    facts: &[CovidDoseFact<'_>],
) -> bool {
    let age_65_at_eval = age_ge(patient, eval_date, crate::time_period!("65y"));
    let has_current_season_supported_dose = facts
        .iter()
        .any(|fact| fact.season == CovidSeason::Aug2025 && fact.supported_by_java_covid);

    if age_65_at_eval && !has_current_season_supported_dose {
        return true;
    }

    let first_current_season_dose = facts
        .iter()
        .filter(|fact| fact.season == CovidSeason::Aug2025 && fact.supported_by_java_covid)
        .min_by_key(|fact| fact.raw.date);

    if let Some(first_dose) = first_current_season_dose {
        if first_dose.age_at_dose.at_least_65_years {
            return true;
        }
        if turns_65_within_12_months_of_aug2025_season_start(patient) {
            return true;
        }
    }

    false
}

pub fn should_select_aug2025_age_2_to_64(
    patient: &Patient,
    eval_date: NaiveDate,
    facts: &[CovidDoseFact<'_>],
) -> bool {
    !should_select_aug2025_lt2(patient, eval_date, facts)
        && !should_select_aug2025_gte65(patient, eval_date, facts)
}

fn turns_65_within_12_months_of_aug2025_season_start(patient: &Patient) -> bool {
    let season_start = CovidSeason::Aug2025.start_date();
    let season_start_plus_12m = crate::time_period!("12m").add_to(season_start);
    let birthday_65 = crate::time_period!("65y").add_to(patient.birth_date);
    birthday_65 >= season_start && birthday_65 <= season_start_plus_12m
}

fn age_ge(patient: &Patient, date: NaiveDate, age: crate::date_utils::TimePeriod) -> bool {
    compare_elapsed(patient.birth_date, date, &age) != std::cmp::Ordering::Less
}

fn age_lt(patient: &Patient, date: NaiveDate, age: crate::date_utils::TimePeriod) -> bool {
    compare_elapsed(patient.birth_date, date, &age) == std::cmp::Ordering::Less
}
