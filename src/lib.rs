pub mod date_utils;
pub mod engine;
pub mod errors;
pub mod fhir;
pub mod forecaster_generated;
pub mod legacy_models;
pub mod models;
pub mod rules;
pub mod schedule;
pub mod test_dsl;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

use crate::date_utils::SmallVec;
use chrono::NaiveDate;
use models::{Dose, Patient, VaccineGroupForecast};

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Once;

#[cfg(not(target_arch = "wasm32"))]
static INIT_RAYON_POOL: Once = Once::new();

#[cfg(not(target_arch = "wasm32"))]
pub fn init_rayon_pool() {
    INIT_RAYON_POOL.call_once(|| {
        rayon::ThreadPoolBuilder::new()
            .stack_size(8 * 1024 * 1024)
            .build_global()
            .expect("failed to initialize global rayon thread pool");
    });
}

pub fn parse_request(
    content: &str,
) -> Result<models::ForecastRequest, crate::errors::ForecasterError> {
    // Try to parse as simplified format first
    if let Ok(req) = serde_json::from_str::<models::ForecastRequest>(content) {
        return Ok(req);
    }

    // Otherwise, parse as legacy REST format and translate
    let legacy_req: legacy_models::LegacyEvaluateRequest = serde_json::from_str(content)
        .map_err(|e| crate::errors::ForecasterError::ParseError(e.to_string()))?;
    legacy_req.translate()
}

pub fn evaluate_patient_all_groups(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
) -> SmallVec<[VaccineGroupForecast; 24]> {
    let mut results = SmallVec::<[VaccineGroupForecast; 24]>::new();
    for ruleset in rules::get_all_groups() {
        if let Some(group_selection) = ruleset.group_selection {
            let mut candidate_forecasts =
                SmallVec::<[(&'static str, VaccineGroupForecast); 4]>::new();
            for series in &ruleset.series {
                let mut engine = engine::EvaluationEngine::new(series);
                engine.param_overrides = &ruleset.param_overrides;
                engine.completion_rules = &ruleset.completion_rules;
                engine.rec_overrides = &ruleset.rec_overrides;
                let legacy_policy = crate::rules::LegacyHookPolicy {
                    custom_forecast_hook: ruleset.custom_forecast_hook,
                    custom_switch_hook: ruleset.custom_switch_hook,
                    custom_evaluation_hook: ruleset.custom_evaluation_hook,
                    custom_dose_number_hook: ruleset.custom_dose_number_hook,
                    custom_extra_dose_hook: ruleset.custom_extra_dose_hook,
                    custom_completion_hook: ruleset.custom_completion_hook,
                };
                let policy_ref: &dyn crate::engine::EvaluationPolicy =
                    ruleset.policy.as_deref().unwrap_or(&legacy_policy);
                engine.policy = Some(policy_ref);

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
                let legacy_policy = crate::rules::LegacyHookPolicy {
                    custom_forecast_hook: ruleset.custom_forecast_hook,
                    custom_switch_hook: ruleset.custom_switch_hook,
                    custom_evaluation_hook: ruleset.custom_evaluation_hook,
                    custom_dose_number_hook: ruleset.custom_dose_number_hook,
                    custom_extra_dose_hook: ruleset.custom_extra_dose_hook,
                    custom_completion_hook: ruleset.custom_completion_hook,
                };
                let policy_ref: &dyn crate::engine::EvaluationPolicy =
                    ruleset.policy.as_deref().unwrap_or(&legacy_policy);
                engine.policy = Some(policy_ref);

                let forecast =
                    engine.evaluate_patient(patient, history, eval_date, &ruleset.series);
                results.push(forecast);
            }
        }
    }

    // Cross-group post-processing
    rules::cross_group::post_process_all_groups(patient, history, eval_date, &mut results);

    results
}
