# DTP Java Logic Map

This document tracks the Java ICE sources that drive DTP behavior and the
current LAVA parity interpretation. Keep direct source facts separate from
behavior inferred from recorded Java output snapshots.

## Current Parity Snapshot

Latest verified checkpoint after the prior-pertussis adolescent Tdap refinement:

- DTP fuzz subset: 3181 / 3359 passing.
- H1N1 fuzz subset: 2905 / 2905 passing.
- `tests/cases --group DTP` matched 0 cases in the current workspace layout,
  so curated DTP case coverage needs a fixture-layout check before using it as
  a signal.

Remaining DTP fuzz failures are mostly in three buckets:

- Post-primary or adult extra doses where Java distinguishes Valid, Accepted,
  and Invalid by target-dose state rather than only product family.
- Same-day DTP handling where Java sometimes lets both products remain usable
  when the pair is not both primary-series target doses.
- Forecast/date selection after an extra dose is classified differently from
  Rust's current valid-dose count.

Latest working log for handoff:

- `tests/relative/tmp/dtp_run_after_dtp_prior_pertussis_ge7_guard.txt`

Bucket summary from that log:

- Evaluation-only failures: 131 cases.
- Forecast-only failures: 13 cases.
- Combined evaluation and forecast failures: 34 cases.
- Failed cases with same-day DTP doses: 50 cases.
- Largest status transitions:
  - Rust Valid, Java Invalid: 60.
  - Rust Valid, Java Accepted: 53.
  - Rust Accepted, Java Valid: 45.
  - Rust Invalid, Java Valid: 22.
- Largest product transitions:
  - CVX 113 Valid -> Accepted: 17.
  - CVX 138 Valid -> Accepted: 14.
  - CVX 139 Valid -> Accepted: 9.
  - CVX 195 Invalid -> Valid: 7.
  - CVX 107 Valid -> Invalid: 7.
  - CVX 20 Valid -> Invalid: 6.
  - CVX 28 Valid -> Accepted: 6.
  - CVX 28 Invalid -> Valid: 6.

Good next target:

- The remaining forecast-only failures all have same-day DTP-family doses.
  That suggests the next forecast fixes should be handled alongside DTP
  same-day counting/ordering rather than as isolated forecast-date rules.
  Representative remaining cases include `fuzz_fail_dtp_20260608_27482`,
  `fuzz_fail_dtp_20260608_62378`, `fuzz_fail_dtp_20260608_68525`,
  `fuzz_fail_dtp_20260608_158219`, and
  `fuzz_fail_dtp_20260608_215276`.
- Next broad bucket: same-day and Td-family Valid/Accepted classification with
  Drools traces for representative counterexamples.

Do not repeat this failed broad experiment:

- A tempting rule was to treat DTP 5-dose histories with four valid prior doses
  as making later Td/Tdap-family dose-5 products Accepted before age 10. It
  improved some fuzz examples but regressed curated CDSi dose-5 DT/Tdap cases.
- Cases inspected during that experiment included
  `fuzz_fail_dtp_20260608_13181`, `fuzz_fail_dtp_20260608_150180`,
  `fuzz_fail_dtp_20260608_21859`, `fuzz_fail_dtp_20260608_19089`, and
  `fuzz_fail_dtp_20260608_202786`.
- Do not add blanket DTP5 dose-5 Accepted overrides for CVX 115, 198, 28, or
  the Td-family. First map the exact Java rule/fact flow for DTP5 completion
  exceptions and adolescent Tdap need.

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

## ICE Execution Mechanics That Matter

The Drools rules are not a complete linear algorithm. Several important DTP
behaviors come from generic ICE state transitions:

- `TargetSeries.addTargetDoseToSeries(...)` assigns
  `administeredShotNumberInSeries` in target-dose sort order and recomputes
  `doseNumberInSeries` via `determineDoseNumberInSeries(...)`.
- `TargetDose.setStatus(...)` marks `hasBeenEvaluated=true` for `VALID`,
  `INVALID`, and `ACCEPTED`, but only `VALID` sets `isValid=true`.
