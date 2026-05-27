use ice_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn varicella_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("21"), cvx!("94")];

    CompiledSeries::builder("VARICELLA_2_DOSE_SERIES")
        .code("VARICELLA_2_DOSE_SERIES")
        .vaccine_group("VARICELLA")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("1y-4d"))
            .min_age(crate::time_period!("1y"))
            .earliest_recommended_age(crate::time_period!("1y"))
            .latest_recommended_age(crate::time_period!("16m+4w"))
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("13m"))
            .min_age(crate::time_period!("15m"))
            .earliest_recommended_age(crate::time_period!("4y"))
            .latest_recommended_age(crate::time_period!("7y+4w"))
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("28d"))
            .min_interval(crate::time_period!("84d"))
            .earliest_recommended_interval(crate::time_period!("84d"))
            .latest_recommended_interval(crate::time_period!("6y+4w"))
        )
        .build()
}
