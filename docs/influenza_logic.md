# LAVA Forecaster: Influenza (INFLUENZA) Logic Guide

This document details the Influenza forecasting and evaluation logic, highlighting custom rules and design decisions implemented to achieve parity with the legacy Drools-based Java ICE engine.

## Key Concepts & Rules

### 1. Unsupported-Product Handling

To ensure that non-US or otherwise unsupported seasonal influenza products do not incorrectly count toward the patient's seasonal immunization requirements:

- **Unsupported CVX Codes**: `144`, `161`, `166`, `194`, `200`, `201`, `202`, `231`, `331`, `337`
- **Rule**: Any administered dose matching these CVX codes is evaluated as **Invalid**.
- **Implementation**: Centralized under `is_ice_unsupported_influenza_cvx`, which is called inside:
  - `influenza_custom_evaluation_hook` (forces status to `Invalid` and sets the reason to `VaccineNotAllowedInUs`).
  - `count_valid_prior_doses` and `evaluate_history_seasonally` helpers to ensure history re-evaluation paths ignore these doses.

### 2. Removal of ACIP High-Dose Age Invalidation (CVX 135 / 197)

Under strict ACIP guidelines, high-dose influenza products are only indicated for patients aged 65 years and older. However, the legacy ICE engine does not invalidate these doses solely due to age.

- **Rule**: Doses of CVX `135` and `197` administered to patients under 65 years are **not** invalidated by age restrictions. They count normally toward the seasonal dose requirements.
- **Implementation**: Removed the custom age-invalidation block for CVX `135` and `197` from the evaluation hook and seasonal history helpers.

### 3. Completed Season Forecast Date Layout

When the current season's influenza dose requirement is fully satisfied, the forecast dates for the subsequent season must be laid out to match ICE's structure.

- **Rule**: 
  - `earliest_date` is set to `None` (left blank).
  - `recommended_date` is set to the start date of the next season, adjusted to satisfy a minimum 28-day interval from the patient's latest administered dose.
  - `overdue_date` and `latest_date` are set to `None`.
- **Implementation**: Handled in the completion checks inside `influenza_custom_forecast_hook`.
