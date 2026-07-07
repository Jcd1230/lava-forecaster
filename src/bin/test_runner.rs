use clap::Parser;
use lava_forecaster::{
    evaluate_patient_all_groups,
    models::{DoseEvaluation, DoseStatus, ExpectedResults, SeriesForecast, UnifiedTestCase},
};
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

#[path = "test_runner/cdc_csv.rs"]
mod cdc_csv;
#[path = "test_runner/db.rs"]
mod db;
#[path = "test_runner/eval_match.rs"]
mod eval_match;
#[path = "test_runner/fuzzer.rs"]
mod fuzzer;
#[path = "test_runner/importer.rs"]
mod importer;
#[path = "test_runner/java_client.rs"]
mod java_client;
#[path = "test_runner/summary_report.rs"]
mod summary_report;
#[path = "test_runner/ui.rs"]
mod ui;

use cdc_csv::run_cdc_csv_mode;
use db::{cleanup_empty_dirs, load_cases_from_source, read_test_pack, write_test_pack};
use eval_match::pair_evaluations_by_occurrence;
use importer::import_python_cases;
use java_client::{
    build_evaluate_payload, get_java_expected_results, get_java_expected_results_bulk,
    is_contraindicated, is_group_supported_by_java, is_immune, parse_legacy_xml,
    query_java_service, query_rust_rest_service,
};
use summary_report::{RunSummaryReport, build_case_summary, summarize_file, write_run_summary};
use ui::{SummaryCounts, print_comparison_table, print_summary, record_summary_result};

#[derive(Parser, Debug)]
#[command(name = "test_runner", about = "LAVA Forecaster Test Runner", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Inspects test cases from a directory or `.ltp` database file without running evaluations
    #[command(name = "inspect")]
    Inspect {
        /// Directory containing JSON test cases or a single `.ltp` database file
        cases_path: std::path::PathBuf,
        /// Filter cases only for a specific group (e.g. POLIO)
        #[arg(long)]
        group: Option<String>,
        /// Filter a single case by exact name
        #[arg(long)]
        case: Option<String>,
        /// Print only the number of matched cases and per-group counts
        #[arg(long)]
        count: bool,
        /// Print only matched group/name pairs
        #[arg(long)]
        list_cases: bool,
    },
    /// Extracts one named test case into a readable JSON fixture
    #[command(name = "promote")]
    Promote {
        /// Directory, JSON test case, `.test` file, or `.ltp` database to read from
        source: std::path::PathBuf,
        /// Output JSON fixture path
        output_json: std::path::PathBuf,
        /// Exact case name to promote
        #[arg(long)]
        case: String,
        /// Replace the output file if it already exists
        #[arg(long)]
        force: bool,
    },
    /// Lists test cases from a directory or `.ltp` database file
    #[command(name = "list")]
    List {
        /// Directory containing JSON test cases or a single `.ltp` database file
        cases_path: std::path::PathBuf,
        /// Filter cases only for a specific group (e.g. POLIO)
        #[arg(long)]
        group: Option<String>,
        /// Filter a single case by exact name
        #[arg(long)]
        case: Option<String>,
    },
    /// Imports Python JSON suites into individual test files
    #[command(name = "import-cdsi")]
    ImportCdsi {
        /// Input suite json file path
        input_suite: std::path::PathBuf,
        /// Output directory path
        output_dir: std::path::PathBuf,
    },
    /// Runs compliance testing against a CDC-formatted CSV sheet
    #[command(name = "run-cdc-csv")]
    RunCdcCsv {
        /// CSV file path
        csv_path: std::path::PathBuf,
        /// Filter cases only for a specific group (e.g. POLIO)
        #[arg(long)]
        group: Option<String>,
        /// Filter a single case by name or test ID
        #[arg(long)]
        case: Option<String>,
        /// Query a remote Rust forecaster REST service instead of native library calls
        #[arg(long)]
        rest_url: Option<String>,
        /// Print detailed side-by-side evaluation tables for all cases
        #[arg(long, short)]
        verbose: bool,
    },
    /// Runs fuzzer with CDSi guided case generator comparing Rust against live Java ICE
    #[command(name = "fuzz")]
    Fuzz {
        /// Number of fuzz cases to generate and test
        count: usize,
        /// Filter fuzzing for a specific vaccine group
        #[arg(long)]
        group: Option<String>,
        /// Java ICE server URL
        #[arg(long, default_value = "http://localhost:8080")]
        compare: String,
        /// Set a specific random seed for deterministic reproducibility
        #[arg(long)]
        fuzz_seed: Option<u64>,
        /// Shrink failing fuzzer history to minimal reproducing case
        #[arg(long)]
        shrink: bool,
        /// Batch requests to Java ICE in bulk (requires bulk ICE endpoint)
        #[arg(long)]
        bulk: bool,
        /// Save fuzzer failing cases directly into a compact `.ltp` file
        #[arg(long)]
        output_db: Option<String>,
    },
    /// Queries a live Java ICE server and records expected output snapshots
    #[command(name = "record")]
    Record {
        /// Cases directory or `.ltp` database file path
        cases_path: std::path::PathBuf,
        /// Java ICE server URL
        #[arg(default_value = "http://localhost:8080")]
        java_url: String,
    },
    /// Runs tests from a directory containing JSON test cases or a single `.ltp` database file
    #[command(name = "run")]
    Run {
        /// Directory containing JSON test cases or a single `.ltp` database file
        cases_path: std::path::PathBuf,
        /// Filter cases only for a specific group (e.g. POLIO)
        #[arg(long)]
        group: Option<String>,
        /// Filter a single case by name or test ID
        #[arg(long)]
        case: Option<String>,
        /// Compare outputs dynamically against a live Java ICE server
        #[arg(long, num_args = 0..=1, default_missing_value = "http://localhost:8080")]
        compare: Option<String>,
        /// Query a remote Rust forecaster REST service instead of native library calls
        #[arg(long)]
        rest_url: Option<String>,
        /// Print detailed side-by-side evaluation tables for all cases
        #[arg(long, short)]
        verbose: bool,
        /// Print a step-by-step decision trace to stdout; COVID19 also prints normalized policy facts
        #[arg(long, aliases = ["explain"])]
        trace: bool,
        /// Write structured mismatch summary JSON to this path
        #[arg(long)]
        summary: Option<std::path::PathBuf>,
        /// Print only the final summary and omit individual case failures/details
        #[arg(long)]
        summary_only: bool,
    },
    /// Summarizes a structured mismatch summary JSON file
    #[command(name = "summarize")]
    Summarize {
        /// Summary JSON file produced by `run --summary <path>`
        summary_path: std::path::PathBuf,
    },
    /// Runs regression check and re-categorizes test cases
    #[command(name = "reorganize")]
    Reorganize {
        /// Source path (directory or `.ltp` file)
        source: std::path::PathBuf,
        /// Target path (directory or `.ltp` file)
        target: std::path::PathBuf,
        /// Filter cases only for a specific group (e.g. POLIO)
        #[arg(long)]
        group: Option<String>,
        /// Filter a single case by name or test ID
        #[arg(long)]
        case: Option<String>,
        /// Compare outputs dynamically against a live Java ICE server
        #[arg(long, num_args = 0..=1, default_missing_value = "http://localhost:8080")]
        compare: Option<String>,
        /// Query a remote Rust forecaster REST service instead of native library calls
        #[arg(long)]
        rest_url: Option<String>,
        /// Print detailed side-by-side evaluation tables for all cases
        #[arg(long, short)]
        verbose: bool,
        /// Print a step-by-step decision trace to stdout; COVID19 also prints normalized policy facts
        #[arg(long, aliases = ["explain"])]
        trace: bool,
        /// Print only the final summary and omit individual case failures/details
        #[arg(long)]
        summary_only: bool,
    },
}

