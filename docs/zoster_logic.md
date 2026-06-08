# LAVA Forecaster: Zoster Logic Guide

This document reverse-engineers the Zoster forecasting logic from the Java ICE engine to ensure 100% behavioral parity.

## Key Concepts & Rules

### 1. Live Virus Spacing (8-Week Interval)

The primary complexity for the Zoster (Shingrix) series is ensuring adequate spacing from prior live vaccines to avoid potential interference. An 8-week (56-day) interval is required after the administration of a live zoster or varicella vaccine.

- **Source of Truth**: `Recommendation^Zoster.dslr`
  - Rule: `"Zoster: If patient is administered a CVX 121 or 188, the minimum interval and recommended interval is 8 weeks"`
  - Rule: `"Zoster: If patient is administered adult varicella (CVX 21) doses, the minimum interval and recommended interval... is 8 weeks"`

- **Logic**: When forecasting for a Zoster dose, the engine must check the patient's history for any doses of Zostavax (CVX 121, 188) or Varicella (CVX 21). If found, the `earliest_date` and `recommended_date` for the next Zoster dose must be anchored to be no earlier than 8 weeks after the date of the live virus vaccine.

- **Verification**: Test cases where a patient has recently received a Varicella vaccine. The forecast for Zoster should be pushed out to respect the 8-week interval, and the `overdue_date` should be similarly re-anchored.

### 2. Legacy Vaccine Evaluation

The older, live Zostavax vaccine (Zostavax®, CVX 121/188) is not part of the recommended 2-dose Shingrix series.

- **Source of Truth**: `Evaluation^Zoster.dslr`
  - Rule: `"Zoster: Evaluate shot as Accepted/VACCINE_NOT_PART_OF_THIS_SERIES if administered CVX 121 or CVX 188"`

- **Logic**: Any administered dose of Zostavax should be evaluated with the status `Accepted` and a reason of `VaccineNotPartOfSeries`. This correctly removes it from consideration as a valid dose in the series without marking it as an outright error.

- **Verification**: Test cases where a patient has a history of receiving Zostavax. The dose should be evaluated as `Accepted`, and the forecast should still recommend the full 2-dose Shingrix series.
