# LAVA Forecaster: Hepatitis B (HEP_B) Logic Guide

This document details the Hepatitis B forecasting and evaluation logic, highlighting custom rules and design decisions implemented to achieve parity with the legacy Drools-based Java ICE engine.

## Key Concepts & Rules

### 1. Invalidation of Underage Adult Hep B Products

To prevent pediatric/adolescent patients from receiving adult-only Hepatitis B formulations, specific CVX codes are restricted based on the patient's age at administration.

- **CVX Codes Covered**:
  - `189`: Heplisav-B (Adult 2-dose product)
  - `220`: PreHevbrio (Adult 3-dose product)
- **Rule**: Any dose of CVX `189` or `220` administered before **18 years minus 4 days** (relative to the patient's birth date) is evaluated as **Invalid**.
- **Implementation**:
  - Covered in `hep_b_custom_evaluation_hook` (clears generic reasons and marks the dose status as `Invalid`).
  - Covered in `hep_b_custom_extra_dose_hook` to ensure the same invalidation occurs when evaluated under extra-dose processing paths.

### 2. Series Selection and Heuristic Refinement

Series selection for child/adolescent Hepatitis B groups does not rely on birth-dose or combination-vaccine heuristics to pre-select the 4-dose series.

- **Rule**:
  - The series priority defaults child/adolescent selection to `HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES`, then tries `HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES`.
  - Specific series-switching rules and complete-series tie-breaker checks handle transitioning patients to the 4-dose track when necessary, matching ICE's logic.
- **Implementation**:
  - Removed the `child_requires_four_dose_series` heuristic inside `hep_b_group_selection`.
  - Priority list updated in `hep_b_group_selection` to try the 3-dose series first.

### 3. Child/Adolescent Dose 2 Forecast Overdue Dates

Some overly aggressive overrides previously forced overdue dates to equal the earliest or recommended dates for child/adolescent dose 2 forecasts (particularly when combination vaccines or birth doses were present).

- **Rule**: Standard schedules compute the overdue date using the base schedule rules (latest recommended/overdue calculation) instead of collapsing overdue dates to the earliest/recommended date.
- **Implementation**:
  - Removed the forecast overrides that artificially advanced the overdue date for dose 1 / dose 2 in the presence of Pediarix (`CVX 110`) or monovalent birth doses.

### 4. Forecast Spacing Post-Invalid Underage Adult Dose

If the patient's most recent administered Hepatitis B vaccine is an invalid underage adult-product attempt, the forecast dates for subsequent doses must still be spaced relative to that invalid attempt.

- **Rule**:
  - When the latest history dose is an invalid underage adult-product dose (`189` or `220`), and it occurred after the latest valid counted dose, the next forecast date incorporates a spacing constraint from this invalid attempt.
  - **Spacing Applied**:
    - **28 days** if the forecaster is still targeting dose 2.
    - **56 days** if targeting dose 3 or later.
- **Implementation**:
  - Managed dynamically inside `hep_b_custom_forecast_hook` for both `HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES` and `HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES`.
