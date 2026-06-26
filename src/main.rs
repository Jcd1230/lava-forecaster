use chrono::NaiveDate;
use rayon::prelude::*;
use std::time::Instant;

#[cfg(all(
    feature = "jemalloc",
    not(target_os = "windows"),
    not(target_arch = "wasm32")
))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use lava_forecaster::{
    evaluate_patient_all_groups,
    models::{self, Cvx, Dose, ForecastResponse, Gender, Patient},
    parse_request, rules,
};

async fn health_handler() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

async fn evaluate_handler(body: String) -> impl axum::response::IntoResponse {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum::response::IntoResponse;
    let req_start = Instant::now();

    let is_legacy = body.contains("evaluationRequest");
    let format_str = if is_legacy { "Legacy" } else { "Simplified" };

    let req = match parse_request(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Failed to parse request: {}", e),
            )
                .into_response();
        }
    };

    let results = evaluate_patient_all_groups(&req.patient, &req.history, req.execution_date);

    let response_data = ForecastResponse {
        vaccine_groups: results,
    };

    let response_json = match serde_json::to_string(&response_data) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                format!("Failed to serialize response: {}", e),
            )
                .into_response();
        }
    };

    let elapsed = req_start.elapsed();
    println!("INFO: processed {} request in {:?}", format_str, elapsed);

    let elapsed_us = elapsed.as_micros().to_string();
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    if let Ok(val) = HeaderValue::from_str(&elapsed_us) {
        resp_headers.insert("X-Process-Time-Us", val);
    }

    (StatusCode::OK, resp_headers, response_json).into_response()
}

async fn evaluate_bulk_handler(body: String) -> impl axum::response::IntoResponse {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum::response::IntoResponse;
    let req_start = Instant::now();

    let req: models::BulkForecastRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Failed to parse bulk request: {}", e),
            )
                .into_response();
        }
    };

    let responses: Vec<ForecastResponse> = req
        .requests
        .into_par_iter()
        .map(|single_req| {
            let results = evaluate_patient_all_groups(
                &single_req.patient,
                &single_req.history,
                single_req.execution_date,
            );
            ForecastResponse {
                vaccine_groups: results,
            }
        })
        .collect();

    let response_data = models::BulkForecastResponse { responses };
    let response_json = match serde_json::to_string(&response_data) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                format!("Failed to serialize bulk response: {}", e),
            )
                .into_response();
        }
    };

    let elapsed = req_start.elapsed();
    println!(
        "INFO: processed Bulk request (size {}) in {:?}",
        response_data.responses.len(),
        elapsed
    );

    let elapsed_us = elapsed.as_micros().to_string();
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    if let Ok(val) = HeaderValue::from_str(&elapsed_us) {
        resp_headers.insert("X-Process-Time-Us", val);
    }

    (StatusCode::OK, resp_headers, response_json).into_response()
}

