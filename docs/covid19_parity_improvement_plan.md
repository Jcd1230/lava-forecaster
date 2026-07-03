# COVID-19 Parity Improvement Plan for LAVA

Created: 2026-07-02  
Target project: `~/projects/lava-forecaster`  
Reference project: `~/projects/java-ice`

## Purpose

This document captures the current research and proposed implementation plan for improving COVID-19 parity between Rust LAVA and Java ICE. It is intended as a future implementation handoff, not as a completed patch plan with source edits already applied.

No Rust source changes were made as part of the research that produced this plan.

## Current known baseline

From the latest full `tests/fuzz-bank.ltp` summary available during research:

| Corpus | Group | Executed | Passed | Failed | Pass rate |
|---|---:|---:|---:|---:|---:|
| `tests/fuzz-bank.ltp` | `COVID19` | 8,815 | 2,789 | 6,026 | 31.64% |
| `tests/suite.ltp` | `COVID19` | 479 | 179 | 300 | 37.37% |

COVID-19 is therefore one of the largest remaining failure groups by absolute failed-case count.

## Important caveat

A focused COVID-only `test_runner` summary was started during research but the Local Dev command channel stopped responding before the final structured output could be retrieved. Before implementing, regenerate the focused summaries below and use them as the true baseline.

```bash
target/release/test_runner run tests/fuzz-bank.ltp \
  --group COVID19 \
  --summary-only \
  --summary tests/relative/tmp/covid19_fuzz_current.json

target/release/test_runner summarize tests/relative/tmp/covid19_fuzz_current.json

target/release/test_runner run tests/suite.ltp \
  --group COVID19 \
  --summary-only \
  --summary tests/relative/tmp/covid19_suite_current.json

target/release/test_runner summarize tests/relative/tmp/covid19_suite_current.json
```

Use `/tmp` for exploratory summaries if `tests/relative/tmp` should remain clean.

## Java ICE source files to use as truth

COVID-19 logic in Java ICE is split across dedicated evaluation, recommendation, duplicate-shot, and series-selection rules.

Primary Java ICE files:

```text
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^COVID19.dslr
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^COVID19^Dec2020Season.dslr
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^COVID19^Sep2023Season.dslr
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Evaluation^COVID19^Aug2025Season.dslr
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^COVID19.dslr
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^COVID19^Dec2020Season.dslr
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^COVID19^Sep2023Season.dslr
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^Recommendation^COVID19^Aug2025Season.dslr
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^DuplicateShotSameDay^COVID19.drl
opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^SeriesSelection.drl
```

Primary Java ICE series-plan YAML files:

```text
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_astra_zeneca_2_dose_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_aug_2025_2_y_to_64_y_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_aug_2025_gte_65_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_aug_2025_lt_2_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_bibp_sinopharm_2_dose_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_corona_vac_sinovac_2_dose_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_covaxin_2_dose_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_janssen_1_dose_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_medicago_2_dose_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_mixed_product_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_moderna_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_novavax_2_dose_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_pfizer_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_sep_2023_gte_5_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_sep_2023_mixed_product_lt_5_y_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_sep_2023_moderna_lt_5_y_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_sep_2023_novavax_series.yml
opencds-decision-support-service/src/main/resources/data/seriesPlanDefinitions/covid_19_sep_2023_pfizer_lt_5_y_series.yml
```

## Rust files to inspect first

Known COVID Rust files:

```text
src/rules/covid19/mod.rs
src/rules/covid19/overrides.rs
src/rules/covid19/schedules.rs
```

Use these commands to confirm any additional touchpoints:

```bash
find src -type f | rg -i 'covid|covid19'
rg -n 'COVID19|COVID_19|Aug2025|Sep2023|Novavax|mixed|series_selection|same_day|duplicate' src tests
```

Likely implementation areas:

- COVID schedule definitions
- COVID season definitions
- COVID custom evaluation hook
- COVID custom forecast hook
- COVID custom series-selection hook
- COVID same-day duplicate policy
- generic series-selection completion logic if COVID currently reuses generic behavior

## High-value hypothesis

The most valuable COVID fixes are likely in **series selection**, not isolated date arithmetic.

Reasoning:

1. COVID has many season-specific series.
2. Java ICE has a large COVID-specific block inside `SeriesSelection.drl`.
3. A wrong selected series cascades into evaluation status, forecast status, availability, earliest date, recommended date, and overdue date mismatches.
4. Current COVID pass rate is very low, suggesting broad routing issues rather than only small interval bugs.

