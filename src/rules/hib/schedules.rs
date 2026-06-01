use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn hib_4_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("17"), cvx!("22"), cvx!("46"), cvx!("47"), cvx!("48"), cvx!("49"), cvx!("50"), cvx!("51"), cvx!("102"), cvx!("120"), cvx!("132"), cvx!("146"), cvx!("148"), cvx!("170"), cvx!("198")];

    CompiledSeries::builder("HIB_4_DOSE_SERIES")
        .code("HIB_4_DOSE_SERIES")
        .vaccine_group("HIB")
        .num_doses(4)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("38d"))
            .min_age(crate::time_period!("42d"))
            .earliest_recommended_age(crate::time_period!("2m"))
            .latest_recommended_age(crate::time_period!("3m+4w"))
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("66d"))
            .min_age(crate::time_period!("70d"))
            .earliest_recommended_age(crate::time_period!("4m"))
            .latest_recommended_age(crate::time_period!("5m+4w"))
            .cvx(allowed_cvx)
        )
        .dose(3, |d| d
            .abs_min_age(crate::time_period!("94d"))
            .min_age(crate::time_period!("98d"))
            .earliest_recommended_age(crate::time_period!("6m"))
            .latest_recommended_age(crate::time_period!("7m+4w"))
            .cvx(allowed_cvx)
        )
        .dose(4, |d| d
            .abs_min_age(crate::time_period!("1y-4d"))
            .min_age(crate::time_period!("1y"))
            .earliest_recommended_age(crate::time_period!("1y"))
            .latest_recommended_age(crate::time_period!("16m+4w"))
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("16w"))
        )
        .interval(2, 3, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("16w"))
        )
        .interval(3, 4, |i| i
            .abs_min_interval(crate::time_period!("52d"))
            .min_interval(crate::time_period!("56d"))
            .earliest_recommended_interval(crate::time_period!("56d"))
            .latest_recommended_interval(crate::time_period!("10m+4w"))
        )
        .max_age_clamp(crate::time_period!("5y"), crate::models::SeriesStatus::ConditionallyRecommended)
        .build()
}

pub fn hib_omp_series() -> CompiledSeries {
    let omp_cvx = &[cvx!("49"), cvx!("51")];
    let allowed_cvx = &[cvx!("17"), cvx!("22"), cvx!("46"), cvx!("47"), cvx!("48"), cvx!("49"), cvx!("50"), cvx!("51"), cvx!("102"), cvx!("120"), cvx!("132"), cvx!("146"), cvx!("148"), cvx!("170"), cvx!("198")];

    CompiledSeries::builder("HIB_OMP_SERIES")
        .code("HIB_OMP_SERIES")
        .vaccine_group("HIB")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("38d"))
            .min_age(crate::time_period!("42d"))
            .earliest_recommended_age(crate::time_period!("2m"))
            .latest_recommended_age(crate::time_period!("3m+4w"))
            .cvx(omp_cvx)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("66d"))
            .min_age(crate::time_period!("70d"))
            .earliest_recommended_age(crate::time_period!("4m"))
            .latest_recommended_age(crate::time_period!("5m+4w"))
            .cvx(omp_cvx)
        )
        .dose(3, |d| d
            .abs_min_age(crate::time_period!("1y-4d"))
            .min_age(crate::time_period!("1y"))
            .earliest_recommended_age(crate::time_period!("1y"))
            .latest_recommended_age(crate::time_period!("16m+4w"))
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("16w"))
        )
        .interval(2, 3, |i| i
            .abs_min_interval(crate::time_period!("52d"))
            .min_interval(crate::time_period!("56d"))
            .earliest_recommended_interval(crate::time_period!("56d"))
            .latest_recommended_interval(crate::time_period!("10m+4w"))
        )
        .max_age_clamp(crate::time_period!("5y"), crate::models::SeriesStatus::ConditionallyRecommended)
        .build()
}
