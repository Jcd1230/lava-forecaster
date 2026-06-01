use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

const ALLOWED_CVX: &[u16] = &[cvx!("08"), cvx!("42"), cvx!("43"), cvx!("44"), cvx!("45"), cvx!("51"), cvx!("102"), cvx!("104"), cvx!("110"), cvx!("132"), cvx!("146"), cvx!("189"), cvx!("198"), cvx!("220")];

pub fn hep_b_3_dose_child_adolescent_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES")
        .code("HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("0d"))
            .min_age(crate::time_period!("0d"))
            .earliest_recommended_age(crate::time_period!("0d"))
            .latest_recommended_age(crate::time_period!("4w"))
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("24d"))
            .min_age(crate::time_period!("28d"))
            .earliest_recommended_age(crate::time_period!("1m"))
            .latest_recommended_age(crate::time_period!("3m+4w"))
            .cvx(ALLOWED_CVX)
        )
        .dose(3, |d| d
            .abs_min_age(crate::time_period!("164d"))
            .min_age(crate::time_period!("168d"))
            .earliest_recommended_age(crate::time_period!("6m"))
            .latest_recommended_age(crate::time_period!("19m+4w"))
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("5m+4w"))
        )
        .interval(2, 3, |i| i
            .abs_min_interval(crate::time_period!("52d"))
            .min_interval(crate::time_period!("56d"))
            .earliest_recommended_interval(crate::time_period!("56d"))
            .latest_recommended_interval(crate::time_period!("18m+4w"))
        )
        .interval(1, 3, |i| i
            .abs_min_interval(crate::time_period!("16w-4d"))
            .min_interval(crate::time_period!("16w"))
            .earliest_recommended_interval(crate::time_period!("16w-4d"))
        )
        .build()
}

pub fn hep_b_4_dose_child_adolescent_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES")
        .code("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(4)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("0d"))
            .min_age(crate::time_period!("0d"))
            .earliest_recommended_age(crate::time_period!("0d"))
            .latest_recommended_age(crate::time_period!("4w"))
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("24d"))
            .min_age(crate::time_period!("28d"))
            .earliest_recommended_age(crate::time_period!("1m"))
            .latest_recommended_age(crate::time_period!("3m+4w"))
            .cvx(ALLOWED_CVX)
        )
        .dose(3, |d| d
            .cvx(ALLOWED_CVX)
        )
        .dose(4, |d| d
            .abs_min_age(crate::time_period!("164d"))
            .min_age(crate::time_period!("168d"))
            .earliest_recommended_age(crate::time_period!("6m"))
            .latest_recommended_age(crate::time_period!("19m+4w"))
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("5m+4w"))
        )
        .interval(2, 4, |i| i
            .abs_min_interval(crate::time_period!("52d"))
            .min_interval(crate::time_period!("56d"))
            .earliest_recommended_interval(crate::time_period!("56d"))
            .latest_recommended_interval(crate::time_period!("18m+4w"))
        )
        .interval(3, 4, |i| i
            .abs_min_interval(crate::time_period!("0d"))
            .min_interval(crate::time_period!("0d"))
            .earliest_recommended_interval(crate::time_period!("0d"))
        )
        .interval(1, 4, |i| i
            .abs_min_interval(crate::time_period!("16w-4d"))
            .min_interval(crate::time_period!("16w"))
            .earliest_recommended_interval(crate::time_period!("16w-4d"))
        )
        .build()
}

pub fn hep_b_3_dose_twinrix_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_3_DOSE_TWINRIX_SERIES")
        .code("HEP_B_3_DOSE_TWINRIX_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("18y-4d"))
            .min_age(crate::time_period!("18y"))
            .earliest_recommended_age(crate::time_period!("18y"))
            .cvx(&[cvx!("104")])
        )
        .dose(2, |d| d
            .cvx(&[cvx!("104")])
        )
        .dose(3, |d| d
            .cvx(&[cvx!("43"), cvx!("104"), cvx!("220")])
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
        )
        .interval(2, 3, |i| i
            .abs_min_interval(crate::time_period!("5m-4d"))
            .min_interval(crate::time_period!("5m"))
            .earliest_recommended_interval(crate::time_period!("5m"))
        )
        .build()
}

pub fn hep_b_4_dose_accelerated_twinrix_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES")
        .code("HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(4)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("18y-4d"))
            .min_age(crate::time_period!("18y"))
            .earliest_recommended_age(crate::time_period!("18y"))
            .cvx(&[cvx!("104")])
        )
        .dose(2, |d| d
            .cvx(&[cvx!("104")])
        )
        .dose(3, |d| d
            .cvx(&[cvx!("104")])
        )
        .dose(4, |d| d
            .cvx(&[cvx!("43"), cvx!("104"), cvx!("220")])
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("7d"))
            .min_interval(crate::time_period!("7d"))
            .earliest_recommended_interval(crate::time_period!("7d"))
        )
        .interval(2, 3, |i| i
            .abs_min_interval(crate::time_period!("14d"))
            .min_interval(crate::time_period!("14d"))
            .earliest_recommended_interval(crate::time_period!("14d"))
            .latest_recommended_interval(crate::time_period!("23d"))
        )
        .build()
}

pub fn hep_b_adult_2_dose_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_ADULT_2_DOSE_SERIES")
        .code("HEP_B_ADULT_2_DOSE_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("18y-4d"))
            .min_age(crate::time_period!("18y"))
            .earliest_recommended_age(crate::time_period!("18y"))
            .latest_recommended_age(crate::time_period!("18y"))
            .cvx(&[cvx!("189")])
        )
        .dose(2, |d| d
            .cvx(&[cvx!("189")])
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
            .latest_recommended_interval(crate::time_period!("56d"))
        )
        .build()
}

pub fn hep_b_adult_3_dose_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_ADULT_3_DOSE_SERIES")
        .code("HEP_B_ADULT_3_DOSE_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("19y"))
            .min_age(crate::time_period!("19y"))
            .earliest_recommended_age(crate::time_period!("19y"))
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .abs_min_age(crate::time_period!("19y"))
            .min_age(crate::time_period!("19y"))
            .earliest_recommended_age(crate::time_period!("19y"))
            .cvx(ALLOWED_CVX)
        )
        .dose(3, |d| d
            .abs_min_age(crate::time_period!("19y"))
            .min_age(crate::time_period!("19y"))
            .earliest_recommended_age(crate::time_period!("19y"))
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval(crate::time_period!("24d"))
            .min_interval(crate::time_period!("28d"))
            .earliest_recommended_interval(crate::time_period!("28d"))
        )
        .interval(2, 3, |i| i
            .abs_min_interval(crate::time_period!("52d"))
            .min_interval(crate::time_period!("56d"))
            .earliest_recommended_interval(crate::time_period!("56d"))
        )
        .build()
}
