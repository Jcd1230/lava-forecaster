use chrono::NaiveDate;
use crate::engine::{ParameterOverrideRule, ConditionalCompletionRule, RecommendationOverrideRule};
use crate::date_utils::{TimePeriod, compare_elapsed};
use crate::models::{Patient, SeriesForecast};

pub fn polio_parameter_overrides() -> Vec<ParameterOverrideRule> {
    vec![
        // Pre-2009 Overrides:
        // "If dose 4 administered before 8/7/2009: abs_min_age = 122d, abs_min_interval for dose 3 = 24d"
        ParameterOverrideRule {
            description: "Polio Pre-2009 Dose 4 Absolute Minimums",
            target_dose_number: 4,
            condition: |ctx| {
                if let Some(dose) = ctx.current_dose {
                    dose.date < NaiveDate::from_ymd_opt(2009, 8, 7).unwrap()
                } else {
                    false
                }
            },
            override_abs_min_age: Some(TimePeriod::parse("122d").unwrap()),
            override_abs_min_interval_from_dose: Some((3, TimePeriod::parse("24d").unwrap())),
        }
    ]
}

pub fn polio_completion_rules() -> Vec<ConditionalCompletionRule> {
    vec![
        // Conditional 3-Dose Completion:
        // "Complete with 3 doses if 3 prior valid doses, child >= 4y-4d at dose 3, and interval 2 to 3 is >= 6m-4d"
        ConditionalCompletionRule {
            description: "Polio 3-Dose Completion Rule (Dose 3 at >= 4 years)",
            condition: |ctx| {
                if ctx.valid_doses.len() >= 3 {
                    let dose_3_date = ctx.valid_doses[2].0;
                    let birth_date = ctx.patient.birth_date;
                    
                    let age_ok = compare_elapsed(
                        birth_date,
                        dose_3_date,
                        &TimePeriod::parse("4y-4d").unwrap(),
                    ) != std::cmp::Ordering::Less;
                    
                    let dose_2_date = ctx.valid_doses[1].0;
                    let interval_ok = compare_elapsed(
                        dose_2_date,
                        dose_3_date,
                        &TimePeriod::parse("6m-4d").unwrap(),
                    ) != std::cmp::Ordering::Less;
                    
                    age_ok && interval_ok
                } else {
                    false
                }
            },
        }
    ]
}

pub fn polio_recommendation_overrides() -> Vec<RecommendationOverrideRule> {
    vec![
        // Pre-2009 Forecast Recommendations:
        // "If evaluation date is before 8/7/2009, min_age of dose 4 is 126d and min_interval for dose 3 is 28d"
        RecommendationOverrideRule {
            description: "Polio Pre-2009 Dose 4 Forecast Overrides",
            target_dose_number: 4,
            condition: |ctx| {
                ctx.eval_date < NaiveDate::from_ymd_opt(2009, 8, 7).unwrap()
            },
            override_min_age: Some(TimePeriod::parse("126d").unwrap()),
            override_min_interval: Some(TimePeriod::parse("28d").unwrap()),
        }
    ]
}

// 2009 Forecast Date Reset Hook:
// "If the 4th dose of polio is recommended, and the recommended date falls before August 7, 2009,
// but the evaluation date is on or after August 7, 2009, the recommended date must be reset to August 7, 2009."
pub fn polio_custom_forecast_hook(
    _patient: &Patient,
    _valid_doses: &[(NaiveDate, usize)],
    eval_date: NaiveDate,
    forecast: &mut SeriesForecast,
) {
    let aug_7_2009 = NaiveDate::from_ymd_opt(2009, 8, 7).unwrap();
    if eval_date >= aug_7_2009 {
        if let Some(ref mut recommended) = forecast.recommended_date {
            if *recommended < aug_7_2009 {
                *recommended = aug_7_2009;
            }
        }
        if let Some(ref mut earliest) = forecast.earliest_date {
            if *earliest < aug_7_2009 {
                *earliest = aug_7_2009;
            }
        }
    }
}
