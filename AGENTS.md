# LAVA Forecaster Agent Guide

Welcome! This guide provides a quick-start reference for AI agents and developers working on the Immunization Calculation Engine (ICE) and its high-performance Rust-based LAVA Forecaster.

## Tech Stack Summary

- **Java ICE Engine**: Legacy Drools-based server requiring **Java 25** and **Maven 3.9**.
- **Rust LAVA Forecaster**: High-performance Rust-based implementation of the evaluation engine.
- **Verification Tests**: Rust-native `test_runner` and benchmarking binaries.
- **Tooling**: Managed via `mise` (for Java and Maven version consistency).

## Reference Guides

- Detailed Porting & Implementation Instructions: [agent_onboarding_guide.md](file://../ice/agent_onboarding_guide.md)
- Parity workflow and debugging playbook: [parity_workflow_notes.md](file://./parity_workflow_notes.md)
- Current Porting Progress & Task Checklist: [remaining_series_tasks.md](file://../ice/remaining_series_tasks.md)
- Original Java Project Setup & Run Details: [README.md](file://../ice/README.md)
- Test Runner Execution Modes Guide: [tests/README.md](file://../ice/tests/README.md)

## Essential Developer Commands

All major tasks are configured as `mise` commands or native Cargo binaries. 

> [!TIP]
> **Preferred Testing Method**: The Rust-native `test_runner` is the supported verification path and evaluates the exact `UnifiedTestCase` formats.

| Command | Description |
|---|---|
| `mise run build` | Builds the Java ICE Maven project (`mvn clean install`). |
| `mise run run` | Runs the Java ICE server (exploded WAR) on `http://localhost:8080`. |
| `mise run scaffold <group_lower> [GROUP_UPPER]` | Scaffolds a new vaccine group module (directory structure, files, mod.rs registration, test JSON). |
| `cargo run --release --bin test_runner -- --run tests/cases` | **(Preferred)** Runs the Rust-native test runner to verify LAVA logic against expected snapshots (offline) from a directory. |
| `cargo run --release --bin test_runner -- --run tests/suite.ltp` | Runs the test runner to verify LAVA logic against expected snapshots (offline) stored in a compact `.ltp` database file. |
| `cargo run --release --bin test_runner -- --run tests/suite.ltp --group <GROUP> --compare` | Runs the Rust test runner on a database file, comparing dynamically against the live Java ICE server. |
| `cargo run --release --bin test_runner -- --run tests/suite.ltp --case <CASE> -v` | Runs a single case from a database file with side-by-side verbose details. |
| `cargo run --release --bin test_runner -- inspect tests/suite.ltp --group <GROUP> --count` | Confirms `.ltp` case membership and per-group counts without running evaluation. |
| `cargo run --release --bin test_runner -- run tests/suite.ltp --group <GROUP> --summary tests/relative/tmp/<group>_summary.json` | Writes structured mismatch JSON for bucket analysis. |
| `cargo run --release --bin test_runner -- summarize tests/relative/tmp/<group>_summary.json` | Prints eval-only, forecast-only, status-transition, CVX-transition, date-delta, and same-day mismatch buckets. |
| `scripts/drools_case.sh <GROUP> <CASE>` | Clears the active Drools log, runs one live Java compare, and saves compare/raw/filtered artifacts under `tests/relative/tmp/drools/`. |
| `scripts/promote_case.sh tests/suite.ltp <CASE> tests/cases/<group>/parity/<behavior>.json` | Extracts one fuzz/database case into a readable named JSON parity fixture while preserving its expected snapshot. |
| `cargo run --release --bin test_runner -- --reorganize tests/cases tests/cases` | Regression tests all JSON files under `tests/cases/` and re-categorizes them into `passed/<GROUP>/` and `failed/<GROUP>/`. |
| `cargo run --release --bin test_runner -- --reorganize tests/cases tests/suite.ltp` | Re-evaluates test cases in the directory and packs them into a single compact `.ltp` database file. |
| `cargo run --release --bin test_runner -- --record tests/suite.ltp` | Connects to the live Java ICE server and records expected output snapshots directly into the `.ltp` database file. |
| `cargo run --release --bin test_runner -- --fuzz 1000 --group <GROUP> --compare --output-db tests/suite.ltp` | Guided fuzzing of a group comparing against Java ICE, appending any minimal reproducing failures to the `.ltp` database. |
| `cargo run --release --bin test_runner -- -h` | Displays the colorized, structured CLI help documentation. |

## Crucial Gotchas & Project Context

- **Java 25 Requirement**: The Java ICE codebase strictly requires Java 25. Running it via `mise` ensures the correct toolchain version is used.
- **Scaffolding New Modules**: When implementing a new vaccine group, always run `mise run scaffold <group_lower>` first to generate boilerplate and register the module in `rules/mod.rs`.
- **Drools Output Mapping**: Do not trust the Drools rules literally. `Mark the shot as Ignored` in Drools maps to `DoseStatus::Accepted` in the output XML. `COMPLETE_HIGH_RISK` combined recommendation status maps to `SeriesStatus::Complete` in the final output. Always verify against recorded Java output.
- **Legacy Rule Definitions**: Legacy rules and support data YAML files are located under the [Series Directory](file://../ice/opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/Series/).
- **Comparing Against Live Java**: When implementing or debugging a vaccine group, you can compare Rust behavior against Java in real time. First start the Java server using `mise run run`, then run `cargo run --release --bin test_runner -- --run tests/suite.ltp --group <name> --compare` in another shell.
- **Test Runner Verbosity**: The test runner is quiet by default, printing only concise error summaries on failure. Pass `--verbose` or `-v` to see full side-by-side comparison tables.
- **Decision Trace**: Pass `--trace` or `--explain` on any `--run` or `--reorganize` invocation to get a step-by-step engine decision log (age checks, interval checks, parameter overrides, forecast hook mutations) for each case. Particularly useful when the side-by-side comparison table doesn't immediately explain a date or status mismatch.
- **Drools Case Capture**: For hard Java parity questions, prefer `scripts/drools_case.sh <GROUP> <CASE>` over manual log handling. It truncates the configured Drools log first and saves a focused evidence bundle.
- **Invalid (Ignored) Annotation**: The evaluation comparison table now shows `Invalid (Ignored)` or `Invalid (Not Ignored)` for `Invalid` doses. Quickly confirms whether a mismatched shot is intentionally ignored for series completion (e.g., bivalent OPV in POLIO) or genuinely counts.
- **Formatting Scope**: Avoid broad `cargo fmt` / `rustfmt` unless you intend to format the whole Rust module tree. Prefer formatting only files you intentionally changed; `rustfmt` can follow `mod.rs` declarations and touch sibling vaccine modules.
- **Corpus Strategy**: Fuzzing discovers ICE behavior; curated JSON fixtures preserve understood behavior. Keep large `.ltp` corpora for discovery/pressure testing, and promote only representative edge cases into named readable fixtures.
- **Offline Bundle Builds**: For ChatGPT/offline containers, prefer `cargo ... --locked --no-default-features` unless allocator behavior is being tested. The default feature set enables `jemalloc`, which can build slowly in constrained sandboxes.

## Recommended Parity Workflow

When reducing Java-vs-Rust discrepancies for an existing group, use this loop:

1. Run a full CDSi compare dynamically:
	 - `cargo run --release --bin test_runner -- --run tests/cases --compare > tests/relative/tmp/ice_cdsi_compare_<label>.txt 2>&1`
2. Pick the next smallest remaining bucket from that log.
3. Run the target group alone with Java comparison:
	 - `cargo run --release --bin test_runner -- --run tests/cases --group <GROUP> --compare`
4. Use `--case <name> -v` for representative edge cases to see side-by-side comparisons.
5. After the group passes, re-run the full compare into a new log under `tests/relative/tmp/`.
6. Diff per-group failure counts between the previous and new full-compare logs to check for regressions outside the target group.

For detailed mismatch-routing guidance, use [parity_workflow_notes.md](file://../ice/parity_workflow_notes.md). For the fuller source-of-truth map and implementation touchpoints, use [agent_onboarding_guide.md](file://../ice/agent_onboarding_guide.md).

## ChatGPT Offline Agent Handoff

Use the repo-local `offline-agent-coordination` skill when handing work to
offline ChatGPT agents or incorporating their returned patches:

- Skill path: [.agents/skills/offline-agent-coordination/SKILL.md](file://./.agents/skills/offline-agent-coordination/SKILL.md)
- Overview: [docs/chatgpt_agent_handoff.md](file://./docs/chatgpt_agent_handoff.md)

This workflow is intentionally skill-driven rather than script-driven so each
packet can be tailored to ICE research, parity development,
research-then-development sessions, feature work, bug triage, documentation, or
test-corpus analysis.

## At-a-Glance Compare Triage

- List failing groups from a saved full compare:
	- `rg -n 'FAIL:|CDSI_' tests/relative/tmp/ice_cdsi_compare_<label>.txt`
- Check whether a target bucket has been eliminated:
	- `rg 'CDSI_HPV' tests/relative/tmp/ice_cdsi_compare_<label>.txt`
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
- **Use `--trace` early**: Add `--trace` to any single-case run when the side-by-side table doesn't explain the mismatch. The trace names the exact engine step, source location, and decision that produced the wrong date or status — cutting rule-reading to the specific function that fired.
- For existing parity buckets, prefer Cargo test runner commands so compare logging stays explicit.

## Version Control (Jujutsu / jj-vcs)

- **Primary VCS**: This project primarily uses Jujutsu (`jj`) for version control.
- **Do not use Git commits**: Do not use `git add` or `git commit` unless the user explicitly asks for Git or a required tool only supports Git.
- **Commit Guideline**: Developers and agents must run `jj commit -m "..."` after completing meaningful units of work to save progress and maintain a clean repository history.
- **Task Scope**: Keep parity work to one vaccine bucket per revision when practical, validate the target-group compare and a fresh full compare, then commit.
