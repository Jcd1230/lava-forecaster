use crate::schedule::CompiledSeries;

pub fn hepa_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &["31", "52", "83", "84", "85", "104"];

    CompiledSeries::builder("HEP_A_2_DOSE_CHILD_ADULT_SERIES")
        .code("HEP_A_2_DOSE_CHILD_ADULT_SERIES")
        .vaccine_group("HEP_A")
        .num_doses(2)
        .dose(1, |d| d
            .abs_min_age("12m-4d")
            .min_age("12m")
            .earliest_recommended_age("12m")
            .latest_recommended_age("24m+4w")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .abs_min_age("18m-4d")
            .min_age("18m")
            .earliest_recommended_age("18m")
            .latest_recommended_age("24m+4w")
            .cvx(allowed_cvx)
        )
        .interval(1, 2, |i| i
            .abs_min_interval("6m-4d")
            .min_interval("6m")
            .earliest_recommended_interval("6m")
            .latest_recommended_interval("19m+4w")
        )
        .build()
}

pub fn hepa_adult_3_dose_series() -> CompiledSeries {
    let allowed_cvx = &["31", "52", "83", "84", "85", "104"];

    CompiledSeries::builder("HEP_A_ADULT_3_DOSE_SERIES")
        .code("HEP_A_ADULT_3_DOSE_SERIES")
        .vaccine_group("HEP_A")
        .num_doses(3)
        .dose(1, |d| d
            .abs_min_age("18y-4d")
            .min_age("19y")
            .earliest_recommended_age("19y")
            .cvx(allowed_cvx)
        )
        .dose(2, |d| d
            .cvx(allowed_cvx)
        )
        .dose(3, |d| d
            .cvx(allowed_cvx)
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

pub fn hepa_4_dose_twinrix_series() -> CompiledSeries {
    let twinrix_cvx = &["104"];
    let other_cvx = &["31", "52", "83", "84", "85", "104"];

    CompiledSeries::builder("HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES")
        .code("HEP_A_4_DOSE_ACCELERATED_TWINRIX_SERIES")
        .vaccine_group("HEP_A")
        .num_doses(4)
        .dose(1, |d| d
            .abs_min_age("18y-4d")
            .min_age("19y")
            .earliest_recommended_age("19y")
            .cvx(twinrix_cvx)
        )
        .dose(2, |d| d
            .cvx(twinrix_cvx)
        )
        .dose(3, |d| d
            .cvx(twinrix_cvx)
        )
        .dose(4, |d| d
            .cvx(other_cvx)
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
