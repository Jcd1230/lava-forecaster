# ICE & Rust Forecaster PoC Agent Guide

Welcome! This guide provides a quick-start reference for AI agents and developers working on the Immunization Calculation Engine (ICE) and its Rust-based Forecaster Proof of Concept (PoC).

## Tech Stack Summary

- **Java ICE Engine**: Legacy Drools-based server requiring **Java 25** and **Maven 3.9**.
- **Rust Forecaster PoC**: High-performance Rust-based implementation of the evaluation engine.
- **Verification Tests**: Python 3 scripts for running verification and comparative test suites.
- **Tooling**: Managed via `mise` (for Java and Maven version consistency).

## Reference Guides

- Detailed Porting & Implementation Instructions: [agent_onboarding_guide.md](file:///home/jason/projects/ice/agent_onboarding_guide.md)
- Parity workflow and debugging notes: [parity_workflow_notes.md](file:///home/jason/projects/ice/parity_workflow_notes.md)
- Current Porting Progress & Task Checklist: [remaining_series_tasks.md](file:///home/jason/projects/ice/remaining_series_tasks.md)
- Original Java Project Setup & Run Details: [README.md](file:///home/jason/projects/ice/README.md)

## Essential Developer Commands

All major tasks are configured as `mise` commands:

| Command | Description |
|---|---|
| `mise run build` | Builds the Java ICE Maven project (`mvn clean install`). |
| `mise run run` | Runs the Java ICE server (exploded WAR) on `http://localhost:8080`. |
| `mise run scaffold <group_lower> [GROUP_UPPER]` | Scaffolds a new vaccine group module (directory structure, files, mod.rs registration, test JSON). |
| `mise run test` | Runs the Python test suite to verify the Rust PoC output against recorded snapshot JSONs (default quiet, only fails/summary). Does *not* require the Java server to be running. |
| `mise run test-group -- <group_lower>` | Runs Rust PoC verification for one vaccine group against recorded snapshots. |
| `mise run test-compare` | Compares Rust PoC outputs directly against the live Java ICE server. **Auto-records missing expected snapshots.** Requires the Java server to be running. |
| `mise run test-record` | Queries the Java ICE server and records its responses as the expected JSON snapshot. Requires the Java server to be running. |

## Crucial Gotchas & Project Context

- **Java 25 Requirement**: The Java ICE codebase strictly requires Java 25. Running it via `mise` ensures the correct toolchain version is used.
- **Scaffolding New Modules**: When implementing a new vaccine group, always run `mise run scaffold <group_lower>` first to generate boilerplate and register the module in `rules/mod.rs`.
- **Drools Output Mapping**: Do not trust the Drools rules literally. `Mark the shot as Ignored` in Drools maps to `DoseStatus::Accepted` in the output XML. `COMPLETE_HIGH_RISK` combined recommendation status maps to `SeriesStatus::Complete` in the final output. Always verify against recorded Java output.
- **Legacy Rule Definitions**: Legacy rules and support data YAML files are located under the [Series Directory](file:///home/jason/projects/ice/opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/Series/).
- **Comparing Against Live Java**: When implementing or debugging a vaccine group, you can compare Rust behavior against Java in real time. First start the Java server using `mise run run`, then run `mise run test-compare -- --group <name>` in another shell.
- **Test Runner Verbosity**: The test runner is quiet by default. Pass `--verbose` or `-v` to see full details of passing tests.
- **Formatting Scope**: Avoid broad `cargo fmt` / `rustfmt` unless you intend to format the whole Rust module tree. Prefer formatting only files you intentionally changed; `rustfmt` can follow `mod.rs` declarations and touch sibling vaccine modules.

## Recommended Parity Workflow

When reducing Java-vs-Rust discrepancies for an existing group, use this loop:

1. Run a full CDSi compare and save the output under `curl-rest-tests/tmp/`:
	 - `python3 curl-rest-tests/run_tests.py --compare --cdsi > curl-rest-tests/tmp/ice_cdsi_compare_<label>.txt 2>&1`
2. Pick the next smallest remaining bucket from that log.
3. Run the target group alone before editing:
	 - `python3 curl-rest-tests/run_tests.py --group cdsi_<group> --compare`
4. Use `--case <name> -v` for representative edge cases while debugging.
5. After the group passes, re-run the full CDSi compare into a new log under `curl-rest-tests/tmp/`.
6. Diff per-group failure counts between the previous and new full-compare logs to check for regressions outside the target group.

## At-a-Glance Compare Triage

- List failing groups from a saved full compare:
	- `rg -n 'FAIL:|CDSI_' curl-rest-tests/tmp/ice_cdsi_compare_<label>.txt`
- Check whether a target bucket has been eliminated:
	- `rg 'CDSI_HPV' curl-rest-tests/tmp/ice_cdsi_compare_<label>.txt`
	- No matches means the bucket is gone.
- Diff group counts between two saved full compares:
	- `awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' <before_log> | sort > before.counts`
	- `awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' <after_log> | sort > after.counts`
	- `join -a1 -a2 -e0 -o 0,1.2,2.2 before.counts after.counts | awk '$2 != $3'`

## Parity Heuristics That Recur

- Many discrepancies are really **series-selection** bugs. Before changing evaluation rules, confirm the selected series and effective dose numbering are the same as Java.
- Forecast rules often operate on **valid/effective doses**, not raw administered history. If Rust is anchoring forecasts too early, check whether Java is using only valid doses or the selected series' effective dose number.
- Adult recommendation statuses may only apply when **no relevant dose history exists**. Once a series has started, Java often keeps the forecast `NotComplete` instead of switching to `ConditionallyRecommended`.
- If Java marks a shot as effectively ignored for completion, Rust often needs `Accepted` plus `OutsideRoutineSeries` to avoid falsely completing the series.
- When a failing case is ambiguous, run the Rust forecaster directly on a hand-built request JSON and inspect `selected_series`, evaluations, and forecasts before patching.

## Version Control (Jujutsu / jj-vcs)

- **Primary VCS**: This project primarily uses Jujutsu (`jj`) for version control.
- **Commit Guideline**: Developers and agents must run `jj commit -m "..."` after completing meaningful units of work to save progress and maintain a clean repository history.
