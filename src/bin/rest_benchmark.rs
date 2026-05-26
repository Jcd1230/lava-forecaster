use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use reqwest::Client;
use serde_json;
use flatbuffers::FlatBufferBuilder;

use ice_rust_forecaster_poc::models::{UnifiedTestCase, ForecastRequest, BulkForecastRequest};
use ice_rust_forecaster_poc::forecaster_generated::org::cdsframework::ice::flatbuf as fb;

struct ServerGuard(std::process::Child);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        println!("Stopping REST server process...");
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn serialize_flatbuffers_bulk(requests: &[ForecastRequest]) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let mut req_offsets = Vec::with_capacity(requests.len());

    for req in requests {
        let birth_date_str = builder.create_string(&req.patient.birth_date.format("%Y-%m-%d").to_string());
        let gender_str = builder.create_string(match req.patient.gender {
            ice_rust_forecaster_poc::models::Gender::Female => "Female",
            ice_rust_forecaster_poc::models::Gender::Male => "Male",
            ice_rust_forecaster_poc::models::Gender::Unknown => "Unknown",
        });

        let patient_offset = fb::Patient::create(&mut builder, &fb::PatientArgs {
            birth_date: Some(birth_date_str),
            gender: Some(gender_str),
        });

        let mut dose_offsets = Vec::with_capacity(req.history.len());
        for dose in &req.history {
            let dose_date_str = builder.create_string(&dose.date.format("%Y-%m-%d").to_string());
            let cvx_str = builder.create_string(&dose.cvx.0.to_string());
            let dose_offset = fb::Dose::create(&mut builder, &fb::DoseArgs {
                date: Some(dose_date_str),
                cvx: Some(cvx_str),
            });
            dose_offsets.push(dose_offset);
        }
        let history_vec = builder.create_vector(&dose_offsets);

        let exec_date_str = builder.create_string(&req.execution_date.format("%Y-%m-%d").to_string());

        let req_offset = fb::PatientRequest::create(&mut builder, &fb::PatientRequestArgs {
            patient: Some(patient_offset),
            history: Some(history_vec),
            execution_date: Some(exec_date_str),
        });
        req_offsets.push(req_offset);
    }

    let requests_vec = builder.create_vector(&req_offsets);
    let bulk_req = fb::BulkForecastRequest::create(&mut builder, &fb::BulkForecastRequestArgs {
        requests: Some(requests_vec),
    });

    builder.finish(bulk_req, None);
    builder.finished_data().to_vec()
}

async fn wait_for_server(addr: &str, timeout_secs: u64) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(timeout_secs) {
        if TcpStream::connect(addr).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Building REST Server in Release Profile ===");
    let build_status = Command::new("cargo")
        .args(["build", "--release", "--bin", "ice-rust-forecaster-poc", "--features", "jemalloc"])
        .status()?;
    if !build_status.success() {
        panic!("Failed to build REST server binary");
    }

    println!("Starting REST server...");
    let child = Command::new("target/release/ice-rust-forecaster-poc")
        .arg("--server")
        .spawn()?;
    let _guard = ServerGuard(child);

    let server_addr = "127.0.0.1:8081";
    println!("Waiting for server to listen on http://{}...", server_addr);
    if !wait_for_server(server_addr, 10).await {
        panic!("Timeout waiting for REST server to start");
    }
    println!("Server is up and listening!");

    // Load test cases
    let cases_dir = Path::new("tests/cases");
    let mut cases = Vec::new();
    let entries = fs::read_dir(cases_dir)?;
    for entry in entries {
        let path = entry?.path();
        if path.extension().map_or(false, |e| e == "json") && !path.to_string_lossy().contains(".expected") {
            let content = fs::read_to_string(&path)?;
            if let Ok(tc) = serde_json::from_str::<UnifiedTestCase>(&content) {
                cases.push(tc);
            }
        }
    }

    let num_cases = cases.len();
    println!("Loaded {} test cases for request payloads.", num_cases);
    if num_cases == 0 {
        return Ok(());
    }

    // Prepare payloads
    let single_json_payloads: Vec<String> = cases
        .iter()
        .map(|tc| {
            let req = ForecastRequest {
                patient: tc.patient.clone(),
                history: tc.history.clone(),
                execution_date: tc.execution_date,
            };
            serde_json::to_string(&req).unwrap()
        })
        .collect();

    let bulk_req_data = BulkForecastRequest {
        requests: cases
            .iter()
            .map(|tc| ForecastRequest {
                patient: tc.patient.clone(),
                history: tc.history.clone(),
                execution_date: tc.execution_date,
            })
            .collect(),
    };
    let bulk_json_payload = serde_json::to_string(&bulk_req_data)?;
    let bulk_fb_payload = serialize_flatbuffers_bulk(&bulk_req_data.requests);

    let client = Client::builder()
        .pool_max_idle_per_host(100)
        .build()?;

    // Warmup
    println!("\nWarming up endpoints...");
    for i in 0..10 {
        let _ = client.post("http://127.0.0.1:8081/evaluate")
            .body(single_json_payloads[i % num_cases].clone())
            .send()
            .await;
    }
    let _ = client.post("http://127.0.0.1:8081/evaluate_bulk")
        .header("Content-Type", "application/json")
        .body(bulk_json_payload.clone())
        .send()
        .await;
    let _ = client.post("http://127.0.0.1:8081/evaluate_bulk_flatbuffers")
        .header("Content-Type", "application/octet-stream")
        .body(bulk_fb_payload.clone())
        .send()
        .await;
    println!("Warmup complete.");

    // Benchmark runners
    run_benchmark("Single JSON (/evaluate)", &client, &single_json_payloads, "application/json", "http://127.0.0.1:8081/evaluate", 1).await?;
    run_benchmark("Bulk JSON (/evaluate_bulk)", &client, &[bulk_json_payload.clone()], "application/json", "http://127.0.0.1:8081/evaluate_bulk", num_cases).await?;
    run_benchmark("Bulk FlatBuffers (/evaluate_bulk_flatbuffers)", &client, &[bulk_fb_payload], "application/octet-stream", "http://127.0.0.1:8081/evaluate_bulk_flatbuffers", num_cases).await?;

    Ok(())
}

