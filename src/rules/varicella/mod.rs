pub mod overrides;
pub mod schedules;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "VARICELLA",
        series: vec![schedules::varicella_2_dose_series()],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::varicella_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::varicella_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: None,
        custom_completion_hook: None,
        group_selection: None,
        policy: Some(Box::new(overrides::VaricellaPolicy)),
    }
}
