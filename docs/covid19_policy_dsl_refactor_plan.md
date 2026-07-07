# COVID-19 policy DSL refactor plan

Created: 2026-07-06  
Target project: `~/projects/lava-forecaster`  
Reference project: `~/projects/java-ice`

## Purpose

This document describes a staged plan for moving LAVA's COVID-19 parity work away from scattered conditional patches and toward an explicit Rust policy table / DSL that mirrors the Java ICE COVID model.

The goal is not to design a general-purpose vaccine DSL. The goal is a small, COVID-specific representation that captures the concepts Java ICE actually uses for COVID seasons, product families, series selection, evaluation, dose identity, and forecasting.

## Current context

Current COVID parity after the latest focused fixes:

```text
COVID suite:     327 / 479
COVID fuzz-bank: 5228 / 8815
Full suite:      3243 / 4834
```

The remaining failures are increasingly difficult to fix with local patches because the Rust implementation collapses several Java ICE concepts into a small number of ad hoc buckets inside `src/rules/covid19/overrides.rs`.

## Implementation status as of scaffolding commits

The first behavior-neutral implementation passes have already been completed after this plan was written.

Committed scaffolding:

```text
lpsvtxnq 7acdfe9f Add COVID policy model scaffolding
sqtpsyyz ff0c3571 Add COVID dose fact and trace scaffolding
tvozvuyp 3287f9ee Add COVID Aug 2025 selection scaffolding
```

Current COVID files:

```text
src/rules/covid19/facts.rs
src/rules/covid19/mod.rs
src/rules/covid19/overrides.rs
src/rules/covid19/policy.rs
src/rules/covid19/products.rs
src/rules/covid19/schedules.rs
src/rules/covid19/seasons.rs
src/rules/covid19/selection.rs
src/rules/covid19/series.rs
src/rules/covid19/trace.rs
```

Implemented so far:

- `seasons.rs`: `CovidSeason` plus Java ICE season boundaries and ICE key conversion.
- `products.rs`: `CovidProductFamily`, formulation-era classification, Java-supported COVID CVX classification, Aug 2025 current-formulation helper, and legacy pediatric product helper.
- `policy.rs`: core COVID policy model, including `CovidSeriesId`, `CovidSeriesPolicy`, dose identity, overflow, interval anchor, forecast anchor, evaluation, forecast, and source-reference enums/structs.
- `series.rs`: initial `COVID_SERIES_POLICIES` table with Dec 2020, Sep 2023, Aug 2024, and Aug 2025 policy entries, CVX member sets, max-dose hints, source references, and notes.
- `facts.rs`: normalized `CovidDoseFact` and `CovidAgeFacts` helpers, plus relationship calculation against a selected series.
- `trace.rs`: trace data structures that can be built from selected policy, normalized facts, evaluations, and optional forecast.
- `selection.rs`: behavior-neutral Aug 2025 selection helper mirroring the main Java `SeriesSelection.drl` branches for LT2, 2-64, and 65+.

Validation after each scaffolding commit remained behavior-neutral:

```text
COVID suite:     327 / 479
COVID fuzz-bank: 5228 / 8815
Full suite:      3243 / 4834
```

Important: these modules are currently mostly scaffolding. Production COVID behavior still primarily runs through `src/rules/covid19/overrides.rs` and `src/rules/covid19/schedules.rs`. The next agent should not re-add these types; it should begin wiring them into a low-risk trace/classification path or migrate one small behavior branch.

## Recommended next handoff target

The next best step is to make the scaffolding useful for parity work without changing forecast/evaluation behavior.

Recommended target:

1. Add a focused COVID trace/debug pathway that can print the normalized facts and selected Aug 2025 policy for a single case.
2. Prefer wiring this through an existing verbose/debug/test-runner pathway if available.
3. Keep normal test output and behavior unchanged.
4. Validate that COVID suite, COVID fuzz-bank, and full suite remain unchanged.

A good first trace target is not to replace Java parity logic, but to let an agent see facts like this for a failing case:

```text
selected_policy = Aug2025Age2To64
Dose 2025-08-27 CVX 310:
  season = COVID_19_AUG_2025_SEASON
  product_family = PfizerPediatric
  relationship_to_selected_series = CovidButNotThisSeries
  supported_by_java_covid = true
  age_at_dose.under_2_years = false
```

