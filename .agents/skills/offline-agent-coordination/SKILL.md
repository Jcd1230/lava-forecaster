---
name: offline-agent-coordination
description: Use when coordinating offline ChatGPT agent work for this repository, including planning task packets, generating tailored prompts and evidence bundles, receiving patches or result artifacts, validating and incorporating returned work, or running verification after offline agent changes.
---

# Offline Agent Coordination

Use this skill when delegating work from this repository to offline ChatGPT
agent containers and when incorporating their returned patches or artifacts.
The goal is not to automate every step; it is to make handoffs consistent,
reviewable, and easy to validate locally.

## Operating Model

- Local LAVA workspace is authoritative for Java ICE comparison, snapshot
  recording, jj history, and final commits.
- Offline agents receive a vendored source bundle plus task-local evidence.
- Offline agents cannot access the internet or run Java ICE.
- Expected snapshots, local compare logs, summary JSON, promoted fixtures, and
  Drools captures are the truth given to the offline agent.
- Prefer one independent task per offline session. For parity work, usually
  choose one vaccine group plus one mismatch family.

## Plan The Delegation

Before preparing a packet, decide:

- Task mode: research, development, research-then-development, bug triage,
  documentation, or test-corpus analysis.
- Success criteria: exact cases, commands, and expected behavior.
- Allowed edit scope: files/directories the agent may change, plus any areas
  that require explicit justification.
- Evidence needed: offline snapshots only, live Java compare logs, Drools logs,
  promoted fixtures, trace output, or design constraints.
- Return artifact: usually a unified patch from repo root; for analysis tasks,
  a written report may be enough.

Ask the user before delegation only when product intent or scope is ambiguous.
Do not ask where repo facts live; inspect the repo.

Prefer these modes:

- **Research** when the goal is understanding ICE behavior, Java/Drools fact
  flow, or fuzz-generator targets. Return docs/report, not code.
- **Development** when the behavior is already understood and the agent should
  implement a narrow fix or test.
- **Research-then-development** when a long-running offline session should first
  map behavior, then implement only if the evidence supports a concrete change.
  Require the agent to separate the research conclusions from the patch.

## Prepare Evidence Locally

Use commands appropriate to the task; do not run every command by default.

For parity buckets:

```bash
cargo run --release --bin test_runner -- run tests/cases --group <GROUP> \
  --summary tests/relative/tmp/<task-id>_<group>_summary.json
cargo run --release --bin test_runner -- summarize \
  tests/relative/tmp/<task-id>_<group>_summary.json
```

For representative cases:

```bash
cargo run --release --bin test_runner -- run <CASES_OR_LTP> \
  --group <GROUP> --case <CASE> -v --trace > tests/relative/tmp/<task-id>_<case>.txt 2>&1
```

For Java-only evidence, run locally before packaging:

```bash
cargo run --release --bin test_runner -- run <CASES_OR_LTP> \
  --group <GROUP> --case <CASE> -v --trace --compare > tests/relative/tmp/<task-id>_<case>_compare.txt 2>&1
scripts/drools_case.sh <GROUP> <CASE> <CASES_OR_LTP>
```

For readable fuzz repros:

```bash
scripts/promote_case.sh <SOURCE_LTP> <CASE> tests/cases/<group>/parity/<behavior>.json
```

Package the offline source after evidence is ready:

```bash
mise run bundle-source
```

Attach `dist/lava-forecaster-source-bundle.zip`, the tailored prompt, and any
evidence files the agent needs. Keep generated local artifacts under
`tests/relative/tmp/`.

## Tailor The Offline Prompt

Use the prompt mode that fits the assignment. Edit aggressively: remove
irrelevant sections and add concrete file paths, cases, logs, and source areas
when known.

### Development Prompt

