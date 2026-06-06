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
awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' tests/relative/tmp/ice_cdsi_compare_before.txt | sort > tests/relative/tmp/before.counts
awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' tests/relative/tmp/ice_cdsi_compare_after.txt | sort > tests/relative/tmp/after.counts
join -a1 -a2 -e0 -o 0,1.2,2.2 tests/relative/tmp/before.counts tests/relative/tmp/after.counts | awk '$2 != $3'
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
3. Raw case input in `tests/relative/<group>.json`.
4. Expected Java snapshot in `tests/relative/<group>.expected.json`.
5. Java `Evaluation^<Group>.dslr` and `Recommendation^<Group>.dslr`.
6. Java `SeriesSelection.drl` if series choice or dose numbering looks wrong.
7. Supporting-data YAML under `ice-supporting-data/Series/`.
8. Rust `overrides.rs`, then `mod.rs`, then `schedules.rs`.

Treat the raw case JSON as authoritative when a case name sounds misleading.

## Case Files

- `tests/relative/<group>.json`: raw request-style test inputs.
- `tests/relative/<group>.expected.json`: recorded Java output snapshots.
- `tests/relative/tmp/ice_cdsi_compare_<label>.txt`: saved before/after compare logs used for bucket triage and regression checks.

When a label and the actual dates disagree, trust the dates in the raw case file.

## Debugging Heuristics

- Many discrepancies are really series-selection bugs. Verify `selected_series` and effective dose numbering before changing evaluation rules.
- Forecast drift often comes from anchoring to raw history instead of valid or effective doses in the selected series.
- Adult conditional or not-recommended policy rules may apply only when no relevant series history exists. Started series often remain `NotComplete` with real dates.
- `Accepted` does not necessarily mean the dose should count toward completion. If Java treats the dose as ignored for completion, Rust often needs `Accepted` plus `OutsideRoutineSeries`.
- If a failing case is ambiguous, run the Rust forecaster directly on a one-off request JSON and inspect raw `selected_series`, evaluations, and forecast output before patching.
- **Ignored-shot annotation**: The comparison table (with `-v`) now shows `Invalid (Ignored)` or `Invalid (Not Ignored)` for `Invalid` evaluations. When Rust and Java agree on `Invalid` status but the shot is OPV/bivalent (POLIO group), the `(Ignored)` annotation confirms that both sides correctly treat it as ignored for series completion — a useful quick-check before diving into overrides.

## Tooling Notes

- For existing buckets, prefer Cargo test runner commands (`cargo run --release --bin test_runner -- --run tests/cases --group <GROUP> --compare`) so compare logging stays explicit.
- Use `cargo run --release --bin test_runner -- --record tests/cases` when you want to record Java ICE snapshots into test JSONs.
- Keep compare-log postprocessing shell-simple. `rg`, `awk`, saved logs, and small one-liners are usually enough.
- Use `--trace` or `--explain` on any single-case run to get a step-by-step decision log for that case without touching the code:
  ```bash
  cargo run --release --bin test_runner -- --run tests/cases --case <test_case_name> --trace
  ```
  The trace output shows each age check, interval check, parameter override trigger, forecast date calculation step, max-age clamp event, and custom hook mutation side-by-side with the source file and line number. Combine with `--compare` to trace live-comparison runs, or use standalone against recorded snapshots.

## Direct Rust Inspection

When a compare case is still ambiguous, run the Rust binary directly on a one-off JSON file and inspect the raw output before patching.

1. Save a simplified or legacy-format request JSON to a temporary file in the workspace.
2. Run the binary from the Rust crate directory:

```bash
# From the project root:
cargo run -- /path/to/request.json
```

The binary writes parsed patient/history debug lines to stderr and the full forecast response JSON to stdout. Use that output to inspect `vaccine_groups`, evaluations, and forecast dates/statuses without going through the compare harness.

## Test Runner Notes

- Use workspace-local temp logs under `tests/relative/tmp/` instead of `/tmp/`.
- `tests/relative` `RelativeDateResolver` only applies the first offset chunk after the base reference. Use a single offset expression such as `birth + 406d` instead of chained forms like `birth + 1y+40d`.

## Engine Behavior Notes

- Engine completion counts `Accepted` doses unless they carry `OutsideRoutineSeries` or `VaccineNotLicensedForMales`.
- For groups like MCV or HPV, add `OutsideRoutineSeries` when Java marks an accepted dose as ignored for completion.
- Many parity fixes belong in selection or override hooks, not `schedules.rs`. For already-ported groups, treat `schedules.rs` as the last place to edit unless the Java supporting data itself is clearly different.

## Immunity, Contraindication, and Unsupported Group Parity

### 1. Immunity Coding (ICD-9 vs. SNOMED)
* **Java ICE Expectation**: When sending immunity data via XML payloads to Java ICE, the `observationFocus` element must use **ICD-9-CM** codes (e.g., code system `2.16.840.1.113883.6.103`), such as `070.30` for Hepatitis B, rather than SNOMED codes.
* **Rust LAVA Forecaster Mapping**: In the test runner XML generator, make sure to map the patient's immunity diseases to their corresponding ICD-9 codes. 
* **Reverse Mapping in `legacy_models.rs`**: When parsing the incoming legacy XML payload from Java ICE, the parser must reverse map the ICD-9, ICD-10, LOINC, and SNOMED codes back into internal disease name strings (e.g., `070.30` or `B19.10` -> `"HepB"`).

### 2. Unsupported Java ICE Groups
* Groups such as `Cholera`, `JEV`, `Typhoid`, and `Yellow Fever` are only supported in the Rust LAVA Forecaster, not in the legacy Java ICE rules.
* In `--compare` mode, the test runner must filter these groups out of bulk/individual requests to Java ICE and instead compare them against their own pre-recorded or hand-written expected JSON snapshots.