pub mod schedules;
pub mod overrides;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "HIB",
        series: vec![
            schedules::hib_4_dose_series(),
            schedules::hib_omp_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::hib_custom_forecast_hook),
        custom_switch_hook: Some(overrides::hib_custom_switch_hook),
        custom_evaluation_hook: Some(overrides::hib_custom_evaluation_hook),
        custom_dose_number_hook: Some(overrides::hib_custom_dose_number_hook),
        custom_extra_dose_hook: None,
        group_selection: Some(overrides::hib_group_selection),
    }
}
