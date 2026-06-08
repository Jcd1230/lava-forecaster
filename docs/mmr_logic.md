# LAVA Forecaster: MMR Logic Guide

This document reverse-engineers the Measles, Mumps, and Rubella (MMR) forecasting logic from the Java ICE engine to ensure 100% behavioral parity. The logic for MMR is unique as it is primarily "component-based."

## Key Concepts & Rules

### 1. Component-Based Completion

Unlike most other vaccine groups which count doses, the MMR group tracks the three antigens (Measles, Mumps, and Rubella) separately. The series is only complete when the patient has received a sufficient number of valid antigens for *each* of the three diseases.

- **Source of Truth**: This logic is implicit in the Drools rules and is handled by the core Java engine helpers (e.g., `ICELogicHelper.java`). The `.dslr` files act on the *outcome* of this component counting rather than performing the count themselves.
- **Logic**:
  - The engine maintains a running total of valid Measles, Mumps, and Rubella antigens.
  - A plain MMR vaccine (CVX 03) provides one of each.
  - A Measles-only vaccine (CVX 05) provides only a Measles antigen.
  - An MR vaccine provides one Measles and one Rubella antigen.
  - The standard series requires **2 valid doses of each antigen**.
- **Verification**: Test cases where a patient has a mixed history, such as one MMR dose and one separate Rubella dose. The forecast should indicate that Measles and Mumps are still needed, but Rubella is complete.

### 2. Series Completion Status ("Complete High Risk")

A patient who has successfully completed the MMR series is not considered simply "Complete." Due to the importance of ensuring immunity, they are perpetually considered to be at high risk for acquiring the diseases if their immunity wanes.

- **Source of Truth**: `Recommendation^MMR.dslr`
  - Comment: `(MMR should never have a recommendation of Not Recommended/Complete.)`
  - Rule: `"MMR: If a patient completed the series, recommendation is Not Recommended / COMPLETE_HIGH_RISK"`
- **Logic**: Once the engine determines that the patient has received at least two valid doses of all three antigens, the series forecast status must be set to `Complete` with a reason of `COMPLETE_HIGH_RISK`.
- **Verification**: Any test case where a patient has a history of two valid MMR doses. The final forecast status must be `Complete` and the reason `COMPLETE_HIGH_RISK`.

### 3. Born Before 1957 Presumptive Immunity

Individuals born before 1957 are generally presumed to be immune to measles and mumps.

- **Source of Truth**: `Recommendation^MMR.dslr`
  - Rule: `"MMR: Recommend conditional/high risk if born prior to 1/1/1957 and series not complete"`
- **Logic**: If a patient was born before 1957 and has an incomplete vaccination history, they should receive a `ConditionallyRecommended` forecast rather than a `NotComplete` one.
- **Verification**: A test case with a patient born in 1956 with no MMR vaccinations. The forecast should be `ConditionallyRecommended`.

### 4. Same-Day Vaccine Preference

When multiple MMR-containing vaccines are administered on the same day, the engine prioritizes the most comprehensive vaccine.

- **Source of Truth**: `DuplicateShotSameDay^MMR.drl`
- **Rules**:
  - MMRV (CVX 94) is preferred over plain MMR (CVX 03).
  - Plain MMR (CVX 03) is preferred over single-antigen vaccines.
- **Logic**: If two valid MMR-containing shots are given on the same day, the less comprehensive one is marked as `Invalid` with a `DUPLICATE_SAME_DAY` reason, ensuring that only the antigens from the preferred vaccine are counted.
- **Verification**: A test case where a patient receives both an MMR and a separate Mumps vaccine on the same day. The Mumps-only vaccine should be ignored.
