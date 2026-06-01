#!/usr/bin/env python3
import os
import csv
import json
import re
from datetime import datetime

# Path Configuration
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, ".."))
CSV_PATH = os.path.join(PROJECT_ROOT, "cdsi-healthy-childhood-and-adult-test-cases.csv")
CASES_DIR = os.path.join(PROJECT_ROOT, "tests", "relative")

# Mapping from CDSi Vaccine_Group to internal group name and focus code
GROUP_MAP = {
    'COVID-19': ('COVID19', '850'),
    'DTAP': ('DTP', '200'),
    'FLU': ('INFLUENZA', '800'),
    'HIB': ('HIB', '300'),
    'HPV': ('HPV', '840'),
    'HepA': ('HEP_A', '810'),
    'HepB': ('HEP_B', '100'),
    'MCV': ('MCV', '830'),
    'MENB': ('MENB', '835'),
    'MMR': ('MMR', '500'),
    'PCV': ('PNEUMOCOCCAL', '750'),
    'POL': ('POLIO', '400'),
    'ROTA': ('ROTAVIRUS', '820'),
    'RSV': ('RSV', '875'),
    'VAR': ('VARICELLA', '600'),
    'ZOSTER': ('ZOSTER', '620')
}

def convert_date(date_str):
    if not date_str or not date_str.strip():
        return ""
    # Try parsing MM/DD/YYYY, MM/DD/YY, or YYYY-MM-DD
    for fmt in ("%m/%d/%Y", "%m/%d/%y", "%Y-%m-%d"):
        try:
            dt = datetime.strptime(date_str.strip(), fmt)
            return dt.strftime("%Y-%m-%d")
        except ValueError:
            continue
    raise ValueError(f"Could not parse date: '{date_str}'")

def sanitize_name(test_id, name):
    full_name = f"cdsi_{test_id}_{name}"
    # Replace non-alphanumeric with underscores
    clean = re.sub(r'[^a-zA-Z0-9]+', '_', full_name)
    # Convert to lowercase and strip leading/trailing underscores
    return clean.strip('_').lower()

def main():
    if not os.path.exists(CSV_PATH):
        print(f"Error: CDSi CSV file not found at {CSV_PATH}")
        return 1

    print(f"Reading test cases from {CSV_PATH}...")
    
    # Store test cases grouped by lower-case mapped group name
    grouped_cases = {key.lower().replace('-', '_'): [] for key in GROUP_MAP.keys()}
    
    total_processed = 0
    with open(CSV_PATH, mode='r', encoding='utf-8-sig') as f:
        reader = csv.DictReader(f)
        for row in reader:
            test_id = row.get('CDC_Test_ID', '').strip()
            if not test_id:
                continue
                
            csv_group = row.get('Vaccine_Group', '').strip()
            if not csv_group:
                continue
                
            if csv_group not in GROUP_MAP:
                print(f"Warning: Unknown vaccine group '{csv_group}' in test case {test_id}. Skipping.")
                continue
                
            group_name, focus_code = GROUP_MAP[csv_group]
            
            try:
                dob = convert_date(row.get('DOB', ''))
                eval_date = convert_date(row.get('Assessment_Date', ''))
                if not eval_date:
                    eval_date = dob
            except ValueError as e:
                print(f"Error parsing date for test case {test_id}: {e}. Skipping.")
                continue
                
            # Extract doses
            doses = []
            for i in range(1, 8):
                date_col = f"Date_Administered_{i}"
                cvx_col = f"CVX_{i}"
                if date_col in row and cvx_col in row:
                    admin_date_str = row[date_col].strip()
                    cvx_str = row[cvx_col].strip()
                    if admin_date_str and cvx_str:
                        try:
                            # Normalize CVX (strip potential float .0 suffix if exists)
                            cvx_val = str(int(float(cvx_str)))
                        except ValueError:
                            cvx_val = cvx_str
                        
                        try:
                            formatted_admin_date = convert_date(admin_date_str)
                            doses.append(f"{formatted_admin_date}:{cvx_val}")
                        except ValueError as e:
                            print(f"Warning: Skipping invalid dose date in test case {test_id}, dose {i}: {e}")
                            
            test_case_name = sanitize_name(test_id, row.get('Test_Case_Name', '').strip())
            
            case_obj = {
                "name": test_case_name,
                "dob": dob,
                "gender": row.get('gender', 'F').strip() or "F",
                "eval_date": eval_date,
                "doses": doses,
                "group": group_name,
                "focus": focus_code
            }
            
            # Map group name to the output file suffix
            group_key = csv_group.lower().replace('-', '_')
            grouped_cases[group_key].append(case_obj)
            total_processed += 1

    print(f"Processed {total_processed} test cases.")
    
    # Write JSON files
    os.makedirs(CASES_DIR, exist_ok=True)
    written_count = 0
    for group_key, cases in grouped_cases.items():
        if not cases:
            continue
            
        output_file = os.path.join(CASES_DIR, f"cdsi_{group_key}.json")
        output_data = {
            "histories": {},
            "test_cases": cases
        }
        
        with open(output_file, 'w', encoding='utf-8') as out_f:
            json.dump(output_data, out_f, indent=2)
            out_f.write('\n')
            
        print(f"Saved {len(cases)} cases to {output_file}")
        written_count += len(cases)
        
    print(f"Successfully wrote {written_count} cases across {len(grouped_cases)} files.")
    return 0

if __name__ == "__main__":
    import sys
    sys.exit(main())
