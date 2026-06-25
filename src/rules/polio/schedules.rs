use crate::schedule::CompiledSeries;
use lava_cvx_macro::cvx;

pub fn polio_4_dose_series() -> CompiledSeries {
    let allowed_cvx = &[
        cvx!("10"),
        cvx!("89"),
        cvx!("02"),
        cvx!("110"),
        cvx!("120"),
        cvx!("130"),
        cvx!("132"),
        cvx!("146"),
        cvx!("170"),
        cvx!("182"),
        cvx!("195"),
        cvx!("178"),
        cvx!("179"),
    ];

    CompiledSeries::builder("POLIO_4_DOSE_SERIES")
        .code("POLIO_4_DOSE_SERIES")
        .vaccine_group("POLIO")
        .num_doses(5)
        .dose(1, |d| {
            d.abs_min_age(crate::time_period!("38d"))
                .min_age(crate::time_period!("42d"))
                .earliest_recommended_age(crate::time_period!("2m"))
                .latest_recommended_age(crate::time_period!("3m+4w"))
                .cvx(allowed_cvx)
        })
        .dose(2, |d| {
            d.abs_min_age(crate::time_period!("66d"))
                .min_age(crate::time_period!("70d"))
                .earliest_recommended_age(crate::time_period!("4m"))
                .latest_recommended_age(crate::time_period!("5m+4w"))
                .cvx(allowed_cvx)
        })
        .dose(3, |d| {
            d.abs_min_age(crate::time_period!("94d"))
                .min_age(crate::time_period!("98d"))
                .earliest_recommended_age(crate::time_period!("6m"))
                .latest_recommended_age(crate::time_period!("19m+4w"))
                .cvx(allowed_cvx)
        })
        .dose(4, |d| {
            d.abs_min_age(crate::time_period!("4y-4d"))
                .min_age(crate::time_period!("4y"))
                .earliest_recommended_age(crate::time_period!("4y"))
                .latest_recommended_age(crate::time_period!("7y+4w"))
                .cvx(allowed_cvx)
        })
        .dose(5, |d| {
            d.abs_min_age(crate::time_period!("4y-4d"))
                .min_age(crate::time_period!("4y"))
                .earliest_recommended_age(crate::time_period!("4y"))
                .latest_recommended_age(crate::time_period!("7y+4w"))
                .cvx(allowed_cvx)
        })
        .interval(1, 2, |i| {
            i.abs_min_interval(crate::time_period!("24d"))
                .min_interval(crate::time_period!("28d"))
                .earliest_recommended_interval(crate::time_period!("28d"))
                .latest_recommended_interval(crate::time_period!("13w"))
        })
        .interval(2, 3, |i| {
            i.abs_min_interval(crate::time_period!("24d"))
                .min_interval(crate::time_period!("28d"))
                .earliest_recommended_interval(crate::time_period!("28d"))
                .latest_recommended_interval(crate::time_period!("15m+4w"))
        })
        .interval(3, 4, |i| {
            i.abs_min_interval(crate::time_period!("6m-4d"))
                .min_interval(crate::time_period!("6m"))
                .earliest_recommended_interval(crate::time_period!("6m"))
                .latest_recommended_interval(crate::time_period!("6y+4w"))
        })
        .interval(4, 5, |i| {
            i.abs_min_interval(crate::time_period!("6m-4d"))
                .min_interval(crate::time_period!("6m"))
                .earliest_recommended_interval(crate::time_period!("6m"))
                .latest_recommended_interval(crate::time_period!("6y+4w"))
        })
        .build()
}

pub fn polio_fipv_series() -> CompiledSeries {
    let allowed_cvx = &[
        cvx!("10"),
        cvx!("89"),
        cvx!("02"),
        cvx!("110"),
        cvx!("120"),
        cvx!("130"),
        cvx!("132"),
        cvx!("146"),
        cvx!("170"),
        cvx!("182"),
        cvx!("195"),
        cvx!("178"),
        cvx!("179"),
    ];

    CompiledSeries::builder("POLIO_FRACTIONAL_IPV_SERIES")
        .code("POLIO_FRACTIONAL_IPV_SERIES")
        .vaccine_group("POLIO")
        .num_doses(6)
        .dose(1, |d| {
            d.abs_min_age(crate::time_period!("38d"))
                .min_age(crate::time_period!("42d"))
                .earliest_recommended_age(crate::time_period!("2m"))
                .latest_recommended_age(crate::time_period!("3m+4w"))
                .cvx(&[cvx!("324")])
        })
        .dose(2, |d| {
            d.abs_min_age(crate::time_period!("38d"))
                .min_age(crate::time_period!("42d"))
                .earliest_recommended_age(crate::time_period!("2m"))
                .latest_recommended_age(crate::time_period!("3m+4w"))
                .cvx(&[cvx!("324")])
        })
        .dose(3, |d| {
            d.abs_min_age(crate::time_period!("66d"))
                .min_age(crate::time_period!("70d"))
                .earliest_recommended_age(crate::time_period!("4m"))
                .latest_recommended_age(crate::time_period!("5m+4w"))
                .cvx(allowed_cvx)
        })
        .dose(4, |d| {
            d.abs_min_age(crate::time_period!("94d"))
                .min_age(crate::time_period!("98d"))
                .earliest_recommended_age(crate::time_period!("6m"))
                .latest_recommended_age(crate::time_period!("19m+4w"))
                .cvx(allowed_cvx)
        })
        .dose(5, |d| {
            d.abs_min_age(crate::time_period!("4y-4d"))
                .min_age(crate::time_period!("4y"))
                .earliest_recommended_age(crate::time_period!("4y"))
                .latest_recommended_age(crate::time_period!("7y+4w"))
                .cvx(allowed_cvx)
        })
        .dose(6, |d| {
            d.abs_min_age(crate::time_period!("4y-4d"))
                .min_age(crate::time_period!("4y"))
                .earliest_recommended_age(crate::time_period!("4y"))
                .latest_recommended_age(crate::time_period!("7y+4w"))
                .cvx(allowed_cvx)
        })
        .interval(1, 2, |i| {
            i.abs_min_interval(crate::time_period!("24d"))
                .min_interval(crate::time_period!("28d"))
                .earliest_recommended_interval(crate::time_period!("28d"))
                .latest_recommended_interval(crate::time_period!("13w"))
        })
        .interval(2, 3, |i| {
            i.abs_min_interval(crate::time_period!("24d"))
                .min_interval(crate::time_period!("28d"))
                .earliest_recommended_interval(crate::time_period!("28d"))
                .latest_recommended_interval(crate::time_period!("15m+4w"))
        })
        .interval(3, 4, |i| {
            i.abs_min_interval(crate::time_period!("24d"))
                .min_interval(crate::time_period!("28d"))
                .earliest_recommended_interval(crate::time_period!("28d"))
                .latest_recommended_interval(crate::time_period!("15m+4w"))
        })
        .interval(4, 5, |i| {
            i.abs_min_interval(crate::time_period!("6m-4d"))
                .min_interval(crate::time_period!("6m"))
                .earliest_recommended_interval(crate::time_period!("6m"))
                .latest_recommended_interval(crate::time_period!("6y+4w"))
        })
        .interval(5, 6, |i| {
            i.abs_min_interval(crate::time_period!("6m-4d"))
                .min_interval(crate::time_period!("6m"))
                .earliest_recommended_interval(crate::time_period!("6m"))
                .latest_recommended_interval(crate::time_period!("6y+4w"))
        })
        .build()
}
