#!/bin/bash
set -e

# Scaffold a new vaccine group in the LAVA Forecaster and add test suite placeholder.
# Usage: ./scripts/scaffold_group.sh <group_lower> [GROUP_UPPER]

if [ -z "$1" ]; then
  echo "Error: Missing vaccine group name."
  echo "Usage: $0 <group_name_lower> [GROUP_NAME_UPPER]"
  echo "Example: $0 menb MENB"
  exit 1
fi

GROUP_LOWER=$(echo "$1" | tr '[:upper:]' '[:lower:]')
GROUP_UPPER=$(echo "${2:-$1}" | tr '[:lower:]' '[:upper:]')

# Correct paths relative to workspace root
RULES_DIR="src/rules"
CASES_DIR="tests/relative"
GROUP_DIR="${RULES_DIR}/${GROUP_LOWER}"

if [ -d "${GROUP_DIR}" ]; then
  echo "Error: Directory ${GROUP_DIR} already exists."
  exit 1
fi

echo "Scaffolding vaccine group: ${GROUP_UPPER} (${GROUP_LOWER})..."

# 1. Create directory
mkdir -p "${GROUP_DIR}"

# 2. Write mod.rs
cat << EOF > "${GROUP_DIR}/mod.rs"
use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod schedules;
pub mod overrides;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "${GROUP_UPPER}",
        series: vec![
            // schedules::some_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::${GROUP_LOWER}_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::${GROUP_LOWER}_custom_evaluation_hook),
        custom_dose_number_hook: None,
        group_selection: None,
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        // schedules::some_series(),
    ]
}
EOF

# 3. Write schedules.rs
cat << EOF > "${GROUP_DIR}/schedules.rs"
use crate::schedule::CompiledSeries;

// Define schedules here, e.g.:
// pub fn some_series() -> CompiledSeries { ... }
EOF

# 4. Write overrides.rs
cat << EOF > "${GROUP_DIR}/overrides.rs"
use chrono::NaiveDate;
use crate::engine::EvaluationContext;
use crate::models::{Patient, Dose, DoseStatus, EvaluationReason, SeriesForecast};

pub fn ${GROUP_LOWER}_custom_evaluation_hook(
    _series_name: &str,
    _target_dose_idx: usize,
    _ctx: &EvaluationContext,
    _reasons: &mut Vec<EvaluationReason>,
    _status: &mut DoseStatus,
) {
    // Implement custom evaluation overrides here
}

pub fn ${GROUP_LOWER}_custom_forecast_hook(
    _patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    _eval_date: NaiveDate,
    _forecast: &mut SeriesForecast,
) {
    // Implement custom forecast overrides here
}
EOF

# 5. Write tests/relative case placeholder
mkdir -p "${CASES_DIR}"
cat << EOF > "${CASES_DIR}/${GROUP_LOWER}.json"
{
  "group": "${GROUP_UPPER}",
  "focus": "FIXME_INSERT_FOCUS_CODE",
  "cases": [
    {
      "name": "${GROUP_LOWER}_standard_case_1",
      "dob": "2020-01-01",
      "evalDate": "2020-06-01",
      "gender": "F",
      "shots": [],
      "expected": {
        "status": "Recommended",
        "evaluations": []
      }
    }
  ]
}
EOF

echo "Generated files for ${GROUP_UPPER}."

# 6. Update src/rules/mod.rs using python for reliability
python3 -c "
import sys

file_path = '${RULES_DIR}/mod.rs'
group_lower = '${GROUP_LOWER}'
group_upper = '${GROUP_UPPER}'

with open(file_path, 'r') as f:
    content = f.read()

if f'pub mod {group_lower};' in content:
    print(f'Module {group_lower} already registered in rules/mod.rs.')
    sys.exit(0)

# Add pub mod
lines = content.splitlines()
pub_mod_idx = -1
for i, l in enumerate(lines):
    if l.strip().startswith('pub mod '):
        pub_mod_idx = i

if pub_mod_idx != -1:
    lines.insert(pub_mod_idx + 1, f'pub mod {group_lower};')
else:
    lines.insert(0, f'pub mod {group_lower};')

content = '\n'.join(lines)

# Add map insert
lines = content.splitlines()
map_insert_idx = -1
for i, l in enumerate(lines):
    if 'm.insert(' in l:
        map_insert_idx = i

if map_insert_idx != -1:
    lines.insert(map_insert_idx + 1, f'        m.insert(\"{group_upper}\", {group_lower}::definition());')

content = '\n'.join(lines)

# Add get_all_groups vector element
lines = content.splitlines()
all_groups_idx = -1
for i, l in enumerate(lines):
    if 'get_ruleset(' in l and ').unwrap()' in l:
        all_groups_idx = i

if all_groups_idx != -1:
    lines.insert(all_groups_idx + 1, f'        get_ruleset(\"{group_upper}\").unwrap(),')

content = '\n'.join(lines)

with open(file_path, 'w') as f:
    f.write(content + '\n')
print(f'Successfully registered {group_upper} in rules/mod.rs.')
"

echo "Scaffold complete!"
echo "Make sure to update the focus code in ${CASES_DIR}/${GROUP_LOWER}.json with the correct code."
