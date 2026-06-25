use crate::schedule::CompiledSeries;
use lava_cvx_macro::cvx;

const ALLOWED_CVX: &[u16] = &[
    cvx!("151"),
    cvx!("144"),
    cvx!("149"),
    cvx!("88"),
    cvx!("111"),
    cvx!("155"),
    cvx!("15"),
    cvx!("141"),
    cvx!("153"),
    cvx!("16"),
    cvx!("135"),
    cvx!("150"),
    cvx!("140"),
    cvx!("158"),
    cvx!("161"),
    cvx!("166"),
    cvx!("168"),
    cvx!("171"),
    cvx!("185"),
    cvx!("186"),
    cvx!("194"),
    cvx!("197"),
    cvx!("200"),
    cvx!("201"),
    cvx!("202"),
    cvx!("205"),
    cvx!("231"),
    cvx!("320"),
    cvx!("331"),
    cvx!("333"),
];

pub fn influenza_1_dose_series() -> CompiledSeries {
    CompiledSeries::builder("INFLUENZA_1_DOSE_SERIES")
        .code("INFLUENZA_1_DOSE_SERIES")
        .vaccine_group("INFLUENZA")
        .num_doses(1)
        .dose(1, |d| {
            d.abs_min_age(crate::time_period!("6m-4d"))
                .min_age(crate::time_period!("6m"))
                .earliest_recommended_age(crate::time_period!("6m"))
                .cvx(ALLOWED_CVX)
        })
        .build()
}

pub fn influenza_2_dose_series() -> CompiledSeries {
    CompiledSeries::builder("INFLUENZA_2_DOSE_SERIES")
        .code("INFLUENZA_2_DOSE_SERIES")
        .vaccine_group("INFLUENZA")
        .num_doses(2)
        .dose(1, |d| {
            d.abs_min_age(crate::time_period!("6m-4d"))
                .min_age(crate::time_period!("6m"))
                .earliest_recommended_age(crate::time_period!("6m"))
                .cvx(ALLOWED_CVX)
        })
        .dose(2, |d| d.cvx(ALLOWED_CVX))
        .interval(1, 2, |i| {
            i.abs_min_interval(crate::time_period!("24d"))
                .min_interval(crate::time_period!("28d"))
                .earliest_recommended_interval(crate::time_period!("28d"))
        })
        .build()
}

pub fn influenza_2_dose_default_series() -> CompiledSeries {
    CompiledSeries::builder("INFLUENZA_2_DOSE_DEFAULT_SERIES")
        .code("INFLUENZA_2_DOSE_DEFAULT_SERIES")
        .vaccine_group("INFLUENZA")
        .num_doses(2)
        .dose(1, |d| {
            d.abs_min_age(crate::time_period!("6m-4d"))
                .min_age(crate::time_period!("6m"))
                .earliest_recommended_age(crate::time_period!("6m"))
                .cvx(ALLOWED_CVX)
        })
        .dose(2, |d| d.cvx(ALLOWED_CVX))
        .interval(1, 2, |i| {
            i.abs_min_interval(crate::time_period!("24d"))
                .min_interval(crate::time_period!("28d"))
                .earliest_recommended_interval(crate::time_period!("28d"))
        })
        .build()
}