- `TargetSeries.determineEffectiveNumberOfDosesInSeries...` is documented as
  counting valid and accepted doses, while validity facts and many Drools
  conditions check `isValid` / `DoseStatus.VALID`.
- DTP same-day duplicate rules depend on `isPrimarySeriesShot` and
  `administeredShotNumberInSeries`. Those fields are assigned by generic
  TargetSeries mechanics, so the same Drools duplicate rule can produce
  different results depending on whether ICE considered each same-day target
  dose a primary-series dose.
- Because Drools rules mutate facts and insert ICE facts, output parity often
  depends on fact flow: dose status -> pertussis fact -> adolescent Tdap fact
  -> recurring Td override.

When a snapshot is surprising, first trace which generic TargetSeries/TargetDose
state the Drools condition was probably seeing before adding a Rust override.

For ambiguous DTP cases, prefer a single-case Drools event log over guessing
from rule text alone. Clear the log first, run one `--compare -v --trace` case,
then search targeted terms. DTP terms that have been useful:

- `_ADOLESCENT_TDAP_COMPLETED`
- `_DOSE_OF_PERTUSSIS`
- `_DTP_5_DOSE_SERIES_EXCEPTION1`
- `_DTP_5_DOSE_SERIES_EXCEPTION2`
- `DTP_PERTUSSIS_NEEDED`
- `DuplicateShotSameDay`
- `SeriesSelection.Select3DoseDTPSeriesIfNoShotsPriorTo7YrsAnd7YrsOldOrOlder`
- `SUPPORTED_SERIES.DTP_3_DOSE_SERIES`
- `SUPPORTED_SERIES.DTP_5_DOSE_SERIES`

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

Implementation note from `fuzz_fail_dtp_1780795905_1545` and
`fuzz_fail_dtp_1780795905_903`:

- The Java blocker is a target-dose concept, not raw administration history.
- A very early Td-family dose that Java later evaluates as invalid does not
  block adult 3-dose series selection.
- Java adult-series selection still keys off doses before the exact 7th
  birthday, not the 4-day grace boundary used by some product-level minimum-age
  checks.
- Traced counterexample `fuzz_fail_dtp_20260608_4427`: a CVX 139 dose on
  `2007-08-07` for a `2000-08-08` birth date did force Java into
  `SUPPORTED_SERIES.DTP_5_DOSE_SERIES`, even though the date is within the
  4-day grace window before age 7 and later product evaluation paths accept
  adult-family doses there.
- Therefore the series-selection blocker and the adult-product evaluation
  boundary are not identical in Java.

## Evaluation Rules

`Evaluation^DTP.dslr` is the main source for DTP evaluation overrides.

Recurring Td after adolescent Tdap completion:

- Once the primary series is complete and `_ADOLESCENT_TDAP_COMPLETED` exists,
  any DTP shot can satisfy recurring Td age, interval, and extra-dose checks.
- The Java comment says the minimum interval is 0 days for recurring Td.
- This explains why post-completion doses can become Valid booster anchors.
- `_ADOLESCENT_TDAP_COMPLETED` is inserted by `Evaluation^DTP.dslr` only after
  a `_DOSE_OF_PERTUSSIS` fact exists for a DTP shot at or after age 10 that
  targets pertussis, diphtheria, and tetanus.
- `_DOSE_OF_PERTUSSIS` is itself derived from either a valid primary-series
  pertussis/diphtheria/tetanus shot, an invalid primary-series D/T/P shot with
  `D_AND_T_INVALID/P_VALID`, or a valid D/T/P shot at or after age 7 when the
  series is complete.
- Therefore `_ADOLESCENT_TDAP_COMPLETED` is not equivalent to "the primary
  series contained pertussis." LAVA approximates the recurring-Td gate by
  checking for a valid qualifying pertussis-containing DTP-family dose after
  primary completion / completion exception. Until that gate exists, adult
  Td-only extra doses such as CVX 09, 138, 139, and 196 are commonly Accepted
  rather than Valid.
