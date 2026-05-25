use ice_cvx_macro::cvx;
use chrono::NaiveDate;
use crate::date_utils::{add_years, add_months, compare_elapsed, TimePeriod};
use crate::engine::EvaluationContext;
use crate::models::{Cvx, 
    Dose, DoseStatus, EvaluationReason, Patient, SeriesForecast, SeriesStatus,
    VaccineGroupForecast, DoseEvaluation,
};
use std::collections::HashMap;

fn get_covid_season(date: NaiveDate) -> &'static str {
    if date < NaiveDate::from_ymd_opt(2023, 9, 12).unwrap() {
        "COVID_19_DEC_2020_SEASON"
    } else if date < NaiveDate::from_ymd_opt(2024, 8, 22).unwrap() {
        "COVID_19_SEP_2023_SEASON"
    } else if date < NaiveDate::from_ymd_opt(2025, 8, 27).unwrap() {
        "COVID_19_AUG_2024_SEASON"
    } else {
        "COVID_19_AUG_2025_SEASON"
    }
}

fn get_season_start_date(season: &str) -> NaiveDate {
    match season {
        "COVID_19_DEC_2020_SEASON" => NaiveDate::from_ymd_opt(2020, 12, 14).unwrap(),
        "COVID_19_SEP_2023_SEASON" => NaiveDate::from_ymd_opt(2023, 9, 12).unwrap(),
        "COVID_19_AUG_2024_SEASON" => NaiveDate::from_ymd_opt(2024, 8, 22).unwrap(),
        _ => NaiveDate::from_ymd_opt(2025, 8, 27).unwrap(), // COVID_19_AUG_2025_SEASON
    }
}

fn season_start() -> NaiveDate {
    NaiveDate::from_ymd_opt(2025, 8, 27).unwrap()
}

fn age_ge(birth_date: NaiveDate, date_to_check: NaiveDate, age: &str) -> bool {
    compare_elapsed(birth_date, date_to_check, &TimePeriod::parse(age).unwrap())
        != std::cmp::Ordering::Less
}

fn age_lt(birth_date: NaiveDate, date_to_check: NaiveDate, age: &str) -> bool {
    compare_elapsed(birth_date, date_to_check, &TimePeriod::parse(age).unwrap())
        == std::cmp::Ordering::Less
}

fn has_in_season_dose_before_age(patient: &Patient, history: &[Dose], age: &str) -> bool {
    let cutoff = TimePeriod::parse(age).unwrap().add_to(patient.birth_date);
    history
        .iter()
        .any(|dose| dose.date >= season_start() && dose.date < cutoff)
}

fn get_min_interval_days(
    patient: &Patient,
    prior_dose: &DoseEvaluation,
    current_dose_cvx: Cvx,
    current_dose_date: NaiveDate,
    active_series_name: &str,
    season_valid_doses_len: usize,
) -> i64 {
    let season = get_covid_season(current_dose_date);
    
    let is_pfizer = |cvx: Cvx| {
        matches!(cvx.0, cvx!("208") | cvx!("217") | cvx!("218") | cvx!("219") | cvx!("300") | cvx!("301") | cvx!("302") | cvx!("308") | cvx!("309") | cvx!("310") | cvx!("502"))
    };
    let is_novavax = |cvx: Cvx| matches!(cvx.0, cvx!("211") | cvx!("313"));
    let is_unspecified = |cvx: Cvx| cvx.0 == cvx!("213");

    let is_same_brand_or_unspec = (is_pfizer(prior_dose.cvx) && is_pfizer(current_dose_cvx))
        || (is_novavax(prior_dose.cvx) && is_novavax(current_dose_cvx))
        || is_unspecified(prior_dose.cvx)
        || is_unspecified(current_dose_cvx);

    if season == "COVID_19_AUG_2025_SEASON" {
        if active_series_name == "COVID_19_AUG_2025_LT_2_SERIES" {
            if is_same_brand_or_unspec {
                17
            } else {
                24
            }
        } else {
            52
        }
    } else if season == "COVID_19_DEC_2020_SEASON" {
        if season_valid_doses_len == 1 {
            if is_same_brand_or_unspec {
                17
            } else {
                24
            }
        } else {
            52
        }
    } else {
        let season_start_dt = get_season_start_date(season);
        let age_at_season_start = compare_elapsed(patient.birth_date, season_start_dt, &TimePeriod::parse("5y").unwrap());
        if season_valid_doses_len == 1 {
            if age_at_season_start == std::cmp::Ordering::Less {
                if is_same_brand_or_unspec {
                    17
                } else {
                    24
                }
            } else {
                let is_novavax_or_unspec = |cvx: Cvx| is_novavax(cvx) || is_unspecified(cvx);
                if is_novavax_or_unspec(prior_dose.cvx) && is_novavax_or_unspec(current_dose_cvx) && is_same_brand_or_unspec {
                    17
                } else {
                    52
                }
            }
        } else {
            52
        }
    }
}

