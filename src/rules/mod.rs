pub mod polio;
pub mod hepa;
pub mod mmr;
pub mod varicella;
pub mod zoster;
pub mod dtp;
pub mod hep_b;
pub mod hpv;

use std::collections::HashMap;
use std::sync::OnceLock;
use crate::schedule::CompiledSeries;
use crate::engine::{
    ParameterOverrideRule, ConditionalCompletionRule, RecommendationOverrideRule,
    CustomForecastHook, CustomSwitchHook, CustomEvaluationHook, GroupSelectionAndPostProcess,
};

pub struct VaccineGroupDefinition {
    pub group_name: &'static str,
    pub series: Vec<CompiledSeries>,
    pub param_overrides: Vec<ParameterOverrideRule>,
    pub completion_rules: Vec<ConditionalCompletionRule>,
    pub rec_overrides: Vec<RecommendationOverrideRule>,
    pub custom_forecast_hook: Option<CustomForecastHook>,
    pub custom_switch_hook: Option<CustomSwitchHook>,
    pub custom_evaluation_hook: Option<CustomEvaluationHook>,
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
    ]
}