After trace support exists, use it to bucket remaining failures by semantic cause rather than raw date/status deltas.


Examples of concepts currently entangled in `overrides.rs`:

- COVID season classification.
- Supported vs unsupported COVID CVX handling.
- Product-family membership.
- Series selection.
- Dose evaluation.
- Dose numbering / dose identity.
- Accepted overflow handling.
- Current-season vs prior-season behavior.
- Evaluation interval anchors.
- Forecast interval anchors.
- Special handling for invalid but not-ignored doses.

The recent fixes show that the remaining failures are often not simply “missing interval X.” They are usually caused by Rust selecting the wrong conceptual bucket compared to Java ICE.

## Main diagnosis

The main blocker to 100% COVID parity is semantic mismatch, not Rust implementation mechanics.

Java ICE behaves like a season/product/series rule system:

```text
1. Determine COVID season.
2. Determine product family / CVX relationship.
3. Select the applicable COVID series.
4. Evaluate doses against the selected series and relevant historical series context.
5. Preserve Java target dose identity.
6. Forecast from explicit evaluated state.
```

The Rust implementation currently approximates much of this through custom post-processing and conditional forecast hooks. That approach has delivered many wins, but it has reached the point where small changes often fix one bucket and regress another because the underlying Java concept is not represented directly.

## Recommended direction

Use Rust code itself as the canonical “rule table.”

Avoid making a large Markdown table and separately translating it into code. Instead, define a small set of Rust structs, enums, and constructor functions that are readable enough to serve as documentation while also being directly usable by the COVID evaluation and forecast logic.

The Markdown documentation should explain the shape and implementation checklist. The canonical rules should eventually live in Rust as policy data.

## Non-goals

- Do not build a generic vaccine-rule DSL for all vaccine groups.
- Do not rewrite all COVID logic in one large commit.
- Do not replace the existing schedule engine globally.
- Do not encode broad behavior from fuzz deltas alone without Java ICE source support or snapshot evidence.
- Do not make the DSL macro-heavy until the data shape has stabilized.

## Proposed module layout

The end state should split COVID logic into focused modules:

```text
src/rules/covid19/
  mod.rs
  seasons.rs
  products.rs
  policy.rs
  series.rs
  selection.rs
  evaluation.rs
  forecasting.rs
  trace.rs
  legacy_overrides.rs
```

Suggested responsibilities:

```text
seasons.rs
  Maps dates to COVID seasons and season start dates.

products.rs
  Maps CVX codes to COVID product families, formulation families, and support status.

policy.rs
  Defines the COVID policy DSL structs/enums.

series.rs
  Defines CovidSeriesId and the static COVID policy table.

selection.rs
  Mirrors Java ICE SeriesSelection.drl behavior.

evaluation.rs
  Evaluates doses using selected policy and product/season facts.

forecasting.rs
  Forecasts from explicit evaluated COVID state.

trace.rs
  Emits compact parity traces for selected cases.

legacy_overrides.rs
  Temporary bridge for behavior not yet moved out of overrides.rs.
```

`src/rules/covid19/overrides.rs` should shrink over time rather than being replaced all at once.

## Core Rust policy model

Start with simple Rust data and helper constructors. Avoid a macro DSL until the shape is proven.

