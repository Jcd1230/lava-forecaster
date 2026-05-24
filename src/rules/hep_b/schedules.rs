use crate::schedule::CompiledSeries;

const ALLOWED_CVX: &[u16] = &[8, 42, 43, 44, 45, 51, 102, 104, 110, 132, 146, 189, 198, 220];

pub fn hep_b_3_dose_child_adolescent_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES")
        .code("HEP_B_3_DOSE_CHILD_ADOLESCENT_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age("0d")
            .min_age("0d")
            .earliest_recommended_age("0d")
            .latest_recommended_age("4w")
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .abs_min_age("24d")
            .min_age("28d")
            .earliest_recommended_age("1m")
            .latest_recommended_age("3m+4w")
            .cvx(ALLOWED_CVX)
        )
        .dose(3, |d| d
            .abs_min_age("164d")
            .min_age("168d")
            .earliest_recommended_age("6m")
            .latest_recommended_age("19m+4w")
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("5m+4w")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("52d")
            .min_interval("56d")
            .earliest_recommended_interval("56d")
            .latest_recommended_interval("18m+4w")
        )
        .interval(1, 3, |i| i
            .abs_min_interval("112d")
            .min_interval("16w")
            .earliest_recommended_interval("112d")
        )
        .build()
}

pub fn hep_b_4_dose_child_adolescent_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES")
        .code("HEP_B_4_DOSE_CHILD_ADOLESCENT_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(4)
        .dose(1, |d| d
            .abs_min_age("0d")
            .min_age("0d")
            .earliest_recommended_age("0d")
            .latest_recommended_age("4w")
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .abs_min_age("24d")
            .min_age("28d")
            .earliest_recommended_age("1m")
            .latest_recommended_age("3m+4w")
            .cvx(ALLOWED_CVX)
        )
        .dose(3, |d| d
            .cvx(ALLOWED_CVX)
        )
        .dose(4, |d| d
            .abs_min_age("164d")
            .min_age("168d")
            .earliest_recommended_age("6m")
            .latest_recommended_age("19m+4w")
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("5m+4w")
        )
        .interval(2, 4, |i| i
            .abs_min_interval("52d")
            .min_interval("56d")
            .earliest_recommended_interval("56d")
            .latest_recommended_interval("18m+4w")
        )
        .interval(3, 4, |i| i
            .abs_min_interval("0d")
            .min_interval("0d")
            .earliest_recommended_interval("0d")
        )
        .interval(1, 4, |i| i
            .abs_min_interval("112d")
            .min_interval("16w")
            .earliest_recommended_interval("112d")
        )
        .build()
}

pub fn hep_b_3_dose_twinrix_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_3_DOSE_TWINRIX_SERIES")
        .code("HEP_B_3_DOSE_TWINRIX_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age("18y-4d")
            .min_age("18y")
            .earliest_recommended_age("18y")
            .cvx(&[104])
        )
        .dose(2, |d| d
            .cvx(&[104])
        )
        .dose(3, |d| d
            .cvx(&[43, 104, 220])
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("5m-4d")
            .min_interval("5m")
            .earliest_recommended_interval("5m")
        )
        .build()
}

pub fn hep_b_4_dose_accelerated_twinrix_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES")
        .code("HEP_B_4_DOSE_ACCELERATED_TWINRIX_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(4)
        .dose(1, |d| d
            .abs_min_age("18y-4d")
            .min_age("18y")
            .earliest_recommended_age("18y")
            .cvx(&[104])
        )
        .dose(2, |d| d
            .cvx(&[104])
        )
        .dose(3, |d| d
            .cvx(&[104])
        )
        .dose(4, |d| d
            .cvx(&[43, 104, 220])
        )
        .interval(1, 2, |i| i
            .abs_min_interval("7d")
            .min_interval("7d")
            .earliest_recommended_interval("7d")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("14d")
            .min_interval("14d")
            .earliest_recommended_interval("14d")
            .latest_recommended_interval("23d")
        )
        .build()
}

pub fn hep_b_adult_2_dose_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_ADULT_2_DOSE_SERIES")
        .code("HEP_B_ADULT_2_DOSE_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age("18y-4d")
            .min_age("18y")
            .earliest_recommended_age("18y")
            .latest_recommended_age("18y")
            .cvx(&[189])
        )
        .dose(2, |d| d
            .cvx(&[189])
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
            .latest_recommended_interval("56d")
        )
        .build()
}

pub fn hep_b_adult_3_dose_series() -> CompiledSeries {
    CompiledSeries::builder("HEP_B_ADULT_3_DOSE_SERIES")
        .code("HEP_B_ADULT_3_DOSE_SERIES")
        .vaccine_group("HEP_B")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age("19y")
            .min_age("19y")
            .earliest_recommended_age("19y")
            .cvx(ALLOWED_CVX)
        )
        .dose(2, |d| d
            .abs_min_age("19y")
            .min_age("19y")
            .earliest_recommended_age("19y")
            .cvx(ALLOWED_CVX)
        )
        .dose(3, |d| d
            .abs_min_age("19y")
            .min_age("19y")
            .earliest_recommended_age("19y")
            .cvx(ALLOWED_CVX)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("24d")
            .min_interval("28d")
            .earliest_recommended_interval("28d")
        )
        .interval(2, 3, |i| i
            .abs_min_interval("52d")
            .min_interval("56d")
            .earliest_recommended_interval("56d")
        )
        .build()
}
