pub mod schedules;
pub mod overrides;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "HEP_A",
        series: vec![
            schedules::hepa_2_dose_series(),
            schedules::hepa_adult_3_dose_series(),
            schedules::hepa_4_dose_twinrix_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::hepa_custom_forecast_hook),
        custom_switch_hook: Some(overrides::hepa_custom_switch_hook),
        custom_evaluation_hook: Some(overrides::hepa_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: None,
        group_selection: Some(overrides::hepa_group_selection),
    }
}
