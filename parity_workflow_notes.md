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

Run a single case with full detail:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group <GROUP> --case <test_case_name> -v --trace
```

Compare a single case against a live ICE server:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group <GROUP> --case <test_case_name> -v --trace --compare
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

Summarize status-transition and forecast buckets from a saved group log:

```bash
python - <<'PY'
import collections
import re

path = "tests/relative/tmp/dtp_fuzz_after_dtp3_retry_anchor_codex.txt"
ansi = re.compile(r"\x1b\[[0-9;]*m")
case = None
failcases = []
case_eval = collections.defaultdict(int)
case_forecast = collections.defaultdict(int)
status = collections.Counter()
cvx_status = collections.Counter()

with open(path, errors="replace") as fh:
    for raw in fh:
        line = ansi.sub("", raw.rstrip())
        m = re.match(r"Test Case: (\S+) \(FAIL\)", line)
        if m:
            case = m.group(1)
            failcases.append(case)
            continue
        m = re.search(r"Evaluation status mismatch for dose \([^,]+, (\d+)\): Rust=(\w+).*Expected=(\w+)", line)
        if m:
            cvx, rust, expected = m.groups()
            status[(rust, expected)] += 1
            cvx_status[(cvx, rust, expected)] += 1
            case_eval[case] += 1
            continue
        if "Forecast " in line and "mismatch" in line:
            case_forecast[case] += 1

print("case shapes")
for key, count in collections.Counter((case_eval[c] > 0, case_forecast[c] > 0) for c in failcases).items():
    print(key, count)
print("status transitions")
for (rust, expected), count in status.most_common(20):
    print(f"{rust}->{expected}: {count}")
print("cvx transitions")
for (cvx, rust, expected), count in cvx_status.most_common(30):
    print(f"CVX {cvx} {rust}->{expected}: {count}")
PY
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

## Drools Event Log Workflow

Use Drools event logs when a Java output snapshot is surprising or when the
DSL/Drools text appears to say something broader than the final output. The
goal is to identify the facts and rules ICE actually used, then verify the
final Java output through the test runner.

1. Enable Drools event logging in the Java ICE server config. In this project
   this has been done with `enable-drools-event-logging: true`, plus logger
   configuration that routes `drools-logger` output to a file.
2. Keep the file log concise if possible. A single DTP forecast can produce
   hundreds of kilobytes, so prefer a simple pattern and omit timestamps or
   category names unless they are needed.
3. Before each single-case investigation, truncate the log:

```bash
: > /path/to/drools.log
```

4. Run exactly one focused compare:

```bash
cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group DTP --case <case> -v --trace --compare
```

5. Search the Drools log before opening it. Useful DTP terms include:

```bash
rg -n 'RuleFired|DTP|TargetDose|SUPPORTED_SERIES|ADOLESCENT|PERTUSSIS|Duplicate|Recommendation|SeriesSelection' /path/to/drools.log
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