fn main() {
    let client = reqwest::blocking::Client::new();
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1] == "--run" {
            args[1] = "run".to_string();
        } else if args[1] == "--fuzz" {
            args[1] = "fuzz".to_string();
        } else if args[1] == "--reorganize" {
            args[1] = "reorganize".to_string();
        } else if args[1] == "--record" {
            args[1] = "record".to_string();
        } else if args[1] == "--run-cdc-csv" {
            args[1] = "run-cdc-csv".to_string();
        } else if args[1] == "--import-cdsi" {
            args[1] = "import-cdsi".to_string();
        } else if args[1] == "--list" {
            args[1] = "list".to_string();
        } else if args[1] == "--inspect" {
            args[1] = "inspect".to_string();
        } else if args[1] == "--promote" {
            args[1] = "promote".to_string();
        } else if args[1] == "--summarize" {
            args[1] = "summarize".to_string();
        }
    }

    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::Inspect {
            cases_path,
            group,
            case,
            count,
            list_cases,
        } => {
            let filter_group = group;
            let filter_case = case;

            let all_loaded_cases =
                load_cases_from_source(&cases_path).unwrap_or_else(|err| panic!("{}", err));
            let mut test_cases = filter_cases(
                all_loaded_cases,
                filter_group.as_deref(),
                filter_case.as_deref(),
            );
            test_cases.sort_by(|a, b| a.group.cmp(&b.group).then_with(|| a.name.cmp(&b.name)));

            if count {
                print_inspect_counts(&test_cases);
            } else if list_cases {
                print_inspect_case_list(&test_cases);
            } else {
                print_inspect_details(&test_cases);
            }

            if test_cases.is_empty() {
                std::process::exit(1);
            }
        }
        Commands::Promote {
            source,
            output_json,
            case,
            force,
        } => {
            if let Err(err) = promote_case(&source, &case, &output_json, force) {
                eprintln!("{}", err);
                std::process::exit(1);
            }
        }
        Commands::List {
            cases_path,
            group,
            case,
        } => {
            let filter_group = group;
            let filter_case = case;

            let all_loaded_cases =
                load_cases_from_source(&cases_path).unwrap_or_else(|err| panic!("{}", err));
            let test_cases = filter_cases(
                all_loaded_cases,
                filter_group.as_deref(),
                filter_case.as_deref(),
            );

            println!("Matched {} case(s)", test_cases.len());
            for tc in test_cases {
                println!("{}\t{}", tc.group, tc.name);
            }
        }
        Commands::ImportCdsi {
            input_suite,
            output_dir,
        } => {
            fs::create_dir_all(&output_dir).unwrap();
            import_python_cases(&input_suite, &output_dir);
        }
        Commands::RunCdcCsv {
            csv_path,
            group,
            case,
            rest_url,
            verbose,
        } => {
            run_cdc_csv_mode(&client, &csv_path, group, case, rest_url, verbose);
        }
        Commands::Fuzz {
            count,
            group,
            compare,
            fuzz_seed,
            shrink,
            bulk,
            output_db,
        } => {
            match fuzzer::run_fuzz(
                &client, count, fuzz_seed, group, &compare, shrink, bulk, output_db,
            ) {
                Ok(_) => std::process::exit(0),
                Err(e) => {
                    println!("Fuzz execution failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Record {
            cases_path,
            java_url,
        } => {
            let java_endpoint = format!(
                "{}/opencds-decision-support-service/api/resources/evaluateAtSpecifiedTime",
                java_url
            );

            if cases_path.is_file() {
                let mut test_cases =
                    read_test_pack(&cases_path).unwrap_or_else(|err| panic!("{}", err));
                let total_cases = test_cases.len();
                println!(
                    "Recording Java snapshots for {} cases in database...",
                    total_cases
                );
                for (i, tc) in test_cases.iter_mut().enumerate() {
                    if i % 10 == 0 || i == total_cases - 1 {
                        println!("Recording progress: {}/{}", i + 1, total_cases);
                    }
                    let payload =
                        build_evaluate_payload(&tc.patient, &tc.history, tc.execution_date);
                    let xml_out = query_java_service(&client, &java_endpoint, payload);
                    let mut java_res = parse_legacy_xml(&xml_out, &tc.focus_code);
                    for f in &mut java_res.forecasts {
                        f.series_name = tc.group.clone().into();
                    }
                    tc.expected = Some(java_res);
                }
                write_test_pack(&cases_path, &test_cases).unwrap_or_else(|err| panic!("{}", err));
                println!(
                    "Successfully recorded snapshots for {} cases in database.",
                    total_cases
                );
            } else {
                let entries = fs::read_dir(&cases_path).expect("Failed to read cases dir");
                let mut count = 0;
                for entry in entries {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    let extension = path.extension().map_or("", |e| e.to_str().unwrap_or(""));
                    if extension == "json" || extension == "test" {
                        let content = fs::read_to_string(&path).unwrap();
                        let tc_opt = if extension == "json" {
                            serde_json::from_str::<UnifiedTestCase>(&content).ok()
                        } else {
                            lava_forecaster::test_dsl::parse_test_case_dsl(&content).ok()
                        };
                        if let Some(mut tc) = tc_opt {
                            println!("Recording Java snapshot for: {}", tc.name);
                            let payload =
                                build_evaluate_payload(&tc.patient, &tc.history, tc.execution_date);
                            let xml_out = query_java_service(&client, &java_endpoint, payload);
                            let mut java_res = parse_legacy_xml(&xml_out, &tc.focus_code);

                            // Fill in series names for expected forecasts to match Rust
                            for f in &mut java_res.forecasts {
                                f.series_name = tc.group.clone().into(); // Fallback focus series
                            }

                            tc.expected = Some(java_res);

                            let out_json = serde_json::to_string_pretty(&tc).unwrap();
                            fs::write(&path, out_json).unwrap();
                            count += 1;
                        }
                    }
                }
                println!("Successfully recorded snapshots for {} cases.", count);
            }
        }
        Commands::Run {
            cases_path,
            group,
            case,
            compare,
            rest_url,
            verbose,
            trace,
            summary,
            summary_only,
        } => {
            let filter_group = group;
            let filter_case = case;
            let compare_java_url = compare;
            let trace_mode = trace;
            let summary_path = summary;
            let mut run_summary = summary_path.as_ref().map(|_| {
                RunSummaryReport::new(
                    &cases_path,
                    filter_group.clone(),
                    filter_case.clone(),
                    compare_java_url.is_some(),
                    rest_url.is_some(),
                )
            });

            let all_loaded_cases =
                load_cases_from_source(&cases_path).unwrap_or_else(|err| panic!("{}", err));
            let test_cases = filter_cases(
                all_loaded_cases,
                filter_group.as_deref(),
                filter_case.as_deref(),
            );

            let mut java_expected_map = HashMap::new();
            if let Some(ref j_url) = compare_java_url {
                let java_test_cases: Vec<&UnifiedTestCase> = test_cases
                    .iter()
                    .filter(|tc| is_group_supported_by_java(&tc.group))
                    .collect();
                if !java_test_cases.is_empty() {
                    println!(
                        "Querying {} cases in bulk from Java ICE at {}...",
                        java_test_cases.len(),
                        j_url
                    );
                    for chunk in java_test_cases.chunks(500) {
                        match get_java_expected_results_bulk(&client, j_url, chunk) {
                            Ok(results) => {
                                for (i, res) in results.into_iter().enumerate() {
                                    let name = chunk[i].name.clone();
                                    java_expected_map.insert(name, res);
                                }
                            }
                            Err(e) => {
                                println!(
                                    "Java bulk query failed: {}. Falling back to individual requests.",
                                    e
                                );
                                for &tc in chunk {
                                    let name = tc.name.clone();
                                    let ind_res = get_java_expected_results(&client, j_url, tc);
                                    java_expected_map.insert(name, ind_res);
                                }
                            }
                        }
                    }
                }
            }

            struct CaseOutcome {
                name: String,
                group: String,
                is_ok: bool,
                errors: Vec<String>,
                rust_evals: Vec<lava_forecaster::models::DoseEvaluation>,
                rust_fc: Option<SeriesForecast>,
                expected_results: Option<ExpectedResults>,
                traces: Vec<lava_forecaster::engine::DecisionTrace>,
                covid_trace: Option<String>,
                case_summary: Option<crate::summary_report::CaseSummaryReport>,
            }

            let java_expected_map = std::sync::Mutex::new(java_expected_map);

            let outcomes: Vec<CaseOutcome> = test_cases
                .par_iter()
                .map(|tc| {
                    let (rust_evals, rust_fc, traces) = if let Some(ref r_url) = rest_url {
                        match query_rust_rest_service(&client, r_url, tc) {
                            Ok(resp) => {
                                let rust_group = resp
                                    .vaccine_groups
                                    .iter()
                                    .find(|rg| rg.vaccine_group == tc.group)
                                    .cloned();
                                let (rust_evals, rust_fc) = match rust_group {
                                    Some(rg) => {
                                        (rg.evaluations.to_vec(), rg.forecasts.first().cloned())
                                    }
                                    None => (Vec::new(), None),
                                };
                                (rust_evals, rust_fc, Vec::new())
                            }
                            Err(e) => {
                                let errors = vec![format!("Rust REST Error: {}", e)];
                                let case_summary = build_case_summary(
                                    tc,
                                    false,
                                    &[],
                                    None,
                                    None,
                                    &errors,
                                    compare_java_url.is_some(),
                                );
                                return CaseOutcome {
                                    name: tc.name.clone(),
                                    group: tc.group.clone(),
                                    is_ok: false,
                                    errors,
                                    rust_evals: Vec::new(),
                                    rust_fc: None,
                                    expected_results: None,
                                    traces: Vec::new(),
                                    covid_trace: None,
                                    case_summary: Some(case_summary),
                                };
                            }
                        }
                    } else {
                        if trace_mode {
                            lava_forecaster::engine::clear_traces();
                            lava_forecaster::engine::set_trace_enabled(true);
                        }
                        let rust_results =
                            evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
                        let traces = if trace_mode {
                            let t = lava_forecaster::engine::get_traces();
                            lava_forecaster::engine::set_trace_enabled(false);
                            t
                        } else {
                            Vec::new()
                        };
                        let rust_group = rust_results.iter().find(|rg| rg.vaccine_group == tc.group);
                        let (rust_evals, rust_fc) = match rust_group {
                            Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                            None => (Vec::new(), None),
                        };
                        (rust_evals, rust_fc, traces)
                    };

                    let covid_trace = build_covid_policy_trace(
                        tc,
                        &rust_evals,
                        rust_fc.as_ref(),
                        trace_mode,
                    );

                    let expected_results = if compare_java_url.is_some()
                        && is_group_supported_by_java(&tc.group)
                    {
                        let mut map = java_expected_map.lock().unwrap();
                        match map.remove(&tc.name) {
                            Some(Ok(res)) => Some(res),
                            Some(Err(e)) => {
                                let errors = vec![format!("Java Query Error: {}", e)];
                                let case_summary = build_case_summary(
                                    tc,
                                    false,
                                    &rust_evals,
                                    rust_fc.as_ref(),
                                    None,
                                    &errors,
                                    compare_java_url.is_some(),
                                );
                                return CaseOutcome {
                                    name: tc.name.clone(),
                                    group: tc.group.clone(),
                                    is_ok: false,
                                    errors,
                                    rust_evals,
                                    rust_fc,
                                    expected_results: None,
                                    traces: Vec::new(),
                                    covid_trace,
                                    case_summary: Some(case_summary),
                                };
                            }
                            None => {
                                let errors = vec!["Java expected results missing from bulk results".to_string()];
                                let case_summary = build_case_summary(
                                    tc,
                                    false,
                                    &rust_evals,
                                    rust_fc.as_ref(),
                                    None,
                                    &errors,
                                    compare_java_url.is_some(),
                                );
                                return CaseOutcome {
                                    name: tc.name.clone(),
                                    group: tc.group.clone(),
                                    is_ok: false,
                                    errors,
                                    rust_evals,
                                    rust_fc,
                                    expected_results: None,
                                    traces: Vec::new(),
                                    covid_trace,
                                    case_summary: Some(case_summary),
                                };
                            }
                        }
                    } else {
                        tc.expected.clone()
                    };

                    let mut is_ok = true;
                    let mut errors = Vec::new();

                    if let Some(expected) = &expected_results {
                        // 1. Verify evaluations
                        for ((_date, _cvx), re, ee) in
                            pair_evaluations_by_occurrence(&rust_evals, &expected.evaluations)
                        {
                            match (re, ee) {
                                (Some(re), Some(ee)) => {
                                    let mut status_matches = re.status == ee.status;
                                    if !status_matches {
                                        if re.status == DoseStatus::Valid
                                            && ee.status == DoseStatus::Accepted
                                            && is_immune(&tc.patient, &tc.group, tc.execution_date)
                                        {
                                            status_matches = true;
                                        } else if compare_java_url.is_some()
                                            && !tc.patient.contraindications.is_empty()
                                        {
                                            status_matches = true;
                                        }
                                    }
                                    if !status_matches {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Evaluation status mismatch for dose ({:?}, {}): Rust={:?} (reasons={:?}), Expected={:?}",
                                            re.dose_date, re.cvx, re.status, re.reasons, ee.status
                                        ));
                                    }
                                }
                                (None, Some(ee)) => {
                                    is_ok = false;
                                    errors.push(format!(
                                        "Evaluation missing in Rust for dose ({:?}, {})",
                                        ee.dose_date, ee.cvx
                                    ));
                                }
                                (Some(re), None) => {
                                    is_ok = false;
                                    errors.push(format!(
                                        "Evaluation missing in Expected for dose ({:?}, {})",
                                        re.dose_date, re.cvx
                                    ));
                                }
                                (None, None) => {}
                            }
                        }
                        // 2. Verify forecasts
                        let exp_fc = expected.forecasts.first();

                        match (rust_fc.as_ref(), exp_fc) {
                            (Some(rf), Some(ef)) => {
                                let is_comp = compare_java_url.is_some();
                                let is_contra =
                                    is_contraindicated(&tc.patient, &tc.group, tc.execution_date)
                                        || !tc.patient.contraindications.is_empty();
                                if !(is_comp && is_contra) {
                                    if rf.status != ef.status {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast status mismatch: Rust={:?}, Expected={:?}",
                                            rf.status, ef.status
                                        ));
                                    }
                                    if rf.status.earliest_date() != ef.status.earliest_date() {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast earliest date mismatch: Rust={:?}, Expected={:?}",
                                            rf.status.earliest_date(),
                                            ef.status.earliest_date()
                                        ));
                                    }
                                    if rf.status.recommended_date() != ef.status.recommended_date() {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast recommended date mismatch: Rust={:?}, Expected={:?}",
                                            rf.status.recommended_date(), ef.status.recommended_date()
                                        ));
                                    }
                                    if rf.status.overdue_date() != ef.status.overdue_date() {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast overdue date mismatch: Rust={:?}, Expected={:?}",
                                            rf.status.overdue_date(),
                                            ef.status.overdue_date()
                                        ));
                                    }
                                }
                            }
                            (None, None) => {}
                            _ => {
                                is_ok = false;
                                errors.push(
                                    "Forecast availability mismatch (one is missing)".to_string(),
                                );
                            }
                        }
                    } else {
                        is_ok = false;
                        errors.push("No expected snapshots recorded in test case file. Run with --record first or use --compare.".to_string());
                    }

                    // Build CaseSummaryReport
                    let case_summary = build_case_summary(
                        tc,
                        is_ok,
                        &rust_evals,
                        rust_fc.as_ref(),
                        expected_results.as_ref(),
                        &errors,
                        compare_java_url.is_some(),
                    );

                    CaseOutcome {
                        name: tc.name.clone(),
                        group: tc.group.clone(),
                        is_ok,
                        errors,
                        rust_evals,
                        rust_fc,
                        expected_results,
                        traces,
                        covid_trace,
                        case_summary: Some(case_summary),
                    }
                })
                .collect();

            let mut total = 0;
            let mut passed = 0;
            let mut failed = 0;
            let mut group_summary: BTreeMap<String, SummaryCounts> = BTreeMap::new();
            let mut failed_details = Vec::new();

            let total_cases_count = test_cases.len();
            for outcome in outcomes {
                total += 1;
                if summary_only && total % 5000 == 0 {
                    println!(
                        "Progress: {} / {} cases processed...",
                        total, total_cases_count
                    );
                }

                // Print trace decisions if trace mode is enabled
                if trace_mode && !outcome.traces.is_empty() {
                    println!("\n=== Trace for: {} ===", outcome.name);
                    println!("{:<40} | {:<15} | {}", "Step", "Location", "Description");
                    println!("{}", "-".repeat(100));
                    for trace in &outcome.traces {
                        let location = format!("{}:{}", trace.source_file, trace.line_number);
                        println!(
                            "{:<40} | {:<15} | {}",
                            trace.step, location, trace.description
                        );
                    }
                    println!("=== End Trace ===");
                }

                if let Some(covid_trace) = &outcome.covid_trace {
                    println!("\n=== COVID Policy Trace for: {} ===", outcome.name);
                    print!("{}", covid_trace);
                    println!("=== End COVID Policy Trace ===");
                }

                if verbose {
                    print_comparison_table(
                        &outcome.name,
                        &outcome.group,
                        &outcome.rust_evals,
                        outcome.rust_fc.as_ref(),
                        outcome
                            .expected_results
                            .as_ref()
                            .map(|e| e.evaluations.as_slice())
                            .unwrap_or(&[]),
                        outcome
                            .expected_results
                            .as_ref()
                            .and_then(|e| e.forecasts.first()),
                        &outcome.errors,
                    );
                } else if !outcome.is_ok {
                    if !summary_only {
                        println!("\nTest Case: \x1b[91m{}\x1b[0m (FAIL)", outcome.name);
                        for err in &outcome.errors {
                            println!("  - \x1b[91m{}\x1b[0m", err);
                        }
                    }
                }

                if outcome.is_ok {
                    passed += 1;
                    record_summary_result(&mut group_summary, &outcome.group, true);
                } else {
                    failed += 1;
                    record_summary_result(&mut group_summary, &outcome.group, false);
                    failed_details.push((outcome.name.clone(), outcome.errors.clone()));
                }

                if let (Some(report), Some(case_summary)) = (&mut run_summary, outcome.case_summary)
                {
                    report.record_case(case_summary);
                }
            }

            if total == 0 && (filter_group.is_some() || filter_case.is_some()) {
                println!("No test cases matched the provided filters.");
                println!(
                    "Hint: use `test_runner inspect <path> [--group GROUP] --list-cases` to inspect available case names."
                );
            }

            print_summary(
                "Rust-Native Test Runner Summary",
                total,
                passed,
                failed,
                &group_summary,
            );

            if let (Some(path), Some(report)) = (summary_path.as_ref(), run_summary.as_ref()) {
                write_run_summary(path, report).unwrap_or_else(|err| panic!("{}", err));
                println!("Wrote structured summary to {:?}", path);
            }

            if failed > 0 {
                if !summary_only {
                    println!("\nFailed Cases:");
                    for (name, errs) in failed_details {
                        println!("  FAIL: {}", name);
                        for err in errs {
                            println!("    - {}", err);
                        }
                    }
                }
                std::process::exit(1);
            } else {
                println!("All checked tests passed successfully!");
                std::process::exit(0);
            }
        }
        Commands::Summarize { summary_path } => {
            summarize_file(&summary_path).unwrap_or_else(|err| panic!("{}", err));
        }
        Commands::Reorganize {
            source,
            target,
            group,
            case,
            compare,
            rest_url,
            verbose,
            trace,
            summary_only,
        } => {
            let source_path = source;
            let target_path = target;
            let filter_group = group;
            let filter_case = case;
            let compare_java_url = compare;
            let trace_mode = trace;

            let mut loaded_cases_with_paths = Vec::new();
            if source_path.is_dir() {
                fn walk_and_load(
                    dir: &Path,
                    list: &mut Vec<(UnifiedTestCase, std::path::PathBuf)>,
                ) {
                    if let Ok(entries) = fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                walk_and_load(&path, list);
                            } else {
                                let extension =
                                    path.extension().map_or("", |e| e.to_str().unwrap_or(""));
                                if extension == "json" || extension == "test" {
                                    if let Ok(content) = fs::read_to_string(&path) {
                                        let tc_opt = if extension == "json" {
                                            serde_json::from_str::<UnifiedTestCase>(&content).ok()
                                        } else {
                                            lava_forecaster::test_dsl::parse_test_case_dsl(&content)
                                                .ok()
                                        };
                                        if let Some(tc) = tc_opt {
                                            list.push((tc, path));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                walk_and_load(&source_path, &mut loaded_cases_with_paths);
            } else if source_path.is_file() {
                let cases = read_test_pack(&source_path).unwrap_or_else(|err| panic!("{}", err));
                for tc in cases {
                    loaded_cases_with_paths.push((tc, source_path.to_path_buf()));
                }
            } else {
                panic!(
                    "Source path {:?} does not exist or is not a file/directory",
                    source_path
                );
            }

            let mut test_cases = Vec::new();
            for (tc, path) in loaded_cases_with_paths {
                if let Some(ref fg) = filter_group {
                    if !tc.group.eq_ignore_ascii_case(fg) {
                        continue;
                    }
                }
                if let Some(ref fc) = filter_case {
                    if tc.name != *fc {
                        continue;
                    }
                }
                test_cases.push((tc, path));
            }

            let mut java_expected_map = HashMap::new();
            if let Some(ref j_url) = compare_java_url {
                let java_test_cases: Vec<&UnifiedTestCase> = test_cases
                    .iter()
                    .map(|(tc, _)| tc)
                    .filter(|tc| is_group_supported_by_java(&tc.group))
                    .collect();
                if !java_test_cases.is_empty() {
                    println!(
                        "Querying {} cases in bulk from Java ICE at {}...",
                        java_test_cases.len(),
                        j_url
                    );
                    for chunk in java_test_cases.chunks(500) {
                        match get_java_expected_results_bulk(&client, j_url, chunk) {
                            Ok(results) => {
                                for (i, res) in results.into_iter().enumerate() {
                                    let name = chunk[i].name.clone();
                                    java_expected_map.insert(name, res);
                                }
                            }
                            Err(e) => {
                                println!(
                                    "Java bulk query failed: {}. Falling back to individual requests.",
                                    e
                                );
                                for &tc in chunk {
                                    let name = tc.name.clone();
                                    let ind_res = get_java_expected_results(&client, j_url, tc);
                                    java_expected_map.insert(name, ind_res);
                                }
                            }
                        }
                    }
                }
            }

            let mut total = 0;
            let mut passed = 0;
            let mut failed = 0;
            let mut group_summary: BTreeMap<String, SummaryCounts> = BTreeMap::new();
            let mut failed_details = Vec::new();

            let mut passed_cases_out = Vec::new();
            let mut failed_cases_out = Vec::new();

            let total_cases_count = test_cases.len();
            for (mut tc, old_path) in test_cases {
                total += 1;
                if summary_only && total % 5000 == 0 {
                    println!(
                        "Progress: {} / {} cases processed...",
                        total, total_cases_count
                    );
                }

                let (rust_evals, rust_fc) = if let Some(ref r_url) = rest_url {
                    match query_rust_rest_service(&client, r_url, &tc) {
                        Ok(resp) => {
                            let rust_group = resp
                                .vaccine_groups
                                .iter()
                                .find(|rg| rg.vaccine_group == tc.group)
                                .cloned();
                            match rust_group {
                                Some(rg) => {
                                    (rg.evaluations.to_vec(), rg.forecasts.first().cloned())
                                }
                                None => (Vec::new(), None),
                            }
                        }
                        Err(e) => {
                            failed += 1;
                            record_summary_result(&mut group_summary, &tc.group, false);
                            failed_details
                                .push((tc.name.clone(), vec![format!("Rust REST Error: {}", e)]));
                            failed_cases_out.push((tc, old_path));
                            continue;
                        }
                    }
                } else {
                    if trace_mode {
                        lava_forecaster::engine::clear_traces();
                        lava_forecaster::engine::set_trace_enabled(true);
                    }
                    let rust_results =
                        evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
                    if trace_mode {
                        lava_forecaster::engine::set_trace_enabled(false);
                    }
                    let rust_group = rust_results.iter().find(|rg| rg.vaccine_group == tc.group);
                    match rust_group {
                        Some(rg) => (rg.evaluations.to_vec(), rg.forecasts.first().cloned()),
                        None => (Vec::new(), None),
                    }
                };

                let expected_results = if compare_java_url.is_some()
                    && is_group_supported_by_java(&tc.group)
                {
                    match java_expected_map.remove(&tc.name) {
                        Some(Ok(res)) => Some(res),
                        Some(Err(e)) => {
                            failed += 1;
                            record_summary_result(&mut group_summary, &tc.group, false);
                            failed_details
                                .push((tc.name.clone(), vec![format!("Java Query Error: {}", e)]));
                            failed_cases_out.push((tc, old_path));
                            continue;
                        }
                        None => {
                            failed += 1;
                            record_summary_result(&mut group_summary, &tc.group, false);
                            failed_details.push((
                                tc.name.clone(),
                                vec!["Java expected results missing from bulk results".to_string()],
                            ));
                            failed_cases_out.push((tc, old_path));
                            continue;
                        }
                    }
                } else {
                    tc.expected.clone()
                };

                if trace_mode {
                    let traces = lava_forecaster::engine::get_traces();
                    if !traces.is_empty() {
                        println!("\n=== Trace for: {} ===", tc.name);
                        for trace in &traces {
                            let location = format!("{}:{}", trace.source_file, trace.line_number);
                            println!(
                                "{:<40} | {:<15} | {}",
                                trace.step, location, trace.description
                            );
                        }
                    }
                }

                if let Some(covid_trace) =
                    build_covid_policy_trace(&tc, &rust_evals, rust_fc.as_ref(), trace_mode)
                {
                    println!("\n=== COVID Policy Trace for: {} ===", tc.name);
                    print!("{}", covid_trace);
                    println!("=== End COVID Policy Trace ===");
                }

                let mut is_ok = true;
                let mut errors = Vec::new();

                if let Some(expected) = &expected_results {
                    for ((_date, _cvx), re, ee) in
                        pair_evaluations_by_occurrence(&rust_evals, &expected.evaluations)
                    {
                        match (re, ee) {
                            (Some(re), Some(ee)) => {
                                let mut status_matches = re.status == ee.status;
                                if !status_matches {
                                    if re.status == DoseStatus::Valid
                                        && ee.status == DoseStatus::Accepted
                                        && is_immune(&tc.patient, &tc.group, tc.execution_date)
                                    {
                                        status_matches = true;
                                    } else if compare_java_url.is_some()
                                        && !tc.patient.contraindications.is_empty()
                                    {
                                        status_matches = true;
                                    }
                                }
                                if !status_matches {
                                    is_ok = false;
                                    errors.push(format!(
                                    "Evaluation status mismatch for dose ({:?}, {}): Rust={:?} (reasons={:?}), Expected={:?}",
                                    re.dose_date, re.cvx, re.status, re.reasons, ee.status
                                ));
                                }
                            }
                            (None, Some(ee)) => {
                                is_ok = false;
                                errors.push(format!(
                                    "Evaluation missing in Rust for dose ({:?}, {})",
                                    ee.dose_date, ee.cvx
                                ));
                            }
                            (Some(re), None) => {
                                is_ok = false;
                                errors.push(format!(
                                    "Evaluation missing in Expected for dose ({:?}, {})",
                                    re.dose_date, re.cvx
                                ));
                            }
                            (None, None) => {}
                        }
                    }
                    let exp_fc = expected.forecasts.first();
                    match (rust_fc.as_ref(), exp_fc) {
                        (Some(rf), Some(ef)) => {
                            let is_comp = compare_java_url.is_some();
                            let is_contra =
                                is_contraindicated(&tc.patient, &tc.group, tc.execution_date)
                                    || !tc.patient.contraindications.is_empty();
                            if !(is_comp && is_contra) {
                                if rf.status != ef.status {
                                    is_ok = false;
                                    errors.push(format!(
                                        "Forecast status mismatch: Rust={:?}, Expected={:?}",
                                        rf.status, ef.status
                                    ));
                                }
                                if rf.status.earliest_date() != ef.status.earliest_date() {
                                    is_ok = false;
                                    errors.push(format!(
                                        "Forecast earliest date mismatch: Rust={:?}, Expected={:?}",
                                        rf.status.earliest_date(),
                                        ef.status.earliest_date()
                                    ));
                                }
                                if rf.status.recommended_date() != ef.status.recommended_date() {
                                    is_ok = false;
                                    errors.push(format!("Forecast recommended date mismatch: Rust={:?}, Expected={:?}", rf.status.recommended_date(), ef.status.recommended_date()));
                                }
                                if rf.status.overdue_date() != ef.status.overdue_date() {
                                    is_ok = false;
                                    errors.push(format!(
                                        "Forecast overdue date mismatch: Rust={:?}, Expected={:?}",
                                        rf.status.overdue_date(),
                                        ef.status.overdue_date()
                                    ));
                                }
                            }
                        }
                        (None, None) => {}
                        _ => {
                            is_ok = false;
                            errors.push(
                                "Forecast availability mismatch (one is missing)".to_string(),
                            );
                        }
                    }
                } else {
                    is_ok = false;
                    errors.push("No expected snapshots recorded.".to_string());
                }

                if verbose {
                    print_comparison_table(
                        &tc.name,
                        &tc.group,
                        &rust_evals,
                        rust_fc.as_ref(),
                        expected_results
                            .as_ref()
                            .map(|e| e.evaluations.as_slice())
                            .unwrap_or(&[]),
                        expected_results.as_ref().and_then(|e| e.forecasts.first()),
                        &errors,
                    );
                } else if !is_ok {
                    if !summary_only {
                        println!("\nTest Case: \x1b[91m{}\x1b[0m (FAIL)", tc.name);
                        for err in &errors {
                            println!("  - \x1b[91m{}\x1b[0m", err);
                        }
                    }
                }

                if let Some(expected) = expected_results {
                    tc.expected = Some(expected);
                }

                if is_ok {
                    passed += 1;
                    record_summary_result(&mut group_summary, &tc.group, true);
                    passed_cases_out.push((tc, old_path));
                } else {
                    failed += 1;
                    record_summary_result(&mut group_summary, &tc.group, false);
                    failed_details.push((tc.name.clone(), errors));
                    failed_cases_out.push((tc, old_path));
                }
            }

            print_summary(
                "Reorganization Regression Summary",
                total,
                passed,
                failed,
                &group_summary,
            );

            let target_is_db = target_path
                .extension()
                .map_or(false, |ext| ext == "ltp" || ext == "bin" || ext == "db");
            if target_is_db
                || (!target_path.exists()
                    && target_path.to_str().map_or(false, |s| {
                        s.ends_with(".ltp") || s.ends_with(".bin") || s.ends_with(".db")
                    }))
            {
                let mut all_cases_out = Vec::new();
                for (tc, _) in passed_cases_out {
                    all_cases_out.push(tc);
                }
                for (tc, _) in failed_cases_out {
                    all_cases_out.push(tc);
                }
                write_test_pack(&target_path, &all_cases_out)
                    .unwrap_or_else(|err| panic!("{}", err));
                println!(
                    "Successfully reorganized: packed all {} cases into database file {:?}",
                    all_cases_out.len(),
                    target_path
                );
            } else {
                let mut files_to_keep = std::collections::HashSet::new();

                for (tc, old_path) in passed_cases_out {
                    let group_dir = target_path.join("passed").join(tc.group.to_uppercase());
                    fs::create_dir_all(&group_dir).unwrap();
                    let new_path = group_dir.join(format!("{}.json", tc.name));

                    let out_json = serde_json::to_string_pretty(&tc).unwrap();
                    fs::write(&new_path, out_json).unwrap();
                    files_to_keep.insert(new_path.canonicalize().unwrap_or(new_path.clone()));

                    let path = &old_path;
                    let canon_old = path.canonicalize().ok();
                    let canon_new = new_path.canonicalize().ok();
                    if canon_old.is_some() && canon_old != canon_new {
                        if path.exists() && path.is_file() {
                            let _ = fs::remove_file(path);
                        }
                    }
                }

                for (tc, old_path) in failed_cases_out {
                    let group_dir = target_path.join("failed").join(tc.group.to_uppercase());
                    fs::create_dir_all(&group_dir).unwrap();
                    let new_path = group_dir.join(format!("{}.json", tc.name));

                    let out_json = serde_json::to_string_pretty(&tc).unwrap();
                    fs::write(&new_path, out_json).unwrap();
                    files_to_keep.insert(new_path.canonicalize().unwrap_or(new_path.clone()));

                    let path = &old_path;
                    let canon_old = path.canonicalize().ok();
                    let canon_new = new_path.canonicalize().ok();
                    if canon_old.is_some() && canon_old != canon_new {
                        if path.exists() && path.is_file() {
                            let _ = fs::remove_file(path);
                        }
                    }
                }

                cleanup_empty_dirs(&target_path);
                println!(
                    "Successfully reorganized test cases under directory {:?}",
                    target_path
                );
            }
        }
    }
}

fn build_covid_policy_trace(
    tc: &UnifiedTestCase,
    rust_evals: &[DoseEvaluation],
    rust_fc: Option<&SeriesForecast>,
    trace_mode: bool,
) -> Option<String> {
    if !trace_mode || !tc.group.eq_ignore_ascii_case("COVID19") {
        return None;
    }

    Some(
        lava_forecaster::rules::covid19::trace::format_aug2025_policy_trace(
            &tc.patient,
            &tc.history,
            tc.execution_date,
            rust_evals,
            rust_fc,
        ),
    )
}

fn filter_cases(
    cases: Vec<UnifiedTestCase>,
    filter_group: Option<&str>,
    filter_case: Option<&str>,
) -> Vec<UnifiedTestCase> {
    cases
        .into_iter()
        .filter(|tc| {
            if let Some(fg) = filter_group {
                if !tc.group.eq_ignore_ascii_case(fg) {
                    return false;
                }
            }
            if let Some(fc) = filter_case {
                if tc.name != fc {
                    return false;
                }
            }
            true
        })
        .collect()
}

fn promote_case(
    source: &Path,
    case_name: &str,
    output_json: &Path,
    force: bool,
) -> Result<(), String> {
    let cases = load_cases_from_source(source)?;
    let matches = filter_cases(cases, None, Some(case_name));

    if matches.is_empty() {
        return Err(format!(
            "No case named `{}` found in {:?}. Use `test_runner inspect {:?} --list-cases` to inspect available cases.",
            case_name, source, source
        ));
    }
    if matches.len() > 1 {
        return Err(format!(
            "Found {} cases named `{}` in {:?}; promotion requires exactly one match.",
            matches.len(),
            case_name,
            source
        ));
    }
    if output_json.exists() && !force {
        return Err(format!(
            "Output file {:?} already exists. Re-run with --force to replace it.",
            output_json
        ));
    }

    if let Some(parent) = output_json.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("Failed to create output directory {:?}: {}", parent, err)
            })?;
        }
    }

    let case = matches.into_iter().next().expect("case existence checked");
    let out_json = serde_json::to_string_pretty(&case)
        .map_err(|err| format!("Failed to serialize promoted case: {}", err))?;
    fs::write(output_json, out_json)
        .map_err(|err| format!("Failed to write promoted case {:?}: {}", output_json, err))?;

    println!(
        "Promoted case `{}` from {:?} to {:?}",
        case_name, source, output_json
    );
    if case.expected.is_some() {
        println!("Expected snapshot: preserved");
    } else {
        println!("Expected snapshot: none");
    }

    Ok(())
}