- CVX 195 can become Valid through the same recurring-Td gate after a
  post-primary pertussis anchor, but it can still be Invalid in in-series
  adult-dose slots when interval/series-selection rules fail.
- Snapshot-derived nuance: in the 3-dose adult series, a first post-primary
  adult Td product CVX 28, 138, or 139 can be Valid at age 10 or later even
  before the post-primary pertussis gate exists, but only when the completed
  primary 3-dose series already contains at least two valid pertussis-containing
  adult doses and there has not already been another post-primary DTP-family
  dose. Later Td-only products remain Accepted unless a valid post-primary
  pertussis-containing dose has occurred. CVX 09 and CVX 196 have not behaved
  like this first-anchor product in the observed cases. Representative valid
  guard: `fuzz_fail_dtp_20260608_4427`. Representative accepted cases:
  `fuzz_fail_dtp_20260608_7647` and `fuzz_fail_dtp_20260608_13589`.
- Additional traced nuance from `fuzz_fail_dtp_20260608_11201`: in the 3-dose
  adult series, the first post-primary CVX 113 can also stay Valid when given
  at age 7 or later, but it follows the same primary-series pertussis and "no
  prior post-primary DTP-family dose" guard. In that shape Java does not
  require age 10 or a prior post-primary pertussis booster before keeping the
  first CVX 113 extra dose valid.
- Snapshot-derived 5-dose nuance: if a DTP 5-dose exception makes the series
  complete before target dose 5, Java can still evaluate later target slots up
  through dose 5 as Valid rather than treating them as ordinary extra doses.
  A pertussis-containing dose 5 at or after age 7 also behaves like the
  recurring-Td gate for later boosters. A routine optional dose 5 before age 7
  can remain Accepted after a valid dose-4 completion exception. Td-family
  products after age 7 can still be Valid when filling the fourth counted dose
  after a 3-dose completion exception; representative case:
  `fuzz_fail_dtp_1780710085_3376`. Once four valid doses already exist,
  Td-family products in dose-5 slots can remain Accepted when they do not
  satisfy the adolescent pertussis need; representative case:
  `fuzz_fail_dtp_20260608_15613`.

Adolescent Tdap:

- If adolescent Tdap is needed and a pertussis/diphtheria/tetanus shot is given
  from age 7 through before age 10, the first such dose may be Valid.
- Further qualifying adolescent Tdap shots in that 7-to-10 window are Accepted
  as extra doses.
- The "first such dose" check is any valid pertussis-containing DTP-family dose
  at or after age 7, not just CVX 115/198. In DTP5 histories, if a valid
  pertussis dose such as CVX 130 already occurred after age 7, a later CVX 115
  before age 10 can be Accepted rather than Valid. Representative fixes:
  `fuzz_fail_dtp_20260608_13181`, `fuzz_fail_dtp_20260608_150180`, and
  `fuzz_fail_dtp_20260608_21859`.
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
- CVX 198 is not ignored just because the patient is under age 7. In the
  child DTP5 series Java can treat CVX 198 as a valid DTaP-containing dose, and
  that valid dose anchors later DTP intervals. Rust previously ignored all
  under-age Tdap-family CVX 115/198 doses for interval anchoring; narrowing
  that to CVX 115 plus Td-family products fixed a large bucket. Representative
  fixes: `fuzz_fail_dtp_20260608_3980`,
  `fuzz_fail_dtp_20260608_6145`, `fuzz_fail_dtp_20260608_14024`, and
  `fuzz_fail_dtp_20260608_240182`.
- In Java output this "ignored" path maps to LAVA `Accepted` with an ignored /
  outside-routine effect rather than a normal Valid dose.

DTP adult-series retry interval anchoring:

- Snapshot-derived behavior from `fuzz_fail_dtp_20260608_14684` and
  `fuzz_fail_dtp_20260608_24330`: in the adult `DTP_3_DOSE_SERIES`, if a
  target dose is Invalid but not ignored, Java still uses that invalid attempt
  as the interval anchor for a later retry of the same target dose. This
  differs from Rust's default valid-dose anchor and explains cases where Rust
  previously accepted a later dose 3 by measuring from dose 2, while Java
  invalidated it because it was less than the dose-3 minimum interval from the
  prior invalid dose-3 attempt.

