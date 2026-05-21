pub mod polio;
pub mod hepa;
pub mod mmr;

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
        m
    });
    
    map.get(group_name)
}

pub fn get_all_groups() -> Vec<&'static VaccineGroupDefinition> {
    vec![
        get_ruleset("POLIO").unwrap(),
        get_ruleset("HEP_A").unwrap(),
        get_ruleset("MMR").unwrap(),
    ]
}
