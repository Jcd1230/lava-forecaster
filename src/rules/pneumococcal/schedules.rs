use crate::schedule::CompiledSeries;

pub fn pneumococcal_series() -> CompiledSeries {
    let child_pcv = &[100, 133, 177, 215, 216, 109, 152];
    let adult_pneumo = &[215, 216, 327, 133, 33];

    CompiledSeries::builder("PNEUMOCOCCAL_SERIES")
        .code("PNEUMOCOCCAL_SERIES")
        .vaccine_group("PNEUMOCOCCAL")
        .num_doses(8)
        .dose(1, |d| {
            d.abs_min_age("38d")
                .min_age("42d")
                .earliest_recommended_age("2m")
                .latest_recommended_age("3m+4w")
                .cvx(child_pcv)
        })
        .dose(2, |d| {
            d.abs_min_age("66d")
                .min_age("70d")
                .earliest_recommended_age("4m")
                .latest_recommended_age("5m+4w")
                .cvx(child_pcv)
        })
        .dose(3, |d| {
            d.abs_min_age("94d")
                .min_age("98d")
                .earliest_recommended_age("6m")
                .latest_recommended_age("7m+4w")
                .cvx(child_pcv)
        })
        .dose(4, |d| {
            d.abs_min_age("1y-4d")
                .min_age("1y")
                .earliest_recommended_age("1y")
                .latest_recommended_age("16m+4w")
                .cvx(child_pcv)
        })
        .dose(5, |d| d.cvx(&[133, 215, 216]))
        .dose(6, |d| {
            d.abs_min_age("19y")
                .min_age("50y")
                .earliest_recommended_age("50y")
                .cvx(adult_pneumo)
        })
        .dose(7, |d| {
            d.abs_min_age("19y")
                .min_age("50y")
                .earliest_recommended_age("50y")
                .cvx(adult_pneumo)
        })
        .dose(8, |d| {
            d.abs_min_age("50y")
                .min_age("50y")
                .earliest_recommended_age("50y")
                .cvx(adult_pneumo)
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
        .interval(3, 4, |i| {
            i.abs_min_interval("52d")
                .min_interval("56d")
                .earliest_recommended_interval("56d")
                .latest_recommended_interval("7m+4w")
        })
        .interval(4, 5, |i| {
            i.abs_min_interval("52d")
                .min_interval("56d")
                .earliest_recommended_interval("56d")
        })
        .interval(5, 6, |i| {
            i.abs_min_interval("1d")
                .min_interval("1y")
                .earliest_recommended_interval("1y")
        })
        .interval(6, 7, |i| {
            i.abs_min_interval("1d")
                .min_interval("1y")
                .earliest_recommended_interval("1y")
        })
        .interval(7, 8, |i| {
            i.abs_min_interval("1d")
                .min_interval("5y")
                .earliest_recommended_interval("5y")
        })
        .build()
}
