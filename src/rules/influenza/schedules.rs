use crate::schedule::CompiledSeries;

const ALLOWED_CVX: &[u16] = &[151, 144, 149, 88, 111, 155, 15, 141, 153, 16, 135, 150, 140, 158, 161, 166, 168, 171, 185, 186, 194, 197, 200, 201, 202, 205, 231, 320, 331, 333];

pub fn influenza_1_dose_series() -> CompiledSeries {
    CompiledSeries::builder("INFLUENZA_1_DOSE_SERIES")
        .code("INFLUENZA_1_DOSE_SERIES")
        .vaccine_group("INFLUENZA")
        .num_doses(1)
        .dose(1, |d| d
            .abs_min_age("6m-4d")
            .min_age("6m")
            .earliest_recommended_age("6m")
            .cvx(ALLOWED_CVX)
        )
        .build()
}

pub fn influenza_2_dose_series() -> CompiledSeries {
    CompiledSeries::builder("INFLUENZA_2_DOSE_SERIES")
        .code("INFLUENZA_2_DOSE_SERIES")
        .vaccine_group("INFLUENZA")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age("6m-4d")
            .min_age("6m")
            .earliest_recommended_age("6m")
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
        )
        .build()
}

pub fn influenza_2_dose_default_series() -> CompiledSeries {
    CompiledSeries::builder("INFLUENZA_2_DOSE_DEFAULT_SERIES")
        .code("INFLUENZA_2_DOSE_DEFAULT_SERIES")
        .vaccine_group("INFLUENZA")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age("6m-4d")
            .min_age("6m")
            .earliest_recommended_age("6m")
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
        )
        .build()
}
