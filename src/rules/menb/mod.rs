use crate::rules::VaccineGroupDefinition;
use crate::schedule::CompiledSeries;

pub mod overrides;
pub mod schedules;

pub fn definition() -> VaccineGroupDefinition {
    VaccineGroupDefinition {
        group_name: "MENB",
        series: vec![
            schedules::men_b_4c_2_dose_series(),
            schedules::men_b_4c_3_dose_series(),
            schedules::men_b_fhbp_2_dose_series(),
            schedules::men_b_fhbp_3_dose_series(),
        ],
        param_overrides: Vec::new(),
        completion_rules: Vec::new(),
        rec_overrides: Vec::new(),
        custom_forecast_hook: Some(overrides::menb_custom_forecast_hook),
        custom_switch_hook: Some(overrides::menb_custom_switch_hook),
        custom_evaluation_hook: Some(overrides::menb_custom_evaluation_hook),
        custom_dose_number_hook: None,
        custom_extra_dose_hook: None,
        group_selection: Some(overrides::menb_group_selection),
    }
}

#[allow(dead_code)]
pub fn get_all_schedules() -> Vec<CompiledSeries> {
    vec![
        schedules::men_b_4c_2_dose_series(),
        schedules::men_b_4c_3_dose_series(),
        schedules::men_b_fhbp_2_dose_series(),
        schedules::men_b_fhbp_3_dose_series(),
    ]
}