## Recommended implementation sequence

1. Regenerate COVID-only summaries.
2. Bucket failures by mismatch shape and CVX.
3. Start with Aug 2025 age-based series selection.
4. Then implement pre-Aug-2025 completed-series postprocessing.
5. Then implement Sep 2023 / Aug 2024 child/adult/mixed-product/Novavax routing.
6. Only then investigate same-day duplicate handling.
7. Promote a small number of representative fuzz cases after each behavior is source-backed.

## Task 1: Regenerate COVID-only summary and find buckets

### Goal

Produce a focused baseline with enough structure to choose the first implementation slice.

### Commands

```bash
target/release/test_runner run tests/fuzz-bank.ltp \
  --group COVID19 \
  --summary-only \
  --summary /tmp/lava_covid19_fuzz_current.json

target/release/test_runner summarize /tmp/lava_covid19_fuzz_current.json

target/release/test_runner run tests/suite.ltp \
  --group COVID19 \
  --summary-only \
  --summary /tmp/lava_covid19_suite_current.json

target/release/test_runner summarize /tmp/lava_covid19_suite_current.json
```

### What to record

- Top evaluation status transitions, for example `Valid -> Invalid`, `Valid -> Accepted`, `Invalid -> Valid`
- Top CVX-specific transitions
- Count of same-day failure cases
- Top forecast fields mismatched
- Top forecast date deltas
- Whether worst failures are mostly forecast-only, eval-only, or eval+forecast

### Decision rule

If the worst bucket is forecast availability/status and appears to involve selected series, start with Task 2.

If the worst bucket is same-day `Valid`/`Accepted` transitions, jump to Task 6 after confirming `DuplicateShotSameDay^COVID19.drl`.

If the worst bucket is CVX `211`/`313`, prioritize Task 5.

## Task 2: Aug 2025 age-2 and age-65 series selection

### Java ICE source anchor

In `SeriesSelection.drl`, Java starts an Aug 2025 COVID block around:

```text
Start COVID Series Selection Rules Aug2025+ Season
```

Relevant rules found during research:

```text
SeriesSelection(COVID-19 Aug2025+ Abstract): In Aug2025 season, determine date at which patient turns 2 and 65 years of age
```

```text
Select the Seasonal 2-dose COVID-19 Series (< 2 years) if patient is < 2 years of age as of evaluation date, or patient is >= 2 years and has a shot administered in current season at < 2 years of age
```

```text
Select the Seasonal 1-dose COVID-19 Series (>= 2 - 64 years) if patient has no in-season shots and is >= 2 years and < 65 years as of evaluation date
```

```text
Select the Seasonal 2-dose COVID-19 Series (>= 65 years) if patient has no in-season shots and is >= 65 years as of evaluation date
```

```text
Select the Seasonal 1-dose COVID-19 Series (>= 2 - 64 years) if patient has in-season shots administered at >= 2 years and target dose 1 received at age < 65 years
```

```text
Select the Seasonal 2-dose COVID-19 Series (>= 65 years) if patient has in-season shots administered and target dose 1 received at age >= 65 years
```

```text
Select the Seasonal 2-dose COVID-19 Series (>= 65 years) if patient if dose 1 is administered to a patient that will turn 65 within 12 months of season start date
```

### Behavior to implement in Rust

For the Aug 2025 season:

1. Compute:
   - date patient turns 2;
   - date patient turns 65;
   - season start date;
   - evaluation date;
   - valid/current-season COVID target doses if available.
2. Select `<2` seasonal series if:
   - patient is under 2 at evaluation date; OR
   - patient is 2+ at evaluation but has a current-season COVID dose administered before age 2.
3. Select `2–64` seasonal series if:
   - no current-season COVID doses and evaluation age is >= 2 and < 65; OR
   - current-season target dose 1 was administered at >= 2 and < 65, unless the 65-within-12-months rule applies.
4. Select `>=65` seasonal series if:
   - no current-season COVID doses and evaluation age is >= 65; OR
   - current-season target dose 1 was administered at >= 65; OR
   - target dose 1 was administered before age 65, but the patient turns 65 after season start and within 12 months of season start.

### Important edge cases

- Use **dose 1 age** for patients with current-season shots.
- Use **evaluation age** for patients with no current-season shots.
- The `<2` rule can select the `<2` series even if the patient is no longer under 2 at evaluation, as long as an in-season shot happened before age 2.
- The `>=65` rule can apply even if dose 1 happened before the 65th birthday, if the patient turns 65 within 12 months of season start.
- Confirm whether Java uses all target doses or only valid target doses in each branch. Do not assume; read each rule body.

