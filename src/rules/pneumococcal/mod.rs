pub mod overrides;
pub mod schedules;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "PNEUMOCOCCAL",
        series: vec![schedules::pneumococcal_series()],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::pneumococcal_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::pneumococcal_custom_evaluation_hook),
        custom_dose_number_hook: Some(overrides::pneumococcal_custom_dose_number_hook),
        custom_extra_dose_hook: None,
        group_selection: None,
    }
}
