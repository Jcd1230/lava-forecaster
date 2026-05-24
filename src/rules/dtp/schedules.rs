use crate::schedule::CompiledSeries;

const ALLOWED_CVX: &[u16] = &[1, 20, 106, 107, 22, 50, 102, 110, 120, 130, 132, 146, 115, 28, 9, 138, 139, 113, 170, 195, 196, 198];

pub fn dtp_3_dose_series() -> CompiledSeries {
    CompiledSeries::builder("DTP_3_DOSE_SERIES")
        .code("DTP_3_DOSE_SERIES")
        .vaccine_group("DTP")
        .num_doses(4) // Modeled as 4-dose series to handle pertussis-containing exception
        .dose(1, |d| d
            .abs_min_age("7y")
            .min_age("7y")
            .earliest_recommended_age("7y")
            .latest_recommended_age("7y")
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d.cvx(ALLOWED_CVX))
        .dose(3, |d| d.cvx(ALLOWED_CVX))
        .dose(4, |d| d.cvx(ALLOWED_CVX))
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("28d")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("6m-4d")
            .min_interval("6m")
            .earliest_recommended_interval("6m")
            .latest_recommended_interval("6m")
        )
        .interval(3, 4, |i| i
            .abs_min_interval("0d")
            .min_interval("0d")
            .earliest_recommended_interval("0d")
            .latest_recommended_interval("0d")
        )
        .build()
}

pub fn dtp_5_dose_series() -> CompiledSeries {
    CompiledSeries::builder("DTP_5_DOSE_SERIES")
        .code("DTP_5_DOSE_SERIES")
        .vaccine_group("DTP")
        .num_doses(5)
        .dose(1, |d| d
            .abs_min_age("38d")
            .min_age("42d")
            .earliest_recommended_age("2m")
            .latest_recommended_age("3m+4w")
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .abs_min_age("66d")
            .min_age("70d")
            .earliest_recommended_age("4m")
            .latest_recommended_age("5m+4w")
            .cvx(ALLOWED_CVX)
        )
        .dose(3, |d| d
            .abs_min_age("94d")
            .min_age("98d")
            .earliest_recommended_age("6m")
            .latest_recommended_age("7m+4w")
            .cvx(ALLOWED_CVX)
        )
        .dose(4, |d| d
            .abs_min_age("1y-4d")
            .min_age("15m")
            .earliest_recommended_age("15m")
            .latest_recommended_age("19m+4w")
            .cvx(ALLOWED_CVX)
        )
        .dose(5, |d| d
            .abs_min_age("4y-4d")
            .min_age("4y")
            .earliest_recommended_age("4y")
            .latest_recommended_age("7y")
            .cvx(ALLOWED_CVX)
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
            .latest_recommended_interval("13w")
        )
        .interval(3, 4, |i| i
            .abs_min_interval("4m")
            .min_interval("6m")
            .earliest_recommended_interval("6m")
            .latest_recommended_interval("13m+4w")
        )
        .interval(4, 5, |i| i
            .abs_min_interval("6m-4d")
            .min_interval("6m")
            .earliest_recommended_interval("6m")
            .latest_recommended_interval("4y+4w")
        )
        .build()
}
