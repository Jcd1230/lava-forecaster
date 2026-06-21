# LAVA / ICE Test Runner Guide

The Rust-native `test_runner` binary (`src/bin/test_runner.rs`) is a multi-function tool designed to run offline regression tests, query live Java ICE instances, record expected snapshots, reorganize test cases, import python-formatted suites, and fuzz the evaluation engine.

---

## Offline Bundle Quickstart

When running from a vendored source bundle in a constrained sandbox, prefer
locked, no-default-feature commands unless you specifically need the default
`jemalloc` feature:

```bash
cargo run --locked --no-default-features --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --count
cargo run --locked --no-default-features --bin test_runner -- run tests/fuzz-100k-20260608.ltp --summary tests/relative/tmp/fuzz100k_summary.json
```

Run `inspect ... --count` before long `.ltp` runs. A corpus filename may reflect
the generation batch size rather than the number of matched executable cases.

---

## Major Execution Modes

### 1. Verification / Runner Mode (`--run`)
Evaluates test cases located in a directory or a compact `.ltp` database file and verifies them. By default, it runs against pre-recorded expected snapshots embedded in the test cases offline.

```bash
cargo run --release --bin test_runner -- --run <cases_path_or_db> [options]
```

*   **`<cases_path_or_db>`**: The path to a directory containing test cases (such as `tests/cases/passed/POLIO/` containing `UnifiedTestCase` files) or a single compact MessagePack `.ltp` database file (e.g. `tests/suite.ltp`).

#### Options
*   `--group <group_name>`: Filter cases by vaccine group (e.g. `HPV`, `POLIO`, `MMR`).
*   `--case <case_name>`: Filter cases by case name (runs a single test case).
*   `--compare [java_url]`: Compare LAVA Forecaster outputs dynamically with a live Java ICE server (default URL: `http://localhost:8080`).
*   `--rest-url <rust_url>`: Query a running Rust REST server (e.g. `http://localhost:8081`) instead of executing LAVA in-process.
*   `--verbose` or `-v`: Output detailed, side-by-side comparison tables of evaluations and forecasts. By default (without `--verbose`), failing tests output concise error descriptions to avoid console clutter.
*   `--trace` or `--explain`: Dump a step-by-step decision trace log to stdout after evaluation. Shows the engine's internal decisions for each age check, interval check, parameter override, forecast date calculation, max-age clamp, and every custom hook invocation. Useful for diagnosing forecast date or evaluation status mismatches without manual breakpoints.

#### Comparison Table Details

When running with `-v` or on a failing case in verbose mode, the evaluation comparison table includes ignored-status annotation on `Invalid` evaluations:

```
Date         | CVX  | Rust Evaluation              | Expected Evaluation          | Status
------------------------------------------------------------------------------------------
2016-02-06   | 02   | Valid #1                     | Valid #1                     | OK
2016-05-06   | 178  | Invalid (Ignored) #2         | Invalid (Ignored) #2         | OK
```

The `(Ignored)` / `(Not Ignored)` tag is determined by `is_eval_ignored()` in the engine, which accounts for group-specific rules (e.g., bivalent OPV CVX 178/179 and post-April-2016 CVX 182 shots in the POLIO group are always ignored regardless of evaluation status).

---

### 2. Live Snapshot Recording Mode (`--record`)
Queries a live Java ICE server and records expected output snapshots directly into the test case JSON files or updates a compact `.ltp` database file.

```bash
cargo run --release --bin test_runner -- --record <cases_path_or_db> [java_url]
```

*   **`<cases_path_or_db>`**: Path to a directory containing test cases or a single `.ltp` database file to record/update snapshots for.
*   **`[java_url]`**: URL of the live Java ICE server (default: `http://localhost:8080`).

---

### 3. Fuzzing Mode (`--fuzz`)
Generates random patient histories and performs differential testing to verify parity between LAVA and Java ICE. It tracks execution progress and intercepts `Ctrl+C` (SIGINT) to print metrics and exit gracefully.

