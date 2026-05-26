# Parity Workflow Notes

This file captures practical lessons from reducing Java-vs-Rust discrepancies in already-ported vaccine groups.

## Compare Log Workflow

Use saved full-CDSi compare logs under `curl-rest-tests/tmp/` so you can compare before/after results without rerunning Java for every question.

Create a baseline log:

```bash
cargo run --release --bin test_runner -- --run tests/cases --compare > curl-rest-tests/tmp/ice_cdsi_compare_<label>.txt 2>&1
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
rg -n 'FAIL:|CDSI_' curl-rest-tests/tmp/ice_cdsi_compare_<label>.txt
```

Check whether a bucket has been eliminated:

```bash
rg 'CDSI_HPV' curl-rest-tests/tmp/ice_cdsi_compare_<label>.txt
```

No matches means that bucket is gone.

Diff per-group failure counts between two saved full compares:

```bash
awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' curl-rest-tests/tmp/ice_cdsi_compare_before.txt | sort > curl-rest-tests/tmp/before.counts
awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' curl-rest-tests/tmp/ice_cdsi_compare_after.txt | sort > curl-rest-tests/tmp/after.counts
join -a1 -a2 -e0 -o 0,1.2,2.2 curl-rest-tests/tmp/before.counts curl-rest-tests/tmp/after.counts | awk '$2 != $3'
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
- If the mismatch still looks ambiguous after that pass:
	- Compare raw case input, expected snapshot, and Rust output side by side before patching.

## Source of Truth Order

For an existing parity bucket, this order usually minimizes wasted reading:

1. Saved full compare log for the current baseline.
2. Targeted group compare output.
3. Raw case input in `curl-rest-tests/cases/<group>.json`.
4. Expected Java snapshot in `curl-rest-tests/cases/<group>.expected.json`.
5. Java `Evaluation^<Group>.dslr` and `Recommendation^<Group>.dslr`.
6. Java `SeriesSelection.drl` if series choice or dose numbering looks wrong.
7. Supporting-data YAML under `ice-supporting-data/Series/`.
8. Rust `overrides.rs`, then `mod.rs`, then `schedules.rs`.

Treat the raw case JSON as authoritative when a case name sounds misleading.

## Case Files

- `curl-rest-tests/cases/<group>.json`: raw request-style test inputs.
- `curl-rest-tests/cases/<group>.expected.json`: recorded Java output snapshots.
- `curl-rest-tests/tmp/ice_cdsi_compare_<label>.txt`: saved before/after compare logs used for bucket triage and regression checks.

When a label and the actual dates disagree, trust the dates in the raw case file.

## Debugging Heuristics

- Many discrepancies are really series-selection bugs. Verify `selected_series` and effective dose numbering before changing evaluation rules.
- Forecast drift often comes from anchoring to raw history instead of valid or effective doses in the selected series.
- Adult conditional or not-recommended policy rules may apply only when no relevant series history exists. Started series often remain `NotComplete` with real dates.
- `Accepted` does not necessarily mean the dose should count toward completion. If Java treats the dose as ignored for completion, Rust often needs `Accepted` plus `OutsideRoutineSeries`.
- If a failing case is ambiguous, run the Rust forecaster directly on a one-off request JSON and inspect raw `selected_series`, evaluations, and forecast output before patching.

## Tooling Notes

- For existing buckets, prefer Cargo test runner commands (`cargo run --release --bin test_runner -- --run tests/cases --group <GROUP> --compare`) so compare logging stays explicit.
- Use `cargo run --release --bin test_runner -- --record tests/cases` when you want to record Java ICE snapshots into test JSONs.
- Keep compare-log postprocessing shell-simple. `rg`, `awk`, saved logs, and small one-liners are usually enough.

## Direct Rust Inspection

When a compare case is still ambiguous, run the Rust binary directly on a one-off JSON file and inspect the raw output before patching.

1. Save a simplified or legacy-format request JSON to a temporary file in the workspace.
2. Run the binary from the Rust crate directory:

```bash
cd ice-rust-forecaster-poc
cargo run -- /path/to/request.json
```

The binary writes parsed patient/history debug lines to stderr and the full forecast response JSON to stdout. Use that output to inspect `vaccine_groups`, evaluations, and forecast dates/statuses without going through the compare harness.

## Test Runner Notes

- Use workspace-local temp logs under `curl-rest-tests/tmp/` instead of `/tmp/`.
- `curl-rest-tests` `RelativeDateResolver` only applies the first offset chunk after the base reference. Use a single offset expression such as `birth + 406d` instead of chained forms like `birth + 1y+40d`.

## Engine Behavior Notes

- Engine completion counts `Accepted` doses unless they carry `OutsideRoutineSeries` or `VaccineNotLicensedForMales`.
- For groups like MCV or HPV, add `OutsideRoutineSeries` when Java marks an accepted dose as ignored for completion.
- Many parity fixes belong in selection or override hooks, not `schedules.rs`. For already-ported groups, treat `schedules.rs` as the last place to edit unless the Java supporting data itself is clearly different.