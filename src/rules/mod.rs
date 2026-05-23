pub mod cholera;
pub mod dtp;
pub mod hep_b;
pub mod hepa;
pub mod hib;
pub mod hpv;
pub mod influenza;
pub mod mcv;
pub mod menb;
pub mod mmr;
pub mod pneumococcal;
pub mod polio;
pub mod rotavirus;
pub mod typhoid;
pub mod varicella;
pub mod yellow_fever;
pub mod zoster;
pub mod jev;
pub mod h1n1;
pub mod mpox;
pub mod rsv;

use crate::engine::{
    ConditionalCompletionRule, CustomDoseNumberHook, CustomEvaluationHook, CustomExtraDoseHook,
    CustomForecastHook, CustomSwitchHook, GroupSelectionAndPostProcess, ParameterOverrideRule,
    RecommendationOverrideRule,
};
use crate::schedule::CompiledSeries;
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct VaccineGroupDefinition {
    pub group_name: &'static str,
    pub series: Vec<CompiledSeries>,
    pub param_overrides: Vec<ParameterOverrideRule>,
    pub completion_rules: Vec<ConditionalCompletionRule>,
    pub rec_overrides: Vec<RecommendationOverrideRule>,
    pub custom_forecast_hook: Option<CustomForecastHook>,
    pub custom_switch_hook: Option<CustomSwitchHook>,
    pub custom_evaluation_hook: Option<CustomEvaluationHook>,
    pub custom_dose_number_hook: Option<CustomDoseNumberHook>,
    pub custom_extra_dose_hook: Option<CustomExtraDoseHook>,
    pub group_selection: Option<GroupSelectionAndPostProcess>,
}

pub fn get_ruleset(group_name: &str) -> Option<&'static VaccineGroupDefinition> {
    static REGISTRY: OnceLock<HashMap<&'static str, VaccineGroupDefinition>> = OnceLock::new();

    let map = REGISTRY.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("POLIO", polio::definition());
        m.insert("HEP_A", hepa::definition());
        m.insert("MMR", mmr::definition());
        m.insert("VARICELLA", varicella::definition());
        m.insert("ZOSTER", zoster::definition());
        m.insert("DTP", dtp::definition());
        m.insert("HEP_B", hep_b::definition());
        m.insert("HPV", hpv::definition());
        m.insert("HIB", hib::definition());
        m.insert("PNEUMOCOCCAL", pneumococcal::definition());
        m.insert("MCV", mcv::definition());
        m.insert("MENB", menb::definition());
        m.insert("ROTAVIRUS", rotavirus::definition());
        m.insert("INFLUENZA", influenza::definition());
        m.insert("CHOLERA", cholera::definition());
        m.insert("TYPHOID", typhoid::definition());
        m.insert("YELLOW_FEVER", yellow_fever::definition());
        m.insert("JEV", jev::definition());
        m.insert("H1N1", h1n1::definition());
        m.insert("MPOX", mpox::definition());
        m.insert("RSV", rsv::definition());
        m
    });

    map.get(group_name)
}

pub fn get_all_groups() -> Vec<&'static VaccineGroupDefinition> {
    vec![
        get_ruleset("POLIO").unwrap(),
        get_ruleset("HEP_A").unwrap(),
        get_ruleset("MMR").unwrap(),
        get_ruleset("VARICELLA").unwrap(),
        get_ruleset("ZOSTER").unwrap(),
        get_ruleset("DTP").unwrap(),
        get_ruleset("HEP_B").unwrap(),
        get_ruleset("HPV").unwrap(),
        get_ruleset("HIB").unwrap(),
        get_ruleset("PNEUMOCOCCAL").unwrap(),
        get_ruleset("MCV").unwrap(),
        get_ruleset("MENB").unwrap(),
        get_ruleset("ROTAVIRUS").unwrap(),
        get_ruleset("INFLUENZA").unwrap(),
        get_ruleset("CHOLERA").unwrap(),
        get_ruleset("TYPHOID").unwrap(),
        get_ruleset("YELLOW_FEVER").unwrap(),
        get_ruleset("JEV").unwrap(),
        get_ruleset("H1N1").unwrap(),
        get_ruleset("MPOX").unwrap(),
        get_ruleset("RSV").unwrap(),
    ]
}