Candidate enums:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CovidSeason {
    Dec2020,
    Sep2023,
    Aug2024,
    Aug2025,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CovidProductFamily {
    PfizerPediatric,
    PfizerAdult,
    ModernaPediatric,
    ModernaAdult,
    Novavax,
    Janssen,
    OldMonovalent,
    OldBivalent,
    OtherSupported,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CovidAgeBand {
    Under2AtEvaluation,
    Under5AtSeasonStart,
    Age2To64,
    Age65Plus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CovidSeriesId {
    Dec2020Primary,
    Sep2023PfizerLt5,
    Sep2023ModernaLt5,
    Sep2023MixedLt5,
    Sep2023Gte5,
    Sep2023Novavax,
    Aug2024PfizerLt5,
    Aug2024ModernaLt5,
    Aug2024MixedLt5,
    Aug2024Gte5,
    Aug2024Novavax,
    Aug2025Lt2,
    Aug2025Age2To64,
    Aug2025Age65Plus,
}
```

Candidate policy structs:

```rust
pub struct CovidSeriesPolicy {
    pub id: CovidSeriesId,
    pub ice_name: &'static str,
    pub season: CovidSeason,
    pub product_family: CovidProductFamily,
    pub age_band: CovidAgeBand,
    pub cvx_members: &'static [Cvx],
    pub max_valid_doses: usize,
    pub selection: SeriesSelectionPolicy,
    pub dose_identity: DoseIdentityPolicy,
    pub overflow: OverflowPolicy,
    pub intervals: IntervalPolicy,
    pub evaluation: EvaluationPolicy,
    pub forecast: ForecastPolicy,
    pub sources: IceSourceRefs,
}
```

Supporting policy enums should make Java concepts explicit:

```rust
pub enum CvxRelationship {
    MemberOfSelectedSeries,
    CovidButNotThisSeries,
    SupportedButOldProduct,
    UnsupportedIgnored,
}

pub enum DoseIdentityPolicy {
    ChronologicalWithinCollapsedPriorLt5,
    SeasonLocal,
    ProductSeriesLocal,
    CurrentSeasonLocal,
    ResetOnNewSeason,
    PreserveTargetDoseFromIce,
}

pub enum OverflowPolicy {
    AcceptedKeepsDoseNumber,
    AcceptedAsExtraDose,
    PreserveValidForSeasonDoseOne,
    InvalidWhenBeyondSeries,
}

pub enum IntervalAnchorPolicy {
    LastValidDoseInSelectedSeries,
    LastNotIgnoredCovidDose,
    LastValidOrAcceptedHistoricalDose,
    IgnoreInvalidNonSeries,
    IncludeInvalidNonSeriesForLt2,
}

pub enum ForecastAnchorPolicy {
    SeasonStart,
    LastCurrentSeasonDosePlus56,
    LastInvalidOldProductPlus56,
    LastInvalidLt2Plus28,
    PriorShotWindowClampedToInvalidAttempt,
    NoForecastAnchor,
}
```

The exact enum names can change. The important point is that these concepts must be represented directly instead of implied by nested conditionals.

## Example policy entry

Prefer readable constructor functions over macros at first:

```rust
pub fn aug2025_age_2_to_64_series() -> CovidSeriesPolicy {
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2025Age2To64,
        ice_name: "COVID-19 Aug 2025 2y-64y series",
        season: CovidSeason::Aug2025,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Age2To64,
        cvx_members: &[
            cvx!("213"),
            cvx!("309"),
            cvx!("310"),
            cvx!("311"),
            cvx!("312"),
            cvx!("313"),
            cvx!("334"),
        ],
        max_valid_doses: 1,
        selection: SeriesSelectionPolicy::Age2To64NoCurrentSeasonDose,
        dose_identity: DoseIdentityPolicy::CurrentSeasonLocal,
        overflow: OverflowPolicy::AcceptedAsExtraDose,
        intervals: IntervalPolicy::aug2025_adult(),
        evaluation: EvaluationPolicy::aug2025_adult(),
        forecast: ForecastPolicy::aug2025_adult(),
        sources: IceSourceRefs::new()
            .series_selection("SeriesSelection.drl")
            .evaluation("Evaluation^COVID19^Aug2025Season.dslr")
            .recommendation("Recommendation^COVID19^Aug2025Season.dslr")
            .yaml("covid_19_aug_2025_2_y_to_64_y_series.yml"),
    }
}
```

Eventually, the canonical table should look like this:

```rust
pub static COVID_SERIES_POLICIES: &[CovidSeriesPolicy] = &[
    dec2020_primary_series(),
    sep2023_pfizer_lt5_series(),
    sep2023_moderna_lt5_series(),
    sep2023_mixed_lt5_series(),
    sep2023_gte5_series(),
    sep2023_novavax_series(),
    aug2024_pfizer_lt5_series(),
    aug2024_moderna_lt5_series(),
    aug2024_mixed_lt5_series(),
    aug2024_gte5_series(),
    aug2024_novavax_series(),
    aug2025_lt2_series(),
    aug2025_age_2_to_64_series(),
    aug2025_age_65_plus_series(),
];
```

If const limitations make this awkward, use `fn covid_series_policies() -> Vec<CovidSeriesPolicy>` or `once_cell` only if the project already has an acceptable dependency path. Avoid adding crates unless needed.

## Source traceability

Each policy should include source references to Java ICE files. Do not copy large Java rule text into Rust comments. Keep references compact:

```rust
pub struct IceSourceRefs {
    pub series_selection: &'static [&'static str],
    pub evaluation: &'static [&'static str],
    pub recommendation: &'static [&'static str],
    pub yaml: &'static [&'static str],
    pub notes: &'static [&'static str],
}
```

Example references:

```rust
IceSourceRefs {
    series_selection: &["SeriesSelection.drl:879-891"],
    evaluation: &["Evaluation^COVID19^Aug2025Season.dslr:513-609"],
    recommendation: &["Recommendation^COVID19^Aug2025Season.dslr:273-505"],
    yaml: &["covid_19_aug_2025_lt_2_series.yml"],
    notes: &["LT2 can skip target dose 1 based on prior Moderna or two prior valid pre-season doses"],
}
```

## COVID dose fact model

Before evaluating a dose, normalize it into a COVID-specific fact:

```rust
pub struct CovidDoseFact<'a> {
    pub raw: &'a Dose,
    pub season: CovidSeason,
    pub product_family: CovidProductFamily,
    pub relationship_to_selected_series: CvxRelationship,
    pub age_at_dose: PatientAgeFacts,
    pub age_at_season_start: PatientAgeFacts,
    pub supported: bool,
}
```

This is important because forecasting code should not repeatedly search raw history and rediscover meaning from CVX/date combinations.

## COVID evaluated state model

Forecasting should consume explicit evaluated state, not raw dose history:

```rust
pub struct CovidEvaluatedState<'a> {
    pub selected_series: &'a CovidSeriesPolicy,
    pub dose_facts: Vec<CovidDoseFact<'a>>,
    pub evaluations: Vec<CovidDoseEvaluation>,
    pub latest_not_ignored_covid_shot: Option<usize>,
    pub latest_valid_current_season_shot: Option<usize>,
    pub latest_invalid_current_season_nonseries_shot: Option<usize>,
    pub latest_invalid_forecast_anchor_shot: Option<usize>,
}
```

The forecast layer should ask named questions of this state:

```rust
state.latest_invalid_forecast_anchor_shot()
state.has_current_season_valid_dose()
state.current_season_valid_dose_count()
state.latest_not_ignored_covid_shot()
```

That is preferable to embedding raw history filters in each forecast branch.

## Trace output

Add a COVID parity trace mode before moving large behavior into the new table. The trace should be compact and designed for failure analysis.

Example desired trace:

```text
active_series = Aug2025Age2To64
patient_age_at_eval = 17y

