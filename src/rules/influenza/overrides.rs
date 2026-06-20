use crate::date_utils::SmallVec;
use crate::engine::CandidateForecastsExt;
use lava_cvx_macro::cvx;
use chrono::{Datelike, NaiveDate};
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast, SeriesStatus, VaccineGroupForecast, DoseEvaluation};

pub struct SeasonDates {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub name: &'static str,
}

pub fn get_active_season(eval_date: NaiveDate) -> SeasonDates {
    let year = eval_date.year();
    let (start_year, end_year) = if eval_date.month() >= 7 {
        (year, year + 1)
    } else {
        (year - 1, year)
    };

    let start = NaiveDate::from_ymd_opt(start_year, 7, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(end_year, 6, 30).unwrap();

    let name = match start_year {
        1990 => "19901991_INFLUENZA_SEASON",
        1991 => "19911992_INFLUENZA_SEASON",
        1992 => "19921993_INFLUENZA_SEASON",
        1993 => "19931994_INFLUENZA_SEASON",
        1994 => "19941995_INFLUENZA_SEASON",
        1995 => "19951996_INFLUENZA_SEASON",
        1996 => "19961997_INFLUENZA_SEASON",
        1997 => "19971998_INFLUENZA_SEASON",
        1998 => "19981999_INFLUENZA_SEASON",
        1999 => "19992000_INFLUENZA_SEASON",
        2000 => "20002001_INFLUENZA_SEASON",
        2001 => "20012002_INFLUENZA_SEASON",
        2002 => "20022003_INFLUENZA_SEASON",
        2003 => "20032004_INFLUENZA_SEASON",
        2004 => "20042005_INFLUENZA_SEASON",
        2005 => "20052006_INFLUENZA_SEASON",
        2006 => "20062007_INFLUENZA_SEASON",
        2007 => "20072008_INFLUENZA_SEASON",
        2008 => "20082009_INFLUENZA_SEASON",
        2009 => "20092010_INFLUENZA_SEASON",
        2010 => "20102011_INFLUENZA_SEASON",
        2011 => "20112012_INFLUENZA_SEASON",
        2012 => "20122013_INFLUENZA_SEASON",
        2013 => "20132014_INFLUENZA_SEASON",
        2014 => "20142015_INFLUENZA_SEASON",
        2015 => "20152016_INFLUENZA_SEASON",
        2016 => "20162017_INFLUENZA_SEASON",
        2017 => "20172018_INFLUENZA_SEASON",
        2018 => "20182019_INFLUENZA_SEASON",
        2019 => "20192020_INFLUENZA_SEASON",
        2020 => "20202021_INFLUENZA_SEASON",
        2021 => "20212022_INFLUENZA_SEASON",
        2022 => "20222023_INFLUENZA_SEASON",
        2023 => "20232024_INFLUENZA_SEASON",
        2024 => "20242025_INFLUENZA_SEASON",
        2025 => "20252026_INFLUENZA_SEASON",
        2026 => "20262027_INFLUENZA_SEASON",
        2027 => "20272028_INFLUENZA_SEASON",
        2028 => "20282029_INFLUENZA_SEASON",
        2029 => "20292030_INFLUENZA_SEASON",
        _ => "DEFAULT_INFLUENZA_SEASON",
    };

    SeasonDates { start, end, name }
}

pub fn get_active_season_name(eval_date: NaiveDate) -> &'static str {
    get_active_season(eval_date).name
}

fn is_ice_unsupported_influenza_cvx(cvx_code: u16) -> bool {
    matches!(
        cvx_code,
        // ICE treats these seasonal influenza products as not usable in the
        // supported US influenza schedules represented by this group.
        cvx!("144") | cvx!("161") | cvx!("166") |
        cvx!("194") | cvx!("200") | cvx!("201") | cvx!("202") |
        cvx!("231") | cvx!("331") | cvx!("337")
    )
}

