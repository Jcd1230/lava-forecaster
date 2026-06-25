use crate::db::{read_test_pack, write_test_pack};
use crate::eval_match::pair_evaluations_by_occurrence;
use crate::java_client::{
    get_java_expected_results, get_java_expected_results_bulk, is_group_supported_by_java,
};
use chrono::{Duration, NaiveDate};
use lava_forecaster::{
    evaluate_patient_all_groups,
    models::{
        Cvx, Dose, DoseEvaluation, ExpectedResults, Gender, Patient, SeriesForecast, SeriesStatus,
        UnifiedTestCase,
    },
    rules::get_all_groups,
};
use reqwest::blocking::Client;
use serde_json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_range(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u64;
        min + (self.next_u64() % range) as i64
    }

    pub fn choose<'a, T>(&mut self, choices: &'a [T]) -> &'a T {
        let idx = (self.next_u64() % choices.len() as u64) as usize;
        &choices[idx]
    }
}

pub fn get_cvx_codes_for_group(group: &str) -> Vec<u16> {
    let mut cvxs = Vec::new();
    for g in get_all_groups() {
        if g.group_name.eq_ignore_ascii_case(group) {
            for s in &g.series {
                for d in &s.doses {
                    for &c in d.allowed_cvx {
                        if !cvxs.contains(&c) {
                            cvxs.push(c);
                        }
                    }
                }
            }
        }
    }
    cvxs
}

pub fn generate_guided_case(
    rng: &mut SimpleRng,
    target_group: &str,
    case_index: usize,
    seed: u64,
) -> UnifiedTestCase {
    let birth_year = rng.next_range(2000, 2026) as i32;
    let birth_month = rng.next_range(1, 13) as u32;
    let birth_day = rng.next_range(1, 29) as u32;
    let dob = NaiveDate::from_ymd_opt(birth_year, birth_month, birth_day).unwrap();

    let gender = match rng.next_range(0, 3) {
        0 => Gender::Female,
        1 => Gender::Male,
        _ => Gender::Unknown,
    };

    let patient = Patient {
        birth_date: dob,
        gender,
        immunities: Vec::new(),
        contraindications: Vec::new(),
    };

    let allowed_cvxs = get_cvx_codes_for_group(target_group);
    if allowed_cvxs.is_empty() {
        panic!("No allowed CVX codes found for group {}", target_group);
    }

    let mut history = Vec::new();
    let num_doses = rng.next_range(0, 7) as usize; // up to 6 doses

    let current_eval_date = dob + Duration::days(rng.next_range(30, 365 * 10));

    for _ in 0..num_doses {
        let rust_results = evaluate_patient_all_groups(&patient, &history, current_eval_date);
        let rust_group = rust_results
            .iter()
            .find(|rg| rg.vaccine_group.eq_ignore_ascii_case(target_group));

        let mut possible_dates = Vec::new();
        if let Some(rg) = rust_group {
            if let Some(fc) = rg.forecasts.first() {
                if let SeriesStatus::NotComplete {
                    earliest_date,
                    recommended_date,
                    overdue_date,
                    ..
                } = &fc.status
                {
                    if let Some(d) = earliest_date {
                        possible_dates.push(*d - Duration::days(1));
                        possible_dates.push(*d);
                        possible_dates.push(*d + Duration::days(1));
                    }
                    if let Some(d) = recommended_date {
                        possible_dates.push(*d);
                    }
                    if let Some(d) = overdue_date {
                        possible_dates.push(*d);
                        possible_dates.push(*d + Duration::days(100));
                    }
                }
            }
        }

        let next_dose_date = if possible_dates.is_empty() || rng.next_range(0, 5) == 0 {
            let prev_date = history.last().map(|d: &Dose| d.date).unwrap_or(dob);
            prev_date + Duration::days(rng.next_range(28, 365))
        } else {
            *rng.choose(&possible_dates)
        };

        let next_dose_date = if next_dose_date < dob {
            dob
        } else {
            next_dose_date
        };
        let cvx_val = *rng.choose(&allowed_cvxs);

        history.push(Dose {
            date: next_dose_date,
            cvx: Cvx(cvx_val),
            is_valid: None,
        });

        history.sort_by_key(|d| d.date);
    }

    let last_dose_date = history.last().map(|d| d.date).unwrap_or(dob);
    let execution_date = if current_eval_date < last_dose_date {
        last_dose_date + Duration::days(rng.next_range(1, 180))
    } else {
        current_eval_date
    };

    UnifiedTestCase {
        name: format!(
            "fuzz_fail_{}_{}_{}",
            target_group.to_lowercase(),
            seed,
            case_index
        ),
        group: target_group.to_string(),
        focus_code: focus_code_for_group(target_group).to_string(),
        patient,
        history,
        execution_date,
        expected: None,
    }
}

