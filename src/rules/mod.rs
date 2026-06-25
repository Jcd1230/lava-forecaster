pub mod cross_group;
pub mod helpers;

use crate::engine::{
    ConditionalCompletionRule, CustomCompletionHook, CustomDoseNumberHook, CustomEvaluationHook,
    CustomExtraDoseHook, CustomForecastHook, CustomSwitchHook, GroupSelectionAndPostProcess,
    ParameterOverrideRule, RecommendationOverrideRule, ValidDoseRef,
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
    pub policy: Option<Box<dyn crate::engine::EvaluationPolicy + Send + Sync>>,
}

pub struct LegacyHookPolicy {
    pub custom_forecast_hook: Option<CustomForecastHook>,
    pub custom_switch_hook: Option<CustomSwitchHook>,
    pub custom_evaluation_hook: Option<CustomEvaluationHook>,
    pub custom_dose_number_hook: Option<CustomDoseNumberHook>,
    pub custom_extra_dose_hook: Option<CustomExtraDoseHook>,
    pub custom_completion_hook: Option<CustomCompletionHook>,
}

pub struct CombinedEvaluationPolicy<'a> {
    pub policy: Option<&'a (dyn crate::engine::EvaluationPolicy + Send + Sync)>,
    pub legacy: LegacyHookPolicy,
}

impl crate::engine::EvaluationPolicy for LegacyHookPolicy {
    fn custom_forecast_hook(
        &self,
        patient: &crate::models::Patient,
        valid_doses: &[ValidDoseRef],
        evaluations: &[crate::models::DoseEvaluation],
        history: &[crate::models::Dose],
        eval_date: chrono::NaiveDate,
        forecast: &mut crate::models::SeriesForecast,
    ) {
        if let Some(hook) = self.custom_forecast_hook {
            (hook)(
                patient,
                valid_doses,
                evaluations,
                history,
                eval_date,
                forecast,
            );
        }
    }

    fn custom_switch_hook(
        &self,
        current_series_name: &str,
        target_dose_idx: usize,
        ctx: &crate::engine::EvaluationContext,
    ) -> Option<&'static str> {
        self.custom_switch_hook
            .and_then(|h| (h)(current_series_name, target_dose_idx, ctx))
    }

    fn custom_evaluation_hook(
        &self,
        series_name: &str,
        target_dose_idx: usize,
        ctx: &crate::engine::EvaluationContext,
        reasons: &mut crate::date_utils::SmallVec<[crate::models::EvaluationReason; 4]>,
        status: &mut crate::models::DoseStatus,
    ) {
        if let Some(hook) = self.custom_evaluation_hook {
            (hook)(series_name, target_dose_idx, ctx, reasons, status);
        }
    }

    fn custom_dose_number_hook(
        &self,
        series_name: &str,
        ctx: &crate::engine::EvaluationContext,
    ) -> Option<usize> {
        self.custom_dose_number_hook.map(|h| (h)(series_name, ctx))
    }

    fn custom_extra_dose_hook(
        &self,
        series_name: &str,
        ctx: &crate::engine::EvaluationContext,
    ) -> Option<(
        crate::models::DoseStatus,
        crate::date_utils::SmallVec<[crate::models::EvaluationReason; 4]>,
    )> {
        self.custom_extra_dose_hook
            .and_then(|h| (h)(series_name, ctx))
    }

    fn custom_completion_hook(&self, ctx: &crate::engine::EvaluationContext) -> Option<bool> {
        self.custom_completion_hook.map(|h| (h)(ctx))
    }
}