pub fn influenza_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    let Some(dose) = ctx.current_dose else {
        return;
    };

    let dose_season = get_active_season(dose.date);
    if dose.date < dose_season.start || dose.date > dose_season.end {
        *status = DoseStatus::Invalid;
        reasons.retain(|r| *r != EvaluationReason::VaccineNotPartOfSeries);
        if !reasons.contains(&EvaluationReason::OutsideFluVacSeason) {
            reasons.push(EvaluationReason::OutsideFluVacSeason);
        }
    }

    if is_ice_unsupported_influenza_cvx(dose.cvx.0) {
        *status = DoseStatus::Invalid;
        reasons.retain(|r| *r != EvaluationReason::VaccineNotPartOfSeries);
        if !reasons.contains(&EvaluationReason::VaccineNotAllowedInUs) {
            reasons.push(EvaluationReason::VaccineNotAllowedInUs);
        }
    }

    // ICE does not appear to apply the ACIP high-dose age restriction as an
    // invalidating rule in this forecast group. CVX 135/197 can still count
    // toward the seasonal influenza dose requirement in fuzz traces.
}

pub fn influenza_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _evaluations: &[crate::models::DoseEvaluation],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let active_season = get_active_season(eval_date);
    let evals = evaluate_history_seasonally(patient, history);
    let season_valid_count = evals.iter()
        .filter(|e| e.status == DoseStatus::Valid && get_active_season(e.dose_date).name == active_season.name)
        .count();
    let is_1_dose = is_1_dose_season(patient, history, active_season.start);
    let needed = if is_1_dose { 1 } else { 2 };

    if forecast.status == SeriesStatus::Complete && season_valid_count >= needed {
        forecast.status = SeriesStatus::default();
        let next_season_start = active_season.end + chrono::Duration::days(1);
        let mut recommended = next_season_start;

        if let Some(last) = history.iter().max_by_key(|d| d.date) {
            let interval_date = last.date + chrono::Duration::days(28);
            if interval_date > recommended { recommended = interval_date; }
        }

        // ICE's process-results output leaves earliest_date blank once the
        // current influenza season has been satisfied. The next actionable
        // date is carried as recommended_date for the following season.
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(Some(recommended));
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        forecast.reasons = crate::reasons!["NOT_COMPLETE"];
        return;
    }

    // Default dates to start of current season
    let mut earliest = active_season.start;
    let mut recommended = active_season.start;

    // Reset status to NotComplete if the current season still needs more doses
    if season_valid_count < needed {
        forecast.status = SeriesStatus::default();
        forecast.reasons = crate::reasons!["NOT_COMPLETE"];
    }

    // Apply 28-day interval from last dose
    let last_dose = history.iter().max_by_key(|d| d.date);
    if let Some(last) = last_dose {
        let interval_date = last.date + chrono::Duration::days(28);
        
        if interval_date > recommended {
            recommended = interval_date;
        }

        let last_dose_season = get_active_season(last.date);
        if last_dose_season.name == active_season.name {
            if interval_date > earliest {
                earliest = interval_date;
            }
        }
    }

    forecast.status = forecast.status.with_earliest_date(Some(earliest));
    forecast.status = forecast.status.with_recommended_date(Some(recommended));
}

fn count_valid_prior_doses(history: &[Dose], patient: &Patient, active_season_start: NaiveDate) -> usize {
    let mut eligible_doses: Vec<NaiveDate> = history.iter()
        .filter(|d| d.date < active_season_start)
        .filter(|d| !is_ice_unsupported_influenza_cvx(d.cvx.0))
        .filter(|d| {
            let tp_6m_4d = crate::time_period!("6m-4d");
            let abs_min_date = tp_6m_4d.add_to(patient.birth_date);
            d.date >= abs_min_date
        })
        .map(|d| d.date)
        .collect();
    
    eligible_doses.sort();
    eligible_doses.dedup();
    
    let mut valid_count = 0;
    let mut last_valid_date: Option<NaiveDate> = None;
    for date in eligible_doses {
        if let Some(prev) = last_valid_date {
            if (date - prev).num_days() >= 24 {
                valid_count += 1;
                last_valid_date = Some(date);
            }
        } else {
            valid_count += 1;
            last_valid_date = Some(date);
        }
    }
    valid_count
}

