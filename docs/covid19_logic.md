# LAVA Forecaster: COVID-19 Logic Guide

This document details the COVID-19 forecasting and evaluation logic, highlighting custom rules and design decisions implemented to achieve parity with the legacy Drools-based Java ICE engine.

## Key Concepts & Rules

### 1. Authorization-Window Invalidation for Old COVID CVX Codes

To prevent older/unlicensed COVID-19 formulations from counting on dates before the modern schedule seasons were active:

- **CVX Codes Covered**: `211`, `213`, `229`, `300`, `301`, `302`, `519`
- **Rule**: Any dose of these CVX codes administered before **December 14, 2020** (the start date of the first standard COVID season, `COVID_19_DEC_2020_SEASON`) is evaluated as **Invalid**.
- **Implementation**: Handled in the authorization checks in `evaluate_doses_seasonally`.

### 2. Upper Authorization Bounds for Selected 2023-Series Products

Products specific to the 2023/2024 season are retired or capped at the boundary of the next season:

- **Rule**: 
  - CVX `308` is evaluated as **Invalid** on or after **August 22, 2024**.
  - CVX `310` and `311` are evaluated as **Invalid** on or after **August 22, 2024** only when the patient is at least **12 years of age** on the dose date. (For younger patients, they remain valid to avoid pediatric regressions).
- **Implementation**: Enforced under the authorization bounds check in `evaluate_doses_seasonally`.

### 3. Extra Seasonal COVID Doses Evaluated as Accepted

To align with ICE, doses administered beyond a specific seasonal cap do not count toward the season requirement but remain reportable.

- **Rule**: 
  - Once the seasonal requirement cap is satisfied (e.g., 3 doses for under 5y prior seasons, 1 dose for Aug 2025 season >= 2y, etc.), any additional doses in the season are evaluated as **Accepted** instead of Valid.
  - The extra dose is excluded from the active seasonal accumulator so subsequent evaluations are assessed against a completed-season state.
- **Implementation**: Handled during dose number capping in `evaluate_doses_seasonally`.

### 4. Below-Minimum-Interval Extras Evaluated as Accepted

Some extra doses administered after the season is already satisfied violate minimum interval spacing. Rather than marking them Invalid, ICE accepts them.

- **Rule**: If the computed dose number exceeds the seasonal cap, and the dose is invalid *solely* due to `BelowMinimumInterval` (without same-day duplicate or prior-to-DOB violations), the status is converted to **Accepted**.
- **Implementation**: Enforced during dose capping and validation checks in `evaluate_doses_seasonally`.

### 5. Forecast Earliest Date Spacing Alignment

For the standard 2-64y and 65y+ COVID forecast tracks, the earliest forecast date is aligned with the recommended date to prevent premature forecasts.

- **Rule**: The earliest forecast date spacing is changed from **52 days** to **56 days** after a prior non-Novavax dose.
- **Implementation**: Configured inside `covid19_custom_forecast_hook`.