DTP5 ignored-shot retry forecast anchoring:

- Snapshot-derived behavior from `fuzz_fail_dtp_20260608_109958`,
  `fuzz_fail_dtp_20260608_148964`, and
  `fuzz_fail_dtp_20260608_246356`: when the last child-series shot is a
  below-minimum-age Td-family shot that Java ignores for interval purposes, the
  next DTP5 forecast can still anchor from an earlier invalid retry attempt
  after the latest valid dose. This matters when the earlier invalid attempt is
  a pertussis-containing product or special CVX 195/198 and failed for
  age/interval timing.
- Do not apply this to all prior invalid shots. Prior invalid Td-family
  products such as CVX 139 do not generally anchor this forecast; guard case:
  `fuzz_fail_dtp_20260608_10237`.
- Do not apply it to invalid attempts before a later valid dose; guard case:
  `fuzz_fail_dtp_20260608_103071`.
- Do not apply it to child-series Tdap attempts that LAVA classifies as
  `InsufficientAntigen`; Java snapshots keep those as due-now / last-shot
  shapes rather than 28-day retry anchors. Guard cases:
  `fuzz_fail_dtp_20260608_112350` and
  `fuzz_fail_dtp_20260608_181198`.

DTP5 zero-valid invalid-history forecast anchoring:

- Snapshot-derived behavior from `fuzz_fail_dtp_20260608_24219` and
  `fuzz_fail_dtp_20260608_81697`: when the selected DTP5 series has no valid
  doses but at least two DTP-family target evaluations, Java can use the first
  invalid DTP-family attempt as a 28-day interval floor for the next forecast.
- This floors earliest and recommended dates only; the routine child overdue
  age boundary remains age-based. In `fuzz_fail_dtp_20260608_81697`, for
  example, earliest moves to first invalid plus 28 days while recommended stays
  at the age-2-month recommendation date.
- Keep this separate from the ignored-shot retry-anchor rule. The zero-valid
  case is about forecasting after no accepted valid series start, not about a
  final ignored Td-family shot after an established valid-dose anchor.

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
- Java can leave `TargetSeries.isSeriesComplete() = true` for the adult
  `DTP_3_DOSE_SERIES` even when the selected-series history still lacks a valid
  pertussis-containing dose.
- In the traced case `fuzz_fail_dtp_20260608_229764`, Java selected
  `SUPPORTED_SERIES.DTP_3_DOSE_SERIES` via
  `SeriesSelection.Select3DoseDTPSeriesIfNoShotsPriorTo7YrsAnd7YrsOldOrOlder`
  and evaluated:
  - CVX 139 on `2024-04-21` as `Valid #1`
  - CVX 195 on `2024-05-19` as `Valid #2`
  - CVX 22 on `2024-10-24` as `Invalid #3`
  - CVX 196 on `2025-04-24` as `Valid #3`
- Java then entered the recommendation workflow for the selected 3-dose adult
  series, temporarily produced a `DUE_NOW` recommendation at `2025-04-24`, and
  later returned the final forecast window:
  - earliest / recommended: `2028-04-21` (11th birthday)
  - overdue: `2030-05-18` (13 years + 4 weeks - 1 day)
- This means Java does not treat this shape as "ordinary incomplete adult dose
  4 now." Instead it treats the adult series as complete enough to suppress the
  immediate recurring-dose path, but still lacking the adolescent
  pertussis-containing recommendation milestone.
- LAVA should model this shape as a special adult-series forecast state:
  `DTP_3_DOSE_SERIES` with at least 3 valid doses but no valid
  pertussis-containing dose forecasts the adolescent Tdap window rather than a
  same-day / last-valid-date recommendation, but only when the history still
  contains some pertussis-containing DTP-family dose. Pure Td-only adult
  histories such as `dtp_adult_no_pertussis` remain immediate due-now cases in
  Java.

- If a pertussis-containing dose occurred from age 7 through before age 10,
  recommend Tdap at age 11, overdue at age 13 years plus 4 weeks.
