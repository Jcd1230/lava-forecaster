pub mod helpers;

use crate::engine::{
    ConditionalCompletionRule, CustomDoseNumberHook, CustomEvaluationHook, CustomExtraDoseHook,
    CustomForecastHook, CustomSwitchHook, GroupSelectionAndPostProcess, ParameterOverrideRule,
    RecommendationOverrideRule, CustomCompletionHook,
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
    pub custom_completion_hook: Option<CustomCompletionHook>,
    pub group_selection: Option<GroupSelectionAndPostProcess>,
}

macro_rules! register_groups {
    ($($group_id:literal => $module:ident),* $(,)?) => {
        $(pub mod $module;)*

        pub fn get_ruleset(group_name: &str) -> Option<&'static VaccineGroupDefinition> {
            static REGISTRY: OnceLock<HashMap<&'static str, VaccineGroupDefinition>> = OnceLock::new();

            let map = REGISTRY.get_or_init(|| {
                let mut m = HashMap::new();
                $(m.insert($group_id, $module::definition());)*
                m
            });

            map.get(group_name)
        }

        pub fn get_all_groups() -> Vec<&'static VaccineGroupDefinition> {
            vec![
                $(get_ruleset($group_id).unwrap(),)*
            ]
        }
    };
}

register_groups! {
    "POLIO" => polio,
    "HEP_A" => hepa,
    "MMR" => mmr,
    "VARICELLA" => varicella,
    "ZOSTER" => zoster,
    "DTP" => dtp,
    "HEP_B" => hep_b,
    "HPV" => hpv,
    "HIB" => hib,
    "PNEUMOCOCCAL" => pneumococcal,
    "MCV" => mcv,
    "MENB" => menb,
    "ROTAVIRUS" => rotavirus,
    "INFLUENZA" => influenza,
    "CHOLERA" => cholera,
    "TYPHOID" => typhoid,
    "YELLOW_FEVER" => yellow_fever,
    "JEV" => jev,
    "H1N1" => h1n1,
    "MPOX" => mpox,
    "RSV" => rsv,
    "COVID19" => covid19,
}
