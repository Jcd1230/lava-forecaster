use ice_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn hpv_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("62"), cvx!("118"), cvx!("137"), cvx!("165")];

    CompiledSeries::builder("HPV_2_DOSE_SERIES")
        .code("HPV_2_DOSE_SERIES")
        .vaccine_group("HPV")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age("9y-4d")
            .min_age("9y")
            .earliest_recommended_age("11y")
            .latest_recommended_age("13y+4w")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("5m-4d")
            .min_interval("5m")
            .earliest_recommended_interval("6m")
            .latest_recommended_interval("13m+4w")
        )
        .build()
}

pub fn hpv_3_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("62"), cvx!("118"), cvx!("137"), cvx!("165")];

    CompiledSeries::builder("HPV_3_DOSE_SERIES")
        .code("HPV_3_DOSE_SERIES")
        .vaccine_group("HPV")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age("9y-4d")
            .min_age("9y")
            .earliest_recommended_age("11y")
            .latest_recommended_age("13y+4w")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .cvx(allowed_cvx)
        )
        .dose(3, |d| d
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("4w")
            .latest_recommended_interval("13m+4w")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("80d")
            .min_interval("84d")
            .earliest_recommended_interval("12w")
        )
        .interval(1, 3, |i| i
            .abs_min_interval("112d")
            .min_interval("5m")
            .earliest_recommended_interval("6m")
            .latest_recommended_interval("13m+4w")
        )
        .build()
}
