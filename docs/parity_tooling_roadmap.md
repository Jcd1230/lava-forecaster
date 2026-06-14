# Parity Tooling Roadmap

This roadmap tracks workflow improvements that should make Java ICE parity
work faster, more accurate, and easier to hand off. The immediate goal is to
reduce ambiguity before changing vaccine logic: inspect the exact cases under
test, bucket failures mechanically, and capture Java/Drools evidence in a
repeatable way.

## 1. LTP Inspect Command

Status: implemented.

Priority: highest.

Problem:

- It is currently too easy to run a filtered command that executes zero cases.
- `.ltp` databases are opaque when trying to answer whether a case exists,
  which group it belongs to, or what input and expected snapshot it contains.

Proposed commands:

```bash
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --group DTP --list-cases
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --group DTP --count
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --case fuzz_fail_dtp_20260608_24219
```

Implementation outline:

1. Add an `inspect` subcommand to `test_runner`.
2. Reuse existing `.ltp` loading and filtering code.
3. Support `--group`, `--case`, `--list-cases`, and `--count`.
4. For a single case, print patient DOB, assessment date, doses, group, and a
   compact expected evaluation/forecast summary.
5. Make zero-match filters explicit with a non-success message or clear warning.

Success criteria:

- A developer can confirm case membership without running evaluation.
- The earlier `Executed: 0` confusion is easy to diagnose from the CLI.

Implemented command examples:

```bash
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --group DTP --list-cases
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --group DTP --count
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --case fuzz_fail_dtp_20260608_24219
```

## 2. Structured Failure Summary

Status: implemented.

Priority: high.

Problem:

- Remaining parity failures are easiest to fix by bucket, but current bucket
  summaries are produced with ad hoc scripts over human logs.
- We need stable counts for eval-only, forecast-only, CVX transitions, date
  deltas, and same-day involvement.

Proposed commands:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group DTP --summary tests/relative/tmp/dtp_summary.json
cargo run --release --bin test_runner -- summarize tests/relative/tmp/dtp_summary.json
```

Implementation outline:

1. Add optional JSON summary output to `run`.
2. Record one structured mismatch entry per case, dose, and forecast mismatch.
3. Include case name, group, CVX, administration date, Rust value, expected
   value, forecast field, date delta, and whether same-day doses are present.
4. Add a `summarize` subcommand that groups:
   - eval-only / forecast-only / combined failures;
   - status transitions such as `Valid -> Accepted`;
   - CVX plus status transitions;
   - forecast field and date-delta buckets;
   - same-day-involved cases.
5. Keep human output concise and deterministic so summaries can be compared
   between commits.

Success criteria:

- The next parity bucket can be chosen from one command.
- Before/after impact can be measured without manually parsing colored logs.

Implemented command examples:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group DTP --summary tests/relative/tmp/dtp_summary.json
cargo run --release --bin test_runner -- summarize tests/relative/tmp/dtp_summary.json
```

The JSON report records per-case evaluation mismatches, forecast mismatches,
status transitions, CVX codes, same-day dose flags, and date deltas. The
summarizer prints eval-only / forecast-only / combined case shapes, status
transition counts, CVX transition counts, forecast field counts, forecast date
deltas, and same-day mismatch counts.

## 3. Drools Single-Case Capture Script

Status: implemented.

Priority: high after the first two CLI improvements.

Problem:

- Drools logs are useful but very large. A single DTP forecast can produce
  hundreds of kilobytes.
- Manual log clearing, compare execution, and targeted searching are easy to
  do inconsistently.

Proposed command:

```bash
scripts/drools_case.sh DTP fuzz_fail_dtp_20260608_24219
```

Proposed artifact layout:

```text
tests/relative/tmp/drools/DTP/fuzz_fail_dtp_20260608_24219/
  compare.txt
  drools_filtered.txt
  drools_raw.log
```

Implementation outline:

