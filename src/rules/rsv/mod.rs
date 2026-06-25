use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod overrides;
pub mod schedules;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "RSV",
        series: vec![
            schedules::rsv_infant_series(),
            schedules::rsv_adult_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::rsv_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::rsv_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: Some(overrides::rsv_custom_extra_dose_hook),
        custom_completion_hook: None,
        group_selection: Some(overrides::rsv_group_selection),
        policy: Some(Box::new(overrides::RsvPolicy)),
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::rsv_infant_series(),
        schedules::rsv_adult_series(),
    ]
}
