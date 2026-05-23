use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod schedules;
pub mod overrides;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "MPOX",
        series: vec![
            schedules::mpox_1_dose_series(),
            schedules::mpox_2_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::mpox_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::mpox_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: Some(overrides::mpox_custom_extra_dose_hook),
        group_selection: Some(overrides::mpox_group_selection),
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::mpox_1_dose_series(),
        schedules::mpox_2_dose_series(),
    ]
}
