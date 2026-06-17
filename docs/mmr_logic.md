# LAVA Forecaster: MMR Logic Guide

This document reverse-engineers the Measles, Mumps, and Rubella (MMR) forecasting and evaluation logic from the legacy Java ICE engine to ensure 100% behavioral parity.

## Key Concepts & Rules

### 1. Component-Based Counting & Completion

Unlike most other vaccine groups that count raw doses, the MMR group tracks the three antigens (Measles, Mumps, and Rubella) separately because a dose can target a subset of these diseases.
- **Rules**:
  - The series is only complete when the patient has received a sufficient number of valid components for *each* of the three diseases (Measles: 2, Mumps: 2, Rubella: 2).
  - A plain MMR vaccine (CVX 03) and MMRV (CVX 94) provide one of each antigen.
  - A Measles-only vaccine (CVX 05) provides only a Measles antigen.
  - An MR vaccine (CVX 04) provides one Measles and one Rubella antigen.
  - A Mumps-only vaccine (CVX 07) provides only a Mumps antigen.
  - A Rubella-only vaccine (CVX 06) provides only a Rubella antigen.
  - Rubella/Mumps vaccine (CVX 38) provides one Rubella and one Mumps antigen.
  - **CVX 168 Warning**: CVX 168 is *Influenza, trivalent, adjuvanted, preservative free* and does **not** count towards the MMR/mumps component count.

### 2. Same-Day Duplicate Invalidation

Java ICE has both MMR-specific and generic same-day duplicate rules:
- **Evaluation Order**:
  - MMRV (CVX 94) is evaluated first.
  - MMR (CVX 03) is evaluated second.
  - Other components (MR, Measles, Mumps, Rubella) are evaluated last.
- **Duplicate Invalidation**:
  - If MMRV (CVX 94) is valid, any other same-day MMR-family vaccine is evaluated as `Invalid (Duplicate Same Day)`.
  - If MMR (CVX 03) is valid, any other same-day MMR-family vaccine (except MMRV) is evaluated as `Invalid (Duplicate Same Day)`.
  - Otherwise, for other vaccines (like single/partial-antigens), the generic **Rule 5a** applies: duplicate same-day invalidation is only triggered if the vaccines target the **exact same set of diseases**.
  - As a result, disjoint same-day partial-antigen combinations (such as CVX 05 Measles + CVX 06 Rubella + CVX 07 Mumps) are **not duplicates** and are all evaluated as `Valid`.
  - **Engine Bypass**: Because these same-day non-duplicate doses target the same dose number but are administered on the same day, the engine must bypass the minimum interval check (treating 0-day intervals as valid) to prevent them from failing as `BelowMinimumInterval`.

### 3. Live Virus Conflict Spacing

Because MMR vaccines are live-virus vaccines, they must be spaced appropriately to prevent interference.
- **Same-Group Spacing (within MMR)**:
  - Normally, two live-virus vaccines in the same vaccine group require a minimum interval of **24 days**.
  - **MMR Custom Exception**: If the *current* vaccine under evaluation is CVX 94 (MMRV) and the *previous* vaccine evaluated in the MMR series is CVX 94 or CVX 03, the required spacing is **28 days**.
- **Different-Group Spacing (cross-group)**:
  - Live vaccines from different vaccine groups (e.g., Varicella CVX 21 and MMR CVX 03) require a minimum interval of **28 days**.
- **Universal Date Clamping**:
  - If a patient has received a live virus vaccine, the next recommendation for any live vaccine group (like MMR) must have its earliest and recommended dates clamped to at least **28 days** after the last live virus dose.

### 4. Early Dose 1 Exception ("Outside Routine Series")

- **Rule**: If CVX 03, 04, or 05 is administered at age $\ge \text{6 months} - \text{4 days}$ and $< \text{1 year} - \text{4 days}$ (absolute minimum age for Dose 1), it is evaluated as `Accepted` with the reason `Outside Routine Series`.
- **Note**: CVX 94 (MMRV) is explicitly excluded from this rule and will be evaluated as `Invalid (Below Minimum Age)` if given early.
- **Accepted vs. Valid**: Doses evaluated as `Accepted` are *not* valid. They do not increment disease-specific component counters and do not count toward series completion.

### 5. Adult Completion & Booster Rules

- **Dose 2 Age 19+**: If Dose 2 is administered at age $\ge 19\text{y}$, it is evaluated as `Accepted / BoosterDose` (Extra Dose) and marks the series complete.
- **Post-Evaluation Age 19+**: If the patient is $\ge 19\text{y}$ at evaluation time, the series is not complete, the effective dose number is 2 (meaning they have exactly 1 valid MMR dose), and no unevaluated MMR shots remain, the series is marked complete.
- **Early Accepted Dose Interaction**: An early accepted dose is not valid and does not increment the valid dose count. Therefore, a patient with only one early accepted dose has an effective dose number of 1 (so the age-19 completion rule does not fire). They must have at least one valid dose to trigger the age-19 completion rule.

### 6. Forecast Recommendation Reason

- **COMPLETE_HIGH_RISK**: When the MMR series is completed, Java ICE's recommendation engine outputs `NOT_RECOMMENDED` with a reason of `COMPLETE_HIGH_RISK`. LAVA normalizes this status to displayed `Complete` with a reason of `COMPLETE_HIGH_RISK`.
- **ConditionallyRecommended**: If a patient was born before Jan 1, 1957, they are presumed immune. If their series is incomplete, the forecast status is set to `ConditionallyRecommended` with the reason `CONDITIONAL` (and all forecast dates are cleared).