```text
You are working offline in the LAVA Forecaster source bundle.

Task:
<one-sentence goal with task type, group/module if applicable, and success criteria>

Authoritative evidence:
- Expected snapshots in tests/cases and/or tests/*.ltp are Java ICE truth.
- Attached logs/summaries/traces/Drools captures describe current behavior.
- You cannot run Java ICE or use the internet. Do not infer Java behavior beyond the provided evidence.

Recommended starting points:
- <files, docs, trace logs, summary JSON, representative cases>

Allowed edit scope:
- <paths the agent may change>
- Shared engine changes require a clear trace-based justification.

Required commands:
- cargo check
- <focused test command>
- <representative case command with -v --trace, if applicable>

Constraints:
- Make the smallest coherent change for the task.
- Do not modify vendor/, dist/, target/, generated files, or VCS metadata.
- Do not create commits.
- Do not run broad formatting; format only intentionally changed files.

Required return:
1. Unified patch from repo root, suitable for git apply --check.
2. Commands run and pass/fail summary.
3. Explanation tied to the provided evidence.
4. Remaining failures, uncertainty, or follow-up recommendations.
```

### Research Prompt

Use this mode for source-grounded ICE/LAVA research, especially before fixing a
hard group or improving fuzz generation. Give the agent both source trees when
possible.

```text
You are researching the Java ICE immunization forecasting engine and the Rust
LAVA forecaster implementation. You have access to both source trees. Produce
source-grounded documentation that helps engineers port or verify ICE behavior
in LAVA.

Research target:
<GROUP, architecture area, fuzzing question, or specific behavior>

Primary goal:
Explain how ICE actually reaches its outputs, not just what the Drools/DSL text
appears to say in isolation.

Important mindset:
- Do not assume Drools text alone is the full algorithm.
- ICE behavior may emerge from series YAML, candidate series initialization,
  target-dose construction/sorting, selected TargetSeries behavior, TargetDose
  fields, inserted ICEFactTypeFinding facts, generic rules, group-specific
  rules, duplicate same-day rules, and ProcessResults output selection.
- Separate directly stated source facts from inferred execution behavior.
- If behavior depends on rule ordering, inserted facts, or selected-series
  output, describe the likely fact flow.
- If comments and observed output conflict, call that out.
- When uncertain, state what trace or test case would confirm the behavior.

Source areas to inspect:
- Java series plan YAML and supporting data.
- Java Drools/DSL rules, including generic HistoryEvaluation,
  DuplicateShotSameDay, SeriesSelection, ProcessResults, and group-specific
  Evaluation/Recommendation rules.
- Java core classes involved in series, target-dose, dose-rule, and result
  construction.
- Rust LAVA engine, group schedules, overrides, tests, parity docs, and group
  logic docs.

Documentation format:
# <TARGET> Java Logic Map
## Current Understanding
Brief summary of how ICE appears to evaluate and forecast this target.
## Source Files
Relevant Java, Drools/DSL, YAML, Rust, test, and doc files. Include rule,
function, class, or nearby unique names; include line numbers when available.
## Series Data
Series names, dose counts, recurring settings, dose-number mode, age/interval
rules, allowed products, and risk/conditional dimensions.
## Series Selection
How ICE selects series, fallback behavior, and how prior invalid/accepted/
ignored doses affect selection if known.
## Evaluation Flow
How doses become Valid, Accepted, Invalid, or ignored. Include generic rules,
group rules, special CVX behavior, same-day behavior, target-dose fields, and
inserted facts.
## Forecast Flow
How recommendations are produced, including generic mechanics, group rules,
completion rules, date quirks, and conditional/not-recommended cases.
## Important ICE Facts
Fact name, where inserted, conditions that insert it, and later rules that
consume it.
## LAVA Mapping Notes
How this likely maps to schedules, override hooks, same-day mechanics,
forecast hooks, and places to avoid broad rules.
## Open Questions
Unresolved behavior and the trace/test that would answer it.
## Representative Cases
Relevant test/fuzz cases and what they demonstrate, if available.

Also produce an architecture note when the target depends on engine mechanics:
- candidate series initialization;
- target-dose initialization;
- administeredShotNumber and doseNumber assignment;
- isPrimarySeriesShot;
- status versus isValid;
- Accepted versus Invalid and Valid in later counting;
- same-day duplicate checks;
- selected-series effect on returned evaluations;
- recommendation phase after evaluation;
- ProcessResults final output selection.

Required return:
1. Source-grounded report or doc patch.
2. List of files/rules/classes inspected.
3. Direct source facts versus inferences.
4. Open questions and recommended confirming traces/tests.
5. Suggested parity fixes or fuzz targets, clearly separated from confirmed
   behavior.
```

