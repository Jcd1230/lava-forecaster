use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn mcv_42_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("147"), cvx!("32"), cvx!("114"), cvx!("136"), cvx!("108"), cvx!("203"), cvx!("316"), cvx!("328")];

    CompiledSeries::builder("MCV_42_DOSE_SERIES")
        .code("MCV_42_DOSE_SERIES")
        .vaccine_group("MCV")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("10y"))
            .min_age(crate::time_period!("11y"))
            .earliest_recommended_age(crate::time_period!("11y"))
            .latest_recommended_age(crate::time_period!("13y+4w"))
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("16y-4d"))
            .min_age(crate::time_period!("16y"))
            .earliest_recommended_age(crate::time_period!("16y"))
            .latest_recommended_age(crate::time_period!("17y+4w"))
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("52d"))
            .min_interval(crate::time_period!("56d"))
            .earliest_recommended_interval(crate::time_period!("56d"))
            .latest_recommended_interval(crate::time_period!("6y+4w"))
        )
        .build()
}
