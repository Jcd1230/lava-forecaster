mod date_utils;
mod models;
mod schedule;
mod engine;

use chrono::NaiveDate;
use date_utils::TimePeriod;
use models::{Patient, Gender, Dose};
use schedule::{IceSupportingData, CompiledSeries};
use engine::{EvaluationEngine, ParameterOverrideRule, ConditionalCompletionRule, RecommendationOverrideRule};
use std::fs;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== High-Performance Rust ICE Forecaster PoC ===");

    // 1. Load and parse Polio YAML configuration
    let yaml_path = "../opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/Series/Polio4DoseSeries.yml";
    let yaml_content = fs::read_to_string(yaml_path)?;
    
    let supporting_data: IceSupportingData = serde_yaml::from_str(&yaml_content)?;
    let km = supporting_data.ice_supporting_data.knowledge_modules.get("[org.nyc.cir^ICE^1.0.0]")
        .ok_or("Knowledge module [org.nyc.cir^ICE^1.0.0] not found")?;
    let polio_yaml = km.series.get("POLIO_4_DOSE_SERIES")
        .ok_or("POLIO_4_DOSE_SERIES not found in YAML")?;
    
    let compiled_series = CompiledSeries::from_yaml("POLIO_4_DOSE_SERIES", polio_yaml)?;
    println!("Successfully loaded schedule: {} (code: {}, target group: {})", 
             compiled_series.name, compiled_series.code, compiled_series.vaccine_group);

    // 2. Configure the Engine with Polio Custom Exception Primitives
    let mut engine = EvaluationEngine::new(compiled_series);

    // A. Pre-2009 Overrides:
    // "If dose 4 administered before 8/7/2009: abs_min_age = 122d, abs_min_interval for dose 3 = 24d"
    engine.param_overrides.push(ParameterOverrideRule {
        condition_date_before: Some(NaiveDate::from_ymd_opt(2009, 8, 7).unwrap()),
        target_dose_number: 4,
        override_abs_min_age: Some(TimePeriod::parse("122d")?),
        override_abs_min_interval_from_dose: Some((3, TimePeriod::parse("24d")?)),
    });

    // B. Conditional 3-Dose Completion:
    // "Complete with 3 doses if 3 prior valid doses, child >= 4y-4d at dose 3, and interval 2 to 3 is >= 6m-4d"
    engine.completion_rules.push(ConditionalCompletionRule {
        required_valid_doses: 3,
        min_age_at_last_dose: TimePeriod::parse("4y-4d")?,
        min_interval_last_to_prev: TimePeriod::parse("6m-4d")?,
    });

    // C. Pre-2009 Forecast Recommendations:
    // "If evaluation date is before 8/7/2009, min_age of dose 4 is 126d and min_interval for dose 3 is 28d"
    engine.rec_overrides.push(RecommendationOverrideRule {
        condition_eval_date_before: Some(NaiveDate::from_ymd_opt(2009, 8, 7).unwrap()),
        target_dose_number: 4,
        override_min_age: Some(TimePeriod::parse("126d")?),
        override_min_interval: Some(TimePeriod::parse("28d")?),
    });

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
    let result_a = engine.evaluate_patient(&patient_a, &history_a, eval_date);
    for (i, eval) in result_a.evaluations.iter().enumerate() {
        println!("Dose {}: date={}, cvx={}, status={:?}, reasons={:?}", 
                 i+1, eval.dose_date, eval.cvx, eval.status, eval.reasons);
    }
    println!("Forecast: status={:?}, earliest={:?}, recommended={:?}, overdue={:?}",
             result_a.forecasts[0].status, result_a.forecasts[0].earliest_date, 
             result_a.forecasts[0].recommended_date, result_a.forecasts[0].overdue_date);

    println!("\n--- Test Case B: 3-Dose Completion Rule (Dose 3 given at >= 4 years) ---");
    let result_b = engine.evaluate_patient(&patient_b, &history_b, eval_date);
    for (i, eval) in result_b.evaluations.iter().enumerate() {
        println!("Dose {}: date={}, cvx={}, status={:?}, reasons={:?}", 
                 i+1, eval.dose_date, eval.cvx, eval.status, eval.reasons);
    }
    println!("Forecast: status={:?} (Expected: Complete)", result_b.forecasts[0].status);

    println!("\n--- Test Case C: Pre-2009 Vaccine Interval Check ---");
    let result_c = engine.evaluate_patient(&patient_c, &history_c, eval_date);
    for (i, eval) in result_c.evaluations.iter().enumerate() {
        println!("Dose {}: date={}, cvx={}, status={:?}, reasons={:?}", 
                 i+1, eval.dose_date, eval.cvx, eval.status, eval.reasons);
    }
    println!("Forecast: status={:?}", result_c.forecasts[0].status);

    // 5. Run Micro-Benchmarks
    println!("\n--- Micro-benchmarking Forecaster Loop Performance ---");
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        // Run mock evaluations
        let _ = engine.evaluate_patient(&patient_a, &history_a, eval_date);
        let _ = engine.evaluate_patient(&patient_b, &history_b, eval_date);
    }
    let duration = start.elapsed();
    let total_evals = iterations * 2;
    let avg_latency = duration.as_secs_f64() / total_evals as f64 * 1_000_000.0;
    println!("Processed {} evaluations in {:?}", total_evals, duration);
    println!("Average latency per patient forecast: {:.3} microseconds", avg_latency);
    println!("Throughput: {:.1} evaluations/sec", total_evals as f64 / duration.as_secs_f64());

    Ok(())
}
