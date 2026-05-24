use crate::schedule::CompiledSeries;

pub fn hib_4_dose_series() -> CompiledSeries {
    let allowed_cvx = &[17, 22, 46, 47, 48, 49, 50, 51, 102, 120, 132, 146, 148, 170, 198];

    CompiledSeries::builder("HIB_4_DOSE_SERIES")
        .code("HIB_4_DOSE_SERIES")
        .vaccine_group("HIB")
        .num_doses(4)
        .dose(1, |d| d
            .abs_min_age("38d")
            .min_age("42d")
            .earliest_recommended_age("2m")
            .latest_recommended_age("3m+4w")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age("66d")
            .min_age("70d")
            .earliest_recommended_age("4m")
            .latest_recommended_age("5m+4w")
            .cvx(allowed_cvx)
        )
        .dose(3, |d| d
            .abs_min_age("94d")
            .min_age("98d")
            .earliest_recommended_age("6m")
            .latest_recommended_age("7m+4w")
            .cvx(allowed_cvx)
        )
        .dose(4, |d| d
            .abs_min_age("1y-4d")
            .min_age("1y")
            .earliest_recommended_age("1y")
            .latest_recommended_age("16m+4w")
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("16w")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("16w")
        )
        .interval(3, 4, |i| i
            .abs_min_interval("52d")
            .min_interval("56d")
            .earliest_recommended_interval("56d")
            .latest_recommended_interval("10m+4w")
        )
        .max_age_clamp("5y", crate::models::SeriesStatus::ConditionallyRecommended)
        .build()
}

pub fn hib_omp_series() -> CompiledSeries {
    let omp_cvx = &[49, 51];
    let allowed_cvx = &[17, 22, 46, 47, 48, 49, 50, 51, 102, 120, 132, 146, 148, 170, 198];

    CompiledSeries::builder("HIB_OMP_SERIES")
        .code("HIB_OMP_SERIES")
        .vaccine_group("HIB")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age("38d")
            .min_age("42d")
            .earliest_recommended_age("2m")
            .latest_recommended_age("3m+4w")
            .cvx(omp_cvx)
        )
        .dose(2, |d| d
            .abs_min_age("66d")
            .min_age("70d")
            .earliest_recommended_age("4m")
            .latest_recommended_age("5m+4w")
            .cvx(omp_cvx)
        )
        .dose(3, |d| d
            .abs_min_age("1y-4d")
            .min_age("1y")
            .earliest_recommended_age("1y")
            .latest_recommended_age("16m+4w")
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("16w")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("52d")
            .min_interval("56d")
            .earliest_recommended_interval("56d")
            .latest_recommended_interval("10m+4w")
        )
        .max_age_clamp("5y", crate::models::SeriesStatus::ConditionallyRecommended)
        .build()
}