async fn run_benchmark(
    name: &str,
    client: &Client,
    payloads: &[impl Into<reqwest::Body> + Clone + Send + 'static],
    content_type: &'static str,
    url: &'static str,
    batch_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n==================================================");
    println!("Benchmarking: {}", name);
    println!("==================================================");

    for concurrency in [1, 10, 50] {
        let duration = Duration::from_secs(3);
        let start = Instant::now();
        let mut tasks = Vec::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Duration>(50000);

        // Spawn workload workers
        for _ in 0..concurrency {
            let client = client.clone();
            let payloads = payloads.to_vec();
            let tx = tx.clone();
            
            let handle = tokio::spawn(async move {
                let mut index = 0;
                while start.elapsed() < duration {
                    let body = payloads[index % payloads.len()].clone();
                    index += 1;
                    
                    let req_start = Instant::now();
                    let resp = client.post(url)
                        .header("Content-Type", content_type)
                        .body(body)
                        .send()
                        .await;
                    
                    if let Ok(r) = resp {
                        if r.status().is_success() {
                            let _ = tx.send(req_start.elapsed()).await;
                        }
                    }
                }
            });
            tasks.push(handle);
        }

        // Drop original tx so rx finishes when all tasks complete
        drop(tx);

        let mut latencies = Vec::new();
        while let Some(lat) = rx.recv().await {
            latencies.push(lat);
        }

        for t in tasks {
            let _ = t.await;
        }

        let elapsed = start.elapsed();
        let total_requests = latencies.len();
        let total_evals = total_requests * batch_size;
        let rps = total_requests as f64 / elapsed.as_secs_f64();
        let eps = total_evals as f64 / elapsed.as_secs_f64();

        if total_requests > 0 {
            latencies.sort();
            let avg = latencies.iter().sum::<Duration>() / total_requests as u32;
            let p50 = latencies[total_requests * 50 / 100];
            let p90 = latencies[total_requests * 90 / 100];
            let p99 = latencies[total_requests * 99 / 100];

            println!(
                "Concurrency: {:2} | Reqs: {:5} | Throughput: {:7.2} reqs/sec ({:8.2} patients/sec) | Avg: {:6.2}ms | p50: {:6.2}ms | p90: {:6.2}ms | p99: {:6.2}ms",
                concurrency,
                total_requests,
                rps,
                eps,
                avg.as_secs_f64() * 1000.0,
                p50.as_secs_f64() * 1000.0,
                p90.as_secs_f64() * 1000.0,
                p99.as_secs_f64() * 1000.0,
            );
        } else {
            println!("Concurrency: {:2} | No successful requests completed.", concurrency);
        }
    }

    Ok(())
}
