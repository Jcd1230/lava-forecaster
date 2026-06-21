# Parity Workflow Notes

This file captures practical lessons from reducing Java-vs-Rust discrepancies in already-ported vaccine groups.

## Compare Log Workflow

Use saved full-CDSi compare logs under `tests/relative/tmp/` so you can compare before/after results without rerunning Java for every question.

The current test runner command shape uses the `run` subcommand after Cargo's
`--` separator. If a command reports `Executed: 0`, first suspect a stale
`--run` invocation, a case name that is not present in the selected input, or a
group filter mismatch.

Create a baseline offline log from recorded snapshots:

```bash
cargo run --release --bin test_runner -- run tests/cases > tests/relative/tmp/ice_cdsi_<label>.txt 2>&1
```

Run a target group only:

```bash
cargo run --release --bin test_runner -- run tests/cases --group <GROUP>
```

Run a target group from a fuzz database:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group <GROUP> > tests/relative/tmp/<group>_fuzz_<label>.txt 2>&1
```

Inspect case membership or a single case without running evaluation:

```bash
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --group DTP --count
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --group DTP --list-cases
cargo run --release --bin test_runner -- inspect tests/fuzz-100k-20260608.ltp --case fuzz_fail_dtp_20260608_24219
```

Run a single case with full detail:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group <GROUP> --case <test_case_name> -v --trace
```

Compare a single case against a live ICE server:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group <GROUP> --case <test_case_name> -v --trace --compare
```

## Corpus Strategy

Use separate corpora for separate purposes:

- `tests/cases/` is the durable regression suite. Promote only cases that
  protect understood behavior, and name files after the behavior.
- `.ltp` databases are discovery and pressure-test corpora. They can be large
  and redundant because their job is breadth, not explanation.
- `tests/relative/` and CDSi-derived inputs are source material for standard
  cases and snapshot refreshes.

Fuzzing should discover rules; curated fixtures should preserve rules. When a
fuzz case teaches a real ICE behavior, promote one or a few representative JSON
fixtures after the behavior is understood. Do not promote dozens of equivalent
cases that all fail for the same missing rule.

Large fuzz corpora are still valuable. Run them as stress tests and keep their
failure summaries, but use mismatch buckets and named fixtures to drive code
changes.

## Iterative Parity Loop

For each group, repeat this loop until fresh fuzzing stops finding novel
behavior:

1. **Stabilize the current frontier**: run the current curated and fuzz corpora,
   then bucket failures by mismatch shape before changing code.
2. **Fix one behavior bucket**: choose a representative failure, confirm it with
   `-v --trace`, and use Java/Drools evidence when the expected behavior is not
   obvious.
3. **Promote the rule**: once the bucket is understood, extract or create a
   readable fixture that names the behavior being guarded.
4. **Verify locally**: run the representative case, target group, target fuzz
   corpus, and any unrelated guard group if shared engine behavior changed.
5. **Regenerate pressure**: after broad failures are low, run a new targeted
   fuzz wave for that group to search for edge cases the previous corpus missed.
6. **Track novelty**: keep failures that introduce new behavior buckets; archive
   or ignore redundant cases that only repeat an existing mismatch.

Avoid running huge new fuzz campaigns while high-level buckets are still
obviously broken. Those runs mostly generate redundant failures. Once the group
is close to green, targeted fuzzing becomes much more useful because each new
failure is more likely to expose a genuinely missing edge rule.

## Targeted Fuzzing Heuristics

Pure random fuzzing is useful, but it will miss sparse clinical boundaries.
Bias future fuzz waves toward known fragile surfaces:

- Exact age boundaries and one-day offsets around minimum, recommended, maximum,
  and catch-up ages.
- Minimum interval boundaries, including 4-day grace windows and just-before /
  just-after dates.
- Same-day products, duplicate doses, combination vaccines, and product
  precedence.
- Series switches, mixed product families, withdrawn products, and unsupported
  CVX codes.
- Seasonal schedules, birth-year cutoffs, assessment-date season boundaries,
  and authorization windows.
- High-risk flags, conditional recommendations, completion overrides, and
  adult-only recommendation statuses.
- Invalid-but-ignored doses, accepted doses that should not count for
  completion, and dose-number reassignment.

When possible, record a compact "failure fingerprint" before deciding whether
to keep a generated case: group, case name, eval-vs-forecast shape, status or
CVX transition, date delta, same-day involvement, selected series, and the Rust
trace decision path if available. A new fingerprint is usually worth
investigating; repeated fingerprints should be sampled, not hoarded.

## At-a-Glance Triage

List failing buckets from a saved full compare:

```bash
rg -n 'FAIL:|CDSI_' tests/relative/tmp/ice_cdsi_compare_<label>.txt
```

Check whether a bucket has been eliminated:

```bash
rg 'CDSI_HPV' tests/relative/tmp/ice_cdsi_compare_<label>.txt
```

No matches means that bucket is gone.

Diff per-group failure counts between two saved full compares:

```bash
awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' tests/relative/tmp/before.txt | sort > before.counts
awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' tests/relative/tmp/after.txt | sort > after.counts
join -a1 -a2 -e0 -o 0,1.2,2.2 before.counts after.counts | awk '$2 != $3'
```

Summarize status-transition and forecast buckets from structured runner output:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group DTP --summary tests/relative/tmp/dtp_summary.json
cargo run --release --bin test_runner -- summarize tests/relative/tmp/dtp_summary.json
```