pub fn evaluate_doses_seasonally(
    patient: &Patient,
    history: &[Dose],
    active_series_name: &str,
) -> Vec<DoseEvaluation> {
    let mut evaluations = Vec::new();
    let mut sorted_history = history.to_vec();
    sorted_history.sort_by_key(|d| d.date);

    let mut valid_doses_by_season: HashMap<String, Vec<NaiveDate>> = HashMap::new();

    for (i, dose) in sorted_history.iter().enumerate() {
        let season = get_covid_season(dose.date);
        let mut status = DoseStatus::Valid;
        let mut reasons = Vec::new();

        // 1. DOB check
        if dose.date < patient.birth_date {
            status = DoseStatus::Invalid;
            reasons.push(EvaluationReason::PriorToDOB);
        }
        // 2. Same-day duplicate check
        else if i > 0 && sorted_history[i - 1].date == dose.date {
            status = DoseStatus::Invalid;
            reasons.push(EvaluationReason::DuplicateShotSameDay);
        }
        else {
            // 3. Authorization check
            let mut auth_ok = true;
            if (dose.cvx.0 == cvx!("308") || dose.cvx.0 == cvx!("309") || dose.cvx.0 == cvx!("310") || dose.cvx.0 == cvx!("311") || dose.cvx.0 == cvx!("312"))
                && dose.date < NaiveDate::from_ymd_opt(2023, 9, 11).unwrap()
            {
                auth_ok = false;
            }
            if dose.cvx.0 == cvx!("313") && dose.date < NaiveDate::from_ymd_opt(2023, 10, 3).unwrap() {
                auth_ok = false;
            }
            let is_pandemic_cvx = matches!(dose.cvx.0, cvx!("207") | cvx!("208") | cvx!("211") | cvx!("212") | cvx!("217") | cvx!("218") | cvx!("219") | cvx!("221") | cvx!("228") | cvx!("229") | cvx!("272") | cvx!("300") | cvx!("301") | cvx!("302") | cvx!("502") | cvx!("519"));
            if is_pandemic_cvx && dose.date >= NaiveDate::from_ymd_opt(2023, 9, 12).unwrap() {
                auth_ok = false;
            }
            if dose.cvx.0 == cvx!("334") && dose.date < NaiveDate::from_ymd_opt(2025, 8, 27).unwrap() {
                auth_ok = false;
            }

            if !auth_ok {
                status = DoseStatus::Invalid;
                reasons.push(EvaluationReason::VaccineNotPartOfSeries);
            } else {
                // 4. Age check
                let mut is_valid_age = true;
                if season == "COVID_19_AUG_2025_SEASON" {
                    if active_series_name == "COVID_19_AUG_2025_GTE_65_SERIES" {
                        let age_65 = add_years(patient.birth_date, 65);
                        let within_12m_of_65 = season_start() < age_65
                            && compare_elapsed(season_start(), age_65, &TimePeriod::parse("12m").unwrap())
                                != std::cmp::Ordering::Greater;
                        if dose.date < age_65 && !within_12m_of_65 {
                            is_valid_age = false;
                        }
                    } else if active_series_name == "COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES" {
                        let age_2y = add_years(patient.birth_date, 2);
                        if dose.date < age_2y {
                            is_valid_age = false;
                        }
                    } else {
                        let age_6m_4d = TimePeriod::parse("6m-4d").unwrap().add_to(patient.birth_date);
                        if dose.date < age_6m_4d {
                            is_valid_age = false;
                        }
                    }
                } else if season == "COVID_19_DEC_2020_SEASON" {
                    let age_0d = patient.birth_date;
                    if dose.date < age_0d {
                        is_valid_age = false;
                    }
                } else {
                    let age_6m_4d = TimePeriod::parse("6m-4d").unwrap().add_to(patient.birth_date);
                    if dose.date < age_6m_4d {
                        is_valid_age = false;
                    }
                }

                if !is_valid_age {
                    status = DoseStatus::Invalid;
                    reasons.push(EvaluationReason::BelowMinimumAge);
                } else {
                    // Novavax GTE 5y season start restriction in prior seasons
                    let is_novavax = matches!(dose.cvx.0, cvx!("211") | cvx!("313"));
                    if is_novavax && (season == "COVID_19_SEP_2023_SEASON" || season == "COVID_19_AUG_2024_SEASON") {
                        let season_start_dt = get_season_start_date(season);
                        let age_at_season_start = compare_elapsed(patient.birth_date, season_start_dt, &TimePeriod::parse("5y").unwrap());
                        let novavax_count_in_season = sorted_history.iter()
                            .filter(|d| {
                                let d_season = get_covid_season(d.date);
                                d_season == season && (d.cvx.0 == cvx!("211") || d.cvx.0 == cvx!("313"))
                            })
                            .count();
                        if age_at_season_start != std::cmp::Ordering::Less && novavax_count_in_season < 2 {
                            let age_12y = add_years(patient.birth_date, 12);
                            if dose.date < age_12y {
                                status = DoseStatus::Accepted;
                                reasons.push(EvaluationReason::VaccineNotPartOfSeries);
                            }
                        }
                    }
                }
            }

            // 5. Interval checks (relative to the last non-duplicate/non-DOB-invalid dose in history)
            if status == DoseStatus::Valid || status == DoseStatus::Accepted {
                let prior_valid_doses: Vec<&DoseEvaluation> = evaluations.iter()
                    .filter(|e: &&DoseEvaluation| {
                        e.status != DoseStatus::Invalid 
                            || (!e.reasons.contains(&EvaluationReason::PriorToDOB) 
                                && !e.reasons.contains(&EvaluationReason::DuplicateShotSameDay))
                    })
                    .collect();

                if let Some(last_prior) = prior_valid_doses.last() {
                    let days_since_last = (dose.date - last_prior.dose_date).num_days();
                    
                    let season_key = if season == "COVID_19_AUG_2025_SEASON" {
                        let age_65_date = add_years(patient.birth_date, 65);
                        if dose.date >= age_65_date {
                            "COVID_19_AUG_2025_SEASON_GTE65".to_string()
                        } else {
                            "COVID_19_AUG_2025_SEASON".to_string()
                        }
                    } else {
                        let is_pediatric = matches!(dose.cvx.0, cvx!("219") | cvx!("228") | cvx!("272") | cvx!("302") | cvx!("308") | cvx!("311"));
                        if is_pediatric {
                            "PRIOR_SEASONS_LT5".to_string()
                        } else {
                            let season_start_dt = get_season_start_date(season);
                            let age_at_season_start = compare_elapsed(patient.birth_date, season_start_dt, &TimePeriod::parse("5y").unwrap());
                            if age_at_season_start != std::cmp::Ordering::Less {
                                season.to_string()
                            } else {
                                "PRIOR_SEASONS_LT5".to_string()
                            }
                        }
                    };

                    let season_valid_doses_len = valid_doses_by_season.get(&season_key).map(|v| v.len()).unwrap_or(0);

                    let mut min_interval_days = get_min_interval_days(
                        patient,
                        last_prior,
                        dose.cvx,
                        dose.date,
                        active_series_name,
                        season_valid_doses_len,
                    );

                    // Adjust if skipped in LT 2y series due to >= 2 prior valid doses
                    if active_series_name == "COVID_19_AUG_2025_LT_2_SERIES" && season == "COVID_19_AUG_2025_SEASON" {
                        let prior_valid_count = evaluations.iter()
                            .filter(|e| e.status == DoseStatus::Valid && e.dose_date < season_start())
                            .count();
                        if prior_valid_count >= 2 {
                            min_interval_days = 52;
                        }
                    }

                    if days_since_last < min_interval_days {
                        status = DoseStatus::Invalid;
                        reasons.retain(|r| *r != EvaluationReason::BelowMinimumAge);
                        if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                            reasons.push(EvaluationReason::BelowMinimumInterval);
                        }
                    }
                }
            }
        }

        // 6. Dose numbering
        let season_key = if season == "COVID_19_AUG_2025_SEASON" {
            let age_65_date = add_years(patient.birth_date, 65);
            if dose.date >= age_65_date {
                "COVID_19_AUG_2025_SEASON_GTE65".to_string()
            } else {
                "COVID_19_AUG_2025_SEASON".to_string()
            }
        } else {
            let is_pediatric = matches!(dose.cvx.0, cvx!("219") | cvx!("228") | cvx!("272") | cvx!("302") | cvx!("308") | cvx!("311"));
            if is_pediatric {
                "PRIOR_SEASONS_LT5".to_string()
            } else {
                let season_start_dt = get_season_start_date(season);
                let age_at_season_start = compare_elapsed(patient.birth_date, season_start_dt, &TimePeriod::parse("5y").unwrap());
                if age_at_season_start != std::cmp::Ordering::Less {
                    season.to_string()
                } else {
                    "PRIOR_SEASONS_LT5".to_string()
                }
            }
        };

        let mut dose_number = if status == DoseStatus::Valid {
            let season_valid_doses = valid_doses_by_season.entry(season_key.clone()).or_insert_with(Vec::new);
            season_valid_doses.push(dose.date);
            
            let mut num = season_valid_doses.len();
            if active_series_name == "COVID_19_AUG_2025_LT_2_SERIES" && season == "COVID_19_AUG_2025_SEASON" {
                let prior_moderna_count = sorted_history.iter()
                    .filter(|d| d.date < season_start() && (d.cvx.0 == cvx!("311") || d.cvx.0 == cvx!("312")))
                    .count();
                let prior_valid_count = sorted_history.iter()
                    .filter(|d| d.date < season_start())
                    .filter(|d| {
                        matches!(d.cvx.0, cvx!("213") | cvx!("308") | cvx!("309") | cvx!("310") | cvx!("311") | cvx!("312") | cvx!("313"))
                    })
                    .count();
                let is_skipped = prior_moderna_count == 1 || prior_valid_count >= 2;
                if is_skipped {
                    num += 1;
                }
            }
            num
        } else {
            let season_valid_doses = valid_doses_by_season.entry(season_key.clone()).or_insert_with(Vec::new);
            let mut num = season_valid_doses.len() + 1;
            if active_series_name == "COVID_19_AUG_2025_LT_2_SERIES" && season == "COVID_19_AUG_2025_SEASON" {
                let prior_moderna_count = sorted_history.iter()
                    .filter(|d| d.date < season_start() && (d.cvx.0 == cvx!("311") || d.cvx.0 == cvx!("312")))
                    .count();
                let prior_valid_count = sorted_history.iter()
                    .filter(|d| d.date < season_start())
                    .filter(|d| {
                        matches!(d.cvx.0, cvx!("213") | cvx!("308") | cvx!("309") | cvx!("310") | cvx!("311") | cvx!("312") | cvx!("313"))
                    })
                    .count();
                let is_skipped = prior_moderna_count == 1 || prior_valid_count >= 2;
                if is_skipped {
                    num += 1;
                }
            }
            num
        };

        // Dose number capping
        let cap = if season_key == "PRIOR_SEASONS_LT5" {
            3
        } else if season == "COVID_19_AUG_2025_SEASON" {
            if active_series_name == "COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES" {
                1
            } else {
                2
            }
        } else {
            3
        };
        dose_number = std::cmp::min(dose_number, cap);

        evaluations.push(DoseEvaluation {
            dose_date: dose.date,
            cvx: dose.cvx.clone(),
            status,
            reasons,
            dose_number: Some(dose_number),
        });
    }

    evaluations
}

