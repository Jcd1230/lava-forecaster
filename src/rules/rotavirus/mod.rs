use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod schedules;
pub mod overrides;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "ROTAVIRUS",
        series: vec![
            schedules::rotavirus_2_dose_series(),
            schedules::rotavirus_3_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::rotavirus_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::rotavirus_custom_evaluation_hook),
        custom_dose_number_hook: Some(overrides::rotavirus_custom_dose_number_hook),
        custom_extra_dose_hook: None,
        custom_completion_hook: None,
        group_selection: Some(overrides::rotavirus_group_selection),
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::rotavirus_2_dose_series(),
        schedules::rotavirus_3_dose_series(),
    ]
}
