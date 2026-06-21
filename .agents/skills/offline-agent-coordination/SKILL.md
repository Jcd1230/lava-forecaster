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

- Task type: parity bucket, feature/refactor, bug triage, documentation, or
  test-corpus analysis.
- Success criteria: exact cases, commands, and expected behavior.
- Allowed edit scope: files/directories the agent may change, plus any areas
  that require explicit justification.
- Evidence needed: offline snapshots only, live Java compare logs, Drools logs,
  promoted fixtures, trace output, or design constraints.
- Return artifact: usually a unified patch from repo root; for analysis tasks,
  a written report may be enough.

Ask the user before delegation only when product intent or scope is ambiguous.
Do not ask where repo facts live; inspect the repo.

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

Use this structure, editing it for the specific task. Remove irrelevant
sections and add concrete file paths/cases when known.

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

For non-parity tasks, change the evidence and commands. Examples:

- Feature/refactor: include design constraints, public API expectations, and
  focused `cargo test` or smoke commands.
- Bug triage: request diagnosis first, patch only if root cause is clear.
- Documentation: require changed docs plus any command/output verification.
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
