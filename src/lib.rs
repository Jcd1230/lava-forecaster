pub mod date_utils;
pub mod engine;
pub mod legacy_models;
pub mod models;
pub mod rules;
pub mod schedule;
pub mod forecaster_generated;

use chrono::NaiveDate;
use models::{Dose, Patient, VaccineGroupForecast, Cvx};
use crate::date_utils::TinyVec;


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
) -> TinyVec<VaccineGroupForecast, 24> {
    let mut results = TinyVec::<VaccineGroupForecast, 24>::new();
    for ruleset in rules::get_all_groups() {
        if let Some(group_selection) = ruleset.group_selection {
            let mut candidate_forecasts = TinyVec::<(&'static str, VaccineGroupForecast), 4>::new();
            for series in &ruleset.series {
                let mut engine = engine::EvaluationEngine::new(series);
                engine.param_overrides = &ruleset.param_overrides;
                engine.completion_rules = &ruleset.completion_rules;
                engine.rec_overrides = &ruleset.rec_overrides;
                engine.custom_forecast_hook = ruleset.custom_forecast_hook;
                engine.custom_switch_hook = ruleset.custom_switch_hook;
                engine.custom_evaluation_hook = ruleset.custom_evaluation_hook;
                engine.custom_dose_number_hook = ruleset.custom_dose_number_hook;
                engine.custom_extra_dose_hook = ruleset.custom_extra_dose_hook;
                engine.custom_completion_hook = ruleset.custom_completion_hook;

                let forecast =
                    engine.evaluate_patient(patient, history, eval_date, &ruleset.series);
                candidate_forecasts.push((series.name, forecast));
            }
            let selected_name =
                (group_selection)(patient, history, eval_date, &mut candidate_forecasts);
            let mut selected_forecast = None;
            for i in 0..candidate_forecasts.len() {
                if candidate_forecasts[i].0 == selected_name {
                    selected_forecast = Some(candidate_forecasts.swap_remove(i).1);
                    break;
                }
            }
            if let Some(selected_forecast) = selected_forecast {
                results.push(selected_forecast);
            }
        } else {
            if let Some(series) = ruleset.series.first() {
                let mut engine = engine::EvaluationEngine::new(series);
                engine.param_overrides = &ruleset.param_overrides;
                engine.completion_rules = &ruleset.completion_rules;
                engine.rec_overrides = &ruleset.rec_overrides;
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
            .filter(|d| d.cvx.0 == Cvx::YELLOW_FEVER || d.cvx.0 == Cvx::YELLOW_FEVER_UNSPECIFIED || d.cvx.0 == Cvx::YELLOW_FEVER_UNKNOWN)
            .map(|d| d.date)
            .max();

        if let Some(yf_date) = last_yf_dose {
            let limit_date = yf_date + chrono::Duration::days(30);
            for g in results.iter_mut() {
                if g.vaccine_group == "YELLOW_FEVER" {
                    continue;
                }

                let is_live_group = g.vaccine_group == "MMR"
                    || g.vaccine_group == "VARICELLA"
                    || g.vaccine_group == "ROTAVIRUS"
                    || g.vaccine_group == "CHOLERA";

                if is_live_group {
                    for f in g.forecasts.iter_mut() {
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


    results
}
