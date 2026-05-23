use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod schedules;
pub mod overrides;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "H1N1",
        series: vec![
            schedules::h1n1_1_dose_series(),
            schedules::h1n1_2_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::h1n1_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::h1n1_custom_evaluation_hook),
        custom_dose_number_hook: None,
        group_selection: Some(overrides::h1n1_group_selection),
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::h1n1_1_dose_series(),
        schedules::h1n1_2_dose_series(),
    ]
}