The summary JSON is written before the runner exits, including on failing test
runs. Use the `summarize` command to get eval-only / forecast-only / combined
case shapes, status transitions, CVX transitions, forecast date deltas, and
same-day mismatch counts.

Promote an important fuzz case into a named JSON parity fixture after the
behavior is understood:

```bash
scripts/promote_case.sh tests/fuzz-100k-20260608.ltp fuzz_fail_dtp_20260608_24219 tests/cases/dtp/parity/<behavior_name>.json
```

Use behavior names rather than fuzz IDs. Keep promoted cases as readable JSON
source fixtures; reserve `.ltp` files for large generated fuzz corpora.

## Mismatch Routing

Use the mismatch shape to decide what to read next before editing code.

- Dose validity, reasons, or accepted-vs-invalid drift:
	- Read Java `Evaluation^<Group>.dslr` first.
	- Then inspect the Rust group's `custom_evaluation_hook` and any dose-number or switch hooks.
- Forecast date, forecast status, or overdue-date drift:
	- Read Java `Recommendation^<Group>.dslr` first.
	- Then inspect the Rust group's `custom_forecast_hook`.
- Wrong selected series, wrong effective dose numbering, or the right rules firing on the wrong series:
	- Read Java `SeriesSelection.drl` and the supporting-data YAML for the group.
	- Then inspect Rust `group_selection`, `custom_switch_hook`, and `custom_dose_number_hook`.
- Same-day duplicate behavior or combination-vaccine precedence drift:
	- Check the engine same-day priority logic before changing group-specific rules.

## Drools Event Log Workflow

Use Drools event logs when a Java output snapshot is surprising or when the
DSL/Drools text appears to say something broader than the final output. The
goal is to identify the facts and rules ICE actually used, then verify the
final Java output through the test runner.

Prefer the capture script for ordinary one-case investigations:

```bash
scripts/drools_case.sh DTP fuzz_fail_dtp_20260608_24219
```

By default the script uses:

```text
../java-ice/opencds-decision-support-service/logs/drools-events.log
```

Override that path when needed:

```bash
DROOLS_LOG=relative/path/to/drools-events.log scripts/drools_case.sh DTP fuzz_fail_dtp_20260608_24219
```

It truncates the Drools log, runs exactly one live compare, then writes:

```text
tests/relative/tmp/drools/<GROUP>/<CASE>/compare.txt
tests/relative/tmp/drools/<GROUP>/<CASE>/drools_filtered.txt
tests/relative/tmp/drools/<GROUP>/<CASE>/drools_raw.log
```