fn focus_code_for_group(group: &str) -> &'static str {
    match group {
        _ if group.eq_ignore_ascii_case("POLIO") => "400",
        _ if group.eq_ignore_ascii_case("HEP_A") => "810",
        _ if group.eq_ignore_ascii_case("MMR") => "500",
        _ if group.eq_ignore_ascii_case("VARICELLA") => "600",
        _ if group.eq_ignore_ascii_case("ZOSTER") => "620",
        _ if group.eq_ignore_ascii_case("DTP") => "200",
        _ if group.eq_ignore_ascii_case("HEP_B") => "100",
        _ if group.eq_ignore_ascii_case("HPV") => "840",
        _ if group.eq_ignore_ascii_case("HIB") => "300",
        _ if group.eq_ignore_ascii_case("PNEUMOCOCCAL") => "750",
        _ if group.eq_ignore_ascii_case("MCV") => "830",
        _ if group.eq_ignore_ascii_case("MENB") => "835",
        _ if group.eq_ignore_ascii_case("ROTAVIRUS") => "820",
        _ if group.eq_ignore_ascii_case("INFLUENZA") => "800",
        _ if group.eq_ignore_ascii_case("COVID19") => "850",
        _ if group.eq_ignore_ascii_case("RSV") => "875",
        _ if group.eq_ignore_ascii_case("MPOX") => "860",
        _ if group.eq_ignore_ascii_case("H1N1") => "890",
        _ => "000",
    }
}

pub fn compare_results(
    _tc: &UnifiedTestCase,
    rust_evals: &[DoseEvaluation],
    rust_fc: Option<&SeriesForecast>,
    expected: &ExpectedResults,
) -> Vec<String> {
    let mut errors = Vec::new();

    for ((_date, _cvx), re, ee) in pair_evaluations_by_occurrence(rust_evals, &expected.evaluations)
    {
        match (re, ee) {
            (Some(re), Some(ee)) => {
                let status_matches = re.status == ee.status;
                if !status_matches {
                    errors.push(format!(
                        "Dose evaluation status mismatch for date {:?}, cvx {}: Rust={:?}, Java={:?}",
                        re.dose_date, re.cvx, re.status, ee.status
                    ));
                }
            }
            (None, Some(ee)) => {
                errors.push(format!(
                    "Dose evaluation missing in Rust for date {:?}, cvx {}",
                    ee.dose_date, ee.cvx
                ));
            }
            (Some(re), None) => {
                errors.push(format!(
                    "Dose evaluation missing in Java for date {:?}, cvx {}",
                    re.dose_date, re.cvx
                ));
            }
            (None, None) => {}
        }
    }

    let exp_fc = expected.forecasts.first();
    match (rust_fc, exp_fc) {
        (Some(rf), Some(ef)) => {
            if rf.status != ef.status {
                errors.push(format!(
                    "Forecast status mismatch: Rust={:?}, Java={:?}",
                    rf.status, ef.status
                ));
            }
            if rf.status.earliest_date() != ef.status.earliest_date() {
                errors.push(format!(
                    "Forecast earliest date mismatch: Rust={:?}, Java={:?}",
                    rf.status.earliest_date(),
                    ef.status.earliest_date()
                ));
            }
            if rf.status.recommended_date() != ef.status.recommended_date() {
                errors.push(format!(
                    "Forecast recommended date mismatch: Rust={:?}, Java={:?}",
                    rf.status.recommended_date(),
                    ef.status.recommended_date()
                ));
            }
            if rf.status.overdue_date() != ef.status.overdue_date() {
                errors.push(format!(
                    "Forecast overdue date mismatch: Rust={:?}, Java={:?}",
                    rf.status.overdue_date(),
                    ef.status.overdue_date()
                ));
            }
        }
        (None, None) => {}
        _ => {
            errors.push("Forecast availability mismatch (one is missing)".to_string());
        }
    }

    errors
}

