pub mod schedules;
pub mod overrides;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "POLIO",
        series: vec![
            schedules::polio_4_dose_series(),
            schedules::polio_fipv_series(),
        ],
        param_overrides: overrides::polio_parameter_overrides(),
        completion_rules: overrides::polio_completion_rules(),
        rec_overrides: overrides::polio_recommendation_overrides(),
        custom_forecast_hook: Some(overrides::polio_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::polio_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: Some(overrides::polio_custom_extra_dose_hook),
        custom_completion_hook: Some(overrides::polio_custom_completion_hook),
        group_selection: Some(overrides::polio_group_selection),
    }
}
