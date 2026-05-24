pub mod date_utils;
pub mod engine;
pub mod legacy_models;
pub mod models;
pub mod rules;
pub mod schedule;

use chrono::NaiveDate;
use models::{Dose, Patient, VaccineGroupForecast, Cvx};

fn is_single_antigen_mmr(cvx: Cvx) -> bool {
    matches!(cvx.0, 3 | 4 | 5 | 6 | 7 | 38)
}

fn has_same_day_separate_mmr_and_varicella(history: &[Dose], eval_date: NaiveDate) -> bool {
    let has_mmr = history
        .iter()
        .any(|dose| dose.date == eval_date && is_single_antigen_mmr(dose.cvx));
    let has_varicella = history
        .iter()
        .any(|dose| dose.date == eval_date && dose.cvx.0 == 21);

    has_mmr && has_varicella
}

pub fn parse_request(content: &str) -> Result<models::ForecastRequest, Box<dyn std::error::Error>> {
    // Try to parse as simplified format first
    if let Ok(req) = serde_json::from_str::<models::ForecastRequest>(content) {
        return Ok(req);
    }

    // Otherwise, parse as legacy REST format and translate
    let legacy_req: legacy_models::LegacyEvaluateRequest = serde_json::from_str(content)?;
    legacy_req.translate()
}

pub fn evaluate_patient_all_groups(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
) -> Vec<VaccineGroupForecast> {
    let mut results = Vec::new();
    for ruleset in rules::get_all_groups() {
        if let Some(group_selection) = ruleset.group_selection {
            let mut candidate_forecasts = std::collections::HashMap::new();
            for series in &ruleset.series {
                let mut engine = engine::EvaluationEngine::new(series.clone());
                engine.param_overrides = ruleset.param_overrides.clone();
                engine.completion_rules = ruleset.completion_rules.clone();
                engine.rec_overrides = ruleset.rec_overrides.clone();
                engine.custom_forecast_hook = ruleset.custom_forecast_hook;
                engine.custom_switch_hook = ruleset.custom_switch_hook;
                engine.custom_evaluation_hook = ruleset.custom_evaluation_hook;
                engine.custom_dose_number_hook = ruleset.custom_dose_number_hook;
                engine.custom_extra_dose_hook = ruleset.custom_extra_dose_hook;
                engine.custom_completion_hook = ruleset.custom_completion_hook;

                let forecast =
                    engine.evaluate_patient(patient, history, eval_date, &ruleset.series);
                candidate_forecasts.insert(series.name.clone(), forecast);
            }
            let selected_name =
                (group_selection)(patient, history, eval_date, &mut candidate_forecasts);
            if let Some(selected_forecast) = candidate_forecasts.remove(&selected_name) {
                results.push(selected_forecast);
            }
        } else {
            if let Some(series) = ruleset.series.first() {
                let mut engine = engine::EvaluationEngine::new(series.clone());
                engine.param_overrides = ruleset.param_overrides.clone();
                engine.completion_rules = ruleset.completion_rules.clone();
                engine.rec_overrides = ruleset.rec_overrides.clone();
                engine.custom_forecast_hook = ruleset.custom_forecast_hook;
                engine.custom_switch_hook = ruleset.custom_switch_hook;
                engine.custom_evaluation_hook = ruleset.custom_evaluation_hook;
                engine.custom_dose_number_hook = ruleset.custom_dose_number_hook;
                engine.custom_extra_dose_hook = ruleset.custom_extra_dose_hook;
                engine.custom_completion_hook = ruleset.custom_completion_hook;

                let forecast =
                    engine.evaluate_patient(patient, history, eval_date, &ruleset.series);
                results.push(forecast);
            }
        }
    }

    // Cross-group post-processing: YellowFever.adjustEarliestDateDueToYellowFeverVaccine
    let yf_complete = results
        .iter()
        .find(|g| g.vaccine_group == "YELLOW_FEVER")
        .and_then(|g| g.forecasts.first())
        .map(|f| f.status == models::SeriesStatus::Complete)
        .unwrap_or(false);

    if yf_complete {
        let last_yf_dose = history
            .iter()
            .filter(|d| d.cvx.0 == 37 || d.cvx.0 == 183 || d.cvx.0 == 184)
            .map(|d| d.date)
            .max();

        if let Some(yf_date) = last_yf_dose {
            let limit_date = yf_date + chrono::Duration::days(30);
            for g in &mut results {
                if g.vaccine_group == "YELLOW_FEVER" {
                    continue;
                }

                let is_live_group = g.vaccine_group == "MMR"
                    || g.vaccine_group == "VARICELLA"
                    || g.vaccine_group == "ROTAVIRUS"
                    || g.vaccine_group == "CHOLERA";

                if is_live_group {
                    for f in &mut g.forecasts {
                        if let Some(ref mut earliest) = f.earliest_date {
                            let gap = *earliest - yf_date;
                            let is_conflict = if yf_date != eval_date {
                                gap.num_days() < 30
                            } else {
                                *earliest > yf_date && gap.num_days() < 30
                            };
                            if is_conflict {
                                *earliest = limit_date;
                                if let Some(ref mut recommended) = f.recommended_date {
                                    if *recommended < limit_date {
                                        *recommended = limit_date;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if has_same_day_separate_mmr_and_varicella(history, eval_date) {
        for g in &mut results {
            if g.vaccine_group != "MMR" {
                continue;
            }

            for f in &mut g.forecasts {
                if f.status == models::SeriesStatus::NotComplete {
                    f.earliest_date = Some(eval_date);
                    f.recommended_date = Some(eval_date);
                }
            }
        }
    }

    results
}
