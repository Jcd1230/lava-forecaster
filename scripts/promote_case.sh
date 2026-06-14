#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/promote_case.sh <SOURCE> <CASE> <OUTPUT_JSON> [--force]

Examples:
  scripts/promote_case.sh tests/fuzz-100k-20260608.ltp fuzz_fail_dtp_20260608_24219 tests/cases/dtp/parity/forecast_dose_2_boundary.json
  scripts/promote_case.sh tests/fuzz-100k-20260608.ltp fuzz_fail_dtp_20260608_24219 tests/cases/dtp/parity/forecast_dose_2_boundary.json --force
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 3 || $# -gt 4 ]]; then
  usage >&2
  exit 2
fi

SOURCE="$1"
CASE="$2"
OUTPUT_JSON="$3"
FORCE_FLAG="${4:-}"

if [[ -n "${FORCE_FLAG}" && "${FORCE_FLAG}" != "--force" ]]; then
  usage >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CMD=(cargo run --release --bin test_runner -- promote "${SOURCE}" "${OUTPUT_JSON}" --case "${CASE}")
if [[ "${FORCE_FLAG}" == "--force" ]]; then
  CMD+=(--force)
fi

cd "${REPO_ROOT}"
"${CMD[@]}"
