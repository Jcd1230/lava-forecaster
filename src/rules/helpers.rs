use chrono::NaiveDate;
use crate::models::Dose;
use crate::date_utils::{compare_elapsed, TimePeriod};

/// Clamps an optional date to be at least the specified minimum date.
/// If the date is None, it is set to Some(min_date).
pub fn clamp_date_at_least(opt_date: &mut Option<NaiveDate>, min_date: NaiveDate) {
    if let Some(date) = opt_date {
        if *date < min_date {
            *date = min_date;
        }
    } else {
        *opt_date = Some(min_date);
    }
}

/// Returns true if the elapsed time from birth_date to date_to_check is strictly less than the given age expression.
pub fn age_lt(birth_date: NaiveDate, date_to_check: NaiveDate, age_expr: &str) -> bool {
    compare_elapsed(birth_date, date_to_check, &TimePeriod::parse(age_expr).unwrap())
        == std::cmp::Ordering::Less
}

/// Returns true if the elapsed time from birth_date to date_to_check is greater than or equal to the given age expression.
pub fn age_ge(birth_date: NaiveDate, date_to_check: NaiveDate, age_expr: &str) -> bool {
    compare_elapsed(birth_date, date_to_check, &TimePeriod::parse(age_expr).unwrap())
        != std::cmp::Ordering::Less
}

/// Gets the maximum date among all administered doses.
pub fn last_dose_date(history: &[Dose]) -> Option<NaiveDate> {
    history.iter().map(|d| d.date).max()
}

/// Returns the number of days between two dates.
pub fn interval_days_between(d1: NaiveDate, d2: NaiveDate) -> i64 {
    (d2 - d1).num_days()
}

/// Returns true if the elapsed interval between d1 and d2 is greater than or equal to the given expression.
pub fn interval_ge(d1: NaiveDate, d2: NaiveDate, interval_expr: &str) -> bool {
    compare_elapsed(d1, d2, &TimePeriod::parse(interval_expr).unwrap())
        != std::cmp::Ordering::Less
}

/// Returns true if the elapsed interval between d1 and d2 is strictly less than the given expression.
pub fn interval_lt(d1: NaiveDate, d2: NaiveDate, interval_expr: &str) -> bool {
    compare_elapsed(d1, d2, &TimePeriod::parse(interval_expr).unwrap())
        == std::cmp::Ordering::Less
}
