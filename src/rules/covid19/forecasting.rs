#![allow(dead_code)]

use crate::models::SeriesForecast;
use crate::rules::covid19::policy::CovidSeriesId;
use crate::rules::covid19::seasons::CovidSeason;
use crate::rules::covid19::state::CovidEvaluatedState;

/// Apply the Aug 2025 2y-64y target-dose-1 forecast for patients with no
/// COVID dose history.
///
/// Java's Aug 2025 2y-64y recommendation rules are selected by evaluation-age
/// routing, not by age at season start. Once this series is selected and there
/// is no prior COVID shot to anchor an 8-week interval, the available date is
/// the Aug 2025 season start.
pub fn apply_aug2025_age_2_to_64_no_history_forecast(
    state: &CovidEvaluatedState<'_>,
    forecast: &mut SeriesForecast,
) -> bool {
    if state.selected_series.id != CovidSeriesId::Aug2025Age2To64 {
        return false;
    }
    if !state.dose_facts.is_empty() {
        return false;
    }

    let due = CovidSeason::Aug2025.start_date();
    forecast.status = forecast.status.with_earliest_date(Some(due));
    forecast.status = forecast.status.with_recommended_date(Some(due));
    forecast.status = forecast.status.with_overdue_date(None);
    forecast.status = forecast.status.with_latest_date(None);
    true
}
