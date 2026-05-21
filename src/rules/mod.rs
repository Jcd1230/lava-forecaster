pub mod polio;

use std::collections::HashMap;
use std::sync::OnceLock;
use crate::schedule::CompiledSeries;
use crate::engine::{
    ParameterOverrideRule, ConditionalCompletionRule, RecommendationOverrideRule, CustomForecastHook
};

pub struct VaccineGroupDefinition {
    pub group_name: &'static str,
    pub series: Vec<CompiledSeries>,
    pub param_overrides: Vec<ParameterOverrideRule>,
    pub completion_rules: Vec<ConditionalCompletionRule>,
    pub rec_overrides: Vec<RecommendationOverrideRule>,
    pub custom_forecast_hook: Option<CustomForecastHook>,
}

pub fn get_ruleset(group_name: &str) -> Option<&'static VaccineGroupDefinition> {
    static REGISTRY: OnceLock<HashMap<&'static str, VaccineGroupDefinition>> = OnceLock::new();
    
    let map = REGISTRY.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("POLIO", polio::definition());
        m
    });
    
    map.get(group_name)
}
