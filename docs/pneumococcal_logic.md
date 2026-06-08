# LAVA Forecaster: Pneumococcal Logic Guide

This document reverse-engineers the PNEUMOCOCCAL forecasting logic from the Java ICE engine to ensure 100% behavioral parity. The rules were derived by observing fuzz test failures and cross-referencing with the ICE source code.

## Key Concepts & Rules

### 1. Catch-up Schedule Completion

The childhood PCV series is considered complete based on the number of valid doses received and the patient's age when the *first* dose was administered. This prevents premature completion for children who start the series late.

- **Source of Truth**: `Evaluation^Pneumococcal.dslr` (rules checking for `_PNEUMOCOCCAL_CHILD_SERIES_COMPLETE` facts)
- **Rules**:
  - Started **< 7 months**: Requires **4 doses**.
  - Started at **7-11 months**: Requires **3 doses**.
  - Started at **12-23 months**: Requires **2 doses**.
  - Started at **>= 24 months**: Requires **1 dose**.
- **Verification**: Test cases where a child starts vaccination late (e.g., at 13 months) and receives only one dose. The series should *not* be considered complete.

### 2. High-Risk Transition at 5 Years

A standard-risk child's routine PCV series is considered closed and complete at age 5. However, for a high-risk child, the series remains open to allow for a supplemental PPSV23 dose.

- **Source of Truth**: `Recommendation^Pneumococcal.dslr` (rules for `Conditional/HIGH_RISK`)
- **Rule**: If a child is >= 5 years old but < 19, and their childhood PCV series was *not* complete, they are considered high-risk. The forecast should be `ConditionallyRecommended`, not `Complete`.
- **Verification**: Test cases where an unvaccinated 6-year-old is evaluated. The expected forecast is `ConditionallyRecommended`.

### 3. Forecast Date Clamping for Catch-up

For children starting the series late with no prior doses, the forecast dates are clamped to specific age milestones to ensure they get on the correct catch-up schedule.

- **Source of Truth**: `Recommendation^Pneumococcal.dslr` (rules like "Set Recommendation Date to 24months")
- **Rules**: If a patient has no valid PCV doses:
  - If current age is **>= 24 months**, all forecast dates are clamped to `birth_date + 2 years`.
  - If current age is **12-23 months**, all forecast dates are clamped to `birth_date + 1 year`.
  - If current age is **7-11 months**, all forecast dates are clamped to `birth_date + 7 months`.
- **Verification**: Test case with a healthy 18-month-old with no prior shots. The forecast `earliest_date` should be their 1st birthday, not a date derived from the standard schedule intervals.

### 4. Overdue Date Alignment

For childhood PCV forecasts that are `NotComplete`, the `overdue_date` must be strictly aligned with the `recommended_date`.

- **Source of Truth**: This is an emergent rule observed across many test cases. The Drools engine appears to have a default behavior or a final rule that enforces this alignment for consistency.
- **Rule**: At the end of the forecasting process for a `NotComplete` childhood series, if the calculated `overdue_date` is different from the `recommended_date`, it must be set equal to the `recommended_date`.
- **Verification**: Test cases where a dose is due (e.g., today), but the default overdue interval from the schedule (`+4m`) would place the overdue date far in the future. The expected `overdue_date` is today.
