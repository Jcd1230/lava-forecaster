# Onboarding & Porting Guide: Implementing Vaccine Groups in Rust

This guide provides step-by-step instructions, codebase pointers, and concrete Rust DSL examples to help developers and AI agents implement new vaccine groups and series in the **High-Performance Rust ICE Forecaster PoC**.

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

## 2. Step-by-Step Implementation Workflow

To port a new vaccine group from Java ICE supporting data (under `opencds-decision-support-service/src/main/resources/data/.../Series/`):

### Step A: Define the Schedules (`schedules.rs`)
Create `src/rules/<vaccine_group>/schedules.rs` and translate the series YAML definitions into our type-safe internal builder DSL.

#### Example: Varicella 2-Dose Series
```rust
use crate::schedule::CompiledSeries;

pub fn varicella_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &["21", "94"]; // Varicella, MMRV

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
use crate::engine::EvaluationContext;
use crate::models::{DoseStatus, EvaluationReason};

pub fn varicella_custom_evaluation_hook(
    _series_name: &str,
    target_dose_idx: usize,
    ctx: &EvaluationContext,
    reasons: &mut Vec<EvaluationReason>,
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
        forecast.reasons = vec!["CONDITIONAL".to_string()];
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
Create `src/rules/<vaccine_group>/mod.rs` to bundle the engine definition and register schedules and hooks.

```rust
use crate::engine::EvaluationEngine;
use crate::schedule::CompiledSeries;

pub fn definition() -> EvaluationEngine {
    let mut engine = EvaluationEngine::new(schedules::varicella_2_dose_series());
    
    // Wire up the custom override closures
    engine.custom_evaluation_hook = Some(overrides::varicella_custom_evaluation_hook);
    engine.custom_forecast_hook = Some(overrides::varicella_custom_forecast_hook);
    
    engine
}

pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![schedules::varicella_2_dose_series()]
}

mod schedules;
mod overrides;
```

---

### Step D: Register with the Global Registry (`src/rules/mod.rs`)
Open `src/rules/mod.rs` and register the new group in the static thread-safe registry:

```rust
// 1. Declare the module
pub mod varicella;

// 2. Add registration inside get_registry()
fn get_registry() -> &'static HashMap<&'static str, EvaluationEngine> {
    REGISTRY.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("POLIO", polio::definition());
        m.insert("HEP_A", hepa::definition());
        m.insert("MMR", mmr::definition());
        m.insert("VARICELLA", varicella::definition()); // <-- REGISTER HERE
        m
    })
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

### 1. Build the Rust Project
Ensure the project builds cleanly in release mode:
```bash
cargo build --release
```

### 2. Run Python Comparison Tests
Go to the test directory (`curl-rest-tests/`) and execute the automated verification script `compare_outputs.py` against a local running Java ICE instance (`http://localhost:8080/`):

```bash
cd ../curl-rest-tests
python3 compare_outputs.py --group <RUST_GROUP_NAME> --focus <JAVA_CONCEPT_CODE> <PAYLOAD_JSON>
```

#### Example: Verifying Varicella
```bash
python3 compare_outputs.py --group VARICELLA --focus 600 varicella_child_standard.json
```

A successful matching run outputs:
```
Running comparison for varicella_child_standard.json (Group: VARICELLA, Focus Code: 600)...
Patient Birth Date: 2020-01-01

--- Dose Evaluations Comparison ---
Date         | CVX  | Legacy Status   | Rust Status     | Legacy Dose# | Rust Dose#
-------------------------------------------------------------------------------------
  2021-01-01 | 21   | Valid           | Valid           | 1            | 1
  2024-01-01 | 21   | Valid           | Valid           | 2            | 2

--- Forecast Comparison ---
Status:      Legacy=Complete        Rust=Complete        [OK]
Earliest:    Legacy=None            Rust=None            [OK]
Recommended: Legacy=None            Rust=None            [OK]
Overdue:     Legacy=None            Rust=None            [OK]

SUCCESS: 1:1 agreement verified!
```
If there are discrepancies in statuses or forecast dates, the script will highlight them as `[MISMATCH]` to guide debugging.
