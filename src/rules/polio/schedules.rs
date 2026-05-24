use crate::schedule::CompiledSeries;

pub fn polio_4_dose_series() -> CompiledSeries {
    let allowed_cvx = &[
        "10", "89", "02", "110", "120", "130", "132", "146", "170", "182", "195", "178", "179",
    ];

    CompiledSeries::builder("POLIO_4_DOSE_SERIES")
        .code("POLIO_4_DOSE_SERIES")
        .vaccine_group("POLIO")
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
            .latest_recommended_age("19m+4w")
            .cvx(allowed_cvx)
        )
        .dose(4, |d| d
            .abs_min_age("4y-4d")
            .min_age("4y")
            .earliest_recommended_age("4y")
            .latest_recommended_age("7y+4w")
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("13w")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("15m+4w")
        )
        .interval(3, 4, |i| i
            .abs_min_interval("6m-4d")
            .min_interval("6m")
            .earliest_recommended_interval("6m")
            .latest_recommended_interval("6y+4w")
        )
        .build()
}

pub fn polio_fipv_series() -> CompiledSeries {
    let allowed_cvx = &[
        "10", "89", "02", "110", "120", "130", "132", "146", "170", "182", "195", "178", "179",
    ];

    CompiledSeries::builder("POLIO_FRACTIONAL_IPV_SERIES")
        .code("POLIO_FRACTIONAL_IPV_SERIES")
        .vaccine_group("POLIO")
        .num_doses(5)
        .dose(1, |d| d
            .abs_min_age("38d")
            .min_age("42d")
            .earliest_recommended_age("2m")
            .latest_recommended_age("3m+4w")
            .cvx(&["324"])
        )
        .dose(2, |d| d
            .abs_min_age("38d")
            .min_age("42d")
            .earliest_recommended_age("2m")
            .latest_recommended_age("3m+4w")
            .cvx(&["324"])
        )
        .dose(3, |d| d
            .abs_min_age("66d")
            .min_age("70d")
            .earliest_recommended_age("4m")
            .latest_recommended_age("5m+4w")
            .cvx(allowed_cvx)
        )
        .dose(4, |d| d
            .abs_min_age("94d")
            .min_age("98d")
            .earliest_recommended_age("6m")
            .latest_recommended_age("19m+4w")
            .cvx(allowed_cvx)
        )
        .dose(5, |d| d
            .abs_min_age("4y-4d")
            .min_age("4y")
            .earliest_recommended_age("4y")
            .latest_recommended_age("7y+4w")
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("13w")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("15m+4w")
        )
        .interval(3, 4, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("15m+4w")
        )
        .interval(4, 5, |i| i
            .abs_min_interval("6m-4d")
            .min_interval("6m")
            .earliest_recommended_interval("6m")
            .latest_recommended_interval("6y+4w")
        )
        .build()
}