fn print_inspect_counts(test_cases: &[UnifiedTestCase]) {
    let mut group_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for tc in test_cases {
        *group_counts.entry(&tc.group).or_default() += 1;
    }

    println!("Matched {} case(s)", test_cases.len());
    if !group_counts.is_empty() {
        println!();
        println!("{:<16} | {:>8}", "Group", "Cases");
        println!("{}", "-".repeat(27));
        for (group, count) in group_counts {
            println!("{:<16} | {:>8}", group, count);
        }
    }
}

fn print_inspect_case_list(test_cases: &[UnifiedTestCase]) {
    println!("Matched {} case(s)", test_cases.len());
    for tc in test_cases {
        println!("{}\t{}", tc.group, tc.name);
    }
}

fn print_inspect_details(test_cases: &[UnifiedTestCase]) {
    print_inspect_counts(test_cases);
    for tc in test_cases {
        println!();
        print_inspect_case_details(tc);
    }
}

fn print_inspect_case_details(tc: &UnifiedTestCase) {
    println!("Case: {}", tc.name);
    println!("Group: {}", tc.group);
    println!("Focus CVX: {}", tc.focus_code);
    println!("Birth Date: {}", tc.patient.birth_date);
    println!("Gender: {:?}", tc.patient.gender);
    println!("Execution Date: {}", tc.execution_date);
    println!("History: {} dose(s)", tc.history.len());
    for (idx, dose) in tc.history.iter().enumerate() {
        let valid_marker = dose
            .is_valid
            .map(|valid| format!(" is_valid={}", valid))
            .unwrap_or_default();
        println!(
            "  {:>2}. {} CVX {}{}",
            idx + 1,
            dose.date,
            dose.cvx,
            valid_marker
        );
    }

    match &tc.expected {
        Some(expected) => print_expected_summary(expected),
        None => println!("Expected Snapshot: none"),
    }
}

