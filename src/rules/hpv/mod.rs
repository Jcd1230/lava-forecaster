pub mod schedules;
pub mod overrides;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "HPV",
        series: vec![
            schedules::hpv_2_dose_series(),
            schedules::hpv_3_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::hpv_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::hpv_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: None,
        custom_completion_hook: None,
        group_selection: Some(overrides::hpv_group_selection),
    }
}