Dose 2025-08-27 CVX 310:
  season = Aug2025
  product_family = PfizerPediatric
  relationship = CovidButNotThisSeries
  eval_series_key = Aug2025Age2To64
  interval_anchor = false
  forecast_anchor = true
  status = Invalid
  target_dose = 1

Forecast:
  rule = adult_invalid_old_nonseries_single
  anchor = 2025-08-27
  interval = 56d
  due = 2025-10-22
```

This should be generated from the same normalized facts and policies that the implementation uses.

## Implementation checklist

### Phase 0: Baseline and safety

- [ ] Confirm the working copy is clean.
- [ ] Build the runner with the constrained command:

  ```bash
  cargo build --release --locked --no-default-features --bin test_runner -q
  ```

- [ ] Regenerate current COVID summaries:

  ```bash
  target/release/test_runner run tests/suite.ltp \
    --group COVID19 \
    --summary-only \
    --summary /tmp/covid_policy_refactor_suite_before.json

  target/release/test_runner run tests/fuzz-bank.ltp \
    --group COVID19 \
    --summary-only \
    --summary /tmp/covid_policy_refactor_fuzz_before.json
  ```

- [ ] Record the current baselines in the implementation commit message or notes.
- [ ] Keep each behavior-changing step net-positive on both `tests/suite.ltp --group COVID19` and `tests/fuzz-bank.ltp --group COVID19`, unless deliberately accepting a temporary refactor-only commit with zero behavior change.

### Phase 1: Add policy types with no behavior change

- [ ] Add `src/rules/covid19/seasons.rs`.
- [ ] Move or mirror season boundary logic into `CovidSeason` helpers.
- [ ] Add `src/rules/covid19/products.rs`.
- [ ] Add `CovidProductFamily` and basic CVX-to-product-family classification.
- [ ] Add `src/rules/covid19/policy.rs`.
- [ ] Define `CovidSeriesId`, `CovidSeriesPolicy`, `SeriesSelectionPolicy`, `DoseIdentityPolicy`, `OverflowPolicy`, `IntervalPolicy`, `EvaluationPolicy`, `ForecastPolicy`, and `IceSourceRefs`.
- [ ] Add `src/rules/covid19/series.rs`.
- [ ] Encode an initial `COVID_SERIES_POLICIES` table or equivalent constructor list.
- [ ] Keep this phase behavior-neutral.
- [ ] Validate that all tests are unchanged.
- [ ] Commit as a pure modeling/documentation step.

### Phase 2: Populate the initial policy table from known Java evidence

- [ ] Encode known season boundaries:
  - [ ] Dec 2020 starts `2020-12-14`.
  - [ ] Sep 2023 starts `2023-09-12`.
  - [ ] Aug 2024 starts `2024-08-22`.
  - [ ] Aug 2025 starts `2025-08-27`.
- [ ] Encode Aug 2025 series policies:
  - [ ] `Aug2025Lt2`.
  - [ ] `Aug2025Age2To64`.
  - [ ] `Aug2025Age65Plus`.
- [ ] Encode known Aug 2025 CVX membership:
  - [ ] LT2: `213`, `309`, `310`, `311`, `312`, `313`, `334`.
  - [ ] 2-64: `213`, `309`, `310`, `311`, `312`, `313`, `334`.
  - [ ] 65+: `213`, `309`, `312`, `313`, `334`.
- [ ] Encode known Sep 2023 / Aug 2024 policy placeholders:
  - [ ] Pfizer under-5.
  - [ ] Moderna under-5.
  - [ ] Mixed product under-5.
  - [ ] GTE5.
  - [ ] Novavax.
- [ ] Include compact Java ICE source references on each policy.
- [ ] Keep behavior unchanged.

### Phase 3: Add normalized COVID dose facts

- [ ] Add `CovidDoseFact`.
- [ ] Add product-family classification for supported COVID CVX values.
- [ ] Add unsupported/ignored classification, including current handling for CVX values not present in Java COVID rules.
- [ ] Add `CvxRelationship` calculation for selected series vs non-series COVID products.
- [ ] Add unit tests for classification helpers if the project style supports small unit tests.
- [ ] Keep behavior unchanged or route only obvious pure helper calls through the new facts.

### Phase 4: Add trace output

- [ ] Add `src/rules/covid19/trace.rs`.
- [ ] Add a compact trace structure for selected COVID cases.
- [ ] Include these trace fields:
  - [ ] selected series,
  - [ ] dose season,
  - [ ] product family,
  - [ ] relationship to selected series,
  - [ ] interval-anchor role,
  - [ ] forecast-anchor role,
  - [ ] evaluation status,
  - [ ] target dose identity,
  - [ ] forecast rule name,
  - [ ] forecast anchor and interval.
- [ ] Wire the trace into `test_runner` only if there is already a natural debug/verbose pathway.
- [ ] Otherwise, add an internal helper callable from targeted debugging without changing normal output.
- [ ] Keep normal test behavior unchanged.

### Phase 5: Mirror Java series selection explicitly

- [ ] Add `selection.rs` with functions named after Java concepts.
- [ ] Start with Aug 2025 selection because its Java evidence is already mapped.
- [ ] Preserve these Aug 2025 selection branches:
  - [ ] LT2 when patient is under 2 at evaluation.
  - [ ] LT2 when patient is 2+ but has an in-season COVID dose before age 2.
  - [ ] 2-64 when no in-season target doses and patient is age 2 through 64.
  - [ ] 65+ when no in-season target doses and patient is age 65+.
  - [ ] 2-64 when in-season target dose 1 was administered age 2 through 64.
  - [ ] 65+ when in-season target dose 1 was administered age 65+.
  - [ ] 65+ switch when dose 1 was before age 65 but patient turns 65 within 12 months of season start.
- [ ] Validate Aug 2025 cases only before broader activation.
- [ ] Commit only if behavior is neutral or net-positive.

### Phase 6: Move evaluation concepts out of `overrides.rs`

- [ ] Introduce an evaluation context that includes selected series policy and normalized dose facts.
- [ ] Move dose numbering into named policy logic.
- [ ] Move overflow handling into `OverflowPolicy` handling.
- [ ] Move interval-anchor selection into `IntervalAnchorPolicy` handling.
- [ ] Keep existing `overrides.rs` branches as a bridge until their behavior is represented by policies.
- [ ] Migrate one failure bucket at a time.

### Phase 7: Move forecast concepts out of `overrides.rs`

- [ ] Introduce a forecast context over `CovidEvaluatedState`.
- [ ] Move current Aug 2025 adult forecast rules into named forecast policies.
- [ ] Move LT2 invalid retry rules into named forecast policies.
- [ ] Move old-product invalid retry rules into named forecast policies.
- [ ] Preserve distinction between evaluation interval anchors and forecast anchors.
- [ ] Validate against both official COVID suite and fuzz-bank after each small migration.

### Phase 8: Attack remaining failures by semantic buckets

- [ ] Replace raw mismatch buckets like `delta -55` with semantic buckets from trace output.
- [ ] Track buckets such as:
  - [ ] Aug 2024 pediatric Pfizer dose identity reset.
  - [ ] Sep 2023 mixed-product under-5 routing.
  - [ ] Novavax non-series invalid behavior.
  - [ ] Pre-season invalid forecast anchoring.
  - [ ] Old monovalent accepted overflow.
  - [ ] LT2 invalid interval anchoring.
  - [ ] Adult current-formulation after invalid old product.
- [ ] For each bucket, identify the Java source file/rule before changing Rust behavior.
- [ ] Add or update policy data first, then route behavior through it.

## Suggested first implementation sequence

Do not start with a behavior rewrite. Start with safe scaffolding.

Recommended first four commits:

1. `Add COVID policy model scaffolding`
   - Add enums/structs/modules only.
   - No behavior change.

2. `Encode initial COVID series policies`
   - Add static policy constructors with source refs.
   - No behavior change.

3. `Add COVID dose fact classification helpers`
   - Add season/product/CVX relationship helpers.
   - No behavior change, or only mechanical helper routing with identical results.

4. `Add COVID parity trace support`
   - Add trace output/helper.
   - No normal behavior change.

Only after these are in place should evaluation or forecasting behavior move to the new policy table.

## Validation commands

Use locked, no-default-feature commands unless testing default features is specifically required:

```bash
cargo build --release --locked --no-default-features --bin test_runner -q

