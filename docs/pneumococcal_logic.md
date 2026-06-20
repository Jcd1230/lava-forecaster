# LAVA Forecaster: Pneumococcal Logic Guide

This document reverse-engineers the PNEUMOCOCCAL forecasting logic from the Java ICE engine to ensure behavioral parity. The rules are derived from cross-referencing fuzz test failures with legacy schedule rules.

## Key Concepts & Rules

### 1. Catch-up Schedule Completion

The childhood PCV series is considered complete based on the number of valid doses received and the patient's age when the *first* dose was administered. This prevents premature completion for children who start the series late.

- **Rules**:
  - Started **< 7 months**: Requires **4 doses**.
  - Started at **7-11 months**: Requires **3 doses**.
  - Started at **12-23 months**: Requires **2 doses**.
  - Started at **>= 24 months**: Requires **1 dose** (but only if it is a modern PCV product; older PCV products require a supplemental PCV).
- **Modern PCV Products**: CVX `133`, `152`, `177`, `215`, `216`, and `327` (with CVX `327` added to prevent `Accepted -> Valid` mismatches).
- **Implementation**: Enforced through the helper `child_pcv_complete` in `overrides.rs`.

### 2. High-Risk Transition at Age 5 to 18

A standard-risk child's routine PCV series is considered closed and complete at age 5. However, if a child is >= 5 years old but < 19, and they have not completed the childhood PCV catch-up requirements, they transition to a high-risk path.

- **Rule**: If a patient is age 5 through 18, and they have *not* completed the child PCV catch-up series, the forecast becomes `ConditionallyRecommended` with the reason `HIGH_RISK` rather than automatically closing as complete.
- **Implementation**: Handled in both `pneumococcal_custom_completion_hook` and `pneumococcal_custom_forecast_hook`.

### 3. Removal of Aggressive Childhood Target-Dose Jumps

Childhood pneumococcal doses remain on their natural sequential target slots. The engine does not jump children into later target-dose slots (such as dose 4 or 5) based on age/timing prior to age 5.

- **Rule**: Custom target-dose logic only jumps to the adult/high-risk portion of the combined series (starting at dose 6) once the patient is at least 5 years old.
- **Implementation**: Handled in `pneumococcal_custom_dose_number_hook`.

### 4. High-Risk Modern PCV Extra-Dose Handling Before Age 5

When the routine child catch-up series is complete, a modern PCV product administered before age 5 can still be counted as the high-risk PCV component.

- **Rule**: If a patient is < 5 years old and has completed the routine child PCV series, a modern PCV dose counts as the high-risk component if no prior valid child-slot modern PCV has already satisfied that component.
- **Implementation**: Evaluated via `pneumococcal_custom_extra_dose_hook`.

### 5. Older PCV Products in Adult/High-Risk Target Slots

Older childhood PCV products (CVX `100` and `177`) are not invalidated by generic vaccine eligibility checks when given in adult/high-risk slots (doses >= 6).

- **Rule**: CVX `100` and `177` doses administered at target dose >= 6 are evaluated as `Accepted` with the reason `VaccineNotPartOfSeries` rather than being marked `Invalid`.
- **Implementation**: Covered in `pneumococcal_custom_evaluation_hook`.

### 6. Forecast Date Clamping & Catch-up Corrections

For children starting the series late or with incomplete histories, forecast dates are clamped or adjusted:

- **Late Start Clamping**:
  - If current age is **>= 24 months** and 0 valid doses exist, all forecast dates clamp to `birth_date + 2 years`.
  - If current age is **12-23 months** and 0 valid doses exist, all forecast dates clamp to `birth_date + 1 year`.
  - If current age is **7-11 months** and 0 valid doses exist, all forecast dates clamp to `birth_date + 7 months`.
- **Single Dose Catch-up Corrections**:
  - A single valid PCV dose received before 12 months forecasts the next catch-up milestone at **24 months**.
  - A single late-start older PCV dose (CVX `100` or `177`) received at or after 24 months does not complete the series and forecasts a supplemental PCV dose **56 days** later.
- **Implementation**: Configured inside `pneumococcal_custom_forecast_hook`.
