pub mod schedules;
pub mod overrides;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "ZOSTER",
        series: vec![
            schedules::zoster_2_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::zoster_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::zoster_custom_evaluation_hook),
        custom_dose_number_hook: None,
        group_selection: None,
    }
}
