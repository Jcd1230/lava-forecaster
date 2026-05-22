use crate::schedule::CompiledSeries;

pub fn mcv_42_dose_series() -> CompiledSeries {
    let allowed_cvx = &["147", "32", "114", "136", "108", "203", "316", "328"];

    CompiledSeries::builder("MCV_42_DOSE_SERIES")
        .code("MCV_42_DOSE_SERIES")
        .vaccine_group("MCV")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age("10y")
            .min_age("11y")
            .earliest_recommended_age("11y")
            .latest_recommended_age("13y+4w")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age("16y-4d")
            .min_age("16y")
            .earliest_recommended_age("16y")
            .latest_recommended_age("17y+4w")
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("52d")
            .min_interval("56d")
            .earliest_recommended_interval("56d")
            .latest_recommended_interval("6y+4w")
        )
        .build()
}
