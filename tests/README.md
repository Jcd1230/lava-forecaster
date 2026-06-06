# LAVA / ICE Test Runner Guide

The Rust-native `test_runner` binary (`src/bin/test_runner.rs`) is a multi-function tool designed to run offline regression tests, query live Java ICE instances, record expected snapshots, import python-formatted suites, and fuzz the evaluation engine.

---

## Major Execution Modes

### 1. Verification / Runner Mode (`--run`)
Evaluates test cases located in a directory and verifies them. By default, it runs against pre-recorded expected snapshots embedded in the test JSON files offline.

```bash
cargo run --release --bin test_runner -- --run <cases_dir> [options]
```

*   **`<cases_dir>`**: The path to a directory containing test cases (such as `tests/cases` which contains `UnifiedTestCase` files).

#### Options
*   `--group <group_name>`: Filter cases by vaccine group (e.g. `HPV`, `POLIO`, `MMR`).
*   `--case <case_name>`: Filter cases by case name (runs a single test case).
*   `--compare [java_url]`: Compare LAVA Forecaster outputs dynamically with a live Java ICE server (default URL: `http://localhost:8080`).
*   `--rest-url <rust_url>`: Query a running Rust REST server (e.g. `http://localhost:8081`) instead of executing LAVA in-process.
*   `--verbose` or `-v`: Output detailed, side-by-side comparison tables of evaluations and forecasts.

---

### 2. Live Snapshot Recording Mode (`--record`)
Queries a live Java ICE server and records expected output snapshots directly into the test case JSON files.

```bash
cargo run --release --bin test_runner -- --record <cases_dir> [java_url]
```

*   **`<cases_dir>`**: Directory containing test case files to record snapshots for.
*   **`[java_url]`**: URL of the live Java ICE server (default: `http://localhost:8080`).

---

### 3. Fuzzing Mode (`--fuzz`)
Generates random patient histories and performs differential testing to verify parity between LAVA and Java ICE.

```bash
cargo run --release --bin test_runner -- --fuzz <count> [options]
```

*   **`<count>`**: Number of random test cases to generate and run.

#### Options
*   `--group <group_name>`: Generate history and evaluate only for a specific vaccine group.
*   `--compare [java_url]`: Live Java ICE server URL (default: `http://localhost:8080`).
*   `--fuzz-seed <seed>`: Specify a seed for reproducibility.
*   `--shrink`: Attempt to shrink failing history into a minimal reproducing case.
*   `--bulk`: Execute queries to the Java server in bulk.

---

### 4. CDSi Suite Import Mode (`--import-cdsi`)
Imports CDSi python-formatted suites (which use relative dates like `birth + 1y`) and resolves them into concrete dates based on the patient's birth date, generating unified test case JSONs.

```bash
cargo run --release --bin test_runner -- --import-cdsi <raw_json_suite> <output_dir>
```

*   **`<raw_json_suite>`**: Path to a raw python JSON suite file (such as the files under `tests/relative/`).
*   **`<output_dir>`**: Output directory to write resolved `UnifiedTestCase` JSON files to.

---

### 5. CDC CSV Spreadsheet Mode (`--run-cdc-csv`)
Runs the evaluation engine against standard CDC spreadsheet test formats.

```bash
cargo run --release --bin test_runner -- --run-cdc-csv <csv_path> [options]
```

*   **`<csv_path>`**: Path to the CDC CDSi spreadsheet CSV file.

---

## File Formats & Directories

1.  **`tests/cases/`**: Contains offline verification files of type `UnifiedTestCase`. They contain the patient data, dose history, and an `expected` block. The test runner parses them directly in `--run` mode.
2.  **`tests/relative/`**: Contains raw relative-date suites (e.g. `cdsi_hpv.json`) and their corresponding expected outputs (`cdsi_hpv.expected.json`). They must be imported/resolved via `--import-cdsi` into `tests/cases/` to be run.