pub fn covid19_custom_evaluation_hook(
    series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        let evals = evaluate_doses_seasonally(ctx.patient, ctx.history, series_name);
        if let Some(matching_eval) = evals.iter().find(|e| e.dose_date == dose.date && e.cvx == dose.cvx) {
            *status = matching_eval.status;
            reasons.clear();
            reasons.extend(matching_eval.reasons.clone());
        }
    }
}

pub fn covid19_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let active_series_name = &forecast.series_name;
    let evaluations = evaluate_doses_seasonally(patient, history, active_series_name);

    let current_season_valid_doses: Vec<&DoseEvaluation> = evaluations.iter()
        .filter(|e| e.status == DoseStatus::Valid && get_covid_season(e.dose_date) == "COVID_19_AUG_2025_SEASON")
        .collect();

    let prior_season_valid_doses: Vec<&DoseEvaluation> = evaluations.iter()
        .filter(|e| e.status == DoseStatus::Valid && get_covid_season(e.dose_date) != "COVID_19_AUG_2025_SEASON")
        .collect();

    let last_prior_dose_date = evaluations.iter()
        .filter(|e| {
            e.dose_date < season_start()
                && !e.reasons.contains(&EvaluationReason::PriorToDOB)
                && !e.reasons.contains(&EvaluationReason::DuplicateShotSameDay)
        })
        .map(|e| e.dose_date)
        .max();

    let last_current_season_dose = history.iter()
        .filter(|d| get_covid_season(d.date) == "COVID_19_AUG_2025_SEASON")
        .max_by_key(|d| d.date);

    let age_6m = TimePeriod::parse("6m").unwrap().add_to(patient.birth_date);
    let age_2y = add_years(patient.birth_date, 2);
    let age_65y = add_years(patient.birth_date, 65);

    if active_series_name == "COVID_19_AUG_2025_LT_2_SERIES" {
        // Skip check
        let prior_moderna_count = history.iter()
            .filter(|d| d.date < season_start() && (d.cvx.0 == cvx!("311") || d.cvx.0 == cvx!("312")))
            .count();
        let prior_valid_count = history.iter()
            .filter(|d| d.date < season_start())
            .filter(|d| {
                matches!(d.cvx.0, cvx!("213") | cvx!("308") | cvx!("309") | cvx!("310") | cvx!("311") | cvx!("312") | cvx!("313"))
            })
            .count();
        let is_skipped = prior_moderna_count == 1 || prior_valid_count >= 2;

        if is_skipped {
            if current_season_valid_doses.len() >= 1 {
                forecast.status = SeriesStatus::Complete;
                forecast.earliest_date = None;
                forecast.recommended_date = None;
                forecast.overdue_date = None;
                forecast.latest_date = None;
                forecast.reasons = vec!["COMPLETE_HIGH_RISK".to_string()];
            } else {
                forecast.status = SeriesStatus::NotComplete;
                forecast.reasons = vec!["NOT_COMPLETE".to_string()];
                
                let mut earliest = season_start().max(age_6m);
                let mut recommended = season_start().max(age_6m);

                if let Some(last_dose) = last_current_season_dose {
                    earliest = earliest.max(last_dose.date + chrono::Duration::days(56));
                    recommended = recommended.max(last_dose.date + chrono::Duration::days(56));
                } else if let Some(last) = last_prior_dose_date {
                    earliest = earliest.max(last + chrono::Duration::days(56));
                    recommended = recommended.max(last + chrono::Duration::days(56));
                }

                forecast.earliest_date = Some(earliest);
                forecast.recommended_date = Some(recommended);
                forecast.overdue_date = None;
                forecast.latest_date = None;
            }
        } else {
            // Not skipped: needs 2 doses in current season
            if current_season_valid_doses.len() >= 2 {
                forecast.status = SeriesStatus::Complete;
                forecast.earliest_date = None;
                forecast.recommended_date = None;
                forecast.overdue_date = None;
                forecast.latest_date = None;
                forecast.reasons = vec!["COMPLETE_HIGH_RISK".to_string()];
            } else if current_season_valid_doses.len() == 1 {
                forecast.status = SeriesStatus::NotComplete;
                forecast.reasons = vec!["NOT_COMPLETE".to_string()];

                let anchor_dose = last_current_season_dose.unwrap();
                let earliest = anchor_dose.date + chrono::Duration::days(28);
                let recommended = anchor_dose.date + chrono::Duration::days(28);
                let overdue = TimePeriod::parse("8w").unwrap().add_to(anchor_dose.date).pred_opt();

                forecast.earliest_date = Some(earliest);
                forecast.recommended_date = Some(recommended);
                forecast.overdue_date = overdue;
                forecast.latest_date = None;
            } else {
                // 0 current season doses
                forecast.status = SeriesStatus::NotComplete;
                forecast.reasons = vec!["NOT_COMPLETE".to_string()];

                if let Some(last_dose) = last_current_season_dose {
                    let earliest = last_dose.date + chrono::Duration::days(28);
                    let recommended = last_dose.date + chrono::Duration::days(28);
                    forecast.earliest_date = Some(earliest);
                    forecast.recommended_date = Some(recommended);
                } else {
                    let mut earliest = season_start().max(age_6m);
                    let mut recommended = season_start().max(age_6m);
                    if let Some(last) = last_prior_dose_date {
                        let last_dose = history.iter().find(|d| d.date == last).unwrap();
                        let is_pfizer_novavax_unspec = matches!(last_dose.cvx.0, cvx!("208") | cvx!("217") | cvx!("218") | cvx!("219") | cvx!("300") | cvx!("301") | cvx!("302") | cvx!("308") | cvx!("309") | cvx!("310") | cvx!("211") | cvx!("313") | cvx!("213"));
                        if is_pfizer_novavax_unspec {
                            earliest = earliest.max(last + chrono::Duration::days(17));
                            recommended = recommended.max(last + chrono::Duration::days(21));
                        } else {
                            earliest = earliest.max(last + chrono::Duration::days(24));
                            recommended = recommended.max(last + chrono::Duration::days(28));
                        }
                    }
                    forecast.earliest_date = Some(earliest);
                    forecast.recommended_date = Some(recommended);
                }
                forecast.overdue_date = None;
                forecast.latest_date = None;
            }
        }

        // Clamp to minimum age for LT 2y series
        forecast.earliest_date = forecast.earliest_date.map(|d| d.max(season_start()).max(age_6m));
        forecast.recommended_date = forecast.recommended_date.map(|d| d.max(season_start()).max(age_6m));

    } else if active_series_name == "COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES" {
        if current_season_valid_doses.len() >= 1 {
            forecast.status = SeriesStatus::Complete;
            forecast.earliest_date = None;
            forecast.recommended_date = None;
            forecast.overdue_date = None;
            forecast.latest_date = None;
            forecast.reasons = vec!["COMPLETE_HIGH_RISK".to_string()];
        } else {
            let is_under_19 = eval_date < add_years(patient.birth_date, 19);
            if is_under_19 {
                if prior_season_valid_doses.is_empty() {
                    forecast.status = SeriesStatus::NotComplete;
                    forecast.reasons = vec!["NOT_COMPLETE".to_string()];
                } else {
                    forecast.status = SeriesStatus::ConditionallyRecommended;
                    forecast.reasons = vec!["HIGH_RISK".to_string()];
                }
            } else {
                forecast.status = SeriesStatus::NotComplete;
                forecast.reasons = vec!["NOT_COMPLETE".to_string()];
            }

            if let Some(last_dose) = last_current_season_dose {
                let earliest = last_dose.date + chrono::Duration::days(56);
                let recommended = last_dose.date + chrono::Duration::days(56);
                forecast.earliest_date = Some(earliest);
                forecast.recommended_date = Some(recommended);
            } else {
                let mut earliest = season_start().max(age_6m);
                let mut recommended = season_start().max(age_6m);

                if let Some(last) = last_prior_dose_date {
                    let last_dose = history.iter().find(|d| d.date == last).unwrap();
                    if last_dose.cvx.0 == cvx!("313") {
                        earliest = earliest.max(last + chrono::Duration::days(17));
                        recommended = recommended.max(last + chrono::Duration::days(17));
                    } else {
                        earliest = earliest.max(last + chrono::Duration::days(52));
                        recommended = recommended.max(last + chrono::Duration::days(56));
                    }
                }

                forecast.earliest_date = Some(earliest);
                forecast.recommended_date = Some(recommended);
            }
            forecast.overdue_date = None;
            forecast.latest_date = None;
        }

        // Clamp to minimum age for 2-64y series
        forecast.earliest_date = forecast.earliest_date.map(|d| d.max(season_start()).max(age_6m));
        forecast.recommended_date = forecast.recommended_date.map(|d| d.max(season_start()).max(age_6m));

    } else if active_series_name == "COVID_19_AUG_2025_GTE_65_SERIES" {
        if current_season_valid_doses.len() >= 2 {
            forecast.status = SeriesStatus::Complete;
            forecast.earliest_date = None;
            forecast.recommended_date = None;
            forecast.overdue_date = None;
            forecast.latest_date = None;
            forecast.reasons = vec!["COMPLETE_HIGH_RISK".to_string()];
        } else if current_season_valid_doses.len() == 1 {
            forecast.status = SeriesStatus::NotComplete;
            forecast.reasons = vec!["NOT_COMPLETE".to_string()];
            
            let anchor_dose = last_current_season_dose.unwrap();
            let earliest = anchor_dose.date + chrono::Duration::days(56);
            let recommended = add_months(anchor_dose.date, 6);

            forecast.earliest_date = Some(earliest);
            forecast.recommended_date = Some(recommended);
            forecast.overdue_date = None;
            forecast.latest_date = None;
        } else {
            // 0 current season doses
            forecast.status = SeriesStatus::NotComplete;
            forecast.reasons = vec!["NOT_COMPLETE".to_string()];

            if let Some(last_dose) = last_current_season_dose {
                let earliest = last_dose.date + chrono::Duration::days(56);
                let recommended = last_dose.date + chrono::Duration::days(56);
                forecast.earliest_date = Some(earliest);
                forecast.recommended_date = Some(recommended);
            } else {
                let mut earliest = season_start().max(age_65y);
                let mut recommended = season_start().max(age_65y);

                if let Some(last) = last_prior_dose_date {
                    let last_dose = history.iter().find(|d| d.date == last).unwrap();
                    if last_dose.cvx.0 == cvx!("313") {
                        earliest = earliest.max(last + chrono::Duration::days(17));
                        recommended = recommended.max(last + chrono::Duration::days(17));
                    } else {
                        earliest = earliest.max(last + chrono::Duration::days(52));
                        recommended = recommended.max(last + chrono::Duration::days(56));
                    }
                }

                forecast.earliest_date = Some(earliest);
                forecast.recommended_date = Some(recommended);
            }
            forecast.overdue_date = None;
            forecast.latest_date = None;
        }

        // Clamp to minimum age for GTE 65y series
        forecast.earliest_date = forecast.earliest_date.map(|d| d.max(season_start()).max(age_65y));
        forecast.recommended_date = forecast.recommended_date.map(|d| d.max(season_start()).max(age_65y));
    }
}

pub fn covid19_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut std::collections::HashMap<String, VaccineGroupForecast>,
) -> String {
    let selected_series_name = if age_lt(patient.birth_date, eval_date, "2y") 
        || has_in_season_dose_before_age(patient, history, "2y") 
    {
        "COVID_19_AUG_2025_LT_2_SERIES".to_string()
    } else {
        let age_65 = add_years(patient.birth_date, 65);
        let within_12m_of_65 = season_start() < age_65
            && compare_elapsed(season_start(), age_65, &TimePeriod::parse("12m").unwrap())
                != std::cmp::Ordering::Greater;

        if age_ge(patient.birth_date, eval_date, "65y") 
            || history.iter().any(|dose| dose.date >= season_start() && dose.date >= age_65) 
            || within_12m_of_65 
        {
            "COVID_19_AUG_2025_GTE_65_SERIES".to_string()
        } else {
            "COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES".to_string()
        }
    };

    if let Some(forecast) = candidate_forecasts.get_mut(&selected_series_name) {
        forecast.evaluations = evaluate_doses_seasonally(patient, history, &selected_series_name);
    }

    selected_series_name
}