pub fn shrink_case(client: &Client, java_url: &str, mut tc: UnifiedTestCase) -> UnifiedTestCase {
    println!(
        "Shrinking failing case: {} (originally {} doses)",
        tc.name,
        tc.history.len()
    );
    let mut i = 0;
    while i < tc.history.len() {
        let mut test_history = tc.history.clone();
        test_history.remove(i);

        let mut shrunked_tc = tc.clone();
        shrunked_tc.history = test_history;

        let rust_results = evaluate_patient_all_groups(
            &shrunked_tc.patient,
            &shrunked_tc.history,
            shrunked_tc.execution_date,
        );
        let rust_group = rust_results
            .iter()
            .find(|rg| rg.vaccine_group.eq_ignore_ascii_case(&shrunked_tc.group));
        let (rust_evals, rust_fc) = match rust_group {
            Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
            None => (Vec::new(), None),
        };

        if let Ok(java_res) = get_java_expected_results(client, java_url, &shrunked_tc) {
            let errors = compare_results(&shrunked_tc, &rust_evals, rust_fc.as_ref(), &java_res);
            if !errors.is_empty() {
                tc = shrunked_tc;
                tc.expected = Some(java_res);
                println!("  Shrunk history to {} doses", tc.history.len());
                continue;
            }
        }
        i += 1;
    }
    tc
}

// LTP database read/write helpers are now imported from crate::db

#[derive(Debug, Clone, Default)]
struct FuzzGroupStats {
    passed: usize,
    failed: usize,
}

