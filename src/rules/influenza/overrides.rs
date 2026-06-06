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
    // Statically defined seasons
    if eval_date >= NaiveDate::from_ymd_opt(2012, 7, 1).unwrap() && eval_date <= NaiveDate::from_ymd_opt(2013, 6, 30).unwrap() {
        return SeasonDates {
            start: NaiveDate::from_ymd_opt(2012, 7, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2013, 6, 30).unwrap(),
            name: "20122013_INFLUENZA_SEASON",
        };
    }
    if eval_date >= NaiveDate::from_ymd_opt(2013, 7, 1).unwrap() && eval_date <= NaiveDate::from_ymd_opt(2014, 6, 30).unwrap() {
        return SeasonDates {
            start: NaiveDate::from_ymd_opt(2013, 7, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2014, 6, 30).unwrap(),
            name: "20132014_INFLUENZA_SEASON",
        };
    }
    if eval_date >= NaiveDate::from_ymd_opt(2014, 7, 1).unwrap() && eval_date <= NaiveDate::from_ymd_opt(2015, 6, 30).unwrap() {
        return SeasonDates {
            start: NaiveDate::from_ymd_opt(2014, 7, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2015, 6, 30).unwrap(),
            name: "20142015_INFLUENZA_SEASON",
        };
    }
    if eval_date >= NaiveDate::from_ymd_opt(2015, 7, 1).unwrap() && eval_date <= NaiveDate::from_ymd_opt(2016, 6, 30).unwrap() {
        return SeasonDates {
            start: NaiveDate::from_ymd_opt(2015, 7, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2016, 6, 30).unwrap(),
            name: "20152016_INFLUENZA_SEASON",
        };
    }

    // Default/Dynamic seasons (before 2012 or after 2016)
    let year = eval_date.year();
    if eval_date.month() >= 7 {
        SeasonDates {
            start: NaiveDate::from_ymd_opt(year, 7, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(year + 1, 6, 30).unwrap(),
            name: "20152016_INFLUENZA_SEASON", // Treated as 2015-2016 name for rules selection
        }
    } else {
        SeasonDates {
            start: NaiveDate::from_ymd_opt(year - 1, 7, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(year, 6, 30).unwrap(),
            name: "20152016_INFLUENZA_SEASON",
        }
    }
}

pub fn get_active_season_name(eval_date: NaiveDate) -> &'static str {
    if eval_date >= NaiveDate::from_ymd_opt(2012, 7, 1).unwrap() && eval_date <= NaiveDate::from_ymd_opt(2013, 6, 30).unwrap() {
        "20122013_INFLUENZA_SEASON"
    } else if eval_date >= NaiveDate::from_ymd_opt(2013, 7, 1).unwrap() && eval_date <= NaiveDate::from_ymd_opt(2014, 6, 30).unwrap() {
        "20132014_INFLUENZA_SEASON"
    } else if eval_date >= NaiveDate::from_ymd_opt(2014, 7, 1).unwrap() && eval_date <= NaiveDate::from_ymd_opt(2015, 6, 30).unwrap() {
        "20142015_INFLUENZA_SEASON"
    } else if eval_date >= NaiveDate::from_ymd_opt(2015, 7, 1).unwrap() && eval_date <= NaiveDate::from_ymd_opt(2016, 6, 30).unwrap() {
        "20152016_INFLUENZA_SEASON"
    } else {
        "DEFAULT_INFLUENZA_SEASON"
    }
}

