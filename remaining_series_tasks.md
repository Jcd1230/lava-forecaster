# Task List: Remaining Vaccine Groups & Series Implementation

This task list tracks the vaccine groups and series from the legacy Drools-based Java ICE engine that have been ported to the high-performance Rust LAVA Forecaster.

To implement a group, follow the guidelines in the [Onboarding & Implementation Guide](file:///home/jason/projects/ice/agent_onboarding_guide.md).

---

## 1. DTaP / Tdap / Td / DTP Series
- [x] Implement `DTP3DoseSeries.yml` (Adult series)
- [x] Implement `DTP5DoseSeries.yml` (Child series)
- [x] Implement same-day priority sorting & custom evaluation logic

## 2. Hepatitis B (`HEP_B`)
- [x] Implement `HepB3DoseChildAdolescentSeries.yml`
- [x] Implement `HepB4DoseChildAdolescentSeries.yml`
- [x] Implement `HepB3DoseTwinrixSeries.yml`
- [x] Implement `HepB4DoseTwinrixSeries.yml`
- [x] Implement `HepBAdult2DoseSeries.yml` (Heplisav-B CVX 189)
- [x] Implement `HepBAdult3DoseSeries.yml`
- [x] Implement custom switch/selection logic between Twinrix, child/adolescent, and adult series

## 3. HPV (Human Papillomavirus)
- [x] Implement `HPV2DoseSeries.yml`
- [x] Implement `HPV3DoseSeries.yml`
- [x] Implement custom series selection based on age of initiation (Dose 1 age < 15y vs >= 15y)

## 4. Hib (Haemophilus influenzae type b)
- [x] Implement `Hib4DoseSeries.yml` (ActHIB, Hiberix, Pentacel, etc.)
- [x] Implement `HibOMPSeries.yml` (PedvaxHIB 3-dose series)
- [x] Implement custom series switching depending on whether OMP or non-OMP vaccines are administered

## 5. Pneumococcal (PCV / PPSV)
- [x] Implement `PneumococcalSeries.yml` (PCV13, PCV15, PCV20, PPSV23)
- [x] Implement complex risk-group and sequence-based evaluation rules

## 6. Meningococcal Conjugate (MCV4)
- [x] Implement `MCV42DoseSeries.yml` (Menactra, Menveo, MenQuadfi)

---

## 7. Meningococcal B (`MENB`)
*Java Concept:* `MENINGOCOCCAL_B` | *Focus Code:* `835`
- [x] Implement `MenB4C2DoseSeries.yml` & `MenB4C3DoseSeries.yml` (Bexsero)
- [x] Implement `MenBFHbp2DoseSeries.yml` & `MenBFHbp3DoseSeries.yml` (Trumenba)
- [x] Implement brand consistency & auto-switching logic:
  - If a Bexsero (CVX 163) dose is given in the FHbp series, switch to the 4C 2-dose series.
  - If a Trumenba (CVX 162/316) dose is given in the 4C series, switch to the FHbp 2-dose series.
- [x] Implement age floor override: CVX 162/163 given at age >= 10y but below series absolute min age are **Accepted** (not Invalid).
- [x] Implement date-dependent duplicate-same-day preference rules (before vs on/after 10/25/2024).
- [x] Modify forecast status to `ConditionallyRecommended / CLINICAL_PATIENT_DISCRETION` if patient is >= 10y, series is incomplete, and they have >= 1 valid dose.

## 8. Rotavirus (`ROTAVIRUS`)
*Java Concept:* `ROTAVIRUS` | *Focus Code:* `820`
- [x] Implement `Rotavirus2DoseSeries.yml` (Rotarix)
- [x] Implement `Rotavirus3DoseSeries.yml` (RotaTeq)
- [x] Enforce strict age clamps:
  - Any dose given at age >= 8 months is evaluated as **Invalid / TOO_OLD**.
  - Forecast recommendation status is forced to `NotRecommended / TOO_OLD` once patient reaches age 8 months.
- [x] Implement vaccine-counting override: invalid unspecified formulation still counts for dose numbering.
- [x] Implement duplicate-same-day CVX preferences split by date 1/1/2000 (withdrawn CVX 74 vs CVX 119).

## 9. Seasonal Influenza (`INFLUENZA`)
*Java Concept:* `INFLUENZA` | *Focus Code:* `800`
- [x] Implement `Influenza1DoseSeries.yml`
- [x] Implement `Influenza2DoseSeries.yml`
- [x] Implement `Influenza2DoseDefaultSeries.yml`
- [x] Implement flu season boundaries: doses administered outside season dates are evaluated as **Invalid / OUTSIDE_FLU_SEASON**.
- [x] Implement 24-day override: if dose is >= 24 days after a valid dose in the *prior* season, it satisfies the interval requirement.
- [x] Implement age-based rules: children < 9y with 0 prior-season valid doses require 2 doses in the current season; children < 9y with >= 1 prior valid dose (and all individuals >= 9y) require only 1 dose.
- [x] Suppress "Insufficient Antigen" reasons for patients >= 9y.

## 10. COVID-19 (`COVID19`)
*Java Concept:* `COVID_19` | *Focus Code:* `850`
> [!NOTE]
> This is a highly complex, multi-session effort. It involves seasonal agenda group routing.
- [x] Implement September 2023 season rules (age < 5y vs >= 5y).
- [x] Implement August 2025 season rules (age < 2y, 2y-64y, >= 65y).
- [x] Implement CVX-specific minimum interval overrides (e.g. CVX 313 -> CVX 313: 17 days absolute min; non-313 -> any COVID: 52 days).
- [x] Implement Moderna dose-skip logic for infants (<2y series) with pre-season doses.
- [x] Implement age-based series auto-switching (switch to >=65y 2-dose series if patient turns 65 within 12 months of season start).
- [x] Enforce complex duplicate-same-day preference rules (Janssen order, Moderna preferred over Pfizer, approved vs WHO-only, etc.).
- [x] Overdue/Forecast date adjustments based on vaccine brand and interval-dependent supplemental text.
- [x] Map series completion to `Not Recommended / COMPLETE_HIGH_RISK`.

## 11. Mpox (`MPOX`)
*Java Concept:* `MPOX` | *Focus Code:* `860`
- [x] Implement `Mpox1DoseSeries.yml` & `Mpox2DoseSeries.yml`
- [x] Implement interval warning: dose 2 given < 28 days after dose 1 is **Accepted** but with supplemental text warnings (not Invalid).
- [x] Exempt Mpox (CVX 206) from standard live-virus inter-group interval checks.
- [x] Support booster dose evaluation (dose 3 evaluated as booster if patient has 2 valid doses).
- [x] Enforce duplicate-same-day precedence (CVX 206/75/105 > CVX 325; Valid > Accepted).
- [x] Map incomplete series to `Conditionally Recommended / HIGH_RISK` and complete to `Not Recommended / COMPLETE_HIGH_RISK`.

## 12. RSV (Respiratory Syncytial Virus - `RSV`)
*Java Concept:* `RSV` | *Focus Code:* `875`
- [x] Implement `RSVAdultSeries.yml` (Arexvy, Abrysvo)
- [x] Implement `RSVInfantSeries.yml` (Beyfortus/Nirsevimab, Synagis/Palivizumab)
- [x] Enforce vaccine availability date limits (adult CVX 303/304/305/314/326, infant CVX 304/306/307/315, and CVX 332). Doses given before the legacy availability dates are invalid.
- [x] Implement infant season-aligned dosing and forecasting, including the under-8-month versus 8-19-month recommendation split.
- [x] Adjust adult RSV recommendations: ages 50-74 -> `ConditionallyRecommended`, >= 75y -> standard completion/date forecasting.

## 13. JEV (Japanese Encephalitis - `JEV`)
*Java Concept:* `JAPANESE_ENCEPHALITIS` | *Focus Code:* `902`
- [x] Implement `JEVCRisk2DoseSeries.yml` & `JEVCRisk2DoseAcceleratedSeries.yml`
- [x] Support 18-65y accelerated series: allows 7-day interval between dose 1 and 2 (normally 28 days).
- [x] Forecast recommendation status is `Not Recommended / TOO_OLD` for accelerated series if patient is >= 66y.

## 14. Cholera (`CHOLERA`)
*Java Concept:* `CHOLERA` | *Focus Code:* `901`
- [x] Implement `Cholera1DoseRiskSeries.yml`
- [x] Unconditionally attach warning supplemental text to all forecasts (except COMPLETE).
- [x] Implement three-tier age-gated recommendations:
  - Age < 2y -> `Not Recommended`
  - Age 2y-64y -> `Conditionally Recommended / HIGH_RISK`
  - Age >= 65y -> `Not Recommended / TOO_OLD`

## 15. Typhoid (`TYPHOID`)
*Java Concept:* `TYPHOID` | *Focus Code:* `904`
- [x] Implement `TyphoidRiskSeries.yml`
- [x] Unconditionally attach warning supplemental text to all forecasts (including COMPLETE).
- [x] Implement conditional status overrides:
  - Age < 2y -> `Not Recommended`
  - Age >= 2y and not complete -> `Conditionally Recommended / HIGH_RISK`
  - Series complete -> `Conditionally Recommended / COMPLETE_HIGH_RISK` (due to travel re-exposure risks)

## 16. Yellow Fever (`YELLOW_FEVER`)
*Java Concept:* `YELLOW_FEVER` | *Focus Code:* `905`
- [x] Implement `YellowFeverRiskSeries.yml`
- [x] Attach live virus warning supplemental text to all recommendations.
- [x] Implement four-tier age-gated recommendations:
  - Age < 6 months -> `Not Recommended`
  - Age 6m-8m -> `Conditionally Recommended / BELOW_REC_AGE_SERIES + HIGH_RISK`
  - Age >= 9 months -> `Conditionally Recommended / HIGH_RISK`
  - Series complete -> `Conditionally Recommended / COMPLETE_HIGH_RISK`
- [x] Implement cross-group live-virus override: if YF series is completed and YF dose was < 30 days before another live vaccine's earliest forecast date, push that other forecast date to YF date + 30 days.

## 17. Historical H1N1 Influenza (`H1N1`)
*Java Concept:* `INFLUENZA_H1N1` | *Focus Code:* `890`
- [x] Implement `H1N11DoseSeries.yml` & `H1N12DoseSeries.yml`
- [x] Check H1N1 season boundaries: doses outside dates are evaluated as **Invalid / OUTSIDE_FLU_SEASON**.
- [x] Ensure H1N1 does not generate recommendations or forecasts (historical database tracking only).
