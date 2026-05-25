#!/usr/bin/env python3
import os
import sys
import json
import base64
import argparse
import subprocess
import requests
import re
from datetime import datetime, date, timedelta, time
import calendar
import tempfile
import xml.etree.ElementTree as ET

# Configuration
ICE_BASE_URI = os.environ.get("ICE_BASE_URI", "http://localhost:8080")
JAVA_ENDPOINT = f"{ICE_BASE_URI}/opencds-decision-support-service/api/resources/evaluateAtSpecifiedTime"
RUST_POC_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "../ice-rust-forecaster-poc"))
CASES_DIR = os.path.join(os.path.dirname(__file__), "cases")
TMP_DIR = os.path.join(os.path.dirname(__file__), "tmp")

os.makedirs(TMP_DIR, exist_ok=True)

# Helper function to add months/years/weeks/days
def add_months(d, months):
    month = d.month - 1 + months
    year = d.year + month // 12
    month = month % 12 + 1
    day = min(d.day, calendar.monthrange(year, month)[1])
    return date(year, month, day)

def add_offset(d, val, unit):
    if unit == 'y':
        return add_months(d, val * 12)
    elif unit == 'm':
        return add_months(d, val)
    elif unit == 'w':
        return d + timedelta(days=val * 7)
    elif unit == 'd':
        return d + timedelta(days=val)
    return d

class RelativeDateResolver:
    def __init__(self, dob):
        self.dob = dob
        self.dose_dates = []

    def resolve(self, expr):
        expr = expr.strip()
        if expr == "birth":
            return self.dob
        
        parts = expr.split("+")
        base_ref = parts[0].strip()
        
        if base_ref == "birth":
            base_date = self.dob
        elif base_ref == "prev":
            base_date = self.dose_dates[-1] if self.dose_dates else self.dob
        elif base_ref.startswith("dose"):
            idx = int(base_ref[4:]) - 1
            base_date = self.dose_dates[idx]
        else:
            # Check if absolute YYYY-MM-DD
            try:
                return datetime.strptime(base_ref, "%Y-%m-%d").date()
            except ValueError:
                raise ValueError(f"Unknown date base reference: {base_ref}")
                
        if len(parts) == 1:
            return base_date
            
        offset_str = parts[1].strip()
        terms = re.findall(r"([+-]?\s*\d+\s*[ymwd])", "+" + offset_str)
        
        current_date = base_date
        for term in terms:
            term = term.replace(" ", "")
            sign = -1 if term.startswith("-") else 1
            val_str = term.lstrip("+-")
            unit = val_str[-1]
            val = int(val_str[:-1]) * sign
            current_date = add_offset(current_date, val, unit)
            
        return current_date