pub fn influenza_custom_evaluation_hook(
    _series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut SmallVec<[EvaluationReason; 4]>,
    status: &mut DoseStatus,
) {
    let Some(dose) = ctx.current_dose else {
        return;
    };

    // 1. Season Boundary Check
    let active_season = get_active_season(ctx.eval_date);
    if dose.date < active_season.start || dose.date > active_season.end {
        *status = DoseStatus::Invalid;
        reasons.retain(|r| *r != EvaluationReason::VaccineNotPartOfSeries);
        if !reasons.contains(&EvaluationReason::OutsideFluVacSeason) {
            reasons.push(EvaluationReason::OutsideFluVacSeason);
        }
    }

    // 2. 24-day absolute minimum interval between seasons
    if target_dose_idx == 1 {
        let prior_dose = ctx.history.iter()
            .filter(|d| d.date < active_season.start)
            .max_by_key(|d| d.date);
        
        if let Some(prior) = prior_dose {
            if (dose.date - prior.date).num_days() < 24 {
                *status = DoseStatus::Invalid;
                if !reasons.contains(&EvaluationReason::BelowMinimumInterval) {
                    reasons.push(EvaluationReason::BelowMinimumInterval);
                }
            }
        }
    }

    // 3. Vaccine Not Allowed In US
    if matches!(dose.cvx.0, cvx!("194") | cvx!("200") | cvx!("201") | cvx!("202") | cvx!("231") | cvx!("331") | cvx!("337")) {
        *status = DoseStatus::Invalid;
        reasons.retain(|r| *r != EvaluationReason::VaccineNotPartOfSeries);
        if !reasons.contains(&EvaluationReason::VaccineNotAllowedInUs) {
            reasons.push(EvaluationReason::VaccineNotAllowedInUs);
        }
    }

    // 4. CVX 161 Age Limit and Suppression
    if dose.cvx.0 == cvx!("161") {
        let tp_3y = crate::time_period!("3y-1d");
        let limit = tp_3y.add_to(ctx.patient.birth_date);
        if dose.date > limit {
            *status = DoseStatus::Invalid;
            reasons.retain(|r| *r != EvaluationReason::VaccineNotPartOfSeries);
            
            let tp_9y = crate::time_period!("9y");
            let age_9y_date = tp_9y.add_to(ctx.patient.birth_date);
            let is_patient_ge_9y = ctx.eval_date >= age_9y_date || dose.date >= age_9y_date;

            if !is_patient_ge_9y {
                if !reasons.contains(&EvaluationReason::InsufficientAntigen) {
                    reasons.push(EvaluationReason::InsufficientAntigen);
                }
            } else {
                reasons.retain(|r| *r != EvaluationReason::InsufficientAntigen);
            }
        }
    }
}

pub fn influenza_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    history: &[Dose],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let active_season = get_active_season(eval_date);

    if forecast.status == SeriesStatus::Complete {
        forecast.status = SeriesStatus::default();
        forecast.status = forecast.status.with_earliest_date(None);
        forecast.status = forecast.status.with_recommended_date(Some(active_season.end + chrono::Duration::days(1)));
        forecast.status = forecast.status.with_overdue_date(None);
        forecast.status = forecast.status.with_latest_date(None);
        forecast.reasons = crate::reasons!["NOT_COMPLETE"];
        return;
    }

    // Clamp earliest and recommended dates to start of current season
    if let Some(Some(earliest)) = forecast.status.earliest_date_mut() {
        if *earliest < active_season.start {
            *earliest = active_season.start;
        }
    } else {
        forecast.status = forecast.status.with_earliest_date(Some(active_season.start));
    }

    if let Some(Some(recommended)) = forecast.status.recommended_date_mut() {
        if *recommended < active_season.start {
            *recommended = active_season.start;
        }
    } else {
        forecast.status = forecast.status.with_recommended_date(Some(active_season.start));
    }

    if let Some(rec_date) = forecast.status.recommended_date() {
        if rec_date > active_season.end {
            let last_dose = history.iter().max_by_key(|d| d.date);
            if let Some(last) = last_dose {
                let tp_6m_4d = crate::time_period!("6m-4d");
                let age_6m_4d = tp_6m_4d.add_to(patient.birth_date);
                if last.date >= age_6m_4d {
                    forecast.status = forecast.status.with_recommended_date(Some(last.date + chrono::Duration::days(28)));
                }
            }
        }
    }

    // Spacing check from invalid doses
    let last_dose = history.iter().max_by_key(|d| d.date);
    if let Some(last) = last_dose {
        let is_valid = _valid_doses.iter().any(|(date, _)| *date == last.date);
        if !is_valid {
            let tp_6m_4d = crate::time_period!("6m-4d");
            let age_6m_4d = tp_6m_4d.add_to(patient.birth_date);
            if last.date >= age_6m_4d {
                let repeat_date = last.date + chrono::Duration::days(28);
                if let Some(Some(earliest)) = forecast.status.earliest_date_mut() {
                    if *earliest < repeat_date {
                        *earliest = repeat_date;
                    }
                } else {
                    forecast.status = forecast.status.with_earliest_date(Some(repeat_date));
                }
                if let Some(Some(recommended)) = forecast.status.recommended_date_mut() {
                    if *recommended < repeat_date {
                        *recommended = repeat_date;
                    }
                } else {
                    forecast.status = forecast.status.with_recommended_date(Some(repeat_date));
                }
            }
        }
    }
}

