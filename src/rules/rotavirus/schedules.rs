use ice_cvx_macro::cvx;
use crate::schedule::CompiledSeries;
use crate::models::SeriesStatus;

pub fn rotavirus_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("119")];

    CompiledSeries::builder("ROTAVIRUS_2_DOSE_SERIES")
        .code("ROTAVIRUS_2_DOSE_SERIES")
        .vaccine_group("ROTAVIRUS")
        .num_doses(2)

        .dose(1, |d| {
            d.abs_min_age("38d")
                .min_age("42d")
                .earliest_recommended_age("2m")
                .cvx(allowed_cvx)
        })
        .dose(2, |d| {
            d.abs_min_age("66d")
                .min_age("70d")
                .earliest_recommended_age("4m")
                .latest_recommended_age("5m+4w")
                .cvx(allowed_cvx)
        })
        .interval(1, 2, |i| {
            i.abs_min_interval("24d")
                .min_interval("28d")
                .earliest_recommended_interval("28d")
                .latest_recommended_interval("13w")
        })
        .build()
}

pub fn rotavirus_3_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("119"), cvx!("122"), cvx!("116"), cvx!("74")];

    CompiledSeries::builder("ROTAVIRUS_3_DOSE_SERIES")
        .code("ROTAVIRUS_3_DOSE_SERIES")
        .vaccine_group("ROTAVIRUS")
        .num_doses(3)

        .dose(1, |d| {
            d.abs_min_age("38d")
                .min_age("42d")
                .earliest_recommended_age("2m")
                .cvx(allowed_cvx)
        })
        .dose(2, |d| {
            d.abs_min_age("66d")
                .min_age("70d")
                .earliest_recommended_age("4m")
                .latest_recommended_age("5m+4w")
                .cvx(allowed_cvx)
        })
        .dose(3, |d| {
            d.abs_min_age("94d")
                .min_age("98d")
                .earliest_recommended_age("6m")
                .latest_recommended_age("7m+4w")
                .cvx(allowed_cvx)
        })
        .interval(1, 2, |i| {
            i.abs_min_interval("24d")
                .min_interval("28d")
                .earliest_recommended_interval("28d")
                .latest_recommended_interval("13w")
        })
        .interval(2, 3, |i| {
            i.abs_min_interval("24d")
                .min_interval("28d")
                .earliest_recommended_interval("28d")
                .latest_recommended_interval("13w")
        })
        .build()
}
