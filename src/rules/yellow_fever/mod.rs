use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod overrides;
pub mod schedules;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "YELLOW_FEVER",
        series: vec![schedules::yellow_fever_risk_series()],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::yellow_fever_custom_forecast_hook),
        custom_switch_hook: None,
        custom_evaluation_hook: Some(overrides::yellow_fever_custom_evaluation_hook),
        custom_dose_number_hook: None,
        group_selection: None,
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![schedules::yellow_fever_risk_series()]
}
