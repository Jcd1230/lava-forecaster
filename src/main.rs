use chrono::NaiveDate;
use std::time::Instant;
use rayon::prelude::*;

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
            .route("/evaluate_bulk", post(evaluate_bulk_handler));

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
    for group_forecast in &result_a {
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
    for group_forecast in &result_b {
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
    for group_forecast in &result_c {
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
