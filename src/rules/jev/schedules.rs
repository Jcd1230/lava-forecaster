use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn jevc_risk_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("134")];

    CompiledSeries::builder("JEVC_RISK_2_DOSE_SERIES")
        .code("JEVC_RISK_2_DOSE_SERIES")
        .vaccine_group("JEV")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("2m-4d"))
            .min_age(crate::time_period!("2m"))
            .earliest_recommended_age(crate::time_period!("2m"))
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
        )
        .build()
}

pub fn jevc_risk_2_dose_accelerated_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("134")];

    CompiledSeries::builder("JEVC_RISK_2_DOSE_ACCELERATED_SERIES")
        .code("JEVC_RISK_2_DOSE_ACCELERATED_SERIES")
        .vaccine_group("JEV")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("18y-4d"))
            .min_age(crate::time_period!("18y"))
            .earliest_recommended_age(crate::time_period!("18y"))
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("7d"))
            .min_interval(crate::time_period!("7d"))
            .earliest_recommended_interval(crate::time_period!("7d"))
            .latest_recommended_interval(crate::time_period!("28d"))
        )
        .build()
}
