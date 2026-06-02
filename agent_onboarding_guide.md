# Onboarding & Porting Guide: Implementing Vaccine Groups in Rust

This guide provides step-by-step instructions, codebase pointers, and concrete Rust DSL examples to help developers and AI agents implement new vaccine groups and series in the **LAVA Forecaster**.

---

## 1. Directory Structure

The Rust forecaster utilizes a modular compile-time DSL architecture under `src/rules/`:

```
src/
├── rules/
│   ├── mod.rs               # Central thread-safe Vaccine Group registry
│   ├── <vaccine_group>/     # E.g., mmr/, varicella/, polio/, hepa/
│   │   ├── mod.rs           # Group registration, exports custom hook bindings
│   │   ├── schedules.rs     # Static CompiledSeries definitions (DSL builder)
│   │   └── overrides.rs     # Custom evaluation, switch, forecast, and selection logic
├── date_utils.rs            # Date manipulation (TimePeriod, compare_elapsed, etc.)
├── engine.rs                # Core chronological evaluation and forecasting engine
├── schedule.rs              # Structs for CompiledSeries, TargetDose, and IntervalRule
└── models.rs                # Core domain data models (Dose, Patient, Forecast)
```

---

## 1b. Error Handling Conventions

The forecaster uses a single `ForecasterError` type (in `src/errors.rs`) for all fallible operations. When writing override code in vaccine group modules:

- **Use `add_years_unchecked()` / `add_months_unchecked()`** for age computations with known-valid patient birth dates (these will never fail on real clinical data).
- **Use `add_years()` / `add_months()`** (which return `Result`) only when processing untrusted external input or in contexts where graceful error propagation is needed.
- **Use `TimePeriod::add_to()`** freely in schedule evaluation — it uses descriptive `expect()` messages since schedule periods are compile-time constants.
- **Never use bare `.unwrap()`**. Prefer `.ok_or("context message")?` or `.expect("invariant description")` so failures are diagnosable.

---

## 2. Step-by-Step Implementation Workflow

To port a new vaccine group from Java ICE supporting data (under `opencds-decision-support-service/src/main/resources/data/.../Series/`):

### Step 0: Scaffold the Module
Run the scaffold script via `mise` to automatically generate the module directory, files, registrations, and test case placeholder:
```bash
mise run scaffold <group_lower> [GROUP_UPPER]
```
Example:
```bash
mise run scaffold menb MENB
```
This generates `src/rules/menb/`, wires it into `src/rules/mod.rs`, and creates `tests/relative/menb.json`.

### Step A: Define the Schedules (`schedules.rs`)
Create `src/rules/<vaccine_group>/schedules.rs` (or modify the scaffolded stub) and translate the series YAML definitions into our type-safe internal builder DSL.

#### Example: Varicella 2-Dose Series
```rust
use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn varicella_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("21"), cvx!("94")]; // Varicella, MMRV

    CompiledSeries::builder("VARICELLA_2_DOSE_SERIES")
        .code("VARICELLA_2_DOSE_SERIES")
        .vaccine_group("VARICELLA")
        .num_doses(2)
        // Dose 1 Requirements
        .dose(1, |d| d
            .abs_min_age("1y-4d")
            .min_age("1y")
            .earliest_recommended_age("1y")
            .latest_recommended_age("16m+4w")
            .cvx(allowed_cvx)
        )
        // Dose 2 Requirements
        .dose(2, |d| d
            .abs_min_age("13m")
            .min_age("15m")
            .earliest_recommended_age("4y")
            .latest_recommended_age("7y+4w")
            .cvx(allowed_cvx)
        )
        // Dose Interval 1 -> 2 Requirements
        .interval(1, 2, |i| i
            .abs_min_interval("28d")
            .min_interval("84d")
            .earliest_recommended_interval("84d")
            .latest_recommended_interval("6y+4w")
        )
        .build()
}
```

*Note: Time period strings like `"1y-4d"` (1 year minus 4 days) or `"16m+4w"` (16 months plus 4 weeks) are compiled and cached as static `TimePeriod` objects at startup, avoiding any runtime allocations or regex parsing during evaluation loops.*

---

### Step B: Implement Custom Logic & Overrides (`overrides.rs`)
Create `src/rules/<vaccine_group>/overrides.rs`. If the vaccine group contains logic exceptions not covered by standard CDSi age/interval parameters (e.g., live virus constraints, series-switching, pre-1980 age exemptions), implement them as closure hooks.

