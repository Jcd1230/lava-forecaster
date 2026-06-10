# DTP Java Logic Map

This document tracks the Java ICE sources that drive DTP behavior and the
current LAVA parity interpretation. Keep direct source facts separate from
behavior inferred from recorded Java output snapshots.

## Current Parity Snapshot

Latest committed checkpoint:

- Curated DTP cases: 312 / 317 passing.
- DTP fuzz subset: 3081 / 3359 passing.
- H1N1 fuzz subset: 2905 / 2905 passing.

Remaining DTP failures are mostly in three buckets:

- Forecast/date selection after child/adolescent/adult boundary rules.
- Adult or post-primary Td-family doses that are Valid vs Accepted depending
  on context.
- Same-day DTP duplicate handling, especially whether the same-day pair is in
  the primary series.

## Java Source Map

Primary current Java checkout:

- `/home/jason/projects/java-ice`

DTP data definitions:

- `opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/dtp_3_dose_series.yml`
- `opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/dtp_5_dose_series.yml`
- `opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/supportedVaccines.yml`
- `opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/supportedVaccineGroups.yml`
- `opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/supportedSupplementalEvaluationReasons.yml`
- `opencds-decision-support-service/src/main/resources/data/knowledgeModule/org.nyc.cir.ice/ice-supporting-data/supportedRecommendationReasons.yml`

DTP-specific Drools rules:

- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^DTP.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^DTP.dslr`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^DuplicateShotSameDay^DTP.drl`
- `opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^SeriesSelection.drl`

Generic mechanics that affect DTP:

- `opencds-decision-support-core/src/main/java/org/cdsframework/ice/service/TargetSeries.java`
- `opencds-decision-support-core/src/main/java/org/cdsframework/ice/service/TargetDose.java`
- `opencds-decision-support-core/src/main/java/org/cdsframework/ice/service/SeriesRules.java`
- `opencds-decision-support-core/src/main/java/org/cdsframework/ice/service/DoseRule.java`
- `opencds-decision-support-core/src/main/java/org/cdsframework/ice/service/PlanDefinitionSeriesDataConsumer.java`

## Series Data Facts

Both DTP series are data-defined PlanDefinitions:

- `DTP_3_DOSE_SERIES`
  - `numberOfDosesInSeries: 3`
  - `recurringDosesAfterSeriesComplete: true`
  - `doseNumberCalculationBasedOnDiseasesTargetedByVaccinesAdministered: false`
  - Dose 1 begins at age 7 years in the Java data. LAVA applies the same
    practical 4-day grace boundary used by Td/Tdap vaccine minimum-age rules.

- `DTP_5_DOSE_SERIES`
  - `numberOfDosesInSeries: 5`
  - `recurringDosesAfterSeriesComplete: true`
  - `doseNumberCalculationBasedOnDiseasesTargetedByVaccinesAdministered: false`
  - Routine child schedule starts at 6 weeks / 2 months with 5 target doses.

The `doseNumberCalculationBasedOnDiseasesTargetedByVaccinesAdministered: false`
flag is important for same-day and mixed-product behavior. Generic
`TargetSeries` dose counting treats the series as a single dose count rather
than maintaining separate D/T/P antigen counters.

## Product Classification

Pertussis-containing DTP-family examples:

- CVX 01, 20, 106, 107, 115, 120, 130, 132, 146, 170, 198 and combination
  products that include pertussis.

Diphtheria/tetanus without pertussis:

- CVX 09, 28, 113, 138, 139, 196.

Special product:

- CVX 195 is DT-IPV adsorbed, non-US. Java `supportedVaccines.yml` targets
  polio, diphtheria, and tetanus, but not pertussis. It is still present in
  DTP series vaccine lists. Recorded output shows it can be Valid, Accepted,
  or Invalid depending on series state and same-day/context, so do not model
  it as always a booster or always an extra dose.

## Series Selection

Java `SeriesSelection.drl` defines the DTP split:

- Select the 3-dose series if the patient is older than 7 years and there are
  no DTP target doses before the age-7 date.
- Otherwise select the 5-dose series.
- The 5-dose fallback fires if the 3-dose rule did not select.

LAVA currently uses an age-7 minus 4-day practical boundary for adult-series
eligibility because recorded Java outputs accept near-boundary Td/Tdap products
according to the vaccine minimum-age grace behavior.

## Evaluation Rules

`Evaluation^DTP.dslr` is the main source for DTP evaluation overrides.

Recurring Td after adolescent Tdap completion:

- Once the primary series is complete and `_ADOLESCENT_TDAP_COMPLETED` exists,
  any DTP shot can satisfy recurring Td age, interval, and extra-dose checks.
- The Java comment says the minimum interval is 0 days for recurring Td.
- This explains why post-completion doses can become Valid booster anchors.
- Recorded output shows `_ADOLESCENT_TDAP_COMPLETED` behavior is not equivalent
  to "the primary series contained pertussis." LAVA treats it as a valid
  post-primary pertussis-containing DTP-family dose at or after age 7. Until
  that gate exists, adult Td-only extra doses such as CVX 09, 138, 139, and
  196 are commonly Accepted rather than Valid.