```bash
cargo run --release --bin test_runner -- --fuzz <count> [options]
```

*   **`<count>`**: Number of random test cases to generate and run.

#### Options
*   `--group <group_name>`: Generate history and evaluate only for a specific vaccine group.
*   `--compare [java_url]`: Live Java ICE server URL (default: `http://localhost:8080`).
*   `--fuzz-seed <seed>`: Specify a seed for reproducibility.
*   `--shrink`: Attempt to shrink failing history into a minimal reproducing case.
*   `--bulk`: Send fuzz cases to Java ICE in bulk batches (faster; requires the bulk endpoint). Use this when fuzzing large counts against a running Java server to avoid per-request overhead.
*   `--output-db <path>`: Save failing cases directly into a compact `.ltp` database file rather than saving individual JSON files under `tests/cases/failed/`.

At exit or termination, it displays detailed statistics (overall and per-group counts, execution totals, passed, failed, and pass percentage).

> [!NOTE]
> **`--run --compare` always bulk-queries automatically** (in chunks of 500). The `--bulk` flag is only needed for `--fuzz` mode.

---

### 4. Reorganization & Regression Mode (`--reorganize`)
Runs regression tests on a set of test cases, re-evaluates them, and moves/re-categorizes them based on whether they passed or failed.

```bash
cargo run --release --bin test_runner -- --reorganize <source> <target> [options]
```

*   **`<source>`**: Source directory of JSON cases or a single `.ltp` database file to read.
*   **`<target>`**: Target path.
    *   If `<target>` is a directory, cases are saved as pretty-printed JSON files grouped under nested folders: `target/passed/<GROUP>/` and `target/failed/<GROUP>/`. Any obsolete test case files at old locations are removed, and empty subdirectories are recursively cleaned up.
    *   If `<target>` has a database extension (`.ltp`, `.bin`, `.db`), all cases (both passing and failing) are packed into a single compact `.ltp` database file.
*   **Options**: Accepts `--group`, `--case`, `--compare`, `--rest-url`, `--verbose` / `-v`, and `--trace` / `--explain`.

---

### 5. CDSi Suite Import Mode (`--import-cdsi`)
Imports CDSi python-formatted suites (which use relative dates like `birth + 1y`) and resolves them into concrete dates based on the patient's birth date, generating unified test case JSONs.

```bash
cargo run --release --bin test_runner -- --import-cdsi <raw_json_suite> <output_dir>
```

*   **`<raw_json_suite>`**: Path to a raw python JSON suite file (such as the files under `tests/relative/`).
*   **`<output_dir>`**: Output directory to write resolved `UnifiedTestCase` JSON files to.

---

### 6. CDC CSV Spreadsheet Mode (`--run-cdc-csv`)
Runs the evaluation engine against standard CDC spreadsheet test formats.

```bash
cargo run --release --bin test_runner -- --run-cdc-csv <csv_path> [options]
```

*   **`<csv_path>`**: Path to the CDC CDSi spreadsheet CSV file.

---

## File Formats & Directories

1.  **`tests/cases/`**: Contains offline verification files of type `UnifiedTestCase`. Historically structured as individual JSON files. Now partitioned into:
    *   `tests/cases/passed/<GROUP>/`: For test cases that successfully match expected/Java ICE results.
    *   `tests/cases/failed/<GROUP>/`: For test cases with outstanding mismatches or failing results.
2.  **`tests/suite.ltp`**: A single, high-density binary database file using MessagePack serialization format. The `.ltp` format includes a 4-byte magic header (`LTP\x01`) followed by the MessagePack representation of the array of `UnifiedTestCase`s, generated with strict struct-map serialization to preserve optional fields. It reduces disk block overhead significantly (e.g. 218 POLIO test cases pack into a single 132 KB file).
3.  **`tests/relative/`**: Contains raw relative-date suites (e.g. `cdsi_hpv.json`) and their corresponding expected outputs (`cdsi_hpv.expected.json`). They must be imported/resolved via `--import-cdsi` into a directory or database to be run.
