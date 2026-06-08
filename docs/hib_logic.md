# LAVA Forecaster: HIB Logic Guide

This document reverse-engineers the Haemophilus Influenzae type b (HIB) forecasting logic from the Java ICE engine to ensure 100% behavioral parity.

## Key Concepts & Rules

### 1. Series Selection: 4-Dose vs. OMP (PedvaxHIB)

The central complexity of HIB is selecting between the standard 4-dose series and the shorter 2- or 3-dose series for the OMP (PedvaxHIB) vaccine.

- **Source of Truth**: `SeriesSelection.drl`
- **Rules**:
  - **Default**: The 4-dose series (`HIB_4_DOSE_SERIES`) is selected by default if no doses have been administered.
    - Rule: `"SeriesSelection.SelectByDefaultHIB_4_DOSE_SERIES"`
  - **OMP Switch**: The engine will select the `HIB_OMP_SERIES` if the first valid dose administered was an OMP-containing vaccine (PedvaxHIB, CVX 49).
    - Rule: `"SeriesSelection.SelectHIB_OMP_SERIESIfFirstDoseHibOMP..."`
  - **Completion Override**: Crucially, if the currently selected series is `NotComplete`, but the patient's history *does* satisfy the completion criteria for the *other* series, the engine will switch its selection to the completed series. This handles cases where a patient may have mixed-and-matched vaccine types.
    - Rule: `"SeriesSelection(Hib): If the Series that was selected is Not Complete but the other Series is Complete, select the other (completed) Series instead"`

- **Verification**: Test cases with different starting vaccines. A patient starting with PRP-T (e.g., ActHIB) should be on the 4-dose track. A patient starting with PRP-OMP (PedvaxHIB) should be on the OMP track. A patient with a mixed history who meets the 4-dose requirement should show as `Complete` even if the engine initially selected the OMP series.

### 2. Dose-Specific Intervals (Booster Dose)

The interval for the final "booster" dose of the 4-dose series is longer than the interval for the primary doses.

- **Source of Truth**: `Recommendation^Hib.dslr`
- **Rule**:
  - The interval for the first 3 doses is **4 weeks** (28 days).
  - The interval for the 4th dose (the booster) is **8 weeks** (56 days) from the previous dose.
    - Rule: `"Hib: Recommended Interval of 8w for Dose 4 of the 4-Dose Hib Series"`

- **Verification**: A test case where a patient has received 3 valid HIB doses. The `recommended_date` for the 4th dose should be 56 days after the 3rd dose, not 28.

### 3. Maximum Age for Routine Vaccination (5 Years)

Routine HIB vaccination is not recommended for healthy children 5 years of age or older.

- **Source of Truth**: `Evaluation^Hib.dslr`
- **Rule**: `"Evaluate Hib Shot As Accepted If Patient >=5 Yrs and Series Not Complete by 5yrs of Age"`
- **Logic**: Any HIB dose administered to a patient >= 5 years is evaluated as `Accepted`. Forecasts for this age group should be `NotRecommended` or `Complete`, but not `NotComplete`.
- **Verification**: A test case for an unvaccinated 6-year-old. The forecast should not recommend any HIB doses.
