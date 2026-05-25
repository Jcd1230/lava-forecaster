use ice_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn h1n1_1_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("125"), cvx!("126"), cvx!("127"), cvx!("128")];

    CompiledSeries::builder("H1N1_1_DOSE_SERIES")
        .code("H1N1_1_DOSE_SERIES")
        .vaccine_group("H1N1")
        .num_doses(1)
        .dose(1, |d| d
            .abs_min_age("6m-4d")
            .min_age("6m")
            .earliest_recommended_age("6m")
            .cvx(allowed_cvx)
        )
        .build()
}

pub fn h1n1_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("125"), cvx!("126"), cvx!("127"), cvx!("128")];

    CompiledSeries::builder("H1N1_2_DOSE_SERIES")
        .code("H1N1_2_DOSE_SERIES")
        .vaccine_group("H1N1")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age("6m-4d")
            .min_age("6m")
            .earliest_recommended_age("6m")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("21d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
        )
        .build()
}
