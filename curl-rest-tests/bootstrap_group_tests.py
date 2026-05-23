#!/usr/bin/env python3
import argparse
import copy
import json
import os
import re
import sys
from datetime import date, timedelta

import requests
import yaml

import run_tests


REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
CASES_DIR = os.path.join(REPO_ROOT, "curl-rest-tests", "cases")
SERIES_DIR = os.path.join(
    REPO_ROOT,
    "opencds-decision-support-service",
    "src",
    "main",
    "resources",
    "data",
    "knowledgeModule",
    "org.nyc.cir.ice",
    "ice-supporting-data",
    "Series",
)
VACCINE_GROUPS_PATH = os.path.join(
    REPO_ROOT,
    "opencds-decision-support-service",
    "src",
    "main",
    "resources",
    "data",
    "knowledgeModule",
    "org.nyc.cir.ice",
    "ice-supporting-data",
    "OtherLists",
    "supportedVaccineGroups.yml",
)
REMAINING_TASKS_PATH = os.path.join(REPO_ROOT, "remaining_series_tasks.md")

GROUP_ALIASES = {
    "covid": ("COVID19", "COVID_19"),
    "covid19": ("COVID19", "COVID_19"),
    "h1n1": ("H1N1", "INFLUENZA_H1N1"),
    "influenza": ("INFLUENZA", "INFLUENZA"),
    "jev": ("JEV", "JAPANESE_ENCEPHALITIS"),
    "japanese_encephalitis": ("JEV", "JAPANESE_ENCEPHALITIS"),
    "menb": ("MENB", "MENINGOCOCCAL_B"),
    "meningococcal_b": ("MENB", "MENINGOCOCCAL_B"),
    "mpox": ("MPOX", "MPOX"),
    "rotavirus": ("ROTAVIRUS", "ROTAVIRUS"),
    "rsv": ("RSV", "RSV"),
    "cholera": ("CHOLERA", "CHOLERA"),
    "typhoid": ("TYPHOID", "TYPHOID"),
    "yellow_fever": ("YELLOW_FEVER", "YELLOW_FEVER"),
    "yellowfever": ("YELLOW_FEVER", "YELLOW_FEVER"),
}


def normalize_key(value):
    return re.sub(r"[^a-z0-9]+", "_", value.lower()).strip("_")


def resolve_group(group_arg):
    key = normalize_key(group_arg)
    if key in GROUP_ALIASES:
        return GROUP_ALIASES[key]
    upper = group_arg.upper()
    return upper, upper


def read_yaml(path):
    with open(path, "r") as f:
        return yaml.safe_load(f)


def series_entries_from_file(path):
    data = read_yaml(path)
    modules = data.get("ice-supporting-data", {}).get("knowledge-modules", {})
    for module in modules.values():
        for series_code, series in module.get("series", {}).items():
            yield series_code, series


def get_vaccine_group_code(series):
    group_map = series.get("vaccine-group", {})
    if not group_map:
        return None
    first = group_map.get("1") or next(iter(group_map.values()))
    return first.get("code")


def find_series_for_group(legacy_group):
    matches = []
    for name in sorted(os.listdir(SERIES_DIR)):
        if not name.endswith(".yml"):
            continue
        path = os.path.join(SERIES_DIR, name)
        for series_code, series in series_entries_from_file(path):
            if get_vaccine_group_code(series) == legacy_group:
                matches.append((series_code, series, path))
    return matches


def find_focus_code(legacy_group):
    data = read_yaml(VACCINE_GROUPS_PATH)
    code_system = data.get("resourceType")
    concepts = data
    # The supporting-data YAML is a CodeSystem-shaped structure, but keep the
    # traversal defensive because generated YAML wrappers vary across files.
    if code_system is None:
        concepts = data.get("ice-supporting-data", data)

    stack = [concepts]
    while stack:
        current = stack.pop()
        if isinstance(current, dict):
            if current.get("code") == legacy_group:
                for prop in current.get("property", []):
                    if prop.get("code") == "outboundCode":
                        coding = prop.get("valueCoding", {})
                        if "code" in coding:
                            return str(coding["code"])
            stack.extend(current.values())
        elif isinstance(current, list):
            stack.extend(current)

    if os.path.exists(REMAINING_TASKS_PATH):
        with open(REMAINING_TASKS_PATH, "r") as f:
            text = f.read()
        pattern = rf"Java Concept:\* `?{re.escape(legacy_group)}`?.*?\*Focus Code:\* `?(\d+)`?"
        match = re.search(pattern, text)
        if match:
            return match.group(1)
    raise RuntimeError(f"Could not resolve focus code for legacy group {legacy_group}")


def dose_map(series):
    return series.get("doses", {}) or {}


def interval_map(series):
    return series.get("dose-intervals", {}) or {}