impl crate::engine::EvaluationPolicy for CombinedEvaluationPolicy<'_> {
    fn same_day_priority(
        &self,
        dose: &crate::models::Dose,
        context: &crate::engine::SameDayPriorityContext,
    ) -> i32 {
        self.policy
            .map(|policy| policy.same_day_priority(dose, context))
            .unwrap_or(0)
    }

    fn compare_same_day_source_order(
        &self,
        left: (usize, &crate::models::Dose),
        right: (usize, &crate::models::Dose),
        context: &crate::engine::SameDayPriorityContext,
    ) -> Option<std::cmp::Ordering> {
        self.policy
            .and_then(|policy| policy.compare_same_day_source_order(left, right, context))
    }

    fn is_evaluation_ignored(
        &self,
        evaluation: &crate::models::DoseEvaluation,
        context: &crate::engine::EvaluationContext,
    ) -> bool {
        self.policy
            .map(|policy| policy.is_evaluation_ignored(evaluation, context))
            .unwrap_or_else(|| crate::engine::default_is_evaluation_ignored(evaluation))
    }

    fn custom_forecast_hook(
        &self,
        patient: &crate::models::Patient,
        valid_doses: &[ValidDoseRef],
        evaluations: &[crate::models::DoseEvaluation],
        history: &[crate::models::Dose],
        eval_date: chrono::NaiveDate,
        forecast: &mut crate::models::SeriesForecast,
    ) {
        if let Some(policy) = self.policy {
            policy.custom_forecast_hook(
                patient,
                valid_doses,
                evaluations,
                history,
                eval_date,
                forecast,
            );
        }
        self.legacy.custom_forecast_hook(
            patient,
            valid_doses,
            evaluations,
            history,
            eval_date,
            forecast,
        );
    }

    fn custom_switch_hook(
        &self,
        current_series_name: &str,
        target_dose_idx: usize,
        ctx: &crate::engine::EvaluationContext,
    ) -> Option<&'static str> {
        self.policy
            .and_then(|policy| policy.custom_switch_hook(current_series_name, target_dose_idx, ctx))
            .or_else(|| {
                self.legacy
                    .custom_switch_hook(current_series_name, target_dose_idx, ctx)
            })
    }

    fn custom_evaluation_hook(
        &self,
        series_name: &str,
        target_dose_idx: usize,
        ctx: &crate::engine::EvaluationContext,
        reasons: &mut crate::date_utils::SmallVec<[crate::models::EvaluationReason; 4]>,
        status: &mut crate::models::DoseStatus,
    ) {
        if let Some(policy) = self.policy {
            policy.custom_evaluation_hook(series_name, target_dose_idx, ctx, reasons, status);
        }
        self.legacy
            .custom_evaluation_hook(series_name, target_dose_idx, ctx, reasons, status);
    }

    fn custom_dose_number_hook(
        &self,
        series_name: &str,
        ctx: &crate::engine::EvaluationContext,
    ) -> Option<usize> {
        self.policy
            .and_then(|policy| policy.custom_dose_number_hook(series_name, ctx))
            .or_else(|| self.legacy.custom_dose_number_hook(series_name, ctx))
    }

    fn custom_extra_dose_hook(
        &self,
        series_name: &str,
        ctx: &crate::engine::EvaluationContext,
    ) -> Option<(
        crate::models::DoseStatus,
        crate::date_utils::SmallVec<[crate::models::EvaluationReason; 4]>,
    )> {
        self.policy
            .and_then(|policy| policy.custom_extra_dose_hook(series_name, ctx))
            .or_else(|| self.legacy.custom_extra_dose_hook(series_name, ctx))
    }

    fn custom_completion_hook(&self, ctx: &crate::engine::EvaluationContext) -> Option<bool> {
        self.policy
            .and_then(|policy| policy.custom_completion_hook(ctx))
            .or_else(|| self.legacy.custom_completion_hook(ctx))
    }

    fn adjust_same_day_target_dose_number(
        &self,
        dose: &crate::models::Dose,
        evaluations: &[crate::models::DoseEvaluation],
    ) -> Option<usize> {
        self.policy
            .and_then(|policy| policy.adjust_same_day_target_dose_number(dose, evaluations))
    }

    fn is_same_day_duplicate(
        &self,
        dose: &crate::models::Dose,
        sorted_history_subset: &[crate::models::Dose],
    ) -> bool {
        self.policy
            .map(|policy| policy.is_same_day_duplicate(dose, sorted_history_subset))
            .unwrap_or_else(|| {
                self.legacy
                    .is_same_day_duplicate(dose, sorted_history_subset)
            })
    }

    fn ignore_evaluation_for_maximum_date(
        &self,
        patient: &crate::models::Patient,
        eval: &crate::models::DoseEvaluation,
    ) -> bool {
        self.policy
            .map(|policy| policy.ignore_evaluation_for_maximum_date(patient, eval))
            .unwrap_or_else(|| {
                self.legacy
                    .ignore_evaluation_for_maximum_date(patient, eval)
            })
    }
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

        pub fn get_all_groups() -> &'static [&'static VaccineGroupDefinition] {
            static ALL_GROUPS: OnceLock<Vec<&'static VaccineGroupDefinition>> = OnceLock::new();

            ALL_GROUPS
                .get_or_init(|| vec![
                    $(get_ruleset($group_id).unwrap(),)*
                ])
                .as_slice()
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
