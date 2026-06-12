use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use lava_forecaster::models::DoseEvaluation;

pub type EvalKey = (NaiveDate, u16);

pub fn pair_evaluations_by_occurrence<'a>(
    rust_evals: &'a [DoseEvaluation],
    expected_evals: &'a [DoseEvaluation],
) -> Vec<(EvalKey, Option<&'a DoseEvaluation>, Option<&'a DoseEvaluation>)> {
    let mut rust_map: BTreeMap<EvalKey, Vec<&DoseEvaluation>> = BTreeMap::new();
    let mut expected_map: BTreeMap<EvalKey, Vec<&DoseEvaluation>> = BTreeMap::new();
    let mut all_keys: BTreeSet<EvalKey> = BTreeSet::new();

    for eval in rust_evals {
        let key = (eval.dose_date, eval.cvx.0);
        rust_map.entry(key).or_default().push(eval);
        all_keys.insert(key);
    }
    for eval in expected_evals {
        let key = (eval.dose_date, eval.cvx.0);
        expected_map.entry(key).or_default().push(eval);
        all_keys.insert(key);
    }

    let mut pairs = Vec::new();
    for key in all_keys {
        let rust_items = rust_map.get(&key).cloned().unwrap_or_default();
        let expected_items = expected_map.get(&key).cloned().unwrap_or_default();
        let max_len = rust_items.len().max(expected_items.len());
        for idx in 0..max_len {
            pairs.push((
                key,
                rust_items.get(idx).copied(),
                expected_items.get(idx).copied(),
            ));
        }
    }

    pairs
}
