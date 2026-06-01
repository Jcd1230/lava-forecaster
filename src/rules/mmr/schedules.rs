use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn mmr_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("03"), cvx!("04"), cvx!("05"), cvx!("06"), cvx!("07"), cvx!("38"), cvx!("94")];

    CompiledSeries::builder("MMR_2_DOSE_SERIES")
        .code("MMR_2_DOSE_SERIES")
        .vaccine_group("MMR")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("1y-4d"))
            .min_age(crate::time_period!("1y"))
            .earliest_recommended_age(crate::time_period!("1y"))
            .latest_recommended_age(crate::time_period!("16m+4w"))
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("13m-4d"))
            .min_age(crate::time_period!("13m"))
            .earliest_recommended_age(crate::time_period!("4y"))
            .latest_recommended_age(crate::time_period!("7y+4w"))
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("6y+4w"))
        )
        .build()
}
