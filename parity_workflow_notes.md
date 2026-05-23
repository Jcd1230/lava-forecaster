# Parity Workflow Notes

This file captures practical lessons from reducing Java-vs-Rust discrepancies in already-ported vaccine groups.

## Compare Log Workflow

Use saved full-CDSi compare logs under `curl-rest-tests/tmp/` so you can compare before/after results without rerunning Java for every question.

Create a baseline log:

```bash
python3 curl-rest-tests/run_tests.py --compare --cdsi > curl-rest-tests/tmp/ice_cdsi_compare_<label>.txt 2>&1
```

Run a target group only:

```bash
python3 curl-rest-tests/run_tests.py --group cdsi_<group> --compare
```

Run a single case with full detail:

```bash
python3 curl-rest-tests/run_tests.py --group cdsi_<group> --case <test_case_name> --compare -v
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

## Debugging Heuristics

- Many discrepancies are really series-selection bugs. Verify `selected_series` and effective dose numbering before changing evaluation rules.
- Forecast drift often comes from anchoring to raw history instead of valid or effective doses in the selected series.
- Adult conditional or not-recommended policy rules may apply only when no relevant series history exists. Started series often remain `NotComplete` with real dates.
- `Accepted` does not necessarily mean the dose should count toward completion. If Java treats the dose as ignored for completion, Rust often needs `Accepted` plus `OutsideRoutineSeries`.
- If a failing case is ambiguous, run the Rust forecaster directly on a one-off request JSON and inspect raw `selected_series`, evaluations, and forecast output before patching.

## Test Runner Notes

- Use workspace-local temp logs under `curl-rest-tests/tmp/` instead of `/tmp/`.
- `curl-rest-tests` `RelativeDateResolver` only applies the first offset chunk after the base reference. Use a single offset expression such as `birth + 406d` instead of chained forms like `birth + 1y+40d`.

## Engine Behavior Notes

- Engine completion counts `Accepted` doses unless they carry `OutsideRoutineSeries` or `VaccineNotLicensedForMales`.
- For groups like MCV or HPV, add `OutsideRoutineSeries` when Java marks an accepted dose as ignored for completion.