fn is_1_dose_season(patient: &Patient, history: &[Dose], season_start: NaiveDate) -> bool {
    let tp_9y = crate::time_period!("9y");
    if season_start >= tp_9y.add_to(patient.birth_date) {
        return true;
    }
    if season_start < NaiveDate::from_ymd_opt(2010, 7, 1).unwrap() {
        return false;
    }
    count_valid_prior_doses(history, patient, season_start) >= 2
}

fn evaluate_history_seasonally(patient: &Patient, history: &[Dose]) -> Vec<DoseEvaluation> {
    let mut evaluations = Vec::new();
    let mut valid_doses_by_season: std::collections::HashMap<String, Vec<NaiveDate>> = std::collections::HashMap::new();

    let mut sorted_history = history.to_vec();
    sorted_history.sort_by_key(|d| d.date);

    for dose in &sorted_history {
        let active_season = get_active_season(dose.date);
        let season_key = active_season.name;

        let mut status = DoseStatus::Valid;
        let mut reasons = SmallVec::<[EvaluationReason; 4]>::new();

        if dose.date < patient.birth_date {
            status = DoseStatus::Invalid;
            reasons.push(EvaluationReason::PriorToDOB);
        } else {
            let is_duplicate = evaluations.iter().any(|e: &DoseEvaluation| {
                e.dose_date == dose.date && e.status != DoseStatus::Invalid
            });
            if is_duplicate {
                status = DoseStatus::Invalid;
                reasons.push(EvaluationReason::DuplicateShotSameDay);
            } else {
                let tp_6m_4d = crate::time_period!("6m-4d");
                if dose.date < tp_6m_4d.add_to(patient.birth_date) {
                    status = DoseStatus::Invalid;
                    reasons.push(EvaluationReason::BelowMinimumAge);
                }

                if is_ice_unsupported_influenza_cvx(dose.cvx.0) {
                    status = DoseStatus::Invalid;
                }

                if status == DoseStatus::Valid {
                    let last_prior_dose_date = evaluations.iter()
                        .filter(|e| {
                            e.dose_date < dose.date &&
                            get_active_season(e.dose_date).name == active_season.name &&
                            e.status != DoseStatus::Invalid
                        })
                        .map(|e| e.dose_date)
                        .last();

                    if let Some(last_date) = last_prior_dose_date {
                        if (dose.date - last_date).num_days() < 24 {
                            status = DoseStatus::Invalid;
                            reasons.push(EvaluationReason::BelowMinimumInterval);
                        }
                    }
                }
            }
        }

        if status == DoseStatus::Valid {
            let is_1_dose = is_1_dose_season(patient, history, active_season.start);
            let max_allowed = if is_1_dose { 1 } else { 2 };
            let season_valid_count = valid_doses_by_season.get(season_key).map_or(0, |v| v.len());
            if season_valid_count >= max_allowed {
                status = DoseStatus::Accepted;
            } else {
                let season_valid_doses = valid_doses_by_season.entry(season_key.to_string()).or_insert_with(Vec::new);
                season_valid_doses.push(dose.date);
            }
        }

        evaluations.push(DoseEvaluation {
            dose_date: dose.date,
            cvx: dose.cvx.clone(),
            status,
            reasons,
            dose_number: Some(valid_doses_by_season.get(season_key).map_or(1, |v| v.len()).max(1)),
            sources: std::collections::HashMap::new(),
        });
    }
    evaluations
}

pub fn influenza_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let active_season = get_active_season(eval_date);
    let use_1_dose = is_1_dose_season(patient, history, active_season.start);
    
    let selected_series_name = if use_1_dose { "INFLUENZA_1_DOSE_SERIES" } else { "INFLUENZA_2_DOSE_SERIES" };

    if let Some(forecast) = candidate_forecasts.get_forecast_mut(selected_series_name) {
        forecast.evaluations = evaluate_history_seasonally(patient, history).into();
    }
    selected_series_name
}