Read `compare.txt` and `drools_filtered.txt` first. Open `drools_raw.log` only
when the filtered view omits a needed rule or fact.

Manual fallback:

1. Enable Drools event logging in the Java ICE server config. In this project
   this has been done with `enable-drools-event-logging: true`, plus logger
   configuration that routes `drools-logger` output to a file.
2. Keep the file log concise if possible. A single DTP forecast can produce
   hundreds of kilobytes, so prefer a simple pattern and omit timestamps or
   category names unless they are needed.
3. Before each single-case investigation, truncate the log:

```bash
: > relative/path/to/drools.log
```

4. Run exactly one focused compare:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group DTP --case <case> -v --trace --compare
```

5. Search the Drools log before opening it. Useful DTP terms include:

```bash
rg -n 'RuleFired|DTP|TargetDose|SUPPORTED_SERIES|ADOLESCENT|PERTUSSIS|Duplicate|Recommendation|SeriesSelection' relative/path/to/drools.log
```

6. Record conclusions in the group logic doc. Separate direct source facts
   from snapshot-derived inference, and include at least one counterexample
   before adding broad product-family rules.

Do not read large log files blindly. Clear the file, run one case, search for
targeted terms, then open only the relevant excerpts.

## Parity Debugging Playbook

This playbook provides a systematic approach to investigating and resolving vaccine logic mismatches.

### 1. Failure Pattern Catalog
When investigating a mismatch, identify the pattern to narrow down the search space:
- **Date Drift (e.g., 1-year drift)**: Suspect an incorrect age-based clamp or failure to respect a catch-up milestone (e.g., 7m, 12m, 24m).
- **Date Drift (e.g., 56-day shift)**: Suspect an incorrect interval calculation or a failure to align overdue/recommended dates.
- **Unexpected "Complete" Status**: Suspect a completion hook issue, especially for high-risk patients who should remain `ConditionallyRecommended`.
- **Unexpected "Accepted" vs "Valid" Evaluation**: Suspect an incorrect target dose index, same-day duplicate handling, or series-switching rule (e.g., OMP vs. standard).

### 2. Source-to-Code Mapping Workflow
1.  **Isolate the case**: Run the test runner for the single failing case with `--trace --verbose`.
2.  **Examine the decision log**: Look for the `trace_decision!` log lines. They explicitly state *which* rule fired and *why* it produced a specific date/status.
3.  **Cross-reference with ICE source**: Search the `.dslr` or `.drl` files in `~/projects/java-ice/` for keywords related to the rule that fired in the trace (e.g., "minimum interval", "age clamp").
4.  **Use Drools logs when needed**: If source text and final Java output still do not line up, clear the Drools log and run one live `--compare` case to see the actual rule/fact flow.
5.  **Update the logic doc**: Add the Java source path, rule name, and snapshot-derived interpretation to `docs/<group>_logic.md`.
6.  **Verification**: Before patching, confirm the hypothesis by running a test case that specifically hits that rule.

### 3. Verification Workflow (The Pre-Flight Checklist)
Before applying a fix, ensure you are in a clean state:
1.  `git status --short` or `jj status` depending on the current workflow.
2.  Read the file content you intend to edit and check nearby patterns.
3.  Perform a scoped change.
4.  Run the representative case first.
5.  Run the target group curated cases and fuzz database.
6.  Run a known unrelated guard if shared engine behavior changed.
7.  Commit the change with a message that names the parity bucket.
8.  **Regression Check**: After a successful fix, run the full compare log to ensure no other groups were broken.

### 4. Known Regression Traps
- **Influenza**: Seasonal boundaries are absolute. Never allow cross-season dates unless explicitly required by a catch-up rule.
- **Pneumococcal**: The 7-month, 12-month, and 24-month catch-up clamps are highly sensitive.
- **MMR**: Antigen component counts are the source of truth, not total dose counts. Always check antigen tracking.