### Research-Then-Development Prompt

Use this mode when the offline agent should work for a long session but still
avoid speculative patches. The research phase must justify the development
phase.

```text
You are working offline with the Java ICE source and Rust LAVA source.

Mission:
First research <GROUP/behavior>. Then implement a narrow LAVA parity improvement
only if the research identifies a concrete, source-backed change.

Phase 1: Research
- Produce the Java Logic Map sections requested in the research prompt.
- Identify representative failing or edge cases.
- State the exact ICE behavior to preserve and whether it is directly sourced
  or inferred from traces/snapshots.

Development gate:
- Proceed to code only if you can state a specific rule, hook, schedule, test,
  or fuzz-generation change that follows from the evidence.
- If evidence is insufficient, return the research report and do not patch.

Phase 2: Development
- Make the smallest coherent LAVA change.
- Prefer group-specific schedules/overrides/docs/tests over shared engine
  changes unless the research proves shared behavior is wrong.
- Add or promote representative fixtures only when they document a distinct
  behavior.
- Run cargo check and focused test_runner commands available offline.

Required return:
1. Research report with source references.
2. Decision: patched or research-only, with rationale.
3. Unified patch if patched.
4. Commands run and pass/fail summary.
5. Remaining uncertainty and recommended local Java/Drools verification.
```

For non-parity tasks, change the evidence and commands. Examples:

- Feature/refactor: include design constraints, public API expectations, and
  focused `cargo test` or smoke commands.
- Bug triage: request diagnosis first, patch only if root cause is clear.
- Documentation/research: require source references and separate facts from
  inferences.
- Test-corpus analysis: request findings and candidate fixtures, not code.

## Validate A Returned Patch

Save returned patches under:

```text
tests/relative/tmp/agent_returns/<task-id>.patch
```

Before applying:

```bash
git apply --check tests/relative/tmp/agent_returns/<task-id>.patch
git apply --numstat tests/relative/tmp/agent_returns/<task-id>.patch
```

Review the patch manually:

- Reject or split unrelated refactors.
- Watch for edits to `vendor/`, `dist/`, `target/`, generated files, or VCS
  metadata.
- For parity tasks, verify changed rule files match the assigned group unless
  shared behavior is explicitly justified.
- Confirm tests/fixtures were intentionally changed and not regenerated
  broadly.

Apply only after the patch passes and scope looks right:

```bash
git apply tests/relative/tmp/agent_returns/<task-id>.patch
```

If the patch does not apply cleanly, inspect it as evidence. Do not manually
salvage large chunks without revalidating the resulting diff.

## Local Verification And Commit

Run validation appropriate to the blast radius.

Minimum for most returned patches:

```bash
cargo check
```

For parity changes:

```bash
cargo run --release --bin test_runner -- run tests/cases --group <GROUP>
```

When Java behavior matters, compare locally against ICE:

```bash
cargo run --release --bin test_runner -- run tests/cases --group <GROUP> --compare
```

For shared engine changes, run at least one unrelated guard group and the full
offline suite:

```bash
cargo run --release --bin test_runner -- run tests/cases
```

Commit only after local validation:

```bash
jj commit -m "<GROUP or area>: <concise result>"
```

In the final report, mention the source offline task, what was accepted or
changed locally, validation run, remaining risk, and the jj commit.