async fn evaluate_bulk_flatbuffers_handler(
    body: axum::body::Bytes,
) -> impl axum::response::IntoResponse {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum::response::IntoResponse;
    use lava_forecaster::forecaster_generated::org::cdsframework::ice::flatbuf as fb;
    let req_start = Instant::now();

    let epoch_days_to_date = |days: u16| {
        NaiveDate::from_ymd_opt(1970, 1, 1).unwrap() + chrono::Duration::days(days as i64)
    };

    let date_to_epoch_days = |date: NaiveDate| {
        let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        (date - epoch).num_days() as u16
    };

    let bulk_req = match ::flatbuffers::root::<fb::BulkForecastRequest>(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Failed to parse FlatBuffers bulk request: {}", e),
            )
                .into_response();
        }
    };

    let reqs = match bulk_req.requests() {
        Some(r) => r,
        None => {
            let mut builder = ::flatbuffers::FlatBufferBuilder::new();
            let empty_vec =
                builder.create_vector::<::flatbuffers::WIPOffset<fb::ForecastResponse>>(&[]);
            let bulk_resp = fb::BulkForecastResponse::create(
                &mut builder,
                &fb::BulkForecastResponseArgs {
                    responses: Some(empty_vec),
                },
            );
            fb::finish_bulk_forecast_response_buffer(&mut builder, bulk_resp);
            let finished_data = builder.finished_data().to_vec();
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/octet-stream"),
            );
            return (StatusCode::OK, resp_headers, finished_data).into_response();
        }
    };

    let mut parsed_requests = Vec::with_capacity(reqs.len());
    for i in 0..reqs.len() {
        parsed_requests.push(reqs.get(i));
    }

    let internal_requests: Result<Vec<(models::Patient, Vec<models::Dose>, NaiveDate)>, String> =
        parsed_requests
            .iter()
            .map(|req| {
                let exec_date = epoch_days_to_date(req.execution_date());

                let patient_fb = req.patient().ok_or("patient is required")?;
                let birth_date = epoch_days_to_date(patient_fb.birth_date());

                let gender_str = patient_fb.gender().unwrap_or("Unknown");
                let gender = match gender_str {
                    "Female" | "FEMALE" | "F" => models::Gender::Female,
                    "Male" | "MALE" | "M" => models::Gender::Male,
                    _ => models::Gender::Unknown,
                };

                let mut history = Vec::new();
                if let Some(history_fb) = req.history() {
                    for j in 0..history_fb.len() {
                        let dose_fb = history_fb.get(j);
                        let date = epoch_days_to_date(dose_fb.date());
                        let cvx_str = dose_fb.cvx().unwrap_or("");
                        let cvx = models::Cvx(cvx_str.parse::<u16>().unwrap_or(0));
                        history.push(models::Dose {
                            date,
                            cvx,
                            is_valid: None,
                        });
                    }
                }

                Ok((
                    models::Patient {
                        birth_date,
                        gender,
                        immunities: Vec::new(),
                        contraindications: Vec::new(),
                    },
                    history,
                    exec_date,
                ))
            })
            .collect();

    let internal_requests = match internal_requests {
        Ok(r) => r,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Invalid request data: {}", err),
            )
                .into_response();
        }
    };

    let evaluated_responses: Vec<models::ForecastResponse> = internal_requests
        .into_par_iter()
        .map(|req_item| {
            let patient = &req_item.0;
            let history = &req_item.1;
            let exec_date = req_item.2;
            let results = evaluate_patient_all_groups(patient, history, exec_date);
            models::ForecastResponse {
                vaccine_groups: results,
            }
        })
        .collect();

    let num_responses = evaluated_responses.len();
    let mut builder = ::flatbuffers::FlatBufferBuilder::new();
    let mut response_offsets = Vec::with_capacity(num_responses);

    let mut string_cache =
        std::collections::HashMap::<String, ::flatbuffers::WIPOffset<&str>>::new();
    macro_rules! get_or_create_string {
        ($val:expr) => {{
            let s: &str = $val.as_ref();
            if let Some(&offset) = string_cache.get(s) {
                offset
            } else {
                let offset = builder.create_string(s);
                string_cache.insert(s.to_string(), offset);
                offset
            }
        }};
    }

    for resp in evaluated_responses {
        let mut vg_offsets = Vec::with_capacity(resp.vaccine_groups.len());

        for vg in resp.vaccine_groups.iter() {
            // Build evaluations vector
            let mut eval_offsets = Vec::with_capacity(vg.evaluations.len());
            for eval in vg.evaluations.iter() {
                let dose_date = date_to_epoch_days(eval.dose_date);

                let cvx_owner;
                let cvx_str = match eval.cvx.0 {
                    10 => "10",
                    11 => "11",
                    15 => "15",
                    17 => "17",
                    20 => "20",
                    21 => "21",
                    22 => "22",
                    28 => "28",
                    31 => "31",
                    33 => "33",
                    43 => "43",
                    45 => "45",
                    47 => "47",
                    48 => "48",
                    49 => "49",
                    52 => "52",
                    62 => "62",
                    83 => "83",
                    85 => "85",
                    88 => "88",
                    94 => "94",
                    100 => "100",
                    104 => "104",
                    106 => "106",
                    107 => "107",
                    109 => "109",
                    110 => "110",
                    113 => "113",
                    114 => "114",
                    115 => "115",
                    116 => "116",
                    118 => "118",
                    119 => "119",
                    120 => "120",
                    121 => "121",
                    122 => "122",
                    130 => "130",
                    133 => "133",
                    135 => "135",
                    136 => "136",
                    137 => "137",
                    140 => "140",
                    141 => "141",
                    148 => "148",
                    150 => "150",
                    152 => "152",
                    153 => "153",
                    155 => "155",
                    158 => "158",
                    161 => "161",
                    162 => "162",
                    163 => "163",
                    168 => "168",
                    171 => "171",
                    178 => "178",
                    179 => "179",
                    185 => "185",
                    186 => "186",
                    189 => "189",
                    197 => "197",
                    200 => "200",
                    207 => "207",
                    208 => "208",
                    210 => "210",
                    212 => "212",
                    213 => "213",
                    221 => "221",
                    228 => "228",
                    other => {
                        cvx_owner = other.to_string();
                        &cvx_owner
                    }
                };
                let cvx = get_or_create_string!(cvx_str);

                let status_str = match eval.status {
                    models::DoseStatus::Valid => "Valid",
                    models::DoseStatus::Invalid => "Invalid",
                    models::DoseStatus::Accepted => "Accepted",
                    models::DoseStatus::Ignored => "Ignored",
                };
                let status = get_or_create_string!(status_str);

                let mut reason_offsets = Vec::with_capacity(eval.reasons.len());
                for r in eval.reasons.iter() {
                    let r_str = r.as_str();
                    reason_offsets.push(get_or_create_string!(r_str));
                }
                let reasons_vec = builder.create_vector(&reason_offsets);

                let dose_number = eval.dose_number.unwrap_or(0) as i32;

                let dose_eval_offset = fb::DoseEvaluation::create(
                    &mut builder,
                    &fb::DoseEvaluationArgs {
                        dose_date,
                        cvx: Some(cvx),
                        status: Some(status),
                        reasons: Some(reasons_vec),
                        dose_number,
                    },
                );
                eval_offsets.push(dose_eval_offset);
            }
            let evals_vec = builder.create_vector(&eval_offsets);

            // Build forecasts vector
            let mut forecast_offsets = Vec::with_capacity(vg.forecasts.len());
            for fc in vg.forecasts.iter() {
                let series_name = get_or_create_string!(&fc.series_name);
                let earliest_date = fc
                    .status
                    .earliest_date()
                    .map(date_to_epoch_days)
                    .unwrap_or(0);
                let recommended_date = fc
                    .status
                    .recommended_date()
                    .map(date_to_epoch_days)
                    .unwrap_or(0);
                let overdue_date = fc
                    .status
                    .overdue_date()
                    .map(date_to_epoch_days)
                    .unwrap_or(0);
                let latest_date = fc.status.latest_date().map(date_to_epoch_days).unwrap_or(0);

                let status_str = match fc.status {
                    models::SeriesStatus::NotComplete { .. } => "NotComplete",
                    models::SeriesStatus::Complete => "Complete",
                    models::SeriesStatus::NotRecommended => "NotRecommended",
                    models::SeriesStatus::ConditionallyRecommended => "ConditionallyRecommended",
                };
                let status = get_or_create_string!(status_str);

                let mut reason_offsets = Vec::with_capacity(fc.reasons.len());
                for r in fc.reasons.iter() {
                    reason_offsets.push(get_or_create_string!(r.as_str()));
                }
                let reasons_vec = builder.create_vector(&reason_offsets);

                let fc_offset = fb::SeriesForecast::create(
                    &mut builder,
                    &fb::SeriesForecastArgs {
                        series_name: Some(series_name),
                        earliest_date,
                        recommended_date,
                        overdue_date,
                        latest_date,
                        status: Some(status),
                        reasons: Some(reasons_vec),
                    },
                );
                forecast_offsets.push(fc_offset);
            }
            let forecasts_vec = builder.create_vector(&forecast_offsets);

            let vaccine_group = get_or_create_string!(&vg.vaccine_group);
            let selected_series = vg
                .selected_series
                .as_ref()
                .map(|s| get_or_create_string!(s));

            let vg_forecast_offset = fb::VaccineGroupForecast::create(
                &mut builder,
                &fb::VaccineGroupForecastArgs {
                    vaccine_group: Some(vaccine_group),
                    evaluations: Some(evals_vec),
                    forecasts: Some(forecasts_vec),
                    selected_series,
                },
            );
            vg_offsets.push(vg_forecast_offset);
        }
        let vg_vec = builder.create_vector(&vg_offsets);

        let forecast_resp = fb::ForecastResponse::create(
            &mut builder,
            &fb::ForecastResponseArgs {
                vaccine_groups: Some(vg_vec),
            },
        );
        response_offsets.push(forecast_resp);
    }

    let responses_vec = builder.create_vector(&response_offsets);
    let bulk_resp = fb::BulkForecastResponse::create(
        &mut builder,
        &fb::BulkForecastResponseArgs {
            responses: Some(responses_vec),
        },
    );

    fb::finish_bulk_forecast_response_buffer(&mut builder, bulk_resp);
    let finished_data = builder.finished_data().to_vec();

    let elapsed = req_start.elapsed();
    println!(
        "INFO: processed Bulk FlatBuffers request (size {}) in {:?}",
        num_responses, elapsed
    );

    let elapsed_us = elapsed.as_micros().to_string();
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/octet-stream"),
    );
    if let Ok(val) = HeaderValue::from_str(&elapsed_us) {
        resp_headers.insert("X-Process-Time-Us", val);
    }

    (StatusCode::OK, resp_headers, finished_data).into_response()
}