### Suggested exploratory commands

```bash
rg -n 'COVID_19_AUG_2025|AUG_2025|dateAt2yrs|dateAt65yrs|65' \
  /home/jason/projects/java-ice/opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^SeriesSelection.drl

rg -n 'Aug2025|AUG_2025|date_at_2|date_at_65|65' src/rules src
```

### Acceptance criteria

After implementation:

- `tests/suite.ltp --group COVID19` does not regress.
- `tests/fuzz-bank.ltp --group COVID19` improves, or at minimum fixes the targeted promoted fixtures without broad regressions.
- One or more representative Aug 2025 fixtures are promoted only after the Java rule branch is clearly identified.

### Risk

Medium. Series selection changes can cascade widely. Keep the first patch strictly limited to Aug 2025.

## Task 3: Pre-Aug-2025 completed-series postprocessing

### Java ICE source anchor

In `SeriesSelection.drl`, Java starts a pre-Aug-2025 COVID block around:

```text
Start COVID Series Selection Rules Pre-Aug2025+ Season
```

Important comment:

```text
This is the only way a non-FDA, WHO-approved series can be selected (it must be complete).
```

Important rule:

```text
SeriesSelection(COVID-19 Pre-Aug25): Select already _completed_ series if a different, _incomplete_ series was previously selected
```

The rule appears to apply when:

- vaccine group is COVID-19;
- season start is before 2025-08-27;
- selected series is incomplete;
- another same-season series is complete.

### Behavior to implement in Rust

For COVID seasons before the Aug 2025 season:

1. After initial series selection, inspect all COVID target series for the same season.
2. If the currently selected series is incomplete and another series is complete, switch to the completed series.
3. Preserve Java exceptions, especially the child/adult Sep 2023 / Aug 2024 exception described in Task 4.
4. Confirm whether Java considers `SERIES_SELECTION_IN_PROCESS`, `SERIES_SELECTION_IN_POSTPROCESS`, or both in each rule.

### Why this is high-value

A completed-series postprocess can flip:

- forecast status from due/recommended to complete/not recommended;
- selected product series;
- expected future dose count;
- forecast availability;
- evaluation dose-number interpretation.

### Suggested exploratory commands

```bash
rg -n 'completed|incomplete|customSeriesSelectionCompletionRules|Pre-Aug25|WHO-approved' \
  /home/jason/projects/java-ice/opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^SeriesSelection.drl

rg -n 'complete|completed|incomplete|selected_series|series_complete|COVID' src
```

### Acceptance criteria

- Fixes targeted cases where Rust currently forecasts additional COVID doses but Java ICE treats a complete series as selected.
- Does not incorrectly select WHO/non-FDA series unless complete.
- Does not override the Sep 2023/Aug 2024 child/adult exception.

### Risk

Medium-high. This is broad, so implement after Task 2 unless summaries show this is overwhelmingly the largest bucket.

## Task 4: Sep 2023 / Aug 2024 adult-vs-child completed-series exception

### Java ICE source anchor

Important rule:

```text
SeriesSelection(COVID-19 Sep2023/Aug2024 Adult Series): Do NOT select already _completed_ Sep2023/Aug2024 _Child_ series if a different, _incomplete_ Sep2023/Aug2024 _Adult_ series was previously selected and there are no doses at < 5 years of age
```

The rule references:

- `SUPPORTED_SERIES.COVID_19_SEP_2023_GTE_5_SERIES`
- `SUPPORTED_SERIES.COVID_19_SEP_2023_NOVAVAX_SERIES`
- `SUPPORTED_SERIES.COVID_19_SEP_2023_MIXED_PRODUCT_LT_5_Y_SERIES`
- `SUPPORTED_SERIES.COVID_19_SEP_2023_PFIZER_LT_5_Y_SERIES`
- `SUPPORTED_SERIES.COVID_19_SEP_2023_MODERNA_LT_5_Y_SERIES`

### Behavior to implement in Rust

When applying completed-series postprocessing for Sep 2023 / Aug 2024:

- Do not switch from an incomplete adult series to a completed child series if there are no current-season COVID doses administered before age 5.
- Adult series include:
  - Sep 2023 `>=5` series;
  - Sep 2023 Novavax series.