# Dynamic XML Generation
def generate_xml_payload(dob, gender, doses):
    sae_templates = []
    for idx, (dt, cvx) in enumerate(doses):
        dt_str = dt.strftime("%Y%m%d") # Format without hyphens to prevent Java date shift bugs
        cvx_padded = str(cvx).zfill(2)
        sae_templates.append(f"""                    <substanceAdministrationEvent>
                        <templateId root="2.16.840.1.113883.3.795.11.9.1.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.10" extension="{1000 + idx}"/>
                        <substanceAdministrationGeneralPurpose code="384810002" codeSystem="2.16.840.1.113883.6.5"/>
                        <substance>
                            <id root="ab0c489e-782a-4c34-9e4e-9094cc2952d7"/>
                            <substanceCode code="{cvx_padded}" displayName="Vaccine" codeSystem="2.16.840.1.113883.12.292"/>
                        </substance>
                        <administrationTimeInterval high="{dt_str}" low="{dt_str}"/>
                    </substanceAdministrationEvent>""")
                    
    sae_str = "\n".join(sae_templates)
    dob_str = dob.strftime("%Y%m%d")
    
    xml = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns3:cdsInput xmlns:ns2="org.opencds.vmr.v1_0.schema.cdsinput.specification"
        xmlns:ns3="org.opencds.vmr.v1_0.schema.cdsinput"
        xmlns:ns4="org.opencds.vmr.v1_0.schema.cdsoutput"
        xmlns:ns5="org.opencds.vmr.v1_0.schema.vmr">
    <templateId root="2.16.840.1.113883.3.795.11.1.1"/>
    <cdsContext>
        <cdsSystemUserPreferredLanguage code="en" codeSystem="2.16.840.1.113883.6.99" displayName="English"/>
    </cdsContext>
    <vmrInput>
        <templateId root="2.16.840.1.113883.3.795.11.1.1"/>
        <patient>
            <templateId root="2.16.840.1.113883.3.795.11.2.1.1"/>
            <id root="2.16.840.1.113883.3.795.12.100.11" extension="43299551" />
            <demographics>
                <birthTime value="{dob_str}"/>
                <gender code="{gender}" codeSystem="2.16.840.1.113883.5.1"/>
            </demographics>
            <clinicalStatements>
                <substanceAdministrationEvents>
{sae_str}
                </substanceAdministrationEvents>
            </clinicalStatements>
        </patient>
    </vmrInput>