def sorted_numeric_values(mapping):
    def sort_key(item):
        key, value = item
        try:
            return int(key)
        except (TypeError, ValueError):
            return int(value.get("dose-number", 0) or value.get("to-dose-number", 0) or 0)

    return [value for _, value in sorted(mapping.items(), key=sort_key)]


def choose_first_cvx(dose):
    vaccines = sorted_numeric_values(dose.get("dose-vaccines", {}) or {})
    if not vaccines:
        return None
    preferred = [v for v in vaccines if v.get("preferred") is True]
    selected = (preferred or vaccines)[0]
    return str(selected.get("vaccine", {}).get("code"))


def choose_period(data, keys):
    for key in keys:
        value = data.get(key)
        if value:
            return str(value)
    return None


def parse_period(period):
    if not period:
        return []
    terms = []
    for value, unit in re.findall(r"([+-]?\d+)\s*([ymwd])", period.replace(" ", "")):
        terms.append((int(value), unit))
    return terms


def add_months(d, months):
    return run_tests.add_months(d, months)


def add_period(d, period):
    current = d
    for value, unit in parse_period(period):
        if unit == "y":
            current = add_months(current, value * 12)
        elif unit == "m":
            current = add_months(current, value)
        elif unit == "w":
            current = current + timedelta(days=value * 7)
        elif unit == "d":
            current = current + timedelta(days=value)
    return current


def period_to_expr(period):
    terms = parse_period(period)
    if not terms:
        return "birth"
    chunks = []
    for value, unit in terms:
        sign = "-" if value < 0 else ""
        chunks.append(f"{sign}{abs(value)}{unit}")
    return "birth + " + "".join(chunks)


def days_from_birth_expr(target, birth):
    days = (target - birth).days
    if days == 0:
        return "birth"
    if days < 0:
        return target.isoformat()
    return f"birth + {days}d"


def safe_name(value):
    return normalize_key(value).replace("_series", "")


def append_case(cases, seen, case):
    base = case["name"]
    name = base
    counter = 2
    while name in seen:
        name = f"{base}_{counter}"
        counter += 1
    case["name"] = name
    seen.add(name)
    cases.append(case)


def generate_cases_for_series(test_group, focus_code, series_code, series):
    birth = date(2020, 1, 1)
    prefix = safe_name(series_code)
    doses = sorted_numeric_values(dose_map(series))
    intervals = sorted_numeric_values(interval_map(series))
    cases = []
    seen = set()
    if not doses:
        return cases

    dose1 = doses[0]
    dose1_cvx = choose_first_cvx(dose1)
    if not dose1_cvx:
        return cases

    dose1_rec = choose_period(
        dose1,
        ["earliest-recommended-age", "minimum-age", "absolute-minimum-age"],
    )
    dose1_abs = choose_period(dose1, ["absolute-minimum-age"])
    dose1_expr = period_to_expr(dose1_rec)
    dose1_date = add_period(birth, dose1_rec) if dose1_rec else birth

    append_case(
        cases,
        seen,
        {
            "name": f"{prefix}_no_history_at_rec_age",
            "dob": birth.isoformat(),
            "gender": "F",
            "eval_date": dose1_expr,
            "doses": [],
            "group": test_group,
            "focus": focus_code,
        },
    )

    append_case(
        cases,
        seen,
        {
            "name": f"{prefix}_dose1_valid",
            "dob": birth.isoformat(),
            "gender": "F",
            "eval_date": days_from_birth_expr(dose1_date + timedelta(days=28), birth),
            "doses": [f"{dose1_expr}:{dose1_cvx}"],
            "group": test_group,
            "focus": focus_code,
        },
    )

    if dose1_abs:
        too_young_date = add_period(birth, dose1_abs) - timedelta(days=1)
        append_case(
            cases,
            seen,
            {
                "name": f"{prefix}_dose1_too_young",
                "dob": birth.isoformat(),
                "gender": "F",
                "eval_date": days_from_birth_expr(too_young_date + timedelta(days=28), birth),
                "doses": [f"{days_from_birth_expr(too_young_date, birth)}:{dose1_cvx}"],
                "group": test_group,
                "focus": focus_code,
            },
        )

    complete_doses = []
    dose_dates = []
    previous_date = None
    for index, dose in enumerate(doses, start=1):
        cvx = choose_first_cvx(dose) or dose1_cvx
        age_period = choose_period(
            dose,
            ["earliest-recommended-age", "minimum-age", "absolute-minimum-age"],
        )
        candidates = []
        if age_period:
            candidates.append(add_period(birth, age_period))
        if previous_date is not None:
            interval = next(
                (
                    i
                    for i in intervals
                    if int(i.get("from-dose-number", 0)) == index - 1
                    and int(i.get("to-dose-number", 0)) == index
                ),
                None,
            )
            if interval:
                interval_period = choose_period(
                    interval,
                    [
                        "earliest-recommended-interval",
                        "minimum-interval",
                        "absolute-minimum-interval",
                    ],
                )
                if interval_period:
                    candidates.append(add_period(previous_date, interval_period))
        dose_date = max(candidates) if candidates else (previous_date or birth)
        dose_dates.append(dose_date)
        complete_doses.append(f"{days_from_birth_expr(dose_date, birth)}:{cvx}")
        previous_date = dose_date

    append_case(
        cases,
        seen,
        {
            "name": f"{prefix}_complete",
            "dob": birth.isoformat(),
            "gender": "F",
            "eval_date": days_from_birth_expr(dose_dates[-1] + timedelta(days=28), birth),
            "doses": complete_doses,
            "group": test_group,
            "focus": focus_code,
        },
    )

    if len(doses) > 1 and intervals:
        first_interval = next(
            (
                i
                for i in intervals
                if int(i.get("from-dose-number", 0)) == 1
                and int(i.get("to-dose-number", 0)) == 2
            ),
            None,
        )
        if first_interval:
            abs_interval = choose_period(first_interval, ["absolute-minimum-interval"])
            dose2_cvx = choose_first_cvx(doses[1]) or dose1_cvx
            if abs_interval and dose2_cvx:
                too_soon_date = add_period(dose1_date, abs_interval) - timedelta(days=1)
                append_case(
                    cases,
                    seen,
                    {
                        "name": f"{prefix}_dose2_too_soon",
                        "dob": birth.isoformat(),
                        "gender": "F",
                        "eval_date": days_from_birth_expr(too_soon_date + timedelta(days=28), birth),
                        "doses": [
                            f"{dose1_expr}:{dose1_cvx}",
                            f"{days_from_birth_expr(too_soon_date, birth)}:{dose2_cvx}",
                        ],
                        "group": test_group,
                        "focus": focus_code,
                    },
                )

    return cases