pub fn run_fuzz(
    client: &Client,
    count: usize,
    seed_opt: Option<u64>,
    group_opt: Option<String>,
    java_url: &str,
    shrink: bool,
    bulk: bool,
    output_db: Option<String>,
) -> Result<(), String> {
    let seed = seed_opt.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    println!("=== Procedural Guided Fuzzing ===");
    println!("Seed: {}", seed);
    println!("Count: {}", count);
    if let Some(ref g) = group_opt {
        println!("Target Group: {}", g);
    } else {
        println!("Target Group: (any supported group)");
    }
    println!("ICE Url: {}", java_url);
    println!("Shrink enabled: {}", shrink);
    println!("Bulk mode enabled: {}", bulk);
    if let Some(ref db_path) = output_db {
        println!("Failing output database: {:?}", db_path);
    } else {
        println!("Failing output directory: tests/cases/failed/");
    }
    println!("---------------------------------");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        println!(
            "\n[Ctrl+C] Intercepted termination signal. Exiting gracefully after this batch..."
        );
        r.store(false, Ordering::SeqCst);
    }) {
        println!("Warning: Failed to set Ctrl-C handler: {}", e);
    }

    let mut rng = SimpleRng::new(seed);

    let supported_groups: Vec<String> = get_all_groups()
        .iter()
        .map(|g| g.group_name.to_string())
        .filter(|g| is_group_supported_by_java(g))
        .collect();

    if supported_groups.is_empty() {
        return Err("No supported vaccine groups found".to_string());
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut stats_map: BTreeMap<String, FuzzGroupStats> = BTreeMap::new();

    let chunk_size = 500;
    let mut i = 0;
    while i < count && running.load(Ordering::SeqCst) {
        let current_chunk_size = (count - i).min(chunk_size);
        let mut chunk_cases = Vec::new();

        for j in 0..current_chunk_size {
            let target_group = match &group_opt {
                Some(g) => g.clone(),
                None => rng.choose(&supported_groups).clone(),
            };
            let tc = generate_guided_case(&mut rng, &target_group, i + j, seed);
            chunk_cases.push(tc);
        }

        let mut rust_results = Vec::new();
        for tc in &chunk_cases {
            let res = evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
            let rust_group = res
                .iter()
                .find(|rg| rg.vaccine_group.eq_ignore_ascii_case(&tc.group));
            let (rust_evals, rust_fc) = match rust_group {
                Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                None => (Vec::new(), None),
            };
            rust_results.push((rust_evals, rust_fc));
        }

        let java_results = if bulk {
            let tc_refs: Vec<&UnifiedTestCase> = chunk_cases.iter().collect();
            match get_java_expected_results_bulk(client, java_url, &tc_refs) {
                Ok(res) => res,
                Err(e) => {
                    println!(
                        "Java bulk query failed: {}. Falling back to individual queries.",
                        e
                    );
                    let mut fallback_results = Vec::new();
                    for tc in &chunk_cases {
                        fallback_results.push(get_java_expected_results(client, java_url, tc));
                    }
                    fallback_results
                }
            }
        } else {
            let mut res = Vec::new();
            for tc in &chunk_cases {
                res.push(get_java_expected_results(client, java_url, tc));
            }
            res
        };

        for j in 0..current_chunk_size {
            let tc = &chunk_cases[j];
            let (rust_evals, rust_fc) = &rust_results[j];
            let case_index = i + j;

            match &java_results[j] {
                Ok(java_res) => {
                    let errors = compare_results(tc, rust_evals, rust_fc.as_ref(), java_res);
                    if errors.is_empty() {
                        passed += 1;
                        stats_map.entry(tc.group.clone()).or_default().passed += 1;
                    } else {
                        failed += 1;
                        stats_map.entry(tc.group.clone()).or_default().failed += 1;
                        println!("Fuzz #{} ({}): FAIL", case_index, tc.group);
                        for err in &errors {
                            println!("  - {}", err);
                        }

                        let mut final_case = tc.clone();
                        final_case.expected = Some(java_res.clone());

                        if shrink {
                            final_case = shrink_case(client, java_url, final_case);
                        }

                        if let Some(ref db_path) = output_db {
                            let mut cases = read_test_pack(Path::new(db_path)).unwrap_or_default();
                            cases.push(final_case);
                            if let Err(e) = write_test_pack(Path::new(db_path), &cases) {
                                println!(
                                    "Failed to save failing case to database {:?}: {}",
                                    db_path, e
                                );
                            } else {
                                println!("Saved minimal failing case to database {:?}", db_path);
                            }
                        } else {
                            let out_dir =
                                Path::new("tests/cases/failed").join(tc.group.to_uppercase());
                            if let Err(e) = fs::create_dir_all(&out_dir) {
                                println!("Failed to create folder {:?}: {}", out_dir, e);
                            }
                            let out_path = out_dir.join(format!("{}.json", final_case.name));
                            if let Ok(out_json) = serde_json::to_string_pretty(&final_case) {
                                if fs::write(&out_path, out_json).is_ok() {
                                    println!("Saved minimal failing case to {:?}", out_path);
                                } else {
                                    println!("Failed to write failing case to {:?}", out_path);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "Fuzz #{} ({}): Java ICE Query Error: {}",
                        case_index, tc.group, e
                    );
                    failed += 1;
                    stats_map.entry(tc.group.clone()).or_default().failed += 1;
                }
            }
        }

        i += current_chunk_size;
        println!(
            "Fuzzing progress: {}/{} completed (passed: {}, failed: {})",
            i, count, passed, failed
        );
    }

    println!("\n=== Fuzz Run Statistics ===");
    let total_executed = passed + failed;
    println!("Executed: {}", total_executed);
    println!("Passed  : {}", passed);
    println!("Failed  : {}", failed);
    if total_executed > 0 {
        println!(
            "Pass %  : {:.2}%",
            (passed as f64 / total_executed as f64) * 100.0
        );
    }

    if !stats_map.is_empty() {
        println!("\nPer-Group Statistics:");
        println!(
            "{:<16} | {:>8} | {:>8} | {:>8} | {:>8}",
            "Group", "Executed", "Passed", "Failed", "Pass %"
        );
        println!("{}", "-".repeat(62));
        for (group, gstats) in &stats_map {
            let total_g = gstats.passed + gstats.failed;
            let pass_pct = if total_g > 0 {
                (gstats.passed as f64 / total_g as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "{:<16} | {:>8} | {:>8} | {:>8} | {:>7.2}%",
                group, total_g, gstats.passed, gstats.failed, pass_pct,
            );
        }
    }
    println!("===========================");

    if failed > 0 {
        Err(format!(
            "{} fuzz tests failed comparison with Java ICE.",
            failed
        ))
    } else {
        Ok(())
    }
}