1. Add a shell script with a clearly documented `DROOLS_LOG` path variable.
2. Truncate the Drools log before each run.
3. Run exactly one focused live compare with `-v --trace --compare`.
4. Copy the raw Drools log into the artifact directory.
5. Produce a filtered log using terms such as `RuleFired`, `DTP`,
   `TargetDose`, `SUPPORTED_SERIES`, `PERTUSSIS`, `ADOLESCENT`, `Duplicate`,
   `Recommendation`, and `SeriesSelection`.
6. Document the script in `parity_workflow_notes.md`.

Success criteria:

- Each hard Java behavior question has a small, reproducible evidence bundle.
- Fresh agents can inspect the same compare output and filtered Drools rule
  flow without rerunning the server.

Implemented command examples:

```bash
scripts/drools_case.sh DTP fuzz_fail_dtp_20260608_24219
DROOLS_LOG=relative/path/to/drools-events.log scripts/drools_case.sh DTP fuzz_fail_dtp_20260608_24219 tests/fuzz-100k-20260608.ltp
```

The script truncates the active Drools log, runs one `-v --trace --compare`
case, and writes `compare.txt`, `drools_raw.log`, and `drools_filtered.txt`
under `tests/relative/tmp/drools/<GROUP>/<CASE>/`.

## 4. Promote Fuzz Cases Into Named Parity Cases

Status: implemented.

Priority: medium.

Problem:

- Fuzz cases are broad and opaque. Once a fuzz failure teaches a real rule, it
  should become a readable regression guard.

Proposed command:

```bash
scripts/promote_case.sh tests/fuzz-100k-20260608.ltp fuzz_fail_dtp_20260608_24219 tests/cases/dtp/parity/<behavior_name>.json
cargo run --release --bin test_runner -- promote tests/fuzz-100k-20260608.ltp tests/cases/dtp/parity/<behavior_name>.json --case fuzz_fail_dtp_20260608_24219
```

Implementation outline:

1. Build on the LTP inspect/extraction code.
2. Extract one case from `.ltp` to readable JSON.
3. Preserve expected Java output from the source database.
4. Name the output file after behavior, not the fuzz ID.
5. Use promoted cases as guards before broad product-family or shared-engine
   changes.

Success criteria:

- Important discovered parity rules survive beyond the fuzz database.
- Regression tests explain behavior through file names and compact fixtures.

Implemented notes:

- Promoted cases are readable JSON fixtures, not `.ltp` files. JSON remains the
  source of truth for named parity regressions; `.ltp` stays reserved for large
  fuzz corpora.
- The native command refuses to overwrite an existing file unless `--force` is
  passed.
- The wrapper script is a thin compatibility layer over `test_runner promote`.

## 5. Structured Rust Trace JSON

Priority: medium-low; valuable, but more invasive than the earlier tooling.

Problem:

- Text traces are useful for humans but hard to bucket or diff between
  revisions.
- Some failures recur because multiple cases hit the same Rust decision path,
  but the runner cannot group by that path yet.

Proposed command:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group DTP --case fuzz_fail_dtp_20260608_24219 --trace-json tests/relative/tmp/case_trace.json
```

Implementation outline:

1. Define trace event structs for selected series, target-dose assignment,
   evaluation decisions, hook mutations, same-day handling, and forecast
   mutations.
2. Emit JSON trace events alongside the existing text trace.
3. Keep text trace behavior unchanged.
4. Later, teach the summary command to group failures by Rust trace event or
   rule identifier.

Success criteria:

- Rust decision paths can be compared across commits.
- Large failure buckets can be associated with the exact LAVA hook or engine
  decision that produced them.

## Recommended Order

1. Implement `inspect`.
2. Generate structured failure summaries.
3. Add `scripts/drools_case.sh`.
4. Add case promotion once case extraction is available.
5. Add JSON traces after the lower-risk workflow tools are in place.

The first three items are now in place. The next highest-value build is case
promotion, because it converts rule discoveries from opaque fuzz IDs into
readable regression guards before adding more DTP-specific exceptions.
