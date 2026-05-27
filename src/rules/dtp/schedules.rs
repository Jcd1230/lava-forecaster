use ice_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

const ALLOWED_CVX: &[u16] = &[cvx!("01"), cvx!("20"), cvx!("106"), cvx!("107"), cvx!("22"), cvx!("50"), cvx!("102"), cvx!("110"), cvx!("120"), cvx!("130"), cvx!("132"), cvx!("146"), cvx!("115"), cvx!("28"), cvx!("09"), cvx!("138"), cvx!("139"), cvx!("113"), cvx!("170"), cvx!("195"), cvx!("196"), cvx!("198")];

pub fn dtp_3_dose_series() -> CompiledSeries {
    CompiledSeries::builder("DTP_3_DOSE_SERIES")
        .code("DTP_3_DOSE_SERIES")
        .vaccine_group("DTP")
        .num_doses(4) // Modeled as 4-dose series to handle pertussis-containing exception
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("7y"))
            .min_age(crate::time_period!("7y"))
            .earliest_recommended_age(crate::time_period!("7y"))
            .latest_recommended_age(crate::time_period!("7y"))
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d.cvx(ALLOWED_CVX))
        .dose(3, |d| d.cvx(ALLOWED_CVX))
        .dose(4, |d| d.cvx(ALLOWED_CVX))
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("28d"))
        )
        .interval(2, 3, |i| i
            .abs_min_interval(crate::time_period!("6m-4d"))
            .min_interval(crate::time_period!("6m"))
            .earliest_recommended_interval(crate::time_period!("6m"))
            .latest_recommended_interval(crate::time_period!("6m"))
        )
        .interval(3, 4, |i| i
            .abs_min_interval(crate::time_period!("0d"))
            .min_interval(crate::time_period!("0d"))
            .earliest_recommended_interval(crate::time_period!("0d"))
            .latest_recommended_interval(crate::time_period!("0d"))
        )
        .build()
}

pub fn dtp_5_dose_series() -> CompiledSeries {
    CompiledSeries::builder("DTP_5_DOSE_SERIES")
        .code("DTP_5_DOSE_SERIES")
        .vaccine_group("DTP")
        .num_doses(5)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("38d"))
            .min_age(crate::time_period!("42d"))
            .earliest_recommended_age(crate::time_period!("2m"))
            .latest_recommended_age(crate::time_period!("3m+4w"))
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("66d"))
            .min_age(crate::time_period!("70d"))
            .earliest_recommended_age(crate::time_period!("4m"))
            .latest_recommended_age(crate::time_period!("5m+4w"))
            .cvx(ALLOWED_CVX)
        )
        .dose(3, |d| d
            .abs_min_age(crate::time_period!("94d"))
            .min_age(crate::time_period!("98d"))
            .earliest_recommended_age(crate::time_period!("6m"))
            .latest_recommended_age(crate::time_period!("7m+4w"))
            .cvx(ALLOWED_CVX)
        )
        .dose(4, |d| d
            .abs_min_age(crate::time_period!("1y-4d"))
            .min_age(crate::time_period!("15m"))
            .earliest_recommended_age(crate::time_period!("15m"))
            .latest_recommended_age(crate::time_period!("19m+4w"))
            .cvx(ALLOWED_CVX)
        )
        .dose(5, |d| d
            .abs_min_age(crate::time_period!("4y-4d"))
            .min_age(crate::time_period!("4y"))
            .earliest_recommended_age(crate::time_period!("4y"))
            .latest_recommended_age(crate::time_period!("7y"))
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("13w"))
        )
        .interval(2, 3, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("13w"))
        )
        .interval(3, 4, |i| i
            .abs_min_interval(crate::time_period!("4m"))
            .min_interval(crate::time_period!("6m"))
            .earliest_recommended_interval(crate::time_period!("6m"))
            .latest_recommended_interval(crate::time_period!("13m+4w"))
        )
        .interval(4, 5, |i| i
            .abs_min_interval(crate::time_period!("6m-4d"))
            .min_interval(crate::time_period!("6m"))
            .earliest_recommended_interval(crate::time_period!("6m"))
            .latest_recommended_interval(crate::time_period!("4y+4w"))
        )
        .build()
}