#### 1. Custom Evaluation Hook (`custom_evaluation_hook`)
Fires during chronological dose validation. Allows modifying the status or adding custom evaluation reasons.

```rust
use crate::date_utils::TinyVec;
use crate::engine::EvaluationContext;
use crate::models::{DoseStatus, EvaluationReason};

pub fn varicella_custom_evaluation_hook(
    _series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut TinyVec<EvaluationReason, 4>,
    status: &mut DoseStatus,
) {
    if let Some(dose) = ctx.current_dose {
        // Example: Relax absolute minimum interval to 24 days if second dose given at age >= 13 years
        if target_dose_idx == 2 {
            let age_13y = crate::date_utils::add_years(ctx.patient.birth_date, 13);
            if dose.date >= age_13y {
                if let Some((prev_date, _)) = ctx.valid_doses.last() {
                    let days_diff = (dose.date - *prev_date).num_days();
                    if days_diff >= 24 && days_diff < 28 {
                        // Override 'BelowMinimumInterval' and accept the dose
                        *status = DoseStatus::Valid;
                        reasons.retain(|r| *r != EvaluationReason::BelowMinimumInterval);
                    }
                }
            }
        }
    }
}
```

#### 2. Custom Forecast Hook (`custom_forecast_hook`)
Fires after evaluation is done, during the forecasting phase. Allows modifying the final recommended/earliest dates or status.

```rust
use crate::models::{Patient, Dose, SeriesForecast, SeriesStatus};
use chrono::NaiveDate;

pub fn varicella_custom_forecast_hook(
    patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    _history: &[Dose],
    _eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    // Example: Patients born prior to 1980 are immune by history unless series is already complete
    let pre_1980 = NaiveDate::from_ymd_opt(1980, 1, 1).unwrap();
    if patient.birth_date < pre_1980 && forecast.status != SeriesStatus::Complete {
        forecast.status = SeriesStatus::ConditionallyRecommended;
        forecast.reasons = crate::reasons!["CONDITIONAL"];
        forecast.earliest_date = None;
        forecast.recommended_date = None;
        forecast.overdue_date = None;
    }
}
```

#### 3. Custom Series Switch Hook (`custom_switch_hook`)
Fires dynamically during chronological evaluation of patient history. Allows switching the active evaluation schedule mid-history (e.g., switching to Twinrix series if Twinrix is administered).

```rust
pub fn hepa_custom_switch_hook(
    current_series_name: &str,
    _target_dose_idx: usize,
    ctx: &EvaluationContext,
) -> Option<&'static str> {
    if let Some(dose) = ctx.current_dose {
        // If Twinrix (CVX 104) is administered and evaluating the 2-Dose series, switch to Twinrix Adult Series
        if dose.cvx == "104" && current_series_name == "HEP_A_2_DOSE_CHILD_ADULT_SERIES" {
            return Some("HEP_A_ADULT_3_DOSE_SERIES");
        }
    }
    None
}
```

---

### Step C: Expose the Ruleset (`mod.rs`)
Create `src/rules/<vaccine_group>/mod.rs` to bundle the group definition and register schedules and hooks.

```rust
pub mod schedules;
pub mod overrides;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "VARICELLA",
        series: vec![schedules::varicella_2_dose_series()],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::varicella_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::varicella_custom_evaluation_hook),
        custom_dose_number_hook: None,
        group_selection: None,
    }
}
```

---

### Step D: Register with the Global Registry (`src/rules/mod.rs`)
Open `src/rules/mod.rs` and register the new group in the static thread-safe registry:

```rust
// 1. Declare the module
pub mod varicella;

// 2. Add registration inside get_ruleset()
m.insert("VARICELLA", varicella::definition());

// 3. Add the group to get_all_groups()
pub fn get_all_groups() -> Vec<&'static VaccineGroupDefinition> {
    vec![
        get_ruleset("VARICELLA").unwrap(),
    ]
}
```

---

### Step E: Add Same-Day Priority Rules (`src/engine.rs`)
If the group contains combined/combination vaccines (e.g. CVX 94 MMRV), we must prefer the combination vaccine over the single-antigen vaccine if given on the same day.
Register the priorities in `get_same_day_priority` inside `src/engine.rs`:

```rust
fn get_same_day_priority(group: &str, cvx: &str, birth_date: NaiveDate, dose_date: NaiveDate) -> i32 {
    match group {
        "VARICELLA" => match cvx {
            "94" => 0, // Prefer MMRV (lower is higher priority)
            "21" => 1, // Single antigen Varicella
            _ => 2,
        },
        _ => 0,
    }
}
```

---

