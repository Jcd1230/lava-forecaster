# ICE & Rust Forecaster PoC Agent Guide

Welcome! This guide provides a quick-start reference for AI agents and developers working on the Immunization Calculation Engine (ICE) and its Rust-based Forecaster Proof of Concept (PoC).

## Tech Stack Summary

- **Java ICE Engine**: Legacy Drools-based server requiring **Java 25** and **Maven 3.9**.
- **Rust Forecaster PoC**: High-performance Rust-based implementation of the evaluation engine.
- **Verification Tests**: Python 3 scripts for running verification and comparative test suites.
- **Tooling**: Managed via `mise` (for Java and Maven version consistency).

## Reference Guides

- Detailed Porting & Implementation Instructions: [agent_onboarding_guide.md](file:///home/jason/projects/ice/agent_onboarding_guide.md)
- Current Porting Progress & Task Checklist: [remaining_series_tasks.md](file:///home/jason/projects/ice/remaining_series_tasks.md)
- Original Java Project Setup & Run Details: [README.md](file:///home/jason/projects/ice/README.md)

## Essential Developer Commands

All major tasks are configured as `mise` commands:

| Command | Description |
|---|---|
| `mise run build` | Builds the Java ICE Maven project (`mvn clean install`). |
| `mise run run` | Runs the Java ICE server (exploded WAR) on `http://localhost:8080`. |
| `mise run test` | Runs the Python test suite to verify the Rust PoC output against recorded snapshot JSONs. Does *not* require the Java server to be running. |
| `mise run test-compare` | Compares Rust PoC outputs directly against the live Java ICE server. **Requires the Java server to be running.** |
| `mise run test-record` | Queries the Java ICE server and records its responses as the expected JSON snapshot. **Requires the Java server to be running.** |

## Crucial Gotchas & Project Context

- **Java 25 Requirement**: The Java ICE codebase strictly requires Java 25. Running it via `mise` ensures the correct toolchain version is used.
- **Legacy Rule Definitions**: Legacy rules and support data YAML files are located under the [Series Directory](file:///home/jason/projects/ice/opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/Series/).
- **Comparing Against Live Java**: When implementing or debugging a vaccine group, you can compare Rust behavior against Java in real time. First start the Java server using `mise run run`, then run `mise run test-compare` in another shell.
- **Snapshot Recording**: When updating tests or adding a new test suite, run the Java server and execute `mise run test-record` to automatically create or update the `*.expected.json` files.

## Version Control (Jujutsu / jj-vcs)

- **Primary VCS**: This project primarily uses Jujutsu (`jj`) for version control.
- **Commit Guideline**: Developers and agents must run `jj commit -m "..."` after completing meaningful units of work to save progress and maintain a clean repository history.