- CVX 195 can become Valid through the same recurring-Td gate after a
  post-primary pertussis anchor, but it can still be Invalid in in-series
  adult-dose slots when interval/series-selection rules fail.
- Snapshot-derived nuance: in the 3-dose adult series, a first post-primary
  adult Td product CVX 138 or 139 can be Valid even before the post-primary
  pertussis gate exists. Later Td-only products remain Accepted unless a valid
  post-primary pertussis-containing dose has occurred. CVX 09 and CVX 196 have
  not behaved like this first-anchor product in the observed cases.

Adolescent Tdap:

- If adolescent Tdap is needed and a pertussis/diphtheria/tetanus shot is given
  from age 7 through before age 10, the first such dose may be Valid.
- Further qualifying adolescent Tdap shots in that 7-to-10 window are Accepted
  as extra doses.
- At age 10 or later, a qualifying pertussis/diphtheria/tetanus dose is Valid
  and creates `_ADOLESCENT_TDAP_COMPLETED`.
- If the interval between pertussis shots is less than 28 days, Java marks the
  dose Accepted / Extra Dose in the adolescent Tdap override path.

Pertussis facts:

- A dose of pertussis is inserted for valid primary-series D/T/P shots.
- A dose of pertussis is also inserted for a primary-series shot Invalid with
  reason `D_AND_T_INVALID/P_VALID`.
- A valid D/T/P dose at or after age 7 in a complete series also creates a dose
  of pertussis.

Td/Tdap minimum-age ignore behavior:

- CVX 09, 113, 138, 139, and 196 below their valid minimum age are marked
  ignored when calculating intervals and forecast target dose dates.
- CVX 115 as target dose 1, 2, or 3 in the 5-dose series and below its valid
  minimum age is also marked ignored.
- In Java output this "ignored" path maps to LAVA `Accepted` with an ignored /
  outside-routine effect rather than a normal Valid dose.

5-dose completion exceptions:

- Exception 1: complete with 3 doses if age is at least 7, dose 1 was at or
  after 12 months, and at least one later valid dose was at or after 4 years.
- Exception 1 can also skip an in-flight target dose 3 to dose 4.
- Exception 2: complete with 4 doses if dose 4 was at or after age 4 and the
  interval from dose 3 to dose 4 was at least 6 months minus 4 days.

3-dose adult-series exception:

- If the 3-dose series appears complete but none of the doses contain
  pertussis, Java marks the series not complete.
- A later pertussis-containing shot beyond target dose 3 can be Valid and mark
  the 3-dose series complete.

## Recommendation and Forecast Rules

`Recommendation^DTP.dslr` is the main source for forecast overrides.

Incomplete series:

- If patient age and recommendation date are both before age 7, recommend DTaP
  NOS (CVX 107).
- If patient is before age 7 but recommendation date is at or after age 7,
  recommend Tdap (CVX 115).
- If patient is at or after age 7 and the series is incomplete:
  - if a pertussis dose exists at or after age 7, forecast Td at age 7;
  - otherwise forecast Tdap at age 7.

Completed primary series with adolescent Tdap still needed:

- If a pertussis-containing dose occurred from age 7 through before age 10,
  recommend Tdap at age 11, overdue at age 13 years plus 4 weeks.
- For 5-dose completion, Java checks adolescent Tdap exception A and B:
  - A: no pertussis dose at or after age 4 minus 4 days.
  - B: fewer than 4 pertussis doses before age 7.
- If an exception occurred, recommend adolescent Tdap at age 7.
- Interval overlay:
  - 0 days from the latest non-pertussis shot.
  - 6 months from the latest pertussis shot.

Completed adolescent Tdap:

- If `_ADOLESCENT_TDAP_COMPLETED` exists, forecast the next booster from the
  most recent shot in the series:
  - earliest: last shot plus 5 years;
  - recommended: last shot plus 10 years;
  - overdue: last shot plus 10 years plus 4 weeks.
- The recommendation reason is "Administer Tdap or Td vaccine".

Six-by-seven:

- If the patient is before age 7, the 5-dose series is incomplete, and the
  number of administered shots excluding duplicate same-day shots is at least
  6, Java forecasts the next dose at age 7.
- This rule counts Java target doses, not simply valid doses. Recorded output
  shows that pertussis-containing same-day duplicates can inflate LAVA's raw
  history count and over-trigger the rule. LAVA now uses final evaluations in
  the DTP forecast hook and excludes same-day duplicate evaluations only when
  the duplicate product contains pertussis.
- This is still an approximation of Java behavior. Three fuzz counterexamples
  (`fuzz_fail_dtp_20260608_158219`, `fuzz_fail_dtp_20260608_177129`,
  `fuzz_fail_dtp_20260608_215276`) suggest Java may count some same-day
  pertussis-containing invalid doses when they are not treated as duplicate
  shots by its DTP same-day rule path.