async fn fhir_recommend_handler(body: String) -> impl axum::response::IntoResponse {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum::response::IntoResponse;
    use lava_forecaster::fhir;

    let params: fhir::Parameters = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Failed to parse FHIR Parameters: {}", e),
            )
                .into_response();
        }
    };

    let mut patient_resource = None;
    let mut history_resources = Vec::new();
    let mut assessment_date = None;

    for param in params.parameter {
        if param.name == "patient" {
            if let Some(fhir::FhirResource::Patient(p)) = param.resource {
                patient_resource = Some(p);
            }
        } else if param.name == "immunization" {
            if let Some(fhir::FhirResource::Immunization(imm)) = param.resource {
                history_resources.push(imm);
            }
        } else if param.name == "assessmentDate" {
            if let Some(val_str) = param.value_string {
                let clean_date = val_str.chars().take(10).collect::<String>();
                if let Ok(d) = NaiveDate::parse_from_str(&clean_date, "%Y-%m-%d") {
                    assessment_date = Some(d);
                }
            }
        }
    }

    let patient_r = match patient_resource {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                "Missing 'patient' parameter in FHIR Parameters request".to_string(),
            )
                .into_response();
        }
    };

    let patient_id = patient_r
        .id
        .clone()
        .unwrap_or_else(|| "anonymous".to_string());

    let internal_patient = match models::Patient::try_from(patient_r) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Invalid patient resource: {}", e),
            )
                .into_response();
        }
    };

    let mut internal_history = Vec::new();
    for imm in history_resources {
        match models::Dose::try_from(imm) {
            Ok(d) => internal_history.push(d),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    HeaderMap::new(),
                    format!("Invalid immunization resource: {}", e),
                )
                    .into_response();
            }
        }
    }

    let exec_date = assessment_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

    let results = evaluate_patient_all_groups(&internal_patient, &internal_history, exec_date);

    // Build the outgoing bundle containing evaluation and recommendation resources
    let mut bundle_entries = Vec::new();

    // 1. Recommendation
    let recommendation = fhir::make_immunization_recommendation(&patient_id, exec_date, &results);
    bundle_entries.push(fhir::OutgoingBundleEntry {
        resource: fhir::OutgoingFhirResource::ImmunizationRecommendation(recommendation),
    });

    // 2. Evaluations
    for vg in results.iter() {
        for (i, eval) in vg.evaluations.iter().enumerate() {
            let evaluation =
                fhir::make_immunization_evaluation(&patient_id, i, eval, &vg.vaccine_group);
            bundle_entries.push(fhir::OutgoingBundleEntry {
                resource: fhir::OutgoingFhirResource::ImmunizationEvaluation(evaluation),
            });
        }
    }

    let response_bundle = fhir::OutgoingBundle {
        resource_type: "Bundle".to_string(),
        bundle_type: "searchset".to_string(),
        entry: bundle_entries,
    };

    let response_json = match serde_json::to_string(&response_bundle) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                format!("Failed to serialize FHIR bundle response: {}", e),
            )
                .into_response();
        }
    };

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/fhir+json"),
    );

    (StatusCode::OK, resp_headers, response_json).into_response()
}

