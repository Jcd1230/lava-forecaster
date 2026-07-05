# COVID Java ICE rule map

This document maps COVID-19 behavior in Java ICE to the source files and line ranges that should drive Rust LAVA parity work. It is intentionally compact: enough detail to implement from Java evidence without copying the Drools files verbatim.

## Core Java source files

- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^SeriesSelection.drl`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^COVID19.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^COVID19^Dec2020Season.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^COVID19^Sep2023Season.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^COVID19^Aug2025Season.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^COVID19.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^COVID19^Dec2020Season.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^COVID19^Sep2023Season.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^COVID19^Aug2025Season.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^DuplicateShotSameDay^COVID19.drl`

Supporting data to cross-check:

- `opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/Series/covid_19_*.yml`
- `opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/supportedVaccines.yml`

## Known season boundaries

- Dec 2020 season starts `2020-12-14`.
- Sep 2023 season starts `2023-09-12`.
- Aug 2024 season starts `2024-08-22`.
- Aug 2025 season starts `2025-08-27`.

Rust currently encodes these in `src/rules/covid19/overrides.rs`.

## Aug 2025 series selection

Java computes the age-2 and age-65 dates before selecting the Aug 2025 series. Source: `SeriesSelection.drl:868-874`.

- Select `COVID_19_AUG_2025_LT_2_SERIES` if the patient is under age 2 at evaluation, or if the patient is already age 2+ but has an in-season COVID shot administered before age 2. Source: `SeriesSelection.drl:879-891`.
- Select `COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES` if there are no in-season COVID target doses and the patient is age 2 through 64 at evaluation. Source: `SeriesSelection.drl:904-912`.
- Select `COVID_19_AUG_2025_GTE_65_SERIES` if there are no in-season COVID target doses and the patient is age 65+ at evaluation. Source: `SeriesSelection.drl:922-930`.
- If in-season shots exist, select 2-64 when target dose 1 was administered at age 2+ and before age 65. Source: `SeriesSelection.drl:940-949`.
- If in-season shots exist, select 65+ when target dose 1 was administered at age 65+. Source: `SeriesSelection.drl:959-965`.
- Select or switch to 65+ if dose 1 was given before age 65 but the patient turns 65 within 12 months of the season start. Source: `SeriesSelection.drl:976-986`; evaluation switch rule: `Evaluation^COVID19^Aug2025Season.dslr:205-235`.

## Aug 2025 2-64 / 65+ evaluation and forecast

- Target dose 1 forecast uses the most recent non-ignored COVID shot + 8 weeks when there is at least one prior COVID shot. Source: `Recommendation^COVID19^Aug2025Season.dslr:60-92`.
- For target dose 1 evaluation, prior CVX 313 followed by current CVX 313 uses a 17-day absolute minimum interval. Source: `Evaluation^COVID19^Aug2025Season.dslr:31-94`.
- For target dose 1 evaluation, non-313 prior shots use an 8-weeks-minus-4-days absolute minimum interval. Source: `Evaluation^COVID19^Aug2025Season.dslr:97-144`.
- Prior CVX 313 followed by a non-313 target dose 1 also uses 8-weeks-minus-4-days. Source: `Evaluation^COVID19^Aug2025Season.dslr:146-195`.
- 65+ target dose 2 forecast includes guidance around 6-month recommended interval and 8-12 week minimum/product interval text. Source: `Recommendation^COVID19^Aug2025Season.dslr:221-245`.

## Aug 2025 LT2 evaluation and forecast

- Forecasts in the LT2 series should recommend CVX 311. Source: `Recommendation^COVID19^Aug2025Season.dslr:255-270`.
- If a prior shot was below the LT2 absolute minimum age, target dose 1 forecast is prior shot + 28 days. Source: `Recommendation^COVID19^Aug2025Season.dslr:273-305`.
- With no doses but one or more prior invalid shots before season start, target dose 1 interval depends on product family. Source: `Recommendation^COVID19^Aug2025Season.dslr:308-338`.
- Prior invalid Pfizer / Novavax / unspecified family uses 21 days. Listed CVX: 208, 217, 218, 219, 300, 301, 302, 308, 309, 310, 211, 313, 213. Source: `Recommendation^COVID19^Aug2025Season.dslr:340-363`.
- Other prior invalid COVID shots use 28 days. Source: `Recommendation^COVID19^Aug2025Season.dslr:365-387`.
- One prior non-Moderna pre-season dose from CVX 213, 308, 309, 310, or 313 creates a separate target-dose-1 path. Source: `Recommendation^COVID19^Aug2025Season.dslr:389-423`.
- In that one-prior-dose path, prior Pfizer / Novavax / unspecified-family shots use a 21-day interval and can set an 8-week overdue date. Source: `Recommendation^COVID19^Aug2025Season.dslr:425-463`.
- In that one-prior-dose path, prior other COVID shots use a 28-day interval and can set an 8-week overdue date. Source: `Recommendation^COVID19^Aug2025Season.dslr:465-505`.
- Exactly one prior valid Moderna CVX 311 or 312 before season start can skip target dose 1 and move to target dose 2 if no other selected valid pre-season doses exist. Source: `Evaluation^COVID19^Aug2025Season.dslr:542-575`.
- When evaluating a current-season shot under that condition, Java can set the current shot to dose 2. Source: `Evaluation^COVID19^Aug2025Season.dslr:579-609`.
- If the patient has at least two valid pre-season doses from CVX 213, 308, 309, 310, 311, 312, or 313, target dose 1 is skipped and target dose 2 uses an 8-week interval. Source: `Evaluation^COVID19^Aug2025Season.dslr:513-539`.

## Sep 2023 / Aug 2024 series-selection themes

- Java has pre-Aug-2025 completed-vs-incomplete selection rules. Source: `SeriesSelection.drl:997-1039`.
- Java computes age 5 and age 12 years minus 4 days for Sep2023/Aug2024 routing. Source: `SeriesSelection.drl:1055-1072`.
- Mixed Product <5 is selected when valid current-season doses before age 5 mix product families: Moderna CVX 311/312, Pfizer CVX 308/309/310, Novavax CVX 211/313, or unspecified. Source: `SeriesSelection.drl:1077-1097`.
- Mixed Product <5 is also selected if the patient is under age 5 and there are no current-season valid doses. Source: `SeriesSelection.drl:1109-1117`.
- Pfizer <5 and Moderna <5 series selection use product-family homogeneity across current and prior season doses. Source: `SeriesSelection.drl:1289-1335`.

## CVX 308 Sep2023/Aug2024 under-5 exception

- In the Sep2023/Aug2024 Pfizer <5 series, CVX 308 has no absolute maximum age for doses 2 and 3. Source: `Evaluation^COVID19^Sep2023Season.dslr:455-478`.
- The same CVX 308 exception applies to the Sep2023/Aug2024 Mixed Product <5 series. Source: `Evaluation^COVID19^Sep2023Season.dslr:641-664`.

## Implementation guidance

Prefer implementing from a season/series/product table rather than from forecast date deltas alone. The remaining failures are likely coupled across:

- selected season,
- selected series,
- product family,
- age band,
- current vs prior season,
- target dose number,
- valid vs invalid vs accepted/ignored status,
- target dose skip/conversion rules,
- evaluation intervals vs recommendation intervals.
