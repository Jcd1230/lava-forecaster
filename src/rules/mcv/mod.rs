use crate::rules::VaccineGroupDefinition;
use crate::engine::ConditionalCompletionRule;
use crate::schedule::CompiledSeries;

pub mod schedules;
pub mod overrides;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "MCV",
        series: vec![
            schedules::mcv_42_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: vec![
            ConditionalCompletionRule {
                description: "MCV 1-Dose Completion >= 16y and < 19y Exception",
                condition: overrides::mcv_completion_condition,
            },
        ],
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::mcv_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::mcv_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: None,
        custom_completion_hook: None,
        group_selection: None,
        policy: None,
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::mcv_42_dose_series(),
    ]
}
