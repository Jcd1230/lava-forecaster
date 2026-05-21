mod date_utils;
mod models;
mod schedule;
mod engine;
mod rules;
mod legacy_models;

use chrono::NaiveDate;
use models::{Patient, Gender, Dose, ForecastResponse, VaccineGroupForecast};
use engine::EvaluationEngine;
use std::time::Instant;

fn parse_request(content: &str) -> Result<models::ForecastRequest, Box<dyn std::error::Error>> {
    // Try to parse as simplified format first
    if let Ok(req) = serde_json::from_str::<models::ForecastRequest>(content) {
        return Ok(req);
    }
    
    // Otherwise, parse as legacy REST format and translate
    let legacy_req: legacy_models::LegacyEvaluateRequest = serde_json::from_str(content)?;
    legacy_req.translate()
}

fn evaluate_patient_all_groups(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
) -> Vec<VaccineGroupForecast> {
    let mut results = Vec::new();
    for ruleset in rules::get_all_groups() {
        if let Some(group_selection) = ruleset.group_selection {
            let mut candidate_forecasts = std::collections::HashMap::new();
            for series in &ruleset.series {
                let mut engine = EvaluationEngine::new(series.clone());
                engine.param_overrides = ruleset.param_overrides.clone();
                engine.completion_rules = ruleset.completion_rules.clone();
                engine.rec_overrides = ruleset.rec_overrides.clone();
                engine.custom_forecast_hook = ruleset.custom_forecast_hook;
                engine.custom_switch_hook = ruleset.custom_switch_hook;
                engine.custom_evaluation_hook = ruleset.custom_evaluation_hook;
                
                let forecast = engine.evaluate_patient(patient, history, eval_date, &ruleset.series);
                candidate_forecasts.insert(series.name.clone(), forecast);
            }
            let selected_name = (group_selection)(patient, history, eval_date, &mut candidate_forecasts);
            if let Some(selected_forecast) = candidate_forecasts.remove(&selected_name) {
                results.push(selected_forecast);
            }
        } else {
            if let Some(series) = ruleset.series.first() {
                let mut engine = EvaluationEngine::new(series.clone());
                engine.param_overrides = ruleset.param_overrides.clone();
                engine.completion_rules = ruleset.completion_rules.clone();
                engine.rec_overrides = ruleset.rec_overrides.clone();
                engine.custom_forecast_hook = ruleset.custom_forecast_hook;
                engine.custom_switch_hook = ruleset.custom_switch_hook;
                engine.custom_evaluation_hook = ruleset.custom_evaluation_hook;
                
                let forecast = engine.evaluate_patient(patient, history, eval_date, &ruleset.series);
                results.push(forecast);
            }
        }
    }
    results
}

fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    use tiny_http::{Server, Response, StatusCode, Header};

    let addr = "0.0.0.0:8081";
    let server = Server::http(addr).map_err(|e| format!("Failed to start server: {}", e))?;
    println!("Rust PoC REST server listening on http://{}", addr);

    for mut request in server.incoming_requests() {
        let req_start = Instant::now();

        if request.method() != &tiny_http::Method::Post {
            let response = Response::from_string("Method not allowed")
                .with_status_code(StatusCode(405));
            let _ = request.respond(response);
            continue;
        }

        let url = request.url().to_string();
        if url != "/evaluate" && url != "/opencds-decision-support-service/api/resources/evaluate" && url != "/evaluate_bulk" {
            let response = Response::from_string("Not Found")
                .with_status_code(StatusCode(404));
            let _ = request.respond(response);
            continue;
        }

        let mut body = String::new();
        if let Err(e) = request.as_reader().read_to_string(&mut body) {
            let response = Response::from_string(format!("Failed to read body: {}", e))
                .with_status_code(StatusCode(400));
            let _ = request.respond(response);
            continue;
        }

        if url == "/evaluate_bulk" {
            let req: models::BulkForecastRequest = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(e) => {
                    let response = Response::from_string(format!("Failed to parse bulk request: {}", e))
                        .with_status_code(StatusCode(400));
                    let _ = request.respond(response);
                    continue;
                }
            };

            let mut responses = Vec::new();
            for single_req in req.requests {
                let results = evaluate_patient_all_groups(&single_req.patient, &single_req.history, single_req.execution_date);
                responses.push(ForecastResponse {
                    vaccine_groups: results,
                });
            }

            let response_data = models::BulkForecastResponse { responses };
            let response_json = match serde_json::to_string(&response_data) {
                Ok(j) => j,
                Err(e) => {
                    let response = Response::from_string(format!("Failed to serialize bulk response: {}", e))
                        .with_status_code(StatusCode(500));
                    let _ = request.respond(response);
                    continue;
                }
            };

            let elapsed = req_start.elapsed();
            println!("INFO: processed Bulk request (size {}) in {:?}", response_data.responses.len(), elapsed);

            let elapsed_us = elapsed.as_micros().to_string();
            let response = Response::from_string(response_json)
                .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                .with_header(Header::from_bytes(&b"X-Process-Time-Us"[..], elapsed_us.into_bytes()).unwrap());
            let _ = request.respond(response);
            continue;
        }

        let is_legacy = body.contains("evaluationRequest");
        let format_str = if is_legacy { "Legacy" } else { "Simplified" };

        let req = match parse_request(&body) {
            Ok(r) => r,
            Err(e) => {
                let response = Response::from_string(format!("Failed to parse request: {}", e))
                    .with_status_code(StatusCode(400));
                let _ = request.respond(response);
                continue;
            }
        };

        let results = evaluate_patient_all_groups(&req.patient, &req.history, req.execution_date);

        let response_data = ForecastResponse {
            vaccine_groups: results,
        };

        let response_json = match serde_json::to_string(&response_data) {
            Ok(j) => j,
            Err(e) => {
                let response = Response::from_string(format!("Failed to serialize response: {}", e))
                    .with_status_code(StatusCode(500));
                let _ = request.respond(response);
                continue;
            }
        };

        let elapsed = req_start.elapsed();
        println!("INFO: processed {} request in {:?}", format_str, elapsed);

        let elapsed_us = elapsed.as_micros().to_string();
        let response = Response::from_string(response_json)
            .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            .with_header(Header::from_bytes(&b"X-Process-Time-Us"[..], elapsed_us.into_bytes()).unwrap());
        let _ = request.respond(response);
    }

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
        println!("Successfully loaded ruleset for vaccine group: {}", ruleset.group_name);
        for series in &ruleset.series {
            println!("  - Schedule: {} (code: {}, target group: {})", 
                     series.name, series.code, series.vaccine_group);
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
        Dose { date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(), cvx: "10".to_string() }, // Dose 1 (2 months) - OK
        Dose { date: NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(), cvx: "10".to_string() }, // Dose 2 (4 months) - OK
        Dose { date: NaiveDate::from_ymd_opt(2020, 7, 1).unwrap(), cvx: "10".to_string() }, // Dose 3 (6 months) - OK
    ];

    // Patient B: 3 doses, with the 3rd dose given after age 4 -> Should complete!
    let patient_b = Patient {
        birth_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        gender: Gender::Male,
    };
    let history_b = vec![
        Dose { date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(), cvx: "10".to_string() }, // Dose 1
        Dose { date: NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(), cvx: "10".to_string() }, // Dose 2
        Dose { date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(), cvx: "10".to_string() }, // Dose 3 (age 4y 5m, interval 4y)
    ];

    // Patient C: Pre-2009 immunization with shorter intervals
    let patient_c = Patient {
        birth_date: NaiveDate::from_ymd_opt(2008, 1, 1).unwrap(),
        gender: Gender::Female,
    };
    let history_c = vec![
        Dose { date: NaiveDate::from_ymd_opt(2008, 2, 10).unwrap(), cvx: "10".to_string() }, // Dose 1
        Dose { date: NaiveDate::from_ymd_opt(2008, 3, 15).unwrap(), cvx: "10".to_string() }, // Dose 2
        Dose { date: NaiveDate::from_ymd_opt(2008, 4, 20).unwrap(), cvx: "10".to_string() }, // Dose 3
        Dose { date: NaiveDate::from_ymd_opt(2008, 5, 20).unwrap(), cvx: "10".to_string() }, // Dose 4 (min interval is 30 days, pre-2009 check should allow 24d)
    ];

    // 4. Run evaluations
    println!("\n--- Test Case A: Standard 4-Dose Series (3 doses given) ---");
    let result_a = evaluate_patient_all_groups(&patient_a, &history_a, eval_date);
    for group_forecast in &result_a {
        println!("Vaccine Group: {}", group_forecast.vaccine_group);
        for (i, eval) in group_forecast.evaluations.iter().enumerate() {
            println!("Dose {}: date={}, cvx={}, status={:?}, reasons={:?}", 
                     i+1, eval.dose_date, eval.cvx, eval.status, eval.reasons);
        }
        if !group_forecast.forecasts.is_empty() {
            println!("Forecast: status={:?}, earliest={:?}, recommended={:?}, overdue={:?}",
                     group_forecast.forecasts[0].status, group_forecast.forecasts[0].earliest_date, 
                     group_forecast.forecasts[0].recommended_date, group_forecast.forecasts[0].overdue_date);
        }
    }

    println!("\n--- Test Case B: 3-Dose Completion Rule (Dose 3 given at >= 4 years) ---");
    let result_b = evaluate_patient_all_groups(&patient_b, &history_b, eval_date);
    for group_forecast in &result_b {
        println!("Vaccine Group: {}", group_forecast.vaccine_group);
        for (i, eval) in group_forecast.evaluations.iter().enumerate() {
            println!("Dose {}: date={}, cvx={}, status={:?}, reasons={:?}", 
                     i+1, eval.dose_date, eval.cvx, eval.status, eval.reasons);
        }
        if !group_forecast.forecasts.is_empty() {
            println!("Forecast: status={:?}", group_forecast.forecasts[0].status);
        }
    }

    println!("\n--- Test Case C: Pre-2009 Vaccine Interval Check ---");
    let result_c = evaluate_patient_all_groups(&patient_c, &history_c, eval_date);
    for group_forecast in &result_c {
        println!("Vaccine Group: {}", group_forecast.vaccine_group);
        for (i, eval) in group_forecast.evaluations.iter().enumerate() {
            println!("Dose {}: date={}, cvx={}, status={:?}, reasons={:?}", 
                     i+1, eval.dose_date, eval.cvx, eval.status, eval.reasons);
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
    println!("Average latency per patient forecast (Polio + HepA): {:.3} microseconds", avg_latency);
    println!("Throughput: {:.1} evaluations/sec", total_evals as f64 / duration.as_secs_f64());

    Ok(())
}
