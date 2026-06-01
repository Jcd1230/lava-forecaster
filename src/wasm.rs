use wasm_bindgen::prelude::*;
use crate::models;
use crate::evaluate_patient_all_groups;

#[wasm_bindgen]
pub fn evaluate_patient(request_json: &str) -> Result<String, String> {
    let req: models::ForecastRequest = serde_json::from_str(request_json)
        .map_err(|e| format!("Failed to parse request: {}", e))?;
    let results = evaluate_patient_all_groups(&req.patient, &req.history, req.execution_date);
    let response = models::ForecastResponse {
        vaccine_groups: results,
    };
    serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}

#[wasm_bindgen]
pub fn evaluate_patient_js(request: JsValue) -> Result<JsValue, JsValue> {
    let req: models::ForecastRequest = serde_wasm_bindgen::from_value(request)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse request: {}", e)))?;
    let results = evaluate_patient_all_groups(&req.patient, &req.history, req.execution_date);
    let response = models::ForecastResponse {
        vaccine_groups: results,
    };
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize response: {}", e)))
}