## 3. Testing and Verification

To verify that the implementation is 100% logically equivalent to the legacy Drools-based Java ICE engine, we execute 1:1 diff comparisons.

### Essential Commands

All testing and verification commands are managed via `mise`:

| Command | Description |
|---|---|
| `mise run test-compare -- --group <name>` | Compares LAVA Forecaster outputs against live Java. **Auto-records missing expected snapshots.** |
| `mise run test -- --group <name>` | Runs LAVA Forecaster verification against recorded snapshots. **Does not require Java.** |
| `mise run test-group -- <name>` | Shortcut for LAVA Forecaster verification of one vaccine group against recorded snapshots. |
| `mise run test-record -- --group <name>` | Queries Java ICE and records snapshots. |

### Workflow for a New Vaccine Group

1. **Start the Java ICE server** in a separate terminal:
   ```bash
   mise run run
   ```
2. **Run comparison tests** for your group:
   ```bash
   mise run test-compare -- --group <group_lower>
   ```
   *Note: If no `<group_lower>.expected.json` file exists yet, the runner will automatically detect it and query Java to record the snapshot before running the comparison. This collapses the recording and comparison into a single step.*
3. **Verify single test cases** (if debugging a discrepancy):
   ```bash
   mise run test-compare -- --group <group_lower> --case <test_case_name> --verbose
   ```
   *Note: By default, the test runner is quiet and only outputs errors and a summary. Pass `-v` or `--verbose` to view details for passing tests.*
4. **Run offline regression tests** (without requiring the Java server):
   ```bash
   mise run test -- --group <group_lower>
   ```
   Or use the shortcut:
   ```bash
   mise run test-group -- <group_lower>
   ```

### Workflow for Fixing an Existing Parity Bucket

When working an already-ported group that still differs from Java, use a tighter compare loop:

1. Produce a fresh full-CDSi baseline log and keep it under `tests/relative/tmp/`:
    ```bash
    cargo run --release --bin test_runner -- --run tests/cases --compare > tests/relative/tmp/ice_cdsi_compare_<label>.txt 2>&1
    ```
2. Pull the target group's failures from that log and cluster them by behavior rather than by individual case name.
3. Run the target group only:
    ```bash
    cargo run --release --bin test_runner -- --run tests/cases --group <GROUP_UPPER> --compare
    ```
4. For edge-case debugging, run a single case with full output:
    ```bash
    cargo run --release --bin test_runner -- --run tests/cases --group <GROUP_UPPER> --case <test_case_name> --compare -v
    ```
5. After the group passes, run a new full-CDSi compare into a second saved log.
6. Compare per-group failure counts between the old and new logs to confirm that only the target bucket moved.

### Source-of-Truth Map for Existing Buckets

For parity fixes on already-ported groups, use this reading order before changing code:

1. Saved full compare log under `tests/relative/tmp/` to identify the current bucket and representative failure shapes.
2. Targeted group compare output to confirm the current mismatch set.
3. Raw case inputs in `tests/relative/<group>.json`.
4. Expected Java snapshots in `tests/relative/<group>.expected.json`.
5. Java `Evaluation^<Group>.dslr` for dose validity and reason mismatches.
6. Java `Recommendation^<Group>.dslr` for forecast date and forecast status mismatches.
7. Java `SeriesSelection.drl` if the selected series or effective dose numbering looks wrong.
8. Supporting-data YAML under `ice-supporting-data/Series/`.
9. Rust `overrides.rs`, then `mod.rs`, then `schedules.rs`.

The raw case JSON wins over the case label if the wording and dates disagree.

### Mismatch-to-Code Routing

- Dose evaluation mismatch:
    - Read Java evaluation rules first.
    - Then inspect Rust `custom_evaluation_hook` and any dose-number logic.
- Forecast date or forecast status mismatch:
    - Read Java recommendation rules first.
    - Then inspect Rust `custom_forecast_hook`.
- Wrong selected series or wrong dose numbering:
    - Read Java series-selection rules and supporting-data YAML.
    - Then inspect Rust `group_selection`, `custom_switch_hook`, and `custom_dose_number_hook`.
- Same-day duplicate issues:
    - Check engine same-day priority handling before editing the vaccine-group module.

For already-ported groups, check selection and overrides before changing `schedules.rs`. Many parity defects come from post-processing semantics rather than schedule-table definitions.

### Command Choice: Raw Runner vs `mise`

