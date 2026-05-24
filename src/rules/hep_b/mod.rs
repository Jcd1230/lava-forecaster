use crate::rules::VaccineGroupDefinition;
use crate::engine::ConditionalCompletionRule;
use crate::schedule::CompiledSeries;

pub mod schedules;
pub mod overrides;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "HEP_B",
        series: vec![
            schedules::hep_b_3_dose_child_adolescent_series(),
            schedules::hep_b_4_dose_child_adolescent_series(),
            schedules::hep_b_3_dose_twinrix_series(),
            schedules::hep_b_4_dose_accelerated_twinrix_series(),
            schedules::hep_b_adult_2_dose_series(),
            schedules::hep_b_adult_3_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: vec![
            ConditionalCompletionRule {
                description: "HEP_B Adolescent 2-Dose Recombivax Completion Exception",
                condition: overrides::hep_b_adolescent_completion_condition,
            },
        ],
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::hep_b_custom_forecast_hook),
        custom_switch_hook: Some(overrides::hep_b_custom_switch_hook),
        custom_evaluation_hook: Some(overrides::hep_b_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: None,
        custom_completion_hook: None,
        group_selection: Some(overrides::hep_b_group_selection),
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::hep_b_3_dose_child_adolescent_series(),
        schedules::hep_b_4_dose_child_adolescent_series(),
        schedules::hep_b_3_dose_twinrix_series(),
        schedules::hep_b_4_dose_accelerated_twinrix_series(),
        schedules::hep_b_adult_2_dose_series(),
        schedules::hep_b_adult_3_dose_series(),
    ]
}
