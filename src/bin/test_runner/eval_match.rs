use chrono::NaiveDate;
use lava_forecaster::date_utils::SmallVec;
use lava_forecaster::models::DoseEvaluation;

pub type EvalKey = (NaiveDate, u16);

pub fn pair_evaluations_by_occurrence<'a>(
    rust_evals: &'a [DoseEvaluation],
    expected_evals: &'a [DoseEvaluation],
) -> SmallVec<
    [(
        EvalKey,
        Option<&'a DoseEvaluation>,
        Option<&'a DoseEvaluation>,
    ); 8],
> {
    let mut all_keys = SmallVec::<[EvalKey; 8]>::new();

    for eval in rust_evals {
        let key = (eval.dose_date, eval.cvx.0);
        if !all_keys.contains(&key) {
            all_keys.push(key);
        }
    }
    for eval in expected_evals {
        let key = (eval.dose_date, eval.cvx.0);
        if !all_keys.contains(&key) {
            all_keys.push(key);
        }
    }

    all_keys.sort_unstable();

    let mut pairs = SmallVec::<
        [(
            EvalKey,
            Option<&'a DoseEvaluation>,
            Option<&'a DoseEvaluation>,
        ); 8],
    >::new();
    for key in all_keys {
        let rust_count = rust_evals
            .iter()
            .filter(|eval| (eval.dose_date, eval.cvx.0) == key)
            .count();
        let expected_count = expected_evals
            .iter()
            .filter(|eval| (eval.dose_date, eval.cvx.0) == key)
            .count();
        let max_len = rust_count.max(expected_count);

        for idx in 0..max_len {
            let rust_eval = rust_evals
                .iter()
                .filter(|eval| (eval.dose_date, eval.cvx.0) == key)
                .nth(idx);
            let expected_eval = expected_evals
                .iter()
                .filter(|eval| (eval.dose_date, eval.cvx.0) == key)
                .nth(idx);

            pairs.push((key, rust_eval, expected_eval));
        }
    }

    pairs
}