- Prefer raw `cargo run --release --bin test_runner -- --run tests/cases --compare ...` commands while debugging an existing parity bucket so saved logs and runner behavior stay explicit.
- Use `mise run test-record` or `mise run test-compare` when you intentionally want recording behavior.
- Use `mise run scaffold` when starting a genuinely new group.

### Direct Rust Inspection for Ambiguous Cases

If compare output still does not make the failure mode obvious, run the Rust binary directly on a one-off request file.

1. Save a simplified or legacy-format request JSON in the workspace.
2. From the Rust crate directory, run:
    ```bash
    cargo run -- /path/to/request.json
    ```
3. Read stderr for the parsed patient/history debug output.
4. Read stdout for the full forecast response JSON.

This is the fastest way to inspect selected vaccine-group output, evaluations, and forecast dates without involving the Java compare harness.

### Quick Compare-Log Commands

- Check whether a bucket is still present in a full compare log:
  ```bash
  rg 'CDSI_HPV' tests/relative/tmp/ice_cdsi_compare_<label>.txt
  ```
  No matches means that group no longer has failures.

- Diff failure counts between two saved full-CDSi logs:
  ```bash
  awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' tests/relative/tmp/ice_cdsi_compare_before.txt | sort > tests/relative/tmp/before.counts
  awk '/FAIL:/{g=$NF; sub(/[()]/, "", g); sub(/[()]/, "", g); count[g]++} END {for (g in count) print g, count[g]}' tests/relative/tmp/ice_cdsi_compare_after.txt | sort > tests/relative/tmp/after.counts
  join -a1 -a2 -e0 -o 0,1.2,2.2 tests/relative/tmp/before.counts tests/relative/tmp/after.counts | awk '$2 != $3'
  ```

### Debugging Heuristics That Save Time

- Verify **selected series** before changing interval or age rules. Several groups differ from Java because the wrong series is selected, not because the active series rule is wrong.
- Forecast dates often key off **valid/effective dose history**, not all administered doses. If Rust dates are too early, compare against Java using only valid doses in the selected series.
- Be cautious with adult policy rules such as `ConditionallyRecommended` and `NotRecommended`: Java often applies them only when no relevant series history exists. Started series frequently remain `NotComplete` with real forecast dates.
- `Accepted` does not necessarily mean “counts toward completion.” If Java treats the dose as ignored for completion, Rust may need `Accepted` plus `OutsideRoutineSeries`.
- If a compare case is still confusing, run the Rust binary directly on a one-off request JSON and inspect the raw `selected_series`, evaluations, and forecast output.

### Formatting Scope

Avoid broad `cargo fmt` / `rustfmt` while porting a single vaccine group unless you intend to accept formatting changes across the Rust module tree. Because the crate uses `mod.rs` declarations for all vaccine groups, formatting from the crate root can touch sibling modules unrelated to the current task. Keep formatting scoped to files you intentionally changed and review `jj diff --name-only` before committing.

---

## 4. Drools DSL → Rust Mapping Gotchas

When writing overrides, do not trust the Drools DSL rules literally. The Java engine often maps abstract Drools rule actions to different output states in the XML response.

### 1. "Mark the shot as Ignored" vs. "Accepted"
In Drools rules (e.g. MCV under-10y doses), you will see:
`Mark the shot $currentShot as Ignored`
*Gotcha:* This does **not** map to `DoseStatus::Ignored` in the Java REST/XML output. It maps to `DoseStatus::Accepted` with the reason `BelowMinimumAge` (or similar). The shot is "ignored" in terms of series advancement, but it is still accepted in the output. Always verify against the XML/snapshot output!

### 2. "COMPLETE_HIGH_RISK" vs. "Complete"
When a series is completed, recommendation rules might evaluate to `NOT_RECOMMENDED` with a sub-reason like `COMPLETE_HIGH_RISK`.
*Gotcha:* The legacy Java engine's mapping layer maps this combination to `SeriesStatus::Complete` in the final output, not `NotRecommended`. Series completion status takes precedence.

---

## 5. Focus Code Reference Table

Every test suite requires a `focus` code (which tells the XML parser which vaccine group forecast to evaluate). Here is the complete lookup table:

