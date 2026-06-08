# LAVA Forecaster: Rotavirus Logic Guide

This document reverse-engineers the Rotavirus forecasting logic from the Java ICE engine to ensure 100% behavioral parity.

## Key Concepts & Rules

### 1. Strict Maximum Age (8 Months)

The most critical and unique aspect of the Rotavirus series is the strict age limit. The series cannot be started or continued in infants who are too old, due to safety concerns identified in clinical trials.

- **Source of Truth**: `Evaluation^Rotavirus.dslr` and `Recommendation^Rotavirus.dslr`
  - Rule: `"Rotavirus: Mark Dose as Accepted if Patient Greater Than 8 Months of Age"`
  - Rule: `"Rotavirus: Series is Complete if Over 8 Years of Age..."` (Note: the rule says 8 years, but the implementation and ACIP guidelines use 8 months, 0 days)

- **Logic**:
  - Any Rotavirus vaccine administered to a patient who is 8 months of age or older must be evaluated as `Accepted`.
  - No Rotavirus doses should be forecast for any patient who is 8 months of age or older. The series is effectively complete and should not be started or continued.

- **Verification**: Test cases where a patient is evaluated after their 8-month birthday. The forecast should be `Complete` or `NotRecommended`, and any doses administered after this date should be marked `Accepted`.

### 2. Series Selection (2-Dose vs. 3-Dose)

The engine must choose between the 2-dose (Rotarix) and 3-dose (RotaTeq) series. This choice is determined by the vaccine type (CVX) of the first dose administered.

- **Source of Truth**: `SeriesSelection.drl`
  - Rule: `"SeriesSelection.SelectROTAVIRUS_2_DOSE_SERIESIf1DoseRV1"`
  - Rule: `"SeriesSelection.SelectROTAVIRUS_3_DOSE_SERIESIfCertainVaccinesAdministered"`

- **Logic**:
  - If the first dose was CVX 119 (Rotarix), the 2-dose series is selected.
  - If the first dose was CVX 116 (RotaTeq) or CVX 122 (NOS), the 3-dose series is selected.
  - If no doses have been administered, the 3-dose series is chosen by default.

- **Verification**: Test cases where the first dose is Rotarix (CVX 119). The engine should forecast for a total of 2 doses. Conversely, if the first dose is RotaTeq (CVX 116), it should forecast for 3.

### 3. Withdrawn Vaccine (RotaShield)

The original Rotavirus vaccine, RotaShield (CVX 74), was withdrawn from the market. ICE contains special logic to handle it, preferring any other valid Rotavirus vaccine administered on the same day.

- **Source of Truth**: `DuplicateShotSameDay^Rotavirus.drl`
  - Rule: `"Duplicate Shots/Same Day Rotavirus Rule #1a: ...evaluate CVX 74 as Invalid / DUPLICATE_SAME_DAY and other Rotavirus CVX as Valid"`

- **Logic**: While the Drools rules contain complex logic for same-day administration, the primary observed behavior is that CVX 74 doses are not preferred and often do not count towards series completion if another valid dose is present. Our implementation simplifies this by marking it `Accepted`, which has the same effect on the forecast.

- **Verification**: Test cases where a patient received the withdrawn CVX 74. The dose should not prevent the engine from recommending the full, modern series.
