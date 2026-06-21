# ChatGPT Agent Handoff Workflow

The authoritative workflow for coordinating offline ChatGPT agent work lives in
the repo-local skill:

```text
.agents/skills/offline-agent-coordination/SKILL.md
```

Use that skill when planning offline delegation, creating task-specific
prompts, packaging Java-derived evidence, validating returned patches, or
incorporating agent results. The skill is intentionally judgment-driven rather
than script-driven so prompts can be tailored for research, parity development,
research-then-development sessions, feature work, bug triage, documentation,
and test-corpus analysis.

Core principles:

- Local LAVA remains the source of truth for Java ICE comparison, snapshots,
  jj history, and final commits.
- Offline agents get a vendored source bundle plus task-local evidence.
- ChatGPT containers cannot run Java ICE or use the internet.
- Returned patches are reviewed and validated locally before they are applied
  or committed.
- Research-only handoffs should return source-grounded reports that separate
  direct Java/ICE facts from inferred execution behavior.
- In constrained offline sandboxes, prompt agents to use
  `cargo ... --locked --no-default-features` unless the task specifically needs
  the default `jemalloc` feature.