| Rust Group Name | Java Concept Code | Focus Code | Display Name |
|---|---|---|---|
| `HEP_B` | `HEP_B` | `100` | Hep B Vaccine Group |
| `DTP` | `DTP` | `200` | DTP Vaccine Group |
| `HIB` | `HIB` | `300` | Hib Vaccine Group |
| `POLIO` | `POLIO` | `400` | Polio Vaccine Group |
| `MMR` | `MMR` | `500` | MMR Vaccine Group |
| `VARICELLA` | `VARICELLA` | `600` | Varicella Vaccine Group |
| `ZOSTER` | `ZOSTER` | `620` | Zoster Vaccine Group |
| `PNEUMOCOCCAL` | `PNEUMOCOCCAL` | `750` | Pneumococcal Vaccine Group |
| `HEP_A` | `HEP_A` | `810` | Hep A Vaccine Group |
| `MCV` | `MENINGOCOCCAL_ACWY` | `830` | Meningococcal ACWY Vaccine Group |
| `HPV` | `HPV` | `840` | HPV Vaccine Group |
| `MENB` | `MENINGOCOCCAL_B` | `835` | Meningococcal B Vaccine Group |
| `INFLUENZA` | `INFLUENZA` | `800` | Influenza Vaccine Group |
| `ROTAVIRUS` | `ROTAVIRUS` | `820` | Rotavirus Vaccine Group |
| `COVID19` | `COVID_19` | `850` | COVID-19 Vaccine Group |
| `MPOX` | `MPOX` | `860` | Mpox Vaccine Group |
| `RSV` | `RSV` | `875` | RSV Vaccine Group |
| `H1N1` | `INFLUENZA_H1N1` | `890` | H1N1 Influenza Vaccine Group |
| `CHOLERA` | `CHOLERA` | `901` | Cholera Vaccine Group |
| `JEV` | `JAPANESE_ENCEPHALITIS` | `902` | Japanese Encephalitis Vaccine Group |
| `TYPHOID` | `TYPHOID` | `904` | Typhoid Vaccine Group |
| `YELLOW_FEVER` | `YELLOW_FEVER` | `905` | Yellow Fever Vaccine Group |

---

## 6. Vaccine-Specific Minimum Ages

Some vaccine groups have age-based exceptions where a dose given below the series-level `absolute-minimum-age` is still evaluated as `Accepted` (rather than `Invalid`).

- **Where they live:** These are defined on a per-CVX basis in `supportedVaccines.yml` under `ice-supporting-data/OtherLists/`.
- **How they are used:** In custom evaluation hooks, check the dose CVX against its vaccine-specific minimum age. If the dose date is greater than or equal to the vaccine minimum age, override the status to `DoseStatus::Accepted` and clear/add the appropriate `EvaluationReason` (such as `BelowMinimumAge`).

---

## 7. Legacy Java ICE Quirks & Edge Cases

When translating rules from the legacy Java Drools engine, be aware of the following quirks:

### CVX Code Edge Cases & Combination Vaccines
Some combination vaccines (like CVX 50 / TriHIBit) or mapping definitions are not explicitly listed in the primary series YAML files (e.g., `Hib4DoseSeries.yml`). Instead, they may be found in:
- Alternative series YAMLs (e.g., `HibOMPSeries.yml`).
- Central mapping files like `cdm.xml` under `opencds-decision-support-service/src/main/resources/config/conceptDeterminationMethods/`.
If a CVX code behaves strangely and isn't in the expected YAML, check `cdm.xml`.

### Forecast Status Triggers (`ConditionallyRecommended`)
The legacy engine sometimes forces a series forecast status to `ConditionallyRecommended` and clears all forecast dates (`earliest_date`, `recommended_date`, `overdue_date`, `latest_date` set to `None`). Common triggers include:
- A patient receiving a booster-only vaccine (e.g., CVX 50) at an invalid age, with no prior valid doses.
- A patient exceeding the absolute maximum age for catch-up (e.g., >= 5 years old for Hib) without completing the series.
If you observe `ConditionallyRecommended` with cleared dates in test outputs, check the custom forecast hook for these types of conditions.

### Test Case Default Focus Codes
By default, the test DSL parser (`test_dsl.rs`) initializes `focus_code` to `"000"`. If a test case does not declare an explicit `Focus: ` line, the runner automatically resolves the focus code from the `Group` field (e.g. `DTP` -> `200`). This ensures that dynamic compare parsing correctly filters evaluations and forecasts when matching Java XML results.

### ICD-9 Immunity Coding Requirement
Always ensure that patient immunities sent in the XML payload query use **ICD-9-CM** codes (e.g. `070.30` for HepB, OID `2.16.840.1.113883.6.103`), which Java ICE expects in `observationFocus`. Avoid using SNOMED OIDs for immunities as Java ICE's `cdm.xml` might fail to map them to the correct internal concepts in certain execution flows. Correspondingly, `legacy_models.rs` will map returned ICD codes back to internal disease string representations (`"HepB"`, `"Varicella"`, etc.).
