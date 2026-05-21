pub mod schedules;
pub mod overrides;

use crate::rules::VaccineGroupDefinition;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "POLIO",
        series: vec![schedules::polio_4_dose_series()],
        param_overrides: overrides::polio_parameter_overrides(),
        completion_rules: overrides::polio_completion_rules(),
        rec_overrides: overrides::polio_recommendation_overrides(),
        custom_forecast_hook: Some(overrides::polio_custom_forecast_hook),
    }
}
