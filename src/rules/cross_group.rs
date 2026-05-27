use chrono::NaiveDate;
use crate::models::{Cvx, Dose, Patient, VaccineGroupForecast};

pub fn post_process_all_groups(
    _patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
    results: &mut [VaccineGroupForecast],
) {
    // Cross-group post-processing: YellowFever.adjustEarliestDateDueToYellowFeverVaccine
    let yf_complete = results
        .iter()
        .find(|g| g.vaccine_group == "YELLOW_FEVER")
        .and_then(|g| g.forecasts.first())
        .map(|f| matches!(f.status, crate::models::SeriesStatus::Complete))
        .unwrap_or(false);

    if yf_complete {
        let last_yf_dose = history
            .iter()
            .filter(|d| d.cvx.0 == Cvx::YELLOW_FEVER || d.cvx.0 == Cvx::YELLOW_FEVER_UNSPECIFIED || d.cvx.0 == Cvx::YELLOW_FEVER_UNKNOWN)
            .map(|d| d.date)
            .max();

        if let Some(yf_date) = last_yf_dose {
            let limit_date = yf_date + chrono::Duration::days(30);
            for g in results.iter_mut() {
                if g.vaccine_group == "YELLOW_FEVER" {
                    continue;
                }

                let is_live_group = g.vaccine_group == "MMR"
                    || g.vaccine_group == "VARICELLA"
                    || g.vaccine_group == "ROTAVIRUS"
                    || g.vaccine_group == "CHOLERA";

                if is_live_group {
                    for f in g.forecasts.iter_mut() {
                        if let Some(earliest) = f.status.earliest_date() {
                            let gap = earliest - yf_date;
                            let is_conflict = if yf_date != eval_date {
                                gap.num_days() < 30
                            } else {
                                earliest > yf_date && gap.num_days() < 30
                            };
                            if is_conflict {
                                f.status = f.status.with_earliest_date(Some(limit_date));
                                if let Some(recommended) = f.status.recommended_date() {
                                    if recommended < limit_date {
                                        f.status = f.status.with_recommended_date(Some(limit_date));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