target/release/test_runner run tests/suite.ltp \
  --group COVID19 \
  --summary-only \
  --summary /tmp/covid_policy_after_suite.json

target/release/test_runner run tests/fuzz-bank.ltp \
  --group COVID19 \
  --summary-only \
  --summary /tmp/covid_policy_after_fuzz.json

target/release/test_runner run tests/suite.ltp --summary-only
```

For focused case validation:

```bash
target/release/test_runner run tests/suite.ltp \
  --group COVID19 \
  --case '<case-name>' \
  -v

target/release/test_runner run tests/fuzz-bank.ltp \
  --group COVID19 \
  --case '<case-name>' \
  -v
```

## Success criteria

A successful implementation should make failures explainable in terms of Java-equivalent concepts.

Instead of this diagnosis:

```text
Rust forecast date is 56 days late.
```

The trace should support this diagnosis:

```text
Rust selected a collapsed prior LT5 bucket, but Java selected Aug2024PfizerLt5.
The disagreement is dose identity policy: Java treats CVX 308 as season-local dose 1.
```

Instead of this diagnosis:

```text
Rust says Valid and Java says Accepted.
```

The trace should support this diagnosis:

```text
The dose exceeds the selected historical series cap.
Java applies AcceptedKeepsDoseNumber for this series.
Rust counted it in the wrong season/product bucket.
```

## Long-term cleanup target

`overrides.rs` should eventually become orchestration glue:

```rust
pub fn evaluate_covid19(...) -> Vec<DoseEvaluation> {
    let facts = normalize_covid_history(patient, history);
    let selected = select_covid_series(patient, eval_date, &facts);
    let state = evaluate_covid_series(patient, selected, &facts);
    state.evaluations
}

pub fn forecast_covid19(...) -> SeriesForecast {
    let facts = normalize_covid_history(patient, history);
    let selected = select_covid_series(patient, eval_date, &facts);
    let state = evaluate_covid_series(patient, selected, &facts);
    forecast_covid_series(patient, eval_date, &state)
}
```

The detailed behavior should live in named policies, not in one large conditional block.
