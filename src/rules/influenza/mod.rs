use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod schedules;
pub mod overrides;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "INFLUENZA",
        series: vec![
            schedules::influenza_1_dose_series(),
            schedules::influenza_2_dose_series(),
            schedules::influenza_2_dose_default_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::influenza_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::influenza_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: None,
        custom_completion_hook: None,
        group_selection: Some(overrides::influenza_group_selection),
    }
}

pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::influenza_1_dose_series(),
        schedules::influenza_2_dose_series(),
        schedules::influenza_2_dose_default_series(),
    ]
}
