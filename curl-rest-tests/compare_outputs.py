#!/usr/bin/env python3
import xml.etree.ElementTree as ET
import base64
import requests
import json
import subprocess
import sys
import os
import tempfile
import re

ICE_BASE_URI = os.environ.get("ICE_BASE_URI", "http://localhost:8080")
ENDPOINT = f"{ICE_BASE_URI}/opencds-decision-support-service/api/resources/evaluate"
RUST_POC_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "../ice-rust-forecaster-poc"))

def parse_date(date_str):
    if not date_str or len(date_str) < 8:
        return None
    return f"{date_str[:4]}-{date_str[4:6]}-{date_str[6:8]}"

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

def parse_legacy_xml(xml_content, focus_code='400'):
    root = ET.fromstring(xml_content)
    
    # 1. Parse patient birth time
    birth_time_el = None
    for el in root.iter():
        if el.tag.endswith('birthTime'):
            birth_time_el = el
            break
    
    birth_date = None
    if birth_time_el is not None:
        birth_date = parse_date(birth_time_el.attrib.get('value'))
        
    evaluations = []
    proposals = []
    
    # 2. Find all substanceAdministrationEvents container
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
                admin_date = parse_date(date_el.attrib.get('low') or date_el.attrib.get('high'))
                
            # Extract CVX code
            cvx = None
            for child in admin_event.iter():
                if child.tag.endswith('substanceCode'):
                    cvx = child.attrib.get('code')
                    break
            
            # Find nested evaluation for Polio (focus code 400)
            polio_eval = None
            for rcs in admin_event:
                if not rcs.tag.endswith('relatedClinicalStatement'):
                    continue
                for sub_event in rcs:
                    if not sub_event.tag.endswith('substanceAdministrationEvent'):
                        continue
                    
                    has_polio_focus = False
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
                                has_polio_focus = True
                                
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
                                
                                # Extract reasons
                                for obs_child in obs:
                                    if obs_child.tag.endswith('interpretation'):
                                        r = obs_child.attrib.get('code')
                                        if r:
                                            reasons.append(r)
                                            
                    if has_polio_focus:
                        for sub_child in sub_event:
                            if sub_child.tag.endswith('doseNumber'):
                                try:
                                    dose_number = int(sub_child.attrib.get('value'))
                                except (ValueError, TypeError):
                                    pass
                                break
                        polio_eval = {
                            'status': status,
                            'dose_number': dose_number,
                            'reasons': reasons
                        }
                        break
            
            if polio_eval is not None:
                evaluations.append({
                    'date': admin_date,
                    'cvx': cvx,
                    'status': map_legacy_dose_status(polio_eval['status']),
                    'dose_number': polio_eval['dose_number'],
                    'reasons': polio_eval['reasons']
                })
    
    # 3. Find Polio proposals
    for proposal in root.iter():
        if not proposal.tag.endswith('substanceAdministrationProposal'):
            continue
        
        sub_code_el = None
        for child in proposal.iter():
            if child.tag.endswith('substanceCode'):
                sub_code_el = child
                break
        # Match by substanceCode.code OR by observationFocus.code inside the proposal
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
                    earliest_date = parse_date(child.attrib.get('low'))
                elif child.tag.endswith('proposedAdministrationTimeInterval'):
                    recommended_date = parse_date(child.attrib.get('low'))
                    overdue_date = parse_date(child.attrib.get('high'))
            
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
                'status': map_legacy_status(status, reasons),
                'reasons': reasons
            })
            
    # Sort evaluations chronologically for easier comparison
    evaluations.sort(key=lambda e: e['date'] or "")
            
    return birth_date, evaluations, proposals

def query_legacy_service(payload_path):
    with open(payload_path, 'rb') as f:
        data = f.read()
    
    headers = {
        'Content-Type': 'application/json',
        'Accept': 'application/json'
    }
    
    response = requests.post(ENDPOINT, headers=headers, data=data)
    response.raise_for_status()
    
    resp_json = response.json()
    b64_payload = resp_json['finalKMEvaluationResponse'][0]['kmEvaluationResultData'][0]['data']['base64EncodedPayload'][0]
    
    clean_b64 = "".join(b64_payload.split())
    xml_bytes = base64.b64decode(clean_b64)
    return xml_bytes.decode('utf-8')