fn count_valid_prior_doses(history: &[Dose], patient: &Patient, active_season_start: NaiveDate) -> usize {
    let disallowed_cvx = [cvx!("194"), cvx!("200"), cvx!("201"), cvx!("202"), cvx!("231"), cvx!("331"), cvx!("337")];
    let mut eligible_doses: Vec<NaiveDate> = history.iter()
        .filter(|d| d.date < active_season_start)
        .filter(|d| !disallowed_cvx.contains(&d.cvx.0))
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

fn check_1dose_conditions_2012_2014(history: &[Dose], patient: &Patient, active_season_start: NaiveDate) -> bool {
    let prior_valid_count = count_valid_prior_doses(history, patient, active_season_start);
    
    if prior_valid_count >= 2 {
        let latest_prior_date = get_latest_valid_prior_dose_date(history, patient, active_season_start);
        if let Some(latest) = latest_prior_date {
            if latest >= NaiveDate::from_ymd_opt(2010, 7, 1).unwrap() {
                return true;
            }
        }
    }
    
    let prior_before_2010_count = count_valid_prior_doses_before_2010(history, patient);
    let has_h1n1 = history.iter().any(|d| {
        matches!(d.cvx.0, cvx!("125") | cvx!("126") | cvx!("127") | cvx!("128")) &&
        d.date >= NaiveDate::from_ymd_opt(2009, 10, 1).unwrap() &&
        d.date <= NaiveDate::from_ymd_opt(2010, 6, 30).unwrap()
    });
    
    prior_before_2010_count >= 2 && has_h1n1
}

fn get_latest_valid_prior_dose_date(history: &[Dose], patient: &Patient, active_season_start: NaiveDate) -> Option<NaiveDate> {
    let disallowed_cvx = [cvx!("194"), cvx!("200"), cvx!("201"), cvx!("202"), cvx!("231"), cvx!("331"), cvx!("337")];
    history.iter()
        .filter(|d| d.date < active_season_start)
        .filter(|d| !disallowed_cvx.contains(&d.cvx.0))
        .filter(|d| {
            let tp_6m_4d = crate::time_period!("6m-4d");
            let abs_min_date = tp_6m_4d.add_to(patient.birth_date);
            d.date >= abs_min_date
        })
        .map(|d| d.date)
        .max()
}

fn count_valid_prior_doses_before_2010(history: &[Dose], patient: &Patient) -> usize {
    let cutoff = NaiveDate::from_ymd_opt(2010, 7, 1).unwrap();
    let disallowed_cvx = [cvx!("194"), cvx!("200"), cvx!("201"), cvx!("202"), cvx!("231"), cvx!("331"), cvx!("337")];
    let mut eligible_doses: Vec<NaiveDate> = history.iter()
        .filter(|d| d.date < cutoff)
        .filter(|d| !disallowed_cvx.contains(&d.cvx.0))
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

fn get_influenza_same_day_priority(cvx: u16, patient_age: chrono::Duration) -> i32 {
    // Disallowed vaccines have lowest preference (highest priority number)
    if matches!(cvx, cvx!("194") | cvx!("200") | cvx!("201") | cvx!("202") | cvx!("231") | cvx!("331") | cvx!("337")) {
        return 100;
    }
    
    // Unspecified formulation (CVX 88) has low preference
    if cvx == cvx!("88") {
        return 90;
    }
    
    // High-dose / adjuvanted vaccines (CVX 135, 197) are restricted to age >= 65y.
    // If patient is under 65y, they should be evaluated last.
    if matches!(cvx, cvx!("135") | cvx!("197")) {
        let age_65y = chrono::Duration::days(65 * 365 + 16); // approximately 65 years
        if patient_age < age_65y {
            return 80;
        } else {
            return 5; // preferred for >= 65y
        }
    }
    
    // Live virus vaccines (LAIV: 111, 149, 151, 333) are preferred and evaluated first
    if matches!(cvx, cvx!("111") | cvx!("149") | cvx!("151") | cvx!("333")) {
        return 10;
    }
    
    // Default preference for standard inactivated vaccines
    20
}

fn evaluate_history_seasonally(patient: &Patient, history: &[Dose]) -> Vec<DoseEvaluation> {
    let mut evaluations = Vec::new();
    let mut valid_doses_by_season: std::collections::HashMap<String, Vec<NaiveDate>> = std::collections::HashMap::new();

    let mut sorted_history = history.to_vec();
    sorted_history.sort_by(|a, b| {
        if a.date != b.date {
            a.date.cmp(&b.date)
        } else {
            let age_a = a.date - patient.birth_date;
            let age_b = b.date - patient.birth_date;
            let prio_a = get_influenza_same_day_priority(a.cvx.0, age_a);
            let prio_b = get_influenza_same_day_priority(b.cvx.0, age_b);
            prio_a.cmp(&prio_b)
        }
    });

    for (i, dose) in sorted_history.iter().enumerate() {
        let active_season = get_active_season(dose.date);
        let season_key = active_season.start.to_string();

        let mut status = DoseStatus::Valid;
        let mut reasons = SmallVec::<[EvaluationReason; 4]>::new();

        // 1. Prior to DOB check
        if dose.date < patient.birth_date {
            status = DoseStatus::Invalid;
            reasons.push(EvaluationReason::PriorToDOB);
        } else {
            // Check same-day duplicate: it is a duplicate if there is already a Valid or Accepted (non-Invalid) dose evaluated on the same day
            let is_duplicate = evaluations.iter().any(|e: &DoseEvaluation| {
                e.dose_date == dose.date && e.status != DoseStatus::Invalid
            });
            if is_duplicate {
                status = DoseStatus::Invalid;
                reasons.push(EvaluationReason::DuplicateShotSameDay);
            } else {
                // 2. Age limit check (minimum age 6m - 4d)
                let tp_6m_4d = crate::time_period!("6m-4d");
                let abs_min_age_date = tp_6m_4d.add_to(patient.birth_date);
                if dose.date < abs_min_age_date {
                    status = DoseStatus::Invalid;
                    reasons.push(EvaluationReason::BelowMinimumAge);
                }

                // 3. US disallowed vaccine check
                if matches!(dose.cvx.0, cvx!("194") | cvx!("200") | cvx!("201") | cvx!("202") | cvx!("231") | cvx!("331") | cvx!("337")) {
                    status = DoseStatus::Invalid;
                    reasons.push(EvaluationReason::VaccineNotAllowedInUs);
                }

                // 4. Allowed CVX check
                const ALLOWED_CVX: &[u16] = &[cvx!("151"), cvx!("144"), cvx!("149"), cvx!("88"), cvx!("111"), cvx!("155"), cvx!("15"), cvx!("141"), cvx!("153"), cvx!("16"), cvx!("135"), cvx!("150"), cvx!("140"), cvx!("158"), cvx!("161"), cvx!("166"), cvx!("168"), cvx!("171"), cvx!("185"), cvx!("186"), cvx!("194"), cvx!("197"), cvx!("200"), cvx!("201"), cvx!("202"), cvx!("205"), cvx!("231"), cvx!("320"), cvx!("331"), cvx!("333")];
                if !ALLOWED_CVX.contains(&dose.cvx.0) {
                    status = DoseStatus::Invalid;
                    reasons.push(EvaluationReason::VaccineNotPartOfSeries);
                }

                // 5. CVX 161 pediatric restriction check
                if dose.cvx.0 == cvx!("161") {
                    let tp_3y = crate::time_period!("3y-1d");
                    let limit = tp_3y.add_to(patient.birth_date);
                    if dose.date > limit {
                        status = DoseStatus::Invalid;
                        let tp_9y = crate::time_period!("9y");
                        let age_9y_date = tp_9y.add_to(patient.birth_date);
                        let is_patient_ge_9y = dose.date >= age_9y_date;
                        if !is_patient_ge_9y {
                            reasons.push(EvaluationReason::InsufficientAntigen);
                        }
                    }
                }

                // 6. Interval checks (only if still Valid)
                if status == DoseStatus::Valid {
                    let last_prior_dose_date = evaluations.iter()
                        .filter(|e| {
                            e.status == DoseStatus::Valid || 
                            (!e.reasons.contains(&EvaluationReason::BelowMinimumAge) && 
                             !e.reasons.contains(&EvaluationReason::PriorToDOB))
                        })
                        .map(|e| e.dose_date)
                        .last();

                    if let Some(last_date) = last_prior_dose_date {
                        let days_since_last = (dose.date - last_date).num_days();
                        if days_since_last < 24 {
                            status = DoseStatus::Invalid;
                            reasons.push(EvaluationReason::BelowMinimumInterval);
                        }
                    }
                }
            }
        }

        let dose_number = if status == DoseStatus::Valid {
            let season_valid_doses = valid_doses_by_season.entry(season_key.clone()).or_insert_with(Vec::new);
            season_valid_doses.push(dose.date);
            season_valid_doses.len()
        } else {
            let season_valid_doses = valid_doses_by_season.entry(season_key.clone()).or_insert_with(Vec::new);
            season_valid_doses.len() + 1
        };

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

pub fn influenza_group_selection(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    candidate_forecasts: &mut [(&'static str, VaccineGroupForecast)],
) -> &'static str {
    let active_season = get_active_season(eval_date);
    let active_season_name = get_active_season_name(eval_date);
    let is_default_season = active_season_name == "DEFAULT_INFLUENZA_SEASON";
    
    let tp_9y = crate::time_period!("9y");
    let age_9y = tp_9y.add_to(patient.birth_date);
    let is_under_9y = eval_date < age_9y;

    let tp_10y = crate::time_period!("10y");
    let age_10y = tp_10y.add_to(patient.birth_date);
    let is_under_10y = eval_date < age_10y;
    
    let use_1_dose;
    if !is_under_9y {
        if is_under_10y {
            let current_season_dose_under_9y = history.iter()
                .filter(|d| d.date >= active_season.start && d.date <= active_season.end)
                .any(|d| d.date < age_9y);
            
            if current_season_dose_under_9y {
                let prior_valid_count = count_valid_prior_doses(history, patient, active_season.start);
                if prior_valid_count >= 2 {
                    use_1_dose = true;
                } else {
                    use_1_dose = false;
                }
            } else {
                use_1_dose = true;
            }
        } else {
            use_1_dose = true;
        }
    } else {
        if active_season.name == "20122013_INFLUENZA_SEASON" || active_season.name == "20132014_INFLUENZA_SEASON" {
            use_1_dose = check_1dose_conditions_2012_2014(history, patient, active_season.start);
        } else if active_season.name == "20142015_INFLUENZA_SEASON" {
            let cond_2012 = check_1dose_conditions_2012_2014(history, patient, active_season.start);
            let has_dose_2013 = history.iter().any(|d| {
                d.date >= NaiveDate::from_ymd_opt(2013, 7, 1).unwrap() &&
                d.date <= NaiveDate::from_ymd_opt(2014, 6, 30).unwrap()
            });
            use_1_dose = cond_2012 || has_dose_2013;
        } else {
            let prior_valid_count = count_valid_prior_doses(history, patient, active_season.start);
            use_1_dose = prior_valid_count >= 2;
        }
    }
    
    let selected_series_name = if is_default_season {
        if use_1_dose {
            if candidate_forecasts.contains_forecast("INFLUENZA_1_DOSE_SERIES") {
                "INFLUENZA_1_DOSE_SERIES"
            } else {
                "INFLUENZA_2_DOSE_DEFAULT_SERIES"
            }
        } else {
            "INFLUENZA_2_DOSE_DEFAULT_SERIES"
        }
    } else {
        if use_1_dose {
            "INFLUENZA_1_DOSE_SERIES"
        } else {
            "INFLUENZA_2_DOSE_SERIES"
        }
    };

    if let Some(forecast) = candidate_forecasts.get_forecast_mut(selected_series_name) {
        forecast.evaluations = evaluate_history_seasonally(patient, history).into();
    }

    selected_series_name
}