- Child series include:
  - Sep 2023 mixed-product `<5` series;
  - Sep 2023 Pfizer `<5` series;
  - Sep 2023 Moderna `<5` series.

### Why this matters

A generic “prefer completed series” rule may over-select child series for patients who should remain in adult routing.

### Acceptance criteria

- At least one fixture shows adult-series selection preserved despite a completed child series.
- No regression in cases where a child series is legitimately selected due to doses before age 5.

### Risk

Medium. This is a guard on Task 3 and should be implemented with Task 3 if Task 3 is touched.

## Task 5: Sep 2023 / Aug 2024 mixed-product and Novavax routing

### Java ICE source anchor: mixed product

Java has a rule:

```text
SeriesSelection(COVID-19 Sep2023/Aug2024): Select the Mixed Product < 5 yrs series if any doses were administered in the current season prior to 5 yrs of age, and there are a mix of products used
```

The rule considers product family groups including:

```text
VACCINE_CVX_311, VACCINE_CVX_312
VACCINE_CVX_308, VACCINE_CVX_309, VACCINE_CVX_310
VACCINE_CVX_211, VACCINE_CVX_313
unspecified formulation
```

### Behavior to verify

Confirm exact product-family mapping in Java and Rust. Likely groups:

- Pfizer pediatric/current formulations: CVX `311`, `312`
- Moderna pediatric/current formulations: CVX `308`, `309`, `310`
- Novavax: CVX `211`, `313`
- unspecified formulation: mixed-product trigger

Do not rely on this mapping without reading the Java rule body and CVX data.

### Behavior to implement

For Sep 2023 / Aug 2024 seasons:

1. Consider current-season COVID doses.
2. Prefer valid doses if Java uses `isValid == true`.
3. If any dose before age 5 exists and product families are mixed, select mixed-product `<5y` series.
4. Treat unspecified formulation the way Java does. The rule text strongly suggests unspecified formulation can trigger mixed-product selection.

### Java ICE source anchor: Novavax

Java comments mention:

```text
If the patient has no prior COVID-19 doses on record before 9/12/2023, and dose 1 in a Sept 2023 series is Novavax (CVX 211, CVX 313), and there are no other doses after dose 1...
```

The rule distinguishes:

- dose 1 Novavax at age >= 12;
- dose 1 Novavax at age < 12;
- dose 2 Novavax;
- dose 1 not Novavax;
- no prior COVID doses before 2023-09-12.

### Suggested implementation slices

Do not implement all Novavax behavior in one broad change. Split it:

1. No prior COVID before 2023-09-12 + dose 1 Novavax + dose 1 age >= 12.
2. Dose 1 Novavax + dose 1 age < 12.
3. Dose 2 Novavax forces Novavax series.
4. Dose 1 non-Novavax forces `>=5` series.
5. Prior COVID before 2023-09-12 exception behavior.

### Suggested exploratory commands

```bash
rg -n 'Novavax|VACCINE_CVX_211|VACCINE_CVX_313|09/12/2023|12 years|12y' \
  /home/jason/projects/java-ice/opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^SeriesSelection.drl

rg -n '211|313|Novavax|novavax|Sep2023|SEP_2023' src tests
```

### Acceptance criteria

- Improved CVX `211`/`313` buckets in focused COVID summary.
- No regressions in non-Novavax Sep 2023 / Aug 2024 COVID cases.
- Promoted fixtures cover at least one Novavax dose-1 case and one dose-2 case if both are changed.

### Risk

Medium. Product routing is complex, but the scope can be kept narrow by CVX and season.

## Task 6: COVID duplicate same-day handling

### Java ICE source anchor

Dedicated file:

```text
DuplicateShotSameDay^COVID19.drl
```

### When to prioritize

Only prioritize this if the focused COVID summary shows many same-day failures or status transitions like:

- `Valid -> Accepted`
- `Accepted -> Valid`
- `Valid -> Invalid`
- `Invalid -> Valid`

on same-day COVID dose dates.

### Behavior to inspect

Read Java same-day rules and answer:

- Which CVX/product wins on the same date?
- Are duplicates accepted, invalid, or ignored?
- Does source order matter?
- Does season matter?
- Are old monovalent/bivalent doses handled differently from current-season doses?
- Are unspecified formulation doses preferred or demoted?

### Suggested exploratory commands