fn print_expected_summary(expected: &ExpectedResults) {
    let mut status_counts: BTreeMap<String, usize> = BTreeMap::new();
    for eval in &expected.evaluations {
        *status_counts
            .entry(format!("{:?}", eval.status))
            .or_default() += 1;
    }

    println!("Expected Snapshot: present");
    println!("Expected Evaluations: {}", expected.evaluations.len());
    if !status_counts.is_empty() {
        let counts = status_counts
            .into_iter()
            .map(|(status, count)| format!("{}={}", status, count))
            .collect::<Vec<_>>()
            .join(", ");
        println!("Expected Status Counts: {}", counts);
    }

    if expected.forecasts.is_empty() {
        println!("Expected Forecasts: 0");
    } else {
        println!("Expected Forecasts: {}", expected.forecasts.len());
        for (idx, forecast) in expected.forecasts.iter().enumerate() {
            print_forecast_summary(idx + 1, forecast);
        }
    }
}

fn print_forecast_summary(index: usize, forecast: &SeriesForecast) {
    println!(
        "  {:>2}. {} status={:?} earliest={} recommended={} overdue={} latest={}",
        index,
        forecast.series_name,
        forecast.status,
        format_optional_date(forecast.status.earliest_date()),
        format_optional_date(forecast.status.recommended_date()),
        format_optional_date(forecast.status.overdue_date()),
        format_optional_date(forecast.status.latest_date()),
    );
}

fn format_optional_date(date: Option<chrono::NaiveDate>) -> String {
    date.map(|d| d.to_string())
        .unwrap_or_else(|| "-".to_string())
}
