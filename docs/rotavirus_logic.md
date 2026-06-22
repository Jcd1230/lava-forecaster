# LAVA Forecaster: Rotavirus Logic Guide

This document reverse-engineers the Rotavirus forecasting logic from the Java ICE engine to ensure 100% behavioral parity.

## Key Concepts & Rules

### 1. Strict Maximum Age (8 Months)

The most critical and unique aspect of the Rotavirus series is the strict age limit. The series cannot be started or continued in infants who are too old, due to safety concerns identified in clinical trials.

- **Source of Truth**: `Evaluation^Rotavirus.dslr` and `Recommendation^Rotavirus.dslr`
  - Rule: `"Rotavirus: Mark Dose as Accepted if Patient Greater Than 8 Months of Age"`
  - Rule: `"Rotavirus: Series is Complete if Over 8 Years of Age..."` (Note: the rule says 8 years, but the implementation and ACIP guidelines use 8 months, 0 days)

- **Logic**:
  - A Rotavirus vaccine administered when the patient is greater than 8 months old is evaluated as `Accepted`.
  - Rotavirus forecast is changed to `NotRecommended` when the patient is currently greater than 8 months old or will be greater than 8 months old as of the routine recommendation date.
  - Rotavirus initiation is separately blocked at age `>= 105d` when there are no valid prior Rotavirus doses.

- **Verification**: Test cases where a patient is evaluated after their 8-month birthday. The forecast should be `NotRecommended`, and any doses administered after this date should be marked `Accepted`.

### 2. Series Selection (2-Dose vs. 3-Dose)

The engine must choose between the 2-dose (Rotarix) and 3-dose (RotaTeq) series. This choice is determined by the vaccine type (CVX) of the first dose administered.

- **Source of Truth**: `SeriesSelection.drl`
  - Rule: `"SeriesSelection.SelectROTAVIRUS_2_DOSE_SERIESIf1DoseRV1"`
  - Rule: `"SeriesSelection.SelectROTAVIRUS_3_DOSE_SERIESIfCertainVaccinesAdministered"`

- **Logic**:
  - If a valid 3-dose product dose is present first (CVX 116, 122, or 74), the 3-dose series is selected.
  - If dose 1 is valid CVX 119 (Rotarix) and no earlier valid 3-dose product selected the 3-dose path, the 2-dose series is selected.
  - If the RV1 path already has two valid CVX 119 doses, later Rotavirus products are accepted as post-completion doses rather than switching the output to the 3-dose series.
  - If no doses have been administered, the 3-dose series is chosen by default.

- **Verification**: Test cases where the first dose is Rotarix (CVX 119). The engine should forecast for a total of 2 doses. Conversely, if the first dose is RotaTeq (CVX 116), it should forecast for 3.

### 3. Withdrawn Vaccine (RotaShield)

The original Rotavirus vaccine, RotaShield (CVX 74), was withdrawn from the market. ICE contains special logic to handle it, preferring any other valid Rotavirus vaccine administered on the same day.

- **Source of Truth**: `DuplicateShotSameDay^Rotavirus.drl`
  - Rule: `"Duplicate Shots/Same Day Rotavirus Rule #1a: ...evaluate CVX 74 as Invalid / DUPLICATE_SAME_DAY and other Rotavirus CVX as Valid"`

- **Logic**: ICE evaluates Rotavirus same-day product preference after initial dose evaluation. On or after 2000-01-01, CVX 74 is invalidated as duplicate same-day when paired with another valid non-NOS Rotavirus CVX, and CVX 119 is invalidated when paired with another valid non-NOS Rotavirus CVX and no valid CVX 74 is present. Before 2000-01-01, CVX 74 is preferred over other non-NOS Rotavirus products, while CVX 119 remains non-preferred when paired with another non-NOS product and no CVX 74 is present.

  CVX 122 is Rotavirus NOS. The Rotavirus product-preference rules require the preferred "other" product to be non-NOS, and the generic ICE same-day NOS rule marks NOS duplicate when a valid non-NOS same-day product exists. LAVA mirrors this through same-day ordering: evaluate the preferred non-NOS product first and let the later same-day dose become the duplicate.

- **Verification**: Test cases where CVX 74 is paired with CVX 119 across the 2000-01-01 policy boundary, plus a same-day CVX 119/CVX 122 NOS case, document product-preference behavior.

### 4. RV1 Product Preference and Mixed Products

Java ICE does not let a later RV5/NOS/RotaShield dose pull a patient who has completed the RV1 path onto the 3-dose series. When the 2-dose candidate has two valid CVX 119 doses, later Rotavirus products are reported as `Accepted` with the next output dose number rather than counted as valid completion of the 3-dose candidate. Mixed products before RV1 completion still use the 3-dose path: the CDSi mixed-product fixtures with CVX 119 and CVX 116 expect the CVX 116 dose to be `Valid` and forecast a third dose.

- **Source of Truth**: expected snapshots in `tests/cases/passed/ROTAVIRUS/cdsi_2013_0776_rotateq_at_2_months_rotarix_at_4_mo.json`, `tests/cases/passed/ROTAVIRUS/cdsi_2013_0777_rotatrix_at_2_mo_rotateq_at_4_mo.json`, and promoted fuzz snapshots such as `tests/cases/passed/ROTAVIRUS/rotavirus_completed_rv1_accepts_later_products.json` and `tests/cases/passed/ROTAVIRUS/rotavirus_incomplete_rv1_later_products_not_complete.json`.
- **LAVA mapping**: group selection should prefer the 2-dose series only after the 2-dose candidate has two valid RV1 doses, or when RV1 is the only valid product path. A valid 3-dose product before RV1 completion should select the 3-dose candidate.
- **Completion detail**: ROTAVIRUS completion should require a truly `Valid` dose at the required target number. Accepted above-age or post-completion doses should not by themselves convert an incomplete series to `Complete`.