```bash
sed -n '1,260p' \
  /home/jason/projects/java-ice/opencds-decision-support-rules/src/main/resources/drools/knowledgeModule/org.nyc.cir.ice/org.nyc.cir^ICE^1.0.0^DuplicateShotSameDay^COVID19.drl

rg -n 'same_day|same-day|duplicate|Duplicate|Accepted|source_order|COVID' src/rules src
```

### Acceptance criteria

- Fixes a representative same-day bucket without affecting non-same-day COVID cases.
- If source-order-sensitive, add/promote a fixture preserving the ordering.
- Avoid generic same-day engine changes unless identical behavior exists across multiple vaccine groups.

### Risk

Medium-high. Same-day ordering can affect dose numbering and interval anchors. Prefer this after selected-series fixes unless the summary shows it is the largest COVID bucket.

## Task 7: Evaluation and recommendation season-specific rules

### Java ICE source anchors

Evaluation:

```text
Evaluation^COVID19.dslr
Evaluation^COVID19^Dec2020Season.dslr
Evaluation^COVID19^Sep2023Season.dslr
Evaluation^COVID19^Aug2025Season.dslr
```

Recommendation:

```text
Recommendation^COVID19.dslr
Recommendation^COVID19^Dec2020Season.dslr
Recommendation^COVID19^Sep2023Season.dslr
Recommendation^COVID19^Aug2025Season.dslr
```

### When to prioritize

After series selection is reasonably aligned, inspect these when failures are still mostly:

- date deltas;
- earliest/recommended/overdue mismatch;
- evaluation validity mismatches where selected series is already correct;
- age/interval-specific mismatches.

### Suggested approach

For each remaining large bucket:

1. Pick one representative failed fuzz case.
2. Run with verbose trace.
3. Identify selected series and target dose number.
4. Read the matching Java evaluation/recommendation season file.
5. Patch only the specific COVID hook needed.
6. Promote one case if it captures a real rule.

### Commands

```bash
target/release/test_runner inspect tests/fuzz-bank.ltp \
  --group COVID19 \
  --list-cases | sed -n '1,200p'

target/release/test_runner run tests/fuzz-bank.ltp \
  --group COVID19 \
  --case <CASE_NAME> \
  -v --trace
```

If the Java service is available:

```bash
target/release/test_runner run tests/fuzz-bank.ltp \
  --group COVID19 \
  --case <CASE_NAME> \
  --compare \
  -v --trace
```

## Fixture promotion guidance

Do not promote every fuzz failure. Promote one fixture per understood rule bucket.

Good fixture categories:

1. Aug 2025 `<2` selected because patient has in-season dose before age 2 but eval age is >=2.
2. Aug 2025 `>=65` selected because patient turns 65 within 12 months of season start.
3. Pre-Aug-2025 completed alternate series selected over incomplete selected series.
4. Sep 2023/Aug 2024 adult incomplete series preserved over completed child series when no dose before age 5.
5. Mixed-product `<5` selected due to mixed product families.
6. Mixed-product `<5` selected due to unspecified formulation, if Java confirms this.
7. Novavax dose 1 age >=12 selects Novavax.
8. Novavax dose 2 selects Novavax.
9. COVID same-day duplicate product preference.

Suggested promotion pattern:

```bash
target/release/test_runner promote tests/fuzz-bank.ltp \
  --group COVID19 \
  --case <CASE_NAME> \
  --output tests/cases/covid19_<short_rule_name>.json
```

Confirm the actual `promote` CLI syntax first:

```bash
target/release/test_runner help promote
```

## Recommended patch sequence

### Patch A: Aug 2025 series selection only

Scope:

- COVID only.
- Aug 2025 season only.
- Age 2 / age 65 routing only.
- No generic engine changes.

Validation:

```bash
target/release/test_runner run tests/suite.ltp --group COVID19 --summary-only

target/release/test_runner run tests/fuzz-bank.ltp \
  --group COVID19 \
  --summary-only \
  --summary /tmp/lava_covid19_after_aug2025.json

target/release/test_runner summarize /tmp/lava_covid19_after_aug2025.json
```

Rollback criteria:

- Suite COVID regressions exceed improvements.
- Fuzz COVID pass count decreases substantially.
- Non-Aug2025 COVID cases change unexpectedly.

### Patch B: Pre-Aug-2025 completed-series postprocess plus child/adult guard

Scope:

- COVID only.
- Seasons before Aug 2025.
- Completed-series switch.
- Sep 2023/Aug 2024 child/adult guard included.

Validation:

```bash
target/release/test_runner run tests/suite.ltp --group COVID19 --summary-only

target/release/test_runner run tests/fuzz-bank.ltp \
  --group COVID19 \
  --summary-only \
  --summary /tmp/lava_covid19_after_completed_series.json

target/release/test_runner summarize /tmp/lava_covid19_after_completed_series.json
```

Rollback criteria:

- Adult/child selection regresses.
- WHO/non-FDA series selected while incomplete.
- Large same-day or dose-number regressions.

### Patch C: Sep 2023 / Aug 2024 mixed-product routing

Scope:

- COVID only.
- Sep 2023 and Aug 2024 seasons.
- Under age 5.
- Product-family mix and unspecified formulation behavior.

Validation:

- CVX `308`, `309`, `310`, `311`, `312`, `211`, `313`, and unspecified buckets improve.
- No broad changes to >=5 adult cases unless Java requires them.

### Patch D: Novavax routing

Scope:

- COVID only.
- Sep 2023/Aug 2024 Novavax.
- CVX `211` and `313`.

Validation:

- CVX `211`/`313` failure buckets improve.
- Adult `>=5` series does not regress.

### Patch E: Same-day duplicate COVID policy

Scope:

- COVID only.
- Same-day cases only.
- Product ordering and duplicate status.

Validation:

- Same-day count improves.
- Non-same-day COVID cases stable.

## Notes on interpreting improvements

Because COVID series selection is highly connected, a patch can improve one bucket while moving failures into another. Evaluate by:

1. total COVID pass count;
2. suite pass count;
3. target bucket count;
4. newly introduced transitions;
5. representative trace correctness.

A good patch should usually improve `tests/suite.ltp` or be neutral there, and improve `tests/fuzz-bank.ltp` for the targeted bucket.

## Avoid initially

Avoid:

- generic engine changes before proving COVID-specific need;
- broad interval changes without verifying selected series;
- same-day changes before reading Java `DuplicateShotSameDay^COVID19.drl`;
- promoting dozens of fuzz cases;
- updating expected outputs to match Rust.

## Minimal handoff prompt for implementation agent

```text
You are working in ~/projects/lava-forecaster with Java ICE available at ~/projects/java-ice.

Goal: improve COVID19 parity without broad generic engine changes.

First regenerate COVID-only summaries for tests/fuzz-bank.ltp and tests/suite.ltp. Use those as baseline.

Start with Aug 2025 COVID series selection. Read Java ICE SeriesSelection.drl COVID Aug2025+ rules and match the Rust COVID series-selection hook to Java's age-2 and age-65 routing:
- <2 series if eval age <2 or any current-season dose occurred before age 2.
- 2–64 series if no in-season doses and eval age is 2–64, or dose 1 occurred at age 2–64, unless the 65-within-12-months rule applies.
- >=65 series if no in-season doses and eval age >=65, or dose 1 occurred at age >=65, or dose 1 occurred before 65 but patient turns 65 within 12 months of season start.

Keep the patch COVID-only and Aug2025-only. Add/promote at most 1–2 representative fixtures after behavior is source-backed. Validate with:
target/release/test_runner run tests/suite.ltp --group COVID19 --summary-only
target/release/test_runner run tests/fuzz-bank.ltp --group COVID19 --summary-only --summary /tmp/lava_covid19_after.json
target/release/test_runner summarize /tmp/lava_covid19_after.json

Report baseline vs after counts, changed files, and the exact Java rules mirrored.
```

## Open questions before implementation

1. Does Rust already represent COVID seasons with exact Java season names and dates?
2. Does Rust currently know the selected target season when running COVID series selection?
3. Are current-season COVID doses filtered by all target doses or only valid target doses in each Java branch?
4. Does Java use `administrationDate < dateAt2yrs` or `<=`? The observed snippet used `<`.
5. Does Java use `administrationDate >= dateAt65yrs` for 65+ dose-1 routing? The observed snippet used `>=`.
6. What is Rust’s representation of dose number in selected/alternate series before postprocessing?
7. Can completed-series postprocessing be implemented COVID-only without touching the generic selector?
8. Does current Rust summary expose selected-series mismatches directly, or must they be inferred from forecast/eval differences?
9. Does the Java service need to be running for `--compare`, or are saved Java expected snapshots enough for the first patches?

## Recommended next action

1. Regenerate the COVID-only baseline summaries.
2. Inspect the Rust COVID implementation files.
3. Pick Patch A unless the focused summary clearly shows same-day or Novavax is the dominant bucket.