def run_rust_poc(payload_path):
    cmd = [
        "cargo", "run", "--release",
        "--manifest-path", f"{RUST_POC_PATH}/Cargo.toml",
        "--", payload_path
    ]
    # Run silently, capturing stdout
    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    
    # Parse output (the last line should be the JSON response)
    lines = result.stdout.strip().split("\n")
    json_line = None
    for line in reversed(lines):
        if line.startswith("{") and line.endswith("}"):
            json_line = line
            break
            
    if not json_line:
        print("Error: Could not find JSON output from Rust PoC.")
        print(f"Stdout:\n{result.stdout}")
        sys.exit(1)
        
    return json.loads(json_line)

def sanitize_and_write_temp(payload_path):
    with open(payload_path, 'r') as f:
        req_json = json.load(f)
        
    # Extract base64 XML
    b64_payload = req_json['evaluationRequest']['dataRequirementItemData'][0]['data']['base64EncodedPayload'][0]
    xml_content = base64.b64decode(b64_payload).decode('utf-8')
    
    # Replace dates like 1999-06-10 with 19990610 in value, low, high attributes
    # e.g., value="1999-06-10" -> value="19990610"
    sanitized_xml = re.sub(r'(value|low|high)="(\d{4})-(\d{2})-(\d{2})"', r'\1="\2\3\4"', xml_content)
    
    # Re-encode to base64
    new_b64 = base64.b64encode(sanitized_xml.encode('utf-8')).decode('utf-8')
    req_json['evaluationRequest']['dataRequirementItemData'][0]['data']['base64EncodedPayload'][0] = new_b64
    
    # Write to a temporary file
    temp_fd, temp_path = tempfile.mkstemp(suffix=".json")
    with os.fdopen(temp_fd, 'w') as f:
        json.dump(req_json, f, indent=2)
        
    return temp_path

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Compare Rust PoC output with legacy Java ICE output")
    parser.add_argument("payload_path", help="Path to test payload (.dat or .json)")
    parser.add_argument("--group", default="POLIO", help="Vaccine group name in Rust output (e.g., POLIO, HEP_A)")
    parser.add_argument("--focus", default="400", help="Focus concept code in Java ICE XML (e.g., 400 for POLIO, 810 for HEP_A)")
    args = parser.parse_args()
        
    payload_path = args.payload_path
    
    print(f"Running comparison for {payload_path} (Group: {args.group}, Focus Code: {args.focus})...")
    
    # Sanitizing input payload dynamically to avoid legacy Java date shifting bug
    try:
        temp_payload_path = sanitize_and_write_temp(payload_path)
    except Exception as e:
        print(f"Error sanitizing payload dates: {e}")
        sys.exit(1)
        
    try:
        # 1. Query legacy Java service
        try:
            xml_content = query_legacy_service(temp_payload_path)
        except Exception as e:
            print(f"Error querying legacy Java service: {e}")
            print("Please verify the Java ICE service is running at http://localhost:8080")
            sys.exit(1)
            
        birth_date, legacy_evals, legacy_forecasts = parse_legacy_xml(xml_content, focus_code=args.focus)
        
        # 2. Run Rust PoC
        try:
            rust_output = run_rust_poc(temp_payload_path)
        except Exception as e:
            print(f"Error running Rust PoC: {e}")
            sys.exit(1)
    finally:
        if os.path.exists(temp_payload_path):
            os.remove(temp_payload_path)
        
    # Extract vaccine group from Rust output
    selected_group = None
    for vg in rust_output.get('vaccine_groups', []):
        if vg.get('vaccine_group') == args.group:
            selected_group = vg
            break
            
    if not selected_group:
        print(f"Error: {args.group} vaccine group not found in Rust PoC output.")
        sys.exit(1)
        
    rust_evals = selected_group.get('evaluations', [])
    # Sort rust evals chronologically for comparison
    rust_evals.sort(key=lambda e: e.get('dose_date') or "")
    
    rust_forecast = selected_group.get('forecasts', [{}])[0]
    
    # 3. Compare Patient DOB
    print(f"Patient Birth Date: {birth_date}")
    
    # 4. Compare Dose Evaluations
    has_discrepancy = False
    
    print("\n--- Dose Evaluations Comparison ---")
    print(f"{'Date':<12} | {'CVX':<4} | {'Legacy Status':<15} | {'Rust Status':<15} | {'Legacy Dose#':<12} | {'Rust Dose#':<12}")
    print("-" * 85)
    
    # We will build a unified list of dates and CVX codes to check side-by-side
    all_keys = sorted(list(set(
        [(e['date'], e['cvx']) for e in legacy_evals] +
        [(e.get('dose_date'), e.get('cvx')) for e in rust_evals]
    )))
    
    for dt, cvx in all_keys:
        leg = next((e for e in legacy_evals if e['date'] == dt and e['cvx'] == cvx), None)
        rst = next((e for e in rust_evals if e.get('dose_date') == dt and e.get('cvx') == cvx), None)
        
        leg_status = leg['status'] if leg else "MISSING"
        rst_status = rst['status'] if rst else "MISSING"
        leg_num = str(leg['dose_number']) if (leg and leg.get('dose_number') is not None) else "-"
        rst_num = str(rst['dose_number']) if (rst and rst.get('dose_number') is not None) else "-"
        
        status_match = leg_status == rst_status
        num_match = leg_num == rst_num
        
        row_marker = " "
        if not status_match or not num_match:
            row_marker = "X"
            has_discrepancy = True
            
        print(f"{row_marker} {dt:<10} | {cvx:<4} | {leg_status:<15} | {rst_status:<15} | {leg_num:<12} | {rst_num:<12}")
        
    # 5. Compare Forecasts
    print("\n--- Forecast Comparison ---")
    leg_f = legacy_forecasts[0] if legacy_forecasts else None
    
    leg_status = leg_f['status'] if leg_f else "MISSING"
    rst_status = rust_forecast.get('status') if rust_forecast else "MISSING"
    
    leg_earliest = leg_f['earliest_date'] if leg_f else None
    rst_earliest = rust_forecast.get('earliest_date') if rust_forecast else None
    
    leg_rec = leg_f['recommended_date'] if leg_f else None
    rst_rec = rust_forecast.get('recommended_date') if rust_forecast else None
    
    leg_overdue = leg_f['overdue_date'] if leg_f else None
    rst_overdue = rust_forecast.get('overdue_date') if rust_forecast else None
    
    print(f"Status:      Legacy={leg_status:<15} Rust={rst_status:<15} {'[OK]' if leg_status == rst_status else '[MISMATCH]'}")
    print(f"Earliest:    Legacy={str(leg_earliest):<15} Rust={str(rst_earliest):<15} {'[OK]' if leg_earliest == rst_earliest else '[MISMATCH]'}")
    print(f"Recommended: Legacy={str(leg_rec):<15} Rust={str(rst_rec):<15} {'[OK]' if leg_rec == rst_rec else '[MISMATCH]'}")
    print(f"Overdue:     Legacy={str(leg_overdue):<15} Rust={str(rst_overdue):<15} {'[OK]' if leg_overdue == rst_overdue else '[MISMATCH]'}")
    
    if leg_status != rst_status or leg_earliest != rst_earliest or leg_rec != rst_rec or leg_overdue != rst_overdue:
        has_discrepancy = True
        
    if has_discrepancy:
        print("\n\033[91mFAILURE: 1:1 comparison failed due to discrepancies.\033[0m")
        sys.exit(1)
    else:
        print("\n\033[92mSUCCESS: 1:1 agreement verified!\033[0m")
        sys.exit(0)

if __name__ == "__main__":
    main()
