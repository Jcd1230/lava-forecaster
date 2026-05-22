use crate::schedule::CompiledSeries;

pub fn zoster_2_dose_series() -> CompiledSeries {
    // CVX 187 = Zoster recombinant (Shingrix) - the primary vaccine
    // CVX 121 = Zoster live (Zostavax) - old live vaccine, counts as Accepted
    // CVX 188 = Zoster recombinant, unspecified - also Accepted
    let allowed_cvx = &["187", "121", "188"];

    CompiledSeries::builder("ZOSTER_2_DOSE_SERIES")
        .code("ZOSTER_2_DOSE_SERIES")
        .vaccine_group("ZOSTER")
        .num_doses(2)
        .dose(1, |d| d
            // No minimum age for evaluation — doses before age 50 are Valid but not recommended.
            // The earliest_recommended_age drives the forecast, not a hard evaluation limit.
            .earliest_recommended_age("50y")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("28d")
            .min_interval("28d")
            .earliest_recommended_interval("56d")
            .latest_recommended_interval("7m+4w")
        )
        .build()
}