</ns3:cdsInput>"""
    return xml

def build_evaluate_payload(dob, gender, doses, eval_date):
    xml_content = generate_xml_payload(dob, gender, doses)
    b64_xml = base64.b64encode(xml_content.encode("utf-8")).decode("utf-8")
    
    # Specified Time in epoch milliseconds
    eval_datetime = datetime.combine(eval_date, time(23, 59, 59))
    ts_ms = int(eval_datetime.timestamp() * 1000)
    
    payload = {
        "interactionId": {
            "scopingEntityId": "org.nyc.cir",
            "interactionId": "123456",
            "submissionTime": ts_ms
        },
        "specifiedTime": ts_ms,
        "evaluationRequest": {
            "clientLanguage": "en",
            "clientTimeZoneOffset": "+0000",
            "kmEvaluationRequest": [{
                "kmId": {
                    "scopingEntityId": "org.nyc.cir",
                    "businessId": "ICE",
                    "version": "1.0.0"
                }
            }],
            "dataRequirementItemData": [{
                "driId": {
                    "containingEntityId": {
                        "scopingEntityId": "org.nyc.cir",
                        "businessId": "ICEData",
                        "version": "1.0.0"
                    },
                    "itemId": "cdsPayload"
                },
                "data": {
                    "informationModelSSId": {
                        "scopingEntityId": "org.opencds.vmr",
                        "businessId": "VMR",
                        "version": "1.0"
                    },
                    "base64EncodedPayload": [b64_xml]
                }
            }]
        }
    }
    return payload

# Output Parsers
def parse_date_only(date_str):
    if not date_str or len(date_str) < 8:
        return None
    # normalize format to YYYY-MM-DD
    clean = "".join(c for c in date_str if c.isdigit())
    return f"{clean[:4]}-{clean[4:6]}-{clean[6:8]}"

def map_legacy_status(legacy_status, reasons):
    if legacy_status == 'COMPLETE':
        return 'Complete'
    if any('COMPLETE' in r for r in reasons):
        return 'Complete'
    if legacy_status == 'CONDITIONAL':
        return 'ConditionallyRecommended'
    if legacy_status == 'NOT_RECOMMENDED':
        return 'NotRecommended'
    if legacy_status in ('RECOMMENDED', 'FUTURE_RECOMMENDED'):
        return 'NotComplete'
    return legacy_status

def map_legacy_dose_status(status):
    if status == 'VALID':
        return 'Valid'
    if status == 'INVALID':
        return 'Invalid'
    if status == 'ACCEPTED':
        return 'Accepted'
    return status

def parse_legacy_xml(xml_content, focus_code):
    root = ET.fromstring(xml_content)
    evaluations = []
    proposals = []
    
    saes_container = None
    for el in root.iter():
        if el.tag.endswith('substanceAdministrationEvents'):
            saes_container = el
            break
            
    if saes_container is not None:
        for admin_event in saes_container:
            if not admin_event.tag.endswith('substanceAdministrationEvent'):
                continue
            
            # Extract date
            date_el = None
            for child in admin_event:
                if child.tag.endswith('administrationTimeInterval'):
                    date_el = child
                    break
            
            admin_date = None
            if date_el is not None:
                admin_date = parse_date_only(date_el.attrib.get('low') or date_el.attrib.get('high'))
                
            # Extract CVX
            cvx = None
            for child in admin_event.iter():
                if child.tag.endswith('substanceCode'):
                    cvx = child.attrib.get('code')
                    break
            
            # Find evaluation for the target group
            group_eval = None
            for rcs in admin_event:
                if not rcs.tag.endswith('relatedClinicalStatement'):
                    continue
                for sub_event in rcs:
                    if not sub_event.tag.endswith('substanceAdministrationEvent'):
                        continue
                    
                    has_group_focus = False
                    status = None
                    dose_number = None
                    reasons = []
                    
                    for sub_rcs in sub_event:
                        if not sub_rcs.tag.endswith('relatedClinicalStatement'):
                            continue
                        for obs in sub_rcs:
                            if not obs.tag.endswith('observationResult'):
                                continue
                            
                            focus_el = None
                            for obs_child in obs:
                                if obs_child.tag.endswith('observationFocus'):
                                    focus_el = obs_child
                                    break
                            if focus_el is not None and focus_el.attrib.get('code') == focus_code:
                                has_group_focus = True
                                
                                # Extract status
                                val_el = None
                                for obs_child in obs:
                                    if obs_child.tag.endswith('observationValue'):
                                        val_el = obs_child
                                        break
                                if val_el is not None:
                                    concept_el = None
                                    for v_child in val_el:
                                        if v_child.tag.endswith('concept'):
                                            concept_el = v_child
                                            break
                                    if concept_el is not None:
                                        status = concept_el.attrib.get('code')
                                
                                # Extract interpretative reasons
                                for obs_child in obs:
                                    if obs_child.tag.endswith('interpretation'):
                                        r = obs_child.attrib.get('code')
                                        if r:
                                            reasons.append(r)
                                            
                    if has_group_focus:
                        for sub_child in sub_event:
                            if sub_child.tag.endswith('doseNumber'):
                                try:
                                    dose_number = int(sub_child.attrib.get('value'))
                                except (ValueError, TypeError):
                                    pass
                                break
                        group_eval = {
                            'status': status,
                            'dose_number': dose_number,
                            'reasons': reasons
                        }
                        break
            
            if group_eval is not None:
                evaluations.append({
                    'date': admin_date,
                    'cvx': cvx,
                    'status': map_legacy_dose_status(group_eval['status']),
                    'dose_number': group_eval['dose_number']
                })
                
    # Parse Proposals
    for proposal in root.iter():
        if not proposal.tag.endswith('substanceAdministrationProposal'):
            continue
        
        sub_code_el = None
        for child in proposal.iter():
            if child.tag.endswith('substanceCode'):
                sub_code_el = child
                break
        
        sub_code_matches = sub_code_el is not None and sub_code_el.attrib.get('code') == focus_code
        obs_focus_matches = False
        if not sub_code_matches:
            for rcs_check in proposal:
                if rcs_check.tag.endswith('relatedClinicalStatement'):
                    for obs_check in rcs_check:
                        if obs_check.tag.endswith('observationResult'):
                            for obs_c in obs_check:
                                if obs_c.tag.endswith('observationFocus') and obs_c.attrib.get('code') == focus_code:
                                    obs_focus_matches = True
                                    
        if sub_code_matches or obs_focus_matches:
            earliest_date = None
            recommended_date = None
            overdue_date = None
            status = None
            reasons = []
            
            for child in proposal:
                if child.tag.endswith('validAdministrationTimeInterval'):
                    earliest_date = parse_date_only(child.attrib.get('low'))
                elif child.tag.endswith('proposedAdministrationTimeInterval'):
                    recommended_date = parse_date_only(child.attrib.get('low'))
                    overdue_date = parse_date_only(child.attrib.get('high'))
            
            for rcs in proposal:
                if not rcs.tag.endswith('relatedClinicalStatement'):
                    continue
                for obs in rcs:
                    if not obs.tag.endswith('observationResult'):
                        continue
                    focus_el = None
                    for obs_child in obs:
                        if obs_child.tag.endswith('observationFocus'):
                            focus_el = obs_child
                            break
                    if focus_el is not None and focus_el.attrib.get('code') == focus_code:
                        val_el = None
                        for obs_child in obs:
                            if obs_child.tag.endswith('observationValue'):
                                val_el = obs_child
                                break
                        if val_el is not None:
                            concept_el = None
                            for v_child in val_el:
                                if v_child.tag.endswith('concept'):
                                    concept_el = v_child
                                    break
                            if concept_el is not None:
                                status = concept_el.attrib.get('code')
                        
                        for obs_child in obs:
                            if obs_child.tag.endswith('interpretation'):
                                r = obs_child.attrib.get('code')
                                if r:
                                    reasons.append(r)
            
            proposals.append({
                'earliest_date': earliest_date,
                'recommended_date': recommended_date,
                'overdue_date': overdue_date,
                'status': map_legacy_status(status, reasons)
            })
            
    evaluations.sort(key=lambda e: e['date'] or "")
    return evaluations, proposals[0] if proposals else None

# Backend Executors
def query_java_service(payload):
    headers = {
        'Content-Type': 'application/json',
        'Accept': 'application/json'
    }
    response = requests.post(JAVA_ENDPOINT, headers=headers, json=payload)
    response.raise_for_status()
    resp_json = response.json()
    
    b64_payload = resp_json['finalKMEvaluationResponse'][0]['kmEvaluationResultData'][0]['data']['base64EncodedPayload'][0]
    clean_b64 = "".join(b64_payload.split())
    xml_bytes = base64.b64decode(clean_b64)
    return xml_bytes.decode('utf-8')

def query_rust_poc(payload):
    # Write payload to a temporary file
    temp_fd, temp_path = tempfile.mkstemp(dir=TMP_DIR, suffix=".json")
    with os.fdopen(temp_fd, 'w') as f:
        json.dump(payload, f)
        
    cmd = [
        "cargo", "run", "--release",
        "--manifest-path", f"{RUST_POC_PATH}/Cargo.toml",
        "--", temp_path
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        # Locate the output JSON line
        lines = result.stdout.strip().split("\n")
        json_line = None
        for line in reversed(lines):
            if line.startswith("{") and line.endswith("}"):
                json_line = line
                break
                
        if not json_line:
            raise RuntimeError(f"Could not find JSON output in Rust stdout.\nStdout:\n{result.stdout}\nStderr:\n{result.stderr}")
            
        return json.loads(json_line)
    finally:
        if os.path.exists(temp_path):
            os.remove(temp_path)

def extract_rust_results(rust_out, group_name):
    selected_group = None
    for vg in rust_out.get('vaccine_groups', []):
        if vg.get('vaccine_group') == group_name:
            selected_group = vg
            break
            
    if not selected_group:
        raise ValueError(f"Vaccine group {group_name} not found in Rust output")
        
    evals = []
    for e in selected_group.get('evaluations', []):
        evals.append({
            'date': e.get('dose_date'),
            'cvx': e.get('cvx'),
            'status': e.get('status'),
            'dose_number': e.get('dose_number')
        })
    evals.sort(key=lambda e: e['date'] or "")
    
    forecasts = selected_group.get('forecasts', [{}])
    forecast = forecasts[0] if forecasts else {}
    
    forecast_out = {
        'status': forecast.get('status'),
        'earliest_date': forecast.get('earliest_date'),
        'recommended_date': forecast.get('recommended_date'),
        'overdue_date': forecast.get('overdue_date')
    }
    return evals, forecast_out

# Run Single Case
def run_test_case(tc, target, group_name, focus_code):
    dob = datetime.strptime(tc["dob"], "%Y-%m-%d").date()
    gender = tc["gender"]
    
    # Resolve relative dates
    resolver = RelativeDateResolver(dob)
    
    doses = []
    for d_expr in tc.get("doses", []):
        date_expr, cvx = d_expr.split(":")
        resolved_date = resolver.resolve(date_expr)
        resolver.dose_dates.append(resolved_date)
        doses.append((resolved_date, cvx))
        
    eval_date = resolver.resolve(tc["eval_date"])
    
    # Build payload
    payload = build_evaluate_payload(dob, gender, doses, eval_date)
    
    result = {}
    if target in ('java', 'both'):
        try:
            xml_out = query_java_service(payload)
            java_evals, java_forecast = parse_legacy_xml(xml_out, focus_code)
            result['java'] = (java_evals, java_forecast)
        except Exception as e:
            result['java_error'] = str(e)
            
    if target in ('rust', 'both'):
        try:
            rust_out = query_rust_poc(payload)
            rust_evals, rust_forecast = extract_rust_results(rust_out, group_name)
            result['rust'] = (rust_evals, rust_forecast)
        except Exception as e:
            result['rust_error'] = str(e)
            
    return result

# Matrix expansion helper
def expand_cases(suite_data):
    expanded = []
    # Regular test cases
    for tc in suite_data.get("test_cases", []):
        expanded.append(tc)
        
    # Matrix test cases
    for mc in suite_data.get("test_matrix_cases", []):
        base_name = mc["name"]
        matrix = mc.get("matrix", {})
        
        # We will support a simple list of parameter keys
        import itertools
        keys = list(matrix.keys())
        values_lists = [matrix[k] for k in keys]
        
        for combination in itertools.product(*values_lists):
            param_map = dict(zip(keys, combination))
            
            # Format name
            suffix = "_".join(str(param_map[k]) for k in keys)
            name = f"{base_name}_{suffix}"
            
            # Helper to replace placeholders
            def subst(val):
                if isinstance(val, str):
                    for k, v in param_map.items():
                        val = val.replace(f"${{{k}}}", str(v))
                    return val
                elif isinstance(val, list):
                    return [subst(item) for item in val]
                elif isinstance(val, dict):
                    return {k: subst(v) for k, v in val.items()}
                return val
                
            tc = {
                "name": name,
                "dob": mc["dob"],
                "gender": mc["gender"],
                "eval_date": subst(mc["eval_date"]),
                "doses": subst(mc.get("doses", [])),
                "group": mc["group"],
                "focus": mc["focus"]
            }
            if "history_ref" in mc:
                tc["history_ref"] = mc["history_ref"]
            expanded.append(tc)
            
    # Resolve history refs
    histories = suite_data.get("histories", {})
    final_cases = []
    for tc in expanded:
        if "history_ref" in tc:
            href = tc["history_ref"]
            if href in histories:
                hist = histories[href]
                # Prepend history doses to test case doses
                tc["doses"] = hist.get("doses", []) + tc.get("doses", [])
                if "dob" not in tc or not tc["dob"]:
                    tc["dob"] = hist.get("dob")
                if "gender" not in tc or not tc["gender"]:
                    tc["gender"] = hist.get("gender")
        final_cases.append(tc)
        
    return final_cases

# Compare results helper
def verify_equivalence(tc_name, java_res, rust_res, expected_res=None):
    errors = []
    
    # 1. Dose evaluations compare
    j_evals, j_fore = java_res if java_res else ([], {})
    r_evals, r_fore = rust_res if rust_res else ([], {})
    e_evals, e_fore = expected_res if expected_res else ([], {})
    
    # Side-by-side verification
    all_dates_cvx = sorted(list(set(
        [(e['date'], e['cvx']) for e in j_evals] +
        [(e['date'], e['cvx']) for e in r_evals] +
        ([(e['date'], e['cvx']) for e in e_evals] if e_evals else [])
    )))
    
    eval_table = []
    for dt, cvx in all_dates_cvx:
        j = next((e for e in j_evals if e['date'] == dt and e['cvx'] == cvx), None)
        r = next((e for e in r_evals if e['date'] == dt and e['cvx'] == cvx), None)
        e = next((e for e in e_evals if e['date'] == dt and e['cvx'] == cvx), None) if e_evals else None
        
        j_status = j['status'] if j else "MISSING"
        r_status = r['status'] if r else "MISSING"
        e_status = e['status'] if e else "MISSING"
        
        j_num = str(j['dose_number']) if (j and j.get('dose_number') is not None) else "-"
        r_num = str(r['dose_number']) if (r and r.get('dose_number') is not None) else "-"
        e_num = str(e['dose_number']) if (e and e.get('dose_number') is not None) else "-"
        
        match = True
        if java_res and rust_res and (j_status != r_status or j_num != r_num):
            match = False
            errors.append(f"Dose {dt} cvx {cvx} mismatch between Java ({j_status} #{j_num}) and Rust ({r_status} #{r_num})")
        if expected_res:
            if java_res and (j_status != e_status or j_num != e_num):
                match = False
                errors.append(f"Dose {dt} cvx {cvx} Java mismatch with Expected snapshot")
            if rust_res and (r_status != e_status or r_num != e_num):
                match = False
                errors.append(f"Dose {dt} cvx {cvx} Rust mismatch with Expected snapshot")
                
        eval_table.append({
            'date': dt,
            'cvx': cvx,
            'java': f"{j_status} #{j_num}",
            'rust': f"{r_status} #{r_num}",
            'expected': f"{e_status} #{e_num}",
            'ok': match
        })
        
    # 2. Forecast compare
    forecast_match = True
    for field in ('status', 'earliest_date', 'recommended_date', 'overdue_date'):
        j_val = j_fore.get(field) if j_fore else None
        r_val = r_fore.get(field) if r_fore else None
        e_val = e_fore.get(field) if e_fore else None
        
        # normalize dates to strings / None
        j_val = str(j_val) if j_val else None
        r_val = str(r_val) if r_val else None
        e_val = str(e_val) if e_val else None
        
        if java_res and rust_res and j_val != r_val:
            forecast_match = False
            errors.append(f"Forecast field {field} mismatch: Java={j_val} Rust={r_val}")
        if expected_res:
            if java_res and j_val != e_val:
                forecast_match = False
                errors.append(f"Forecast field {field} Java mismatch with Expected: Java={j_val} Expected={e_val}")
            if rust_res and r_val != e_val:
                forecast_match = False
                errors.append(f"Forecast field {field} Rust mismatch with Expected: Rust={r_val} Expected={e_val}")
                
    return errors, eval_table, {
        'java': j_fore,
        'rust': r_fore,
        'expected': e_fore
    }

def print_result_table(tc_name, errors, eval_table, forecast, quiet=False):
    if quiet and not errors:
        return
        
    print(f"\nTest Case: {tc_name}")
    if errors:
        print("\033[91mFAIL\033[0m")
        for err in errors:
            print(f" - {err}")
    else:
        print("\033[92mPASS\033[0m")
        
    print(f"{'Date':<12} | {'CVX':<4} | {'Java Evaluation':<18} | {'Rust Evaluation':<18} | {'Status':<5}")
    print("-" * 68)
    for row in eval_table:
        status_str = "\033[92mOK\033[0m" if row['ok'] else "\033[91mFAIL\033[0m"
        print(f"{row['date'] or '-':<12} | {row['cvx'] or '-':<4} | {row['java']:<18} | {row['rust']:<18} | {status_str:<5}")
        
    print("\nForecasts:")
    print(f"{'Field':<15} | {'Java Forecast':<18} | {'Rust Forecast':<18} | {'Expected':<18}")
    print("-" * 75)
    for field in ('status', 'earliest_date', 'recommended_date', 'overdue_date'):
        j_val = str((forecast['java'] or {}).get(field) or '-')
        r_val = str((forecast['rust'] or {}).get(field) or '-')
        e_val = str((forecast['expected'] or {}).get(field) or '-')
        print(f"{field:<15} | {j_val:<18} | {r_val:<18} | {e_val:<18}")

# Main CLI entrypoint
def auto_record_expected(group, cases, expected_path):
    print(f"\033[93mNo expected snapshot found for '{group}' — auto-recording from Java...\033[0m")
    recorded_snapshots = {}
    for tc in cases:
        res = run_test_case(tc, 'java', tc["group"], tc["focus"])
        if 'java' in res:
            java_evals, java_forecast = res['java']
            recorded_snapshots[tc["name"]] = {
                'evaluations': java_evals,
                'forecast': java_forecast
            }
        else:
            print(f"\033[91mError querying Java service to record snapshot: {res.get('java_error')}\033[0m")
            return None
    with open(expected_path, "w") as f:
        json.dump(recorded_snapshots, f, indent=2)
    print(f"Recorded and saved {len(recorded_snapshots)} test cases to {expected_path}")
    return recorded_snapshots

# Main CLI entrypoint
def main():
    parser = argparse.ArgumentParser(description="Centralized ICE REST Test Runner")
    parser.add_argument("--group", default="ALL", help="Vaccine group to run (e.g. varicella, mmr, hepa, or ALL)")
    parser.add_argument("--case", default=None, help="Name of a single test case to run")
    parser.add_argument("--compare", action="store_true", help="Compare outputs of Java and Rust PoC")
    parser.add_argument("--record", action="store_true", help="Record Java responses as expected outputs")
    parser.add_argument("--cdsi", action="store_true", help="Run only CDSi test cases (excluding standard test cases)")
    parser.add_argument("--verbose", "-v", action="store_true", help="Show detailed output for passing tests")
    parser.add_argument("--quiet", "-q", action="store_true", help="Deprecated (quiet mode is now default)")
    args = parser.parse_args()
    
    target = 'both' if args.compare else ('java' if args.record else 'rust')
    quiet_mode = not args.verbose
    
    groups_to_run = []
    if args.group.upper() == 'ALL':
        all_files = [f[:-5] for f in os.listdir(CASES_DIR) if f.endswith(".json") and not f.endswith(".expected.json")]
        if args.cdsi:
            groups_to_run = [g for g in all_files if g.startswith("cdsi_")]
        else:
            groups_to_run = [g for g in all_files if not g.startswith("cdsi_")]
    else:
        groups_to_run = [args.group.lower()]
        
    total_runs = 0
    failed_runs = 0
    results = []
    
    for group in sorted(groups_to_run):
        config_path = os.path.join(CASES_DIR, f"{group}.json")
        expected_path = os.path.join(CASES_DIR, f"{group}.expected.json")
        
        if not os.path.exists(config_path):
            print(f"Error: Test case file not found: {config_path}")
            continue
            
        with open(config_path, "r") as f:
            suite_data = json.load(f)
            
        cases = expand_cases(suite_data)
        
        expected_data = {}
        if not args.record:
            if not os.path.exists(expected_path):
                # Try to ping Java
                java_running = False
                try:
                    requests.get(ICE_BASE_URI, timeout=0.5)
                    java_running = True
                except requests.exceptions.ConnectionError:
                    pass
                except requests.exceptions.RequestException:
                    java_running = True
                
                if java_running:
                    expected_data = auto_record_expected(group, cases, expected_path)
                    if expected_data is None:
                        expected_data = {}
                        failed_runs += len(cases)
                        continue
                else:
                    print(f"\033[91mError: Expected snapshot file not found: {expected_path}\033[0m")
                    print("To record snapshots, run the Java server (mise run run) and execute:")
                    print(f"  mise run test-record -- --group {group}")
                    failed_runs += len(cases)
                    continue
            else:
                with open(expected_path, "r") as f:
                    expected_data = json.load(f)
                
        recorded_snapshots = {}
        
        for tc in cases:
            if args.case and tc["name"] != args.case:
                continue
                
            total_runs += 1
            if not quiet_mode:
                print(f"\nRunning {tc['name']} ({group.upper()})...")
            
            res = run_test_case(tc, target, tc["group"], tc["focus"])
            
            if args.record:
                if 'java' in res:
                    java_evals, java_forecast = res['java']
                    recorded_snapshots[tc["name"]] = {
                        'evaluations': java_evals,
                        'forecast': java_forecast
                    }
                    print(f"Recorded snapshot for {tc['name']}")
                else:
                    print(f"\033[91mError querying Java service to record snapshot: {res.get('java_error')}\033[0m")
                    failed_runs += 1
            else:
                expected = expected_data.get(tc["name"])
                
                # Check for errors in runner execution
                java_error = res.get('java_error')
                rust_error = res.get('rust_error')
                
                if java_error:
                    err_msg = f"Java execution error: {java_error}"
                    print(f"\033[91m{err_msg}\033[0m")
                    failed_runs += 1
                    results.append((tc["name"], group, False, [err_msg]))
                    continue
                if rust_error:
                    err_msg = f"Rust execution error: {rust_error}"
                    print(f"\033[91m{err_msg}\033[0m")
                    failed_runs += 1
                    results.append((tc["name"], group, False, [err_msg]))
                    continue
                    
                java_res = res.get('java')
                rust_res = res.get('rust')
                
                expected_evals = expected.get('evaluations') if expected else None
                expected_forecast = expected.get('forecast') if expected else None
                
                errors, eval_table, forecast = verify_equivalence(
                    tc["name"], 
                    java_res, 
                    rust_res, 
                    expected_res=(expected_evals, expected_forecast) if expected else None
                )
                
                print_result_table(tc["name"], errors, eval_table, forecast, quiet_mode)
                
                if errors:
                    failed_runs += 1
                    results.append((tc["name"], group, False, errors))
                else:
                    results.append((tc["name"], group, True, []))
                    
        if args.record and recorded_snapshots:
            # Merge with existing expected data if running a single test case
            if os.path.exists(expected_path):
                with open(expected_path, "r") as f:
                    try:
                        old_snapshots = json.load(f)
                        old_snapshots.update(recorded_snapshots)
                        recorded_snapshots = old_snapshots
                    except Exception:
                        pass
            with open(expected_path, "w") as f:
                json.dump(recorded_snapshots, f, indent=2)
            print(f"\nSaved snapshots for {len(recorded_snapshots)} test cases to {expected_path}")
            
    print(f"\n--- Test Runner Summary ---")
    print(f"Total test cases executed: {total_runs}")
    if failed_runs > 0:
        print(f"\nFailed test cases:")
        for name, grp, passed, errs in results:
            if not passed:
                print(f"  \033[91mFAIL\033[0m: {name} ({grp.upper()})")
                for err in errs:
                    print(f"    - {err}")
        print(f"\n\033[91m{failed_runs} test cases failed.\033[0m")
        sys.exit(1)
    else:
        print(f"\033[92mAll test cases passed successfully!\033[0m")
        sys.exit(0)

if __name__ == "__main__":
    main()
