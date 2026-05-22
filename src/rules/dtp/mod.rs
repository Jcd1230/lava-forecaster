use crate::rules::VaccineGroupDefinition;
use crate::engine::ConditionalCompletionRule;
use crate::schedule::CompiledSeries;

pub mod schedules;
pub mod overrides;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "DTP",
        series: vec![
            schedules::dtp_3_dose_series(),
            schedules::dtp_5_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: vec![
            ConditionalCompletionRule {
                description: "DTP 3-Dose Adult Series Completion Exception (needs at least 1 pertussis component)",
                condition: overrides::dtp_3_dose_completion_condition,
            },
            ConditionalCompletionRule {
                description: "DTP 5-Dose Child Series Exception 1 (3-dose completion at >= 7 years with dose 1 >= 12 months and a dose >= 4 years)",
                condition: overrides::dtp_5_dose_exception_1_condition,
            },
            ConditionalCompletionRule {
                description: "DTP 5-Dose Child Series Exception 2 (4-dose completion if 4th dose >= 4 years and interval 3->4 is >= 6 months - 4 days)",
                condition: overrides::dtp_5_dose_exception_2_condition,
            },
        ],
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::dtp_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::dtp_custom_evaluation_hook),
        group_selection: Some(overrides::dtp_group_selection),
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::dtp_3_dose_series(),
        schedules::dtp_5_dose_series(),
    ]
}
