# Parity Workflow Notes

This file captures practical lessons from reducing Java-vs-Rust discrepancies in already-ported vaccine groups.

## Compare Log Workflow

Use saved full-CDSi compare logs under `tests/relative/tmp/` so you can compare before/after results without rerunning Java for every question.

Create a baseline log:

```bash
cargo run --release --bin test_runner -- --run tests/cases --compare > tests/relative/tmp/ice_cdsi_compare_<label>.txt 2>&1
```

Run a target group only:

```bash
cargo run --release --bin test_runner -- --run tests/cases --group <group> --compare
```

Run a single case with full detail:

```bash
cargo run --release --bin test_runner -- --run tests/cases --group <group> --case <test_case_name> --compare -v
```

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
4.  **Verification**: Before patching, confirm the hypothesis by running a test case that specifically hits that rule.

### 3. Verification Workflow (The Pre-Flight Checklist)
Before applying a fix, ensure you are in a clean state:
1.  `jj status` (Are we on the right commit?)
2.  `read_file` (Verify the file content is exactly what you expect before editing.)
3.  Perform the change (using `replace` or `write_file`).
4.  Run `cargo check` and verify the test case.
5.  `jj describe -m "..."` (commit the change).
6.  **Regression Check**: After a successful fix, run the full compare log to ensure no other groups were broken.

### 4. Known Regression Traps
- **Influenza**: Seasonal boundaries are absolute. Never allow cross-season dates unless explicitly required by a catch-up rule.
- **Pneumococcal**: The 7-month, 12-month, and 24-month catch-up clamps are highly sensitive.
- **MMR**: Antigen component counts are the source of truth, not total dose counts. Always check antigen tracking.