3-dose no-pertussis recommendation:

- If the 3-dose series has at least 3 doses and no pertussis facts, Java
  recommends Tdap immediately after the most recent dose.

## Same-Day DTP Rules

`DuplicateShotSameDay^DTP.drl` is the main source for DTP same-day behavior.

Evaluation order:

- If one same-day DTP shot contains pertussis and the other does not, Java
  evaluates the pertussis-containing shot first.

Primary-series same-day behavior:

- If both same-day shots are primary-series doses and one contains pertussis
  while the other does not, the non-pertussis shot becomes Invalid with
  `DUPLICATE_SAME_DAY`.
- If both same-day primary-series shots either both contain pertussis or both
  omit pertussis, Java keeps the first processed shot Valid and invalidates the
  second as duplicate.

Non-primary same-day behavior:

- If the same-day pair is not both primary-series doses, Java generally marks
  the duplicate same-day check complete without invalidating either dose.
- This matches the remaining fuzz shape where post-primary same-day Td-family
  doses are often Accepted or Valid rather than duplicate-invalid.

NOS-specific behavior:

- There are separate rules for NOS formulations where one product contains
  pertussis and the other does not.
- If both are primary-series doses, the non-pertussis NOS shot is invalidated.
- If not both are primary-series doses, both can remain usable.

## LAVA Mapping Touchpoints

Current Rust implementation areas:

- `src/rules/dtp/schedules.rs`
  - static DTP series definitions and age/interval boundaries.
- `src/rules/dtp/overrides.rs`
  - DTP group selection, evaluation overrides, completion exceptions,
    adolescent/booster forecast hooks.
- `src/engine.rs`
  - generic same-day duplicate sorting and duplicate suppression.

The clean mapping target is:

- Keep YAML-derived schedule facts in `schedules.rs`.
- Keep DTP clinical/source-specific exceptions in `overrides.rs`.
- Keep only generic same-day mechanics in `engine.rs`; DTP-specific primary vs
  non-primary handling should be exposed through group hooks where possible.

## Open Parity Questions

These are not settled enough for blanket rules:

- CVX 195:
  - It is non-pertussis DT-IPV but can be Valid, Accepted, or Invalid in
    recorded Java output.
  - Compare at least one expected Valid and one expected Accepted/Invalid case
    before changing CVX 195 handling.

- Adult Td-family Valid vs Accepted:
  - Java source says recurring Td is Valid after adolescent Tdap completion,
    but recorded output has contexts where CVX 09/113/138/139/196 are Accepted
    rather than Valid.
  - The likely distinction is whether the dose is satisfying the recurring Td
    slot, an adolescent/extra-dose slot, or a post-primary same-day extra.

- Six-by-seven:
  - The source rule is broad (`administered shots excluding duplicate same-day
    shots >= 6`), but previous broad LAVA changes regressed many fuzz cases.
  - Reconcile Java's target-dose count, ignored-shot treatment, and duplicate
    same-day exclusion before changing this again.

- Same-day ordering:
  - Java evaluates pertussis-containing DTP products first, but later
    duplicate resolution depends on whether both shots are primary-series
    doses.
  - LAVA currently has some ordering behavior in `engine.rs`; final parity
    likely needs DTP primary/non-primary context, not just product priority.
  - This also affects forecast-only six-by-seven behavior because Java's
    six-by-seven count excludes only shots that Java actually labels duplicate
    same-day.

## Representative Cases To Preserve

Use these as guardrails while changing rules:

- `fuzz_fail_dtp_20260608_1869`
  - CVX 195 expected Valid and forecast should anchor from it.
- `fuzz_fail_dtp_20260608_7647`
  - Same-day CVX 113 and CVX 195 after prior history; expected accepted-like
    post-primary behavior, not simple duplicate invalidation.
- `fuzz_fail_dtp_20260608_15146`
  - Same-day adult Td-family extras after a valid same-day Td-family dose.
- `fuzz_fail_dtp_1780795905_4324`
  - Same-day primary-series context where DT/Td can win over a pertussis product
    in the final expected output.
- Forecast-only examples: `fuzz_fail_dtp_20260608_2619`,
  `fuzz_fail_dtp_20260608_6278`, `fuzz_fail_dtp_20260608_6389`.
  These protect against over-applying six-by-seven or age-7 deferral logic.

## Investigation Checklist

For each new DTP rule change:

1. Link the Rust behavior to one of the Java files above or mark it explicitly
   as inferred from recorded output.
2. Test at least one expected Valid and one expected Accepted/Invalid
   counterexample for the same product family.
3. Run representative cases first with `--case <name> -v --trace`.
4. Then run:
   - `cargo run --release --bin test_runner -- --run tests/cases --group DTP`
   - `cargo run --release --bin test_runner -- --run tests/fuzz-100k-20260608.ltp --group DTP`
   - `cargo run --release --bin test_runner -- --run tests/fuzz-100k-20260608.ltp --group H1N1`
