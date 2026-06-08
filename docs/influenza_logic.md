# LAVA Forecaster: Influenza Logic Guide

This document reverse-engineers the seasonal Influenza forecasting logic from the Java ICE engine to ensure 100% behavioral parity. The logic is heavily dependent on the concept of a "flu season" and the patient's age and vaccination history relative to those seasons.

## Key Concepts & Rules

### 1. Seasonal Boundaries

All influenza evaluations and forecasts are performed within the context of a defined influenza season, which runs from **July 1st to June 30th** of the following year.

- **Source of Truth**: `Evaluation^Influenza.dslr`
  - Rule: `"Influenza: Evaluate the Influenza Shot as Outside Flu Season if it does not fall within the Season Start and Stop Dates"`
- **Logic**: Any influenza vaccine administered outside of this July 1 - June 30 window is evaluated as `Invalid` with a reason of `OUTSIDE_FLU_VAC_SEASON`. The forecast for the *current* season always begins on July 1st.
- **Verification**: Test cases where a flu shot is given in mid-June. It should count for the season that is ending, not the one that is about to begin. A shot given in early July counts for the brand new season.

### 2. Series Selection: 1-Dose vs. 2-Dose Requirement

This is the core of the influenza logic. The engine must decide whether a patient needs one or two doses to be considered complete for the current season. This decision is based on the patient's age and their vaccination history in *prior* seasons.

- **Source of Truth**: `SeriesSelection.drl` (rules with `activation-group "InfluenzaSeries...SelectionCheck"`)
- **Logic**:
  - **If the patient is >= 9 years old** at the start of the current flu season, they **always** require only **1 dose**.
  - **If the patient is < 9 years old**, the engine counts the number of valid influenza doses they have received in all *prior* seasons.
    - If they have received **>= 2 valid doses** in prior seasons, they require only **1 dose** for the current season.
    - If they have received **< 2 valid doses** in prior seasons, they require **2 doses** for the current season, spaced at least 28 days apart.
- **Verification**:
  - A test case for an 8-year-old with no prior flu shots. They should be on the 2-dose series.
  - A test case for an 8-year-old who had two flu shots last year. They should be on the 1-dose series this year.
  - A test case for a 10-year-old with no prior history. They should be on the 1-dose series.

### 3. Cross-Season Interval

When a patient requires a dose for the *next* flu season, the forecast dates must still respect the standard 28-day interval from the last dose they received in the *current* season.

- **Source of Truth**: `Recommendation^Influenza.dslr`
  - Rule: `"Influenza(post-recommendation check): If the recommended date is after the season end date, recommend an interval of 4 weeks from the last shot..."`
- **Logic**: If a patient is complete for the current season (e.g., has received their one required dose in October), the forecast for the next season will start on July 1st of the next year. However, if they received a late second dose on June 15th, the forecast for the next season cannot start on July 1st; it must be pushed out to at least July 13th (June 15 + 28 days).
- **Verification**: A test case where a final dose for the current season is given in late June. The `earliest_date` for the *next* season's forecast should be 28 days after that dose, not July 1st.

### 4. Brand-Specific Age Limits

Certain influenza vaccine formulations have strict age limits that result in an `Invalid` evaluation if not met.

- **Source of Truth**: `Evaluation^Influenza.dslr`
- **Logic**:
  - **High-Dose (CVX 135, 197)**: Must be administered at age >= 65 years.
  - **LAIV/FluMist (CVX 161)**: Must be administered between ages 2 and 49 years, inclusive.
  - Other adjuvanted or cell-based vaccines have similar, though less commonly encountered, age restrictions.
- **Verification**: Test cases where a 5-year-old is given FluMist (should be valid) or a 55-year-old is given FluMist (should be invalid).
