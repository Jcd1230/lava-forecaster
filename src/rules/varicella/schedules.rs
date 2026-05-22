use crate::schedule::CompiledSeries;

pub fn varicella_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &["21", "94"];

    CompiledSeries::builder("VARICELLA_2_DOSE_SERIES")
        .code("VARICELLA_2_DOSE_SERIES")
        .vaccine_group("VARICELLA")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age("1y-4d")
            .min_age("1y")
            .earliest_recommended_age("1y")
            .latest_recommended_age("16m+4w")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age("13m")
            .min_age("15m")
            .earliest_recommended_age("4y")
            .latest_recommended_age("7y+4w")
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("28d")
            .min_interval("84d")
            .earliest_recommended_interval("84d")
            .latest_recommended_interval("6y+4w")
        )
        .build()
}
