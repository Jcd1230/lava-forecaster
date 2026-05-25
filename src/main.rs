use chrono::NaiveDate;
use std::time::Instant;
use rayon::prelude::*;

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use ice_rust_forecaster_poc::{
    parse_request, evaluate_patient_all_groups, rules,
    models::{self, Dose, ForecastResponse, Gender, Patient, VaccineGroupForecast, Cvx},
};

async fn evaluate_handler(
    body: String,
) -> impl axum::response::IntoResponse {
    use axum::http::{StatusCode, HeaderMap, HeaderValue};
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
            ).into_response();
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
            ).into_response();
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

async fn evaluate_bulk_handler(
    body: String,
) -> impl axum::response::IntoResponse {
    use axum::http::{StatusCode, HeaderMap, HeaderValue};
    use axum::response::IntoResponse;
    let req_start = Instant::now();

    let req: models::BulkForecastRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Failed to parse bulk request: {}", e),
            ).into_response();
        }
    };

    let responses: Vec<ForecastResponse> = req.requests
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
            ).into_response();
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
    use axum::http::{StatusCode, HeaderMap, HeaderValue};
    use axum::response::IntoResponse;
    use ice_rust_forecaster_poc::forecaster_generated::org::cdsframework::ice::flatbuf as fb;
    let req_start = Instant::now();

    let bulk_req = match ::flatbuffers::root::<fb::BulkForecastRequest>(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Failed to parse FlatBuffers bulk request: {}", e),
            ).into_response();
        }
    };

    let reqs = match bulk_req.requests() {
        Some(r) => r,
        None => {
            let mut builder = ::flatbuffers::FlatBufferBuilder::new();
            let empty_vec = builder.create_vector::<::flatbuffers::WIPOffset<fb::ForecastResponse>>(&[]);
            let bulk_resp = fb::BulkForecastResponse::create(&mut builder, &fb::BulkForecastResponseArgs {
                responses: Some(empty_vec),
            });
            fb::finish_bulk_forecast_response_buffer(&mut builder, bulk_resp);
            let finished_data = builder.finished_data().to_vec();
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert("Content-Type", HeaderValue::from_static("application/octet-stream"));
            return (StatusCode::OK, resp_headers, finished_data).into_response();
        }
    };

    let mut parsed_requests = Vec::with_capacity(reqs.len());
    for i in 0..reqs.len() {
        parsed_requests.push(reqs.get(i));
    }

    let internal_requests: Result<Vec<(models::Patient, Vec<models::Dose>, NaiveDate)>, String> = parsed_requests
        .iter()
        .map(|req| {
            let exec_date_str = req.execution_date().ok_or("execution_date is required")?;
            let exec_date = NaiveDate::parse_from_str(exec_date_str, "%Y-%m-%d")
                .map_err(|e| format!("invalid execution_date '{}': {}", exec_date_str, e))?;

            let patient_fb = req.patient().ok_or("patient is required")?;
            let birth_date_str = patient_fb.birth_date().ok_or("birth_date is required")?;
            let birth_date = NaiveDate::parse_from_str(birth_date_str, "%Y-%m-%d")
                .map_err(|e| format!("invalid birth_date '{}': {}", birth_date_str, e))?;

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
                    let date_str = dose_fb.date().ok_or("dose date is required")?;
                    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                        .map_err(|e| format!("invalid dose date '{}': {}", date_str, e))?;
                    let cvx_str = dose_fb.cvx().unwrap_or("");
                    let cvx = models::Cvx(cvx_str.parse::<u16>().unwrap_or(0));
                    history.push(models::Dose { date, cvx });
                }
            }

            Ok((models::Patient { birth_date, gender }, history, exec_date))
        })
        .collect();

    let internal_requests = match internal_requests {
        Ok(r) => r,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                HeaderMap::new(),
                format!("Invalid request data: {}", err),
            ).into_response();
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

    for resp in evaluated_responses {
        let mut vg_offsets = Vec::with_capacity(resp.vaccine_groups.len());

        for vg in resp.vaccine_groups.iter() {
            // Build evaluations vector
            let mut eval_offsets = Vec::with_capacity(vg.evaluations.len());
            for eval in vg.evaluations.iter() {
                let dose_date = builder.create_string(&eval.dose_date.format("%Y-%m-%d").to_string());
                let cvx = builder.create_string(&eval.cvx.to_string());
                let status_str = match eval.status {
                    models::DoseStatus::Valid => "Valid",
                    models::DoseStatus::Invalid => "Invalid",
                    models::DoseStatus::Accepted => "Accepted",
                    models::DoseStatus::Ignored => "Ignored",
                };
                let status = builder.create_string(status_str);

                let mut reason_offsets = Vec::with_capacity(eval.reasons.len());
                for r in eval.reasons.iter() {
                    let r_str = format!("{:?}", r);
                    reason_offsets.push(builder.create_string(&r_str));
                }
                let reasons_vec = builder.create_vector(&reason_offsets);

                let dose_number = eval.dose_number.unwrap_or(0) as i32;

                let dose_eval_offset = fb::DoseEvaluation::create(&mut builder, &fb::DoseEvaluationArgs {
                    dose_date: Some(dose_date),
                    cvx: Some(cvx),
                    status: Some(status),
                    reasons: Some(reasons_vec),
                    dose_number,
                });
                eval_offsets.push(dose_eval_offset);
            }
            let evals_vec = builder.create_vector(&eval_offsets);

            // Build forecasts vector
            let mut forecast_offsets = Vec::with_capacity(vg.forecasts.len());
            for fc in vg.forecasts.iter() {
                let series_name = builder.create_string(&fc.series_name);
                let earliest_date = fc.earliest_date.map(|d| builder.create_string(&d.format("%Y-%m-%d").to_string()));
                let recommended_date = fc.recommended_date.map(|d| builder.create_string(&d.format("%Y-%m-%d").to_string()));
                let overdue_date = fc.overdue_date.map(|d| builder.create_string(&d.format("%Y-%m-%d").to_string()));
                let latest_date = fc.latest_date.map(|d| builder.create_string(&d.format("%Y-%m-%d").to_string()));

                let status_str = match fc.status {
                    models::SeriesStatus::NotComplete => "NotComplete",
                    models::SeriesStatus::Complete => "Complete",
                    models::SeriesStatus::NotRecommended => "NotRecommended",
                    models::SeriesStatus::ConditionallyRecommended => "ConditionallyRecommended",
                };
                let status = builder.create_string(status_str);

                let mut reason_offsets = Vec::with_capacity(fc.reasons.len());
                for r in fc.reasons.iter() {
                    reason_offsets.push(builder.create_string(r));
                }
                let reasons_vec = builder.create_vector(&reason_offsets);

                let fc_offset = fb::SeriesForecast::create(&mut builder, &fb::SeriesForecastArgs {
                    series_name: Some(series_name),
                    earliest_date,
                    recommended_date,
                    overdue_date,
                    latest_date,
                    status: Some(status),
                    reasons: Some(reasons_vec),
                });
                forecast_offsets.push(fc_offset);
            }
            let forecasts_vec = builder.create_vector(&forecast_offsets);

            let vaccine_group = builder.create_string(&vg.vaccine_group);
            let selected_series = vg.selected_series.as_ref().map(|s| builder.create_string(s));

            let vg_forecast_offset = fb::VaccineGroupForecast::create(&mut builder, &fb::VaccineGroupForecastArgs {
                vaccine_group: Some(vaccine_group),
                evaluations: Some(evals_vec),
                forecasts: Some(forecasts_vec),
                selected_series,
            });
            vg_offsets.push(vg_forecast_offset);
        }
        let vg_vec = builder.create_vector(&vg_offsets);

        let forecast_resp = fb::ForecastResponse::create(&mut builder, &fb::ForecastResponseArgs {
            vaccine_groups: Some(vg_vec),
        });
        response_offsets.push(forecast_resp);
    }

    let responses_vec = builder.create_vector(&response_offsets);
    let bulk_resp = fb::BulkForecastResponse::create(&mut builder, &fb::BulkForecastResponseArgs {
        responses: Some(responses_vec),
    });

    fb::finish_bulk_forecast_response_buffer(&mut builder, bulk_resp);
    let finished_data = builder.finished_data().to_vec();

    let elapsed = req_start.elapsed();
    println!(
        "INFO: processed Bulk FlatBuffers request (size {}) in {:?}",
        num_responses,
        elapsed
    );

    let elapsed_us = elapsed.as_micros().to_string();
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Content-Type", HeaderValue::from_static("application/octet-stream"));
    if let Ok(val) = HeaderValue::from_str(&elapsed_us) {
        resp_headers.insert("X-Process-Time-Us", val);
    }

    (StatusCode::OK, resp_headers, finished_data).into_response()
}

fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        use axum::{
            routing::post,
            Router,
        };
        use std::net::SocketAddr;

        let app = Router::new()
            .route("/evaluate", post(evaluate_handler))
            .route("/opencds-decision-support-service/api/resources/evaluate", post(evaluate_handler))
            .route("/evaluate_bulk", post(evaluate_bulk_handler))
            .route("/evaluate_bulk_flatbuffers", post(evaluate_bulk_flatbuffers_handler));

        let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
        println!("Rust PoC REST server listening on http://{}", addr);

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

    println!("=== High-Performance Rust ICE Forecaster PoC (Compile-Time DSL) ===");

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

    // Patient A: Follows standard 4-dose schedule
    let patient_a = Patient {
        birth_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        gender: Gender::Female,
    };
    let history_a = vec![
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            cvx: Cvx(10),
        }, // Dose 1 (2 months) - OK
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(),
            cvx: Cvx(10),
        }, // Dose 2 (4 months) - OK
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 7, 1).unwrap(),
            cvx: Cvx(10),
        }, // Dose 3 (6 months) - OK
    ];

    // Patient B: 3 doses, with the 3rd dose given after age 4 -> Should complete!
    let patient_b = Patient {
        birth_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        gender: Gender::Male,
    };
    let history_b = vec![
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            cvx: Cvx(10),
        }, // Dose 1
        Dose {
            date: NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(),
            cvx: Cvx(10),
        }, // Dose 2
        Dose {
            date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            cvx: Cvx(10),
        }, // Dose 3 (age 4y 5m, interval 4y)
    ];

    // Patient C: Pre-2009 immunization with shorter intervals
    let patient_c = Patient {
        birth_date: NaiveDate::from_ymd_opt(2008, 1, 1).unwrap(),
        gender: Gender::Female,
    };
    let history_c = vec![
        Dose {
            date: NaiveDate::from_ymd_opt(2008, 2, 10).unwrap(),
            cvx: Cvx(10),
        }, // Dose 1
        Dose {
            date: NaiveDate::from_ymd_opt(2008, 3, 15).unwrap(),
            cvx: Cvx(10),
        }, // Dose 2
        Dose {
            date: NaiveDate::from_ymd_opt(2008, 4, 20).unwrap(),
            cvx: Cvx(10),
        }, // Dose 3
        Dose {
            date: NaiveDate::from_ymd_opt(2008, 5, 20).unwrap(),
            cvx: Cvx(10),
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
                group_forecast.forecasts[0].earliest_date,
                group_forecast.forecasts[0].recommended_date,
                group_forecast.forecasts[0].overdue_date
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
