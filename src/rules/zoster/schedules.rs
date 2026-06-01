use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn zoster_2_dose_series() -> CompiledSeries {
    // CVX 187 = Zoster recombinant (Shingrix) - the primary vaccine
    // CVX 121 = Zoster live (Zostavax) - old live vaccine, counts as Accepted
    // CVX 188 = Zoster recombinant, unspecified - also Accepted
    let allowed_cvx = &[cvx!("187"), cvx!("121"), cvx!("188")];

    CompiledSeries::builder("ZOSTER_2_DOSE_SERIES")
        .code("ZOSTER_2_DOSE_SERIES")
        .vaccine_group("ZOSTER")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("18y"))
            .min_age(crate::time_period!("50y"))
            .earliest_recommended_age(crate::time_period!("50y"))
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("50y"))
            .min_age(crate::time_period!("50y"))
            .earliest_recommended_age(crate::time_period!("50y"))
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("56d"))
            .latest_recommended_interval(crate::time_period!("7m+4w"))
        )
        .build()
}