- If the completed/reopened DTP forecast has no valid pertussis dose at or
  after age 7, Java can make the adolescent Tdap need due immediately at the
  latest valid dose instead of forecasting age 11. Representative fixes:
  `fuzz_fail_dtp_20260608_36519`, `fuzz_fail_dtp_20260608_156288`,
  `fuzz_fail_dtp_20260608_187116`, and
  `fuzz_fail_dtp_20260608_212403`.
- If a valid pertussis dose exists at or after age 7 but before age 10, keep
  the age-11 adolescent window. Representative counter/fix cases:
  `fuzz_fail_dtp_20260608_180155`, `fuzz_fail_dtp_20260608_198770`, and
  `fuzz_fail_dtp_20260608_202956`.
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
- Identical same-day CVX 198 doses usually preserve source order rather than
  using LAVA's older DTP same-CVX reverse ordering. This matches
  `fuzz_fail_dtp_20260608_78041`, `fuzz_fail_dtp_20260608_245513`,
  `fuzz_fail_dtp_20260608_127315`, `fuzz_fail_dtp_20260608_146598`,
  `fuzz_fail_dtp_20260608_174803`, and `fuzz_fail_dtp_20260608_205462`.
- That CVX 198 ordering is not blanket same-CVX behavior. If CVX 115 is also
  present on the same date, recorded Java output still follows the later-source
  duplicate ordering (`fuzz_fail_dtp_20260608_17430`). Identical CVX 139 also
  keeps later-source ordering (`fuzz_fail_dtp_20260608_793`).
- CVX 196 is an exception observed in recorded Java output. It behaves as a
  Td-family product for antigen/completion purposes, but same-day ordering
  follows source order rather than the generic pertussis-first sorting only in
  the no-prior-DTP same-day shape. With earlier DTP history already present,
  Java reverts to the generic mixed-product ordering and then lets the later
  CVX 196 dose survive through the post-primary extra-dose path. This matches:
  - `fuzz_fail_dtp_1780795905_4324`: no prior DTP history, `196` stays first,
    `107` becomes duplicate-invalid.
  - `fuzz_fail_dtp_20260608_2622`: prior 3-dose adult history exists, `20`
    consumes dose `#4`, `196` remains valid as same-day dose `#5`.

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
- In LAVA terms, the clean mapping is:
  - evaluate the primary-eligible same-day product first
  - let DTP's extra-dose hook decide whether the later same-day Td-family dose
    is `Valid` or `Accepted`
  - avoid broad product-wide exceptions for `196`; the ordering exception is
    narrower than that

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
  - Drools traces from `fuzz_fail_dtp_20260608_11201` and
    `fuzz_fail_dtp_20260608_24013` show this is also an effective-series
    selection issue. In `11201`, the final returned valid CVX 113 evaluation is
    from `DTP_5_DOSE_SERIES` after DTP5 exception processing. In `24013`, the
    same product/date role is returned as Accepted from the selected
    `DTP_3_DOSE_SERIES`, even though the DTP5 path also evaluated a target dose
    for the same shot. Do not resolve this bucket with a blanket CVX 113/138/139
    product rule; map Java's selected/returned TargetSeries first.
  - Evidence artifacts:
    `tests/relative/tmp/drools/DTP/fuzz_fail_dtp_20260608_11201/` and
    `tests/relative/tmp/drools/DTP/fuzz_fail_dtp_20260608_24013/`.

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

- DTP5 dose-5/adolescent Tdap interaction:
  - Java can keep some later DTP5 target slots Valid after a completion
    exception, but a broad "four valid prior doses makes Td/Tdap Accepted"
    approximation regressed curated CDSi cases.
  - Resolve this with Drools traces around the DTP5 completion exception facts,
    adolescent Tdap needed/completed facts, and target-dose number rather than
    by product family alone.

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
   - `cargo run --release --bin test_runner -- run tests/cases --group DTP`
   - `cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group DTP`
   - `cargo run --release --bin test_runner -- run tests/fuzz-100k-20260608.ltp --group H1N1`