def generate_suite(test_group, legacy_group):
    focus_code = find_focus_code(legacy_group)
    series_matches = find_series_for_group(legacy_group)
    if not series_matches:
        raise RuntimeError(f"No legacy series YAML found for vaccine group {legacy_group}")

    test_cases = []
    seen = set()
    for series_code, series, _path in series_matches:
        for case in generate_cases_for_series(test_group, focus_code, series_code, series):
            append_case(test_cases, seen, case)

    return {
        "histories": {},
        "test_cases": test_cases,
    }


def record_expected(suite):
    snapshots = {}
    for tc in run_tests.expand_cases(suite):
        res = run_tests.run_test_case(tc, "java", tc["group"], tc["focus"])
        if "java" not in res:
            raise RuntimeError(f"{tc['name']}: {res.get('java_error', 'unknown Java error')}")
        java_evals, java_forecast = res["java"]
        snapshots[tc["name"]] = {
            "evaluations": java_evals,
            "forecast": java_forecast,
        }
        print(f"Recorded snapshot for {tc['name']}")
    return snapshots


def assert_java_reachable():
    try:
        requests.get(run_tests.ICE_BASE_URI, timeout=0.5)
    except requests.exceptions.RequestException as exc:
        raise RuntimeError(
            f"Java ICE is not reachable at {run_tests.ICE_BASE_URI}. "
            "Start it with `mise run run` before bootstrapping."
        ) from exc


def main():
    parser = argparse.ArgumentParser(
        description="Generate baseline group cases and record Java snapshots."
    )
    parser.add_argument("--group", required=True, help="Vaccine group name, e.g. cholera or menb")
    parser.add_argument(
        "--force",
        action="store_true",
        help="Replace existing case and expected snapshot files for the group.",
    )
    args = parser.parse_args()

    test_group, legacy_group = resolve_group(args.group)
    group_file = normalize_key(test_group)
    case_path = os.path.join(CASES_DIR, f"{group_file}.json")
    expected_path = os.path.join(CASES_DIR, f"{group_file}.expected.json")

    existing = [p for p in [case_path, expected_path] if os.path.exists(p)]
    if existing and not args.force:
        paths = "\n  ".join(existing)
        print(f"Refusing to overwrite existing files without --force:\n  {paths}", file=sys.stderr)
        return 1

    suite = generate_suite(test_group, legacy_group)
    if not suite["test_cases"]:
        print(f"No baseline cases generated for {legacy_group}", file=sys.stderr)
        return 1

    try:
        assert_java_reachable()
        snapshots = record_expected(copy.deepcopy(suite))
    except Exception as exc:
        print(f"Bootstrap failed before writing files: {exc}", file=sys.stderr)
        return 1

    os.makedirs(CASES_DIR, exist_ok=True)
    with open(case_path, "w") as f:
        json.dump(suite, f, indent=2)
        f.write("\n")
    with open(expected_path, "w") as f:
        json.dump(snapshots, f, indent=2)
        f.write("\n")

    print(f"\nGenerated {len(suite['test_cases'])} cases: {case_path}")
    print(f"Recorded {len(snapshots)} snapshots: {expected_path}")
    print(f"Next: mise run test -- --group {group_file}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