async fn cds_discovery_handler() -> impl axum::response::IntoResponse {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum::response::IntoResponse;

    let response_json = serde_json::json!({
        "services": [
            {
                "id": "immunization-forecaster",
                "hook": "patient-view",
                "title": "Immunization Forecaster",
                "description": "Calculates immunization evaluation and forecasting according to CDC/ACIP guidelines.",
                "prefetch": {
                    "patient": "Patient/{{context.patientId}}",
                    "immunizations": "Immunization?patient={{context.patientId}}"
                }
            }
        ]
    });

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    (StatusCode::OK, resp_headers, response_json.to_string()).into_response()
}

async fn cds_forecast_handler(body: String) -> impl axum::response::IntoResponse {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum::response::IntoResponse;
    use lava_forecaster::fhir;
    use lava_forecaster::models::SeriesStatus;

    let req: fhir::CDSRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Failed to parse CDS Hook request: {}", e),
            )
                .into_response();
        }
    };

    let patient_val = match req.prefetch.as_ref().and_then(|p| p.get("patient")) {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                "Missing 'patient' resource in prefetch context".to_string(),
            )
                .into_response();
        }
    };

    let patient_r: fhir::Patient = match serde_json::from_value(patient_val.clone()) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Invalid patient resource in prefetch: {}", e),
            )
                .into_response();
        }
    };

    let mut history = Vec::new();
    if let Some(imm_bundle_val) = req.prefetch.as_ref().and_then(|p| p.get("immunizations")) {
        if let Ok(bundle) = serde_json::from_value::<fhir::Bundle>(imm_bundle_val.clone()) {
            if let Some(entries) = bundle.entry {
                for entry in entries {
                    if let fhir::FhirResource::Immunization(imm) = entry.resource {
                        match models::Dose::try_from(imm) {
                            Ok(d) => history.push(d),
                            Err(e) => {
                                return (
                                    StatusCode::BAD_REQUEST,
                                    HeaderMap::new(),
                                    format!("Invalid immunization resource in prefetch: {}", e),
                                )
                                    .into_response();
                            }
                        }
                    }
                }
            }
        }
    }

    let internal_patient = match models::Patient::try_from(patient_r) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Invalid patient details: {}", e),
            )
                .into_response();
        }
    };

    let exec_date = chrono::Utc::now().date_naive();
    let results = evaluate_patient_all_groups(&internal_patient, &history, exec_date);

    let mut due_vaccines = Vec::new();
    for vg in &results {
        for fc in &vg.forecasts {
            match fc.status {
                SeriesStatus::NotComplete { .. } => {
                    if fc
                        .status
                        .overdue_date()
                        .map(|d| exec_date >= d)
                        .unwrap_or(false)
                    {
                        due_vaccines.push(format!("{} (OVERDUE)", vg.vaccine_group));
                    } else if fc
                        .status
                        .recommended_date()
                        .map(|d| exec_date >= d)
                        .unwrap_or(false)
                    {
                        due_vaccines.push(format!("{} (DUE)", vg.vaccine_group));
                    }
                }
                _ => {}
            }
        }
    }

    let summary = if due_vaccines.is_empty() {
        "Patient is up-to-date on all vaccinations.".to_string()
    } else {
        format!("Patient is due for: {}", due_vaccines.join(", "))
    };

    let detail = if due_vaccines.is_empty() {
        None
    } else {
        Some(format!(
            "Based on age and immunization history, the clinical decision support engine recommends administering the following vaccine series: {}",
            due_vaccines.join(", ")
        ))
    };

    let indicator = if due_vaccines.iter().any(|v| v.contains("OVERDUE")) {
        "warning".to_string()
    } else {
        "info".to_string()
    };

    let card = fhir::Card {
        summary,
        detail,
        indicator,
        source: fhir::Source {
            label: "LAVA Forecaster".to_string(),
            url: None,
            icon: None,
        },
        suggestions: None,
        links: None,
    };

    let response = fhir::CDSResponse { cards: vec![card] };

    let response_json = match serde_json::to_string(&response) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                format!("Failed to serialize CDS response: {}", e),
            )
                .into_response();
        }
    };

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    (StatusCode::OK, resp_headers, response_json).into_response()
}

fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    lava_forecaster::init_rayon_pool();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()?;

    rt.block_on(async {
        use axum::{
            Router,
            routing::{get, post},
        };
        use std::net::SocketAddr;

        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/evaluate", post(evaluate_handler))
            .route(
                "/opencds-decision-support-service/api/resources/evaluate",
                post(evaluate_handler),
            )
            .route("/evaluate_bulk", post(evaluate_bulk_handler))
            .route(
                "/evaluate_bulk_flatbuffers",
                post(evaluate_bulk_flatbuffers_handler),
            )
            .route(
                "/fhir/R4/Immunization/$recommend",
                post(fhir_recommend_handler),
            )
            .route("/cds-services", get(cds_discovery_handler))
            .route(
                "/cds-services/immunization-forecaster",
                post(cds_forecast_handler),
            );

        let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
        println!("LAVA Forecaster REST server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--server" {
        run_server()?;
        return Ok(());
    }

    if args.len() > 1 {
        // Run file evaluation mode
        let path = &args[1];
        let content = std::fs::read_to_string(path)?;

        let req = parse_request(&content)?;

        eprintln!("DEBUG: parsed patient = {:?}", req.patient);
        eprintln!("DEBUG: parsed history = {:?}", req.history);
        let results = evaluate_patient_all_groups(&req.patient, &req.history, req.execution_date);

        let response = ForecastResponse {
            vaccine_groups: results,
        };

        println!("{}", serde_json::to_string(&response)?);
        return Ok(());
    }

    println!("=== Lightspeed Antigen & Vaccine Assessment / LAVA Forecaster ===");

    for ruleset in rules::get_all_groups() {
        println!(
            "Successfully loaded ruleset for vaccine group: {}",
            ruleset.group_name
        );
        for series in &ruleset.series {
            println!(
                "  - Schedule: {} (code: {}, target group: {})",
                series.name, series.code, series.vaccine_group
            );
        }
    }

    // 3. Define test patient cases
    let eval_date = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();

    let patient_a = Patient {
        birth_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        gender: Gender::Female,
        immunities: Vec::new(),
        contraindications: Vec::new(),
    };
    let history_a = vec![
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 1 (2 months) - OK
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 2 (4 months) - OK
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 7, 1).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 3 (6 months) - OK
    ];

    let patient_b = Patient {
        birth_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        gender: Gender::Male,
        immunities: Vec::new(),
        contraindications: Vec::new(),
    };
    let history_b = vec![
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 1
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 2
        Dose {
            date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 3 (age 4y 5m, interval 4y)
    ];

    let patient_c = Patient {
        birth_date: NaiveDate::from_ymd_opt(2008, 1, 1).unwrap(),
        gender: Gender::Female,
        immunities: Vec::new(),
        contraindications: Vec::new(),
    };
    let history_c = vec![
        Dose {
            date: NaiveDate::from_ymd_opt(2008, 2, 10).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 1
        Dose {
            date: NaiveDate::from_ymd_opt(2008, 3, 15).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 2
        Dose {
            date: NaiveDate::from_ymd_opt(2008, 4, 20).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 3
        Dose {
            date: NaiveDate::from_ymd_opt(2008, 5, 20).unwrap(),
            cvx: Cvx(10),
            is_valid: None,
        }, // Dose 4 (min interval is 30 days, pre-2009 check should allow 24d)
    ];

    // 4. Run evaluations
    println!("\n--- Test Case A: Standard 4-Dose Series (3 doses given) ---");
    let result_a = evaluate_patient_all_groups(&patient_a, &history_a, eval_date);
    for group_forecast in result_a.iter() {
        println!("Vaccine Group: {}", group_forecast.vaccine_group);
        for (i, eval) in group_forecast.evaluations.iter().enumerate() {
            println!(
                "Dose {}: date={}, cvx={}, status={:?}, reasons={:?}",
                i + 1,
                eval.dose_date,
                eval.cvx,
                eval.status,
                eval.reasons
            );
        }
        if !group_forecast.forecasts.is_empty() {
            println!(
                "Forecast: status={:?}, earliest={:?}, recommended={:?}, overdue={:?}",
                group_forecast.forecasts[0].status,
                group_forecast.forecasts[0].status.earliest_date(),
                group_forecast.forecasts[0].status.recommended_date(),
                group_forecast.forecasts[0].status.overdue_date()
            );
        }
    }

    println!("\n--- Test Case B: 3-Dose Completion Rule (Dose 3 given at >= 4 years) ---");
    let result_b = evaluate_patient_all_groups(&patient_b, &history_b, eval_date);
    for group_forecast in result_b.iter() {
        println!("Vaccine Group: {}", group_forecast.vaccine_group);
        for (i, eval) in group_forecast.evaluations.iter().enumerate() {
            println!(
                "Dose {}: date={}, cvx={}, status={:?}, reasons={:?}",
                i + 1,
                eval.dose_date,
                eval.cvx,
                eval.status,
                eval.reasons
            );
        }
        if !group_forecast.forecasts.is_empty() {
            println!("Forecast: status={:?}", group_forecast.forecasts[0].status);
        }
    }

    println!("\n--- Test Case C: Pre-2009 Vaccine Interval Check ---");
    let result_c = evaluate_patient_all_groups(&patient_c, &history_c, eval_date);
    for group_forecast in result_c.iter() {
        println!("Vaccine Group: {}", group_forecast.vaccine_group);
        for (i, eval) in group_forecast.evaluations.iter().enumerate() {
            println!(
                "Dose {}: date={}, cvx={}, status={:?}, reasons={:?}",
                i + 1,
                eval.dose_date,
                eval.cvx,
                eval.status,
                eval.reasons
            );
        }
        if !group_forecast.forecasts.is_empty() {
            println!("Forecast: status={:?}", group_forecast.forecasts[0].status);
        }
    }

    // 5. Run Micro-Benchmarks
    println!("\n--- Micro-benchmarking Forecaster Loop Performance ---");
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = evaluate_patient_all_groups(&patient_a, &history_a, eval_date);
        let _ = evaluate_patient_all_groups(&patient_b, &history_b, eval_date);
    }
    let duration = start.elapsed();
    let total_evals = iterations * 2;
    let avg_latency = duration.as_secs_f64() / total_evals as f64 * 1_000_000.0;
    println!("Processed {} evaluations in {:?}", total_evals, duration);
    println!(
        "Average latency per patient forecast (Polio + HepA): {:.3} microseconds",
        avg_latency
    );
    println!(
        "Throughput: {:.1} evaluations/sec",
        total_evals as f64 / duration.as_secs_f64()
    );

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
    };
    use serde_json::json;
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_cds_discovery_endpoint() {
        let app = Router::new().route("/cds-services", get(cds_discovery_handler));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/cds-services")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["services"][0]["id"], "immunization-forecaster");
        assert_eq!(body["services"][0]["hook"], "patient-view");
    }

    #[tokio::test]
    async fn test_fhir_recommend_endpoint() {
        let app = Router::new().route(
            "/fhir/R4/Immunization/$recommend",
            post(fhir_recommend_handler),
        );

        let payload = json!({
            "resourceType": "Parameters",
            "parameter": [
                {
                    "name": "patient",
                    "resource": {
                        "resourceType": "Patient",
                        "id": "test-patient",
                        "birthDate": "2020-01-01",
                        "gender": "female"
                    }
                },
                {
                    "name": "immunization",
                    "resource": {
                        "resourceType": "Immunization",
                        "status": "completed",
                        "vaccineCode": {
                            "coding": [
                                {
                                    "system": "http://hl7.org/fhir/sid/cvx",
                                    "code": "10",
                                    "display": "IPV"
                                }
                            ]
                        },
                        "occurrenceDateTime": "2020-03-01T00:00:00Z"
                    }
                }
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/fhir/R4/Immunization/$recommend")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/fhir+json"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["resourceType"], "Bundle");
        assert_eq!(body["type"], "searchset");

        let entries = body["entry"].as_array().unwrap();
        assert!(!entries.is_empty());
        let rec = &entries[0]["resource"];
        assert_eq!(rec["resourceType"], "ImmunizationRecommendation");
        assert_eq!(rec["patient"]["reference"], "Patient/test-patient");
    }

    #[tokio::test]
    async fn test_cds_forecast_endpoint() {
        let app = Router::new().route(
            "/cds-services/immunization-forecaster",
            post(cds_forecast_handler),
        );

        let payload = json!({
            "hook": "patient-view",
            "hookInstance": "d15c7dbf-648b-49e0-8b09-b6957a05051a",
            "context": {
                "userId": "Practitioner/123",
                "patientId": "test-patient"
            },
            "prefetch": {
                "patient": {
                    "resourceType": "Patient",
                    "id": "test-patient",
                    "birthDate": "2020-01-01",
                    "gender": "female"
                },
                "immunizations": {
                    "resourceType": "Bundle",
                    "type": "searchset",
                    "entry": [
                        {
                            "resource": {
                                "resourceType": "Immunization",
                                "status": "completed",
                                "vaccineCode": {
                                    "coding": [
                                        {
                                            "system": "http://hl7.org/fhir/sid/cvx",
                                            "code": "10",
                                            "display": "IPV"
                                        }
                                    ]
                                },
                                "occurrenceDateTime": "2020-03-01T00:00:00Z"
                            }
                        }
                    ]
                }
            }
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cds-services/immunization-forecaster")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        let cards = body["cards"].as_array().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0]["source"]["label"], "LAVA Forecaster");
        assert!(
            cards[0]["summary"]
                .as_str()
                .unwrap()
                .contains("Patient is due for")
        );
    }
}
