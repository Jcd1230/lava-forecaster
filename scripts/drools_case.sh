#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/drools_case.sh <GROUP> <CASE> [CASES_PATH]

Environment:
  DROOLS_LOG  Path to the Java ICE drools event log. If unset, the script uses
              /home/jason/projects/java-ice/opencds-decision-support-service/logs/drools-events.log
              when that file exists.
  JAVA_URL    Java ICE base URL. Defaults to http://localhost:8080.

Example:
  scripts/drools_case.sh DTP fuzz_fail_dtp_20260608_24219
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 2 || $# -gt 3 ]]; then
  usage >&2
  exit 2
fi

GROUP="$1"
CASE="$2"
CASES_PATH="${3:-tests/fuzz-100k-20260608.ltp}"
JAVA_URL="${JAVA_URL:-http://localhost:8080}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEFAULT_DROOLS_LOG="/home/jason/projects/java-ice/opencds-decision-support-service/logs/drools-events.log"

if [[ -z "${DROOLS_LOG:-}" ]]; then
  if [[ -e "${DEFAULT_DROOLS_LOG}" ]]; then
    DROOLS_LOG="${DEFAULT_DROOLS_LOG}"
  else
    echo "DROOLS_LOG is not set and the default log was not found:" >&2
    echo "  ${DEFAULT_DROOLS_LOG}" >&2
    echo "Set DROOLS_LOG=/path/to/drools-events.log and retry." >&2
    exit 2
  fi
fi

if [[ ! -e "${DROOLS_LOG}" ]]; then
  echo "Drools log does not exist: ${DROOLS_LOG}" >&2
  echo "Start Java ICE with drools event logging enabled, or set DROOLS_LOG to the active log file." >&2
  exit 2
fi

ARTIFACT_DIR="${REPO_ROOT}/tests/relative/tmp/drools/${GROUP}/${CASE}"
COMPARE_OUT="${ARTIFACT_DIR}/compare.txt"
RAW_OUT="${ARTIFACT_DIR}/drools_raw.log"
FILTERED_OUT="${ARTIFACT_DIR}/drools_filtered.txt"

mkdir -p "${ARTIFACT_DIR}"

: > "${DROOLS_LOG}"

set +e
(
  cd "${REPO_ROOT}"
  cargo run --release --bin test_runner -- run "${CASES_PATH}" \
    --group "${GROUP}" \
    --case "${CASE}" \
    -v \
    --trace \
    --compare "${JAVA_URL}"
) > "${COMPARE_OUT}" 2>&1
RUN_STATUS=$?
set -e

cp "${DROOLS_LOG}" "${RAW_OUT}"

FILTER_PATTERN="RuleFired|${GROUP}|TargetDose|SUPPORTED_SERIES|ADOLESCENT|ADULT|PERTUSSIS|Duplicate|Recommendation|SeriesSelection|SelectedSeries|Forecast|Evaluation|VALID|INVALID|ACCEPTED"
if command -v rg >/dev/null 2>&1; then
  rg -n "${FILTER_PATTERN}" "${RAW_OUT}" > "${FILTERED_OUT}" || true
else
  grep -En "${FILTER_PATTERN}" "${RAW_OUT}" > "${FILTERED_OUT}" || true
fi

echo "Drools case artifacts:"
echo "  compare : ${COMPARE_OUT}"
echo "  filtered: ${FILTERED_OUT}"
echo "  raw     : ${RAW_OUT}"
echo "  status  : ${RUN_STATUS}"

exit "${RUN_STATUS}"
