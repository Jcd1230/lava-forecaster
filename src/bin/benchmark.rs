use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;
use rayon::prelude::*;

#[cfg(all(feature = "jemalloc", not(target_os = "windows")))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use ice_rust_forecaster_poc::{
    evaluate_patient_all_groups,
    init_rayon_pool,
    models::UnifiedTestCase,
};

fn main() {
    init_rayon_pool();

    let args: Vec<String> = std::env::args().collect();
    let cases_dir_str = args.get(1).map(|s| s.as_str()).unwrap_or("tests/cases");
    let cases_dir = Path::new(cases_dir_str);

    println!("Loading test cases from: {}...", cases_dir.display());
    
    let mut cases = Vec::new();
    let mut total_doses = 0;
    
    let entries = fs::read_dir(cases_dir)
        .unwrap_or_else(|e| panic!("Failed to read cases dir {}: {}", cases_dir.display(), e));

    for entry in entries {
        let entry = entry.expect("failed to read entry");
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") && !path.to_string_lossy().contains(".expected") {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read file {}: {}", path.display(), e));
            match serde_json::from_str::<UnifiedTestCase>(&content) {
                Ok(tc) => {
                    total_doses += tc.history.len();
                    cases.push(tc);
                }
                Err(e) => {
                    // Ignore files that are not valid test cases, if any
                    eprintln!("Warning: could not parse {} as UnifiedTestCase: {}", path.display(), e);
                }
            }
        }
    }

    let num_cases = cases.len();
    if num_cases == 0 {
        println!("No test cases loaded! Exiting.");
        return;
    }

    let avg_doses = total_doses as f64 / num_cases as f64;
    println!("Successfully loaded {} test cases.", num_cases);
    println!("Total doses across all histories: {}", total_doses);
    println!("Average doses per patient history: {:.2}", avg_doses);

    // Find the most complex patient (most doses)
    let max_dose_case = cases.iter()
        .max_by_key(|tc| tc.history.len())
        .cloned()
        .expect("should have at least one case");
    println!("Most complex case: {} with {} doses", max_dose_case.name, max_dose_case.history.len());

    // Find a minimal case (newborn/no doses)
    let min_dose_case = cases.iter()
        .min_by_key(|tc| tc.history.len())
        .cloned()
        .expect("should have at least one case");
    println!("Simplest case: {} with {} doses", min_dose_case.name, min_dose_case.history.len());

    println!("\nWarming up CPU (running all cases 50 times)...");
    for _ in 0..50 {
        for tc in &cases {
            let res = evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
            black_box(res);
        }
    }
    println!("Warmup complete.");

    // === Benchmark 1: Full Suite Throughput ===
    println!("\n=== Benchmark 1: Full Suite Throughput ===");
    let iterations = 1000;
    println!("Running {} iterations of all {} test cases ({} total evaluations)...", iterations, num_cases, iterations * num_cases);
    
    // 1A: Single-Threaded
    println!("\n--- 1A: Single-Threaded ---");
    let start = Instant::now();
    for _ in 0..iterations {
        for tc in &cases {
            let res = evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
            black_box(res);
        }
    }
    let elapsed_st = start.elapsed();
    let total_evals = iterations * num_cases;
    let evals_per_sec_st = total_evals as f64 / elapsed_st.as_secs_f64();
    let avg_latency_us_st = (elapsed_st.as_nanos() as f64 / total_evals as f64) / 1000.0;
    println!("Throughput:         {:.2} evaluations/sec", evals_per_sec_st);
    println!("Average latency:    {:.3} microseconds per patient", avg_latency_us_st);

    // 1B: Multi-Threaded
    println!("\n--- 1B: Multi-Threaded ---");
    let start = Instant::now();
    (0..iterations).into_par_iter().for_each(|_| {
        for tc in &cases {
            let res = evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
            black_box(res);
        }
    });
    let elapsed_mt = start.elapsed();
    let evals_per_sec_mt = total_evals as f64 / elapsed_mt.as_secs_f64();
    let avg_latency_us_mt = (elapsed_mt.as_nanos() as f64 / total_evals as f64) / 1000.0;
    println!("Throughput:         {:.2} evaluations/sec (Speedup: {:.2}x)", evals_per_sec_mt, evals_per_sec_mt / evals_per_sec_st);
    println!("Average latency:    {:.3} microseconds per patient (effective)", avg_latency_us_mt);

    // === Benchmark 2: Complex Case Performance ===
    println!("\n=== Benchmark 2: Complex Case Performance ===");
    println!("Case: {} ({} doses)", max_dose_case.name, max_dose_case.history.len());
    let complex_iterations = 100_000;
    println!("Running {} iterations...", complex_iterations);
    
    // 2A: Single-Threaded
    println!("\n--- 2A: Single-Threaded ---");
    let start = Instant::now();
    for _ in 0..complex_iterations {
        let res = evaluate_patient_all_groups(&max_dose_case.patient, &max_dose_case.history, max_dose_case.execution_date);
        black_box(res);
    }
    let elapsed_st = start.elapsed();
    let evals_per_sec_st = complex_iterations as f64 / elapsed_st.as_secs_f64();
    let avg_latency_us_st = (elapsed_st.as_nanos() as f64 / complex_iterations as f64) / 1000.0;
    println!("Throughput:         {:.2} evaluations/sec", evals_per_sec_st);
    println!("Average latency:    {:.3} microseconds", avg_latency_us_st);

    // 2B: Multi-Threaded
    println!("\n--- 2B: Multi-Threaded ---");
    let start = Instant::now();
    (0..complex_iterations).into_par_iter().for_each(|_| {
        let res = evaluate_patient_all_groups(&max_dose_case.patient, &max_dose_case.history, max_dose_case.execution_date);
        black_box(res);
    });
    let elapsed_mt = start.elapsed();
    let evals_per_sec_mt = complex_iterations as f64 / elapsed_mt.as_secs_f64();
    let avg_latency_us_mt = (elapsed_mt.as_nanos() as f64 / complex_iterations as f64) / 1000.0;
    println!("Throughput:         {:.2} evaluations/sec (Speedup: {:.2}x)", evals_per_sec_mt, evals_per_sec_mt / evals_per_sec_st);
    println!("Average latency:    {:.3} microseconds (effective)", avg_latency_us_mt);

    // === Benchmark 3: Minimal Case Performance ===
    println!("\n=== Benchmark 3: Minimal Case Performance ===");
    println!("Case: {} ({} doses)", min_dose_case.name, min_dose_case.history.len());
    let minimal_iterations = 100_000;
    println!("Running {} iterations...", minimal_iterations);
    
    // 3A: Single-Threaded
    println!("\n--- 3A: Single-Threaded ---");
    let start = Instant::now();
    for _ in 0..minimal_iterations {
        let res = evaluate_patient_all_groups(&min_dose_case.patient, &min_dose_case.history, min_dose_case.execution_date);
        black_box(res);
    }
    let elapsed_st = start.elapsed();
    let evals_per_sec_st = minimal_iterations as f64 / elapsed_st.as_secs_f64();
    let avg_latency_us_st = (elapsed_st.as_nanos() as f64 / minimal_iterations as f64) / 1000.0;
    println!("Throughput:         {:.2} evaluations/sec", evals_per_sec_st);
    println!("Average latency:    {:.3} microseconds", avg_latency_us_st);

    // 3B: Multi-Threaded
    println!("\n--- 3B: Multi-Threaded ---");
    let start = Instant::now();
    (0..minimal_iterations).into_par_iter().for_each(|_| {
        let res = evaluate_patient_all_groups(&min_dose_case.patient, &min_dose_case.history, min_dose_case.execution_date);
        black_box(res);
    });
    let elapsed_mt = start.elapsed();
    let evals_per_sec_mt = minimal_iterations as f64 / elapsed_mt.as_secs_f64();
    let avg_latency_us_mt = (elapsed_mt.as_nanos() as f64 / minimal_iterations as f64) / 1000.0;
    println!("Throughput:         {:.2} evaluations/sec (Speedup: {:.2}x)", evals_per_sec_mt, evals_per_sec_mt / evals_per_sec_st);
    println!("Average latency:    {:.3} microseconds (effective)", avg_latency_us_mt);
    
    println!("\nBenchmark run completed successfully.");
}
