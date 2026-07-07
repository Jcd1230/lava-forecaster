use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod facts;
pub mod forecasting;
pub mod overrides;
pub mod policy;
pub mod products;
pub mod schedules;
pub mod seasons;
pub mod selection;
pub mod series;
pub mod state;
pub mod trace;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "COVID19",
        series: vec![
            schedules::covid19_aug2025_lt2_series(),
            schedules::covid19_aug2025_2y_to_64y_series(),
            schedules::covid19_aug2025_gte65_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::covid19_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::covid19_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: None,
        custom_completion_hook: None,
        group_selection: Some(overrides::covid19_group_selection),
        policy: None,
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::covid19_aug2025_lt2_series(),
        schedules::covid19_aug2025_2y_to_64y_series(),
        schedules::covid19_aug2025_gte65_series(),
    ]
}
