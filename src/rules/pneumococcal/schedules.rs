use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn pneumococcal_series() -> CompiledSeries {
    let child_pcv = &[cvx!("100"), cvx!("133"), cvx!("177"), cvx!("215"), cvx!("216"), cvx!("109"), cvx!("152")];
    let adult_pneumo = &[cvx!("215"), cvx!("216"), cvx!("327"), cvx!("133"), cvx!("33")];

    CompiledSeries::builder("PNEUMOCOCCAL_SERIES")
        .code("PNEUMOCOCCAL_SERIES")
        .vaccine_group("PNEUMOCOCCAL")
        .num_doses(8)
        .dose(1, |d| {
            d.abs_min_age(crate::time_period!("38d"))
                .min_age(crate::time_period!("42d"))
                .earliest_recommended_age(crate::time_period!("2m"))
                .latest_recommended_age(crate::time_period!("3m+4w"))
                .cvx(child_pcv)
        })
        .dose(2, |d| {
            d.abs_min_age(crate::time_period!("66d"))
                .min_age(crate::time_period!("70d"))
                .earliest_recommended_age(crate::time_period!("4m"))
                .latest_recommended_age(crate::time_period!("5m+4w"))
                .cvx(child_pcv)
        })
        .dose(3, |d| {
            d.abs_min_age(crate::time_period!("94d"))
                .min_age(crate::time_period!("98d"))
                .earliest_recommended_age(crate::time_period!("6m"))
                .latest_recommended_age(crate::time_period!("7m+4w"))
                .cvx(child_pcv)
        })
        .dose(4, |d| {
            d.abs_min_age(crate::time_period!("1y-4d"))
                .min_age(crate::time_period!("1y"))
                .earliest_recommended_age(crate::time_period!("1y"))
                .latest_recommended_age(crate::time_period!("16m+4w"))
                .cvx(child_pcv)
        })
        .dose(5, |d| d.cvx(&[cvx!("133"), cvx!("215"), cvx!("216")]))
        .dose(6, |d| {
            d.abs_min_age(crate::time_period!("19y"))
                .min_age(crate::time_period!("50y"))
                .earliest_recommended_age(crate::time_period!("50y"))
                .cvx(adult_pneumo)
        })
        .dose(7, |d| {
            d.abs_min_age(crate::time_period!("19y"))
                .min_age(crate::time_period!("50y"))
                .earliest_recommended_age(crate::time_period!("50y"))
                .cvx(adult_pneumo)
        })
        .dose(8, |d| {
            d.abs_min_age(crate::time_period!("50y"))
                .min_age(crate::time_period!("50y"))
                .earliest_recommended_age(crate::time_period!("50y"))
                .cvx(adult_pneumo)
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
                .latest_recommended_interval(crate::time_period!("13w"))
        })
        .interval(3, 4, |i| {
            i.abs_min_interval(crate::time_period!("52d"))
                .min_interval(crate::time_period!("56d"))
                .earliest_recommended_interval(crate::time_period!("56d"))
                .latest_recommended_interval(crate::time_period!("7m+4w"))
        })
        .interval(4, 5, |i| {
            i.abs_min_interval(crate::time_period!("52d"))
                .min_interval(crate::time_period!("56d"))
                .earliest_recommended_interval(crate::time_period!("56d"))
        })
        .interval(5, 6, |i| {
            i.abs_min_interval(crate::time_period!("1d"))
                .min_interval(crate::time_period!("1y"))
                .earliest_recommended_interval(crate::time_period!("1y"))
        })
        .interval(6, 7, |i| {
            i.abs_min_interval(crate::time_period!("1d"))
                .min_interval(crate::time_period!("1y"))
                .earliest_recommended_interval(crate::time_period!("1y"))
        })
        .interval(7, 8, |i| {
            i.abs_min_interval(crate::time_period!("1d"))
                .min_interval(crate::time_period!("5y"))
                .earliest_recommended_interval(crate::time_period!("5y"))
        })
        .build()
}
