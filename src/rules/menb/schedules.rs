use ice_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn men_b_4c_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("163"), cvx!("328")];

    CompiledSeries::builder("MEN_B_4_C_2_DOSE_SERIES")
        .code("MEN_B_4_C_2_DOSE_SERIES")
        .vaccine_group("MENB")
        .num_doses(2)
        .dose(1, |d| {
            d.abs_min_age("16y-4d")
                .min_age("16y")
                .earliest_recommended_age("16y")
                .cvx(allowed_cvx)
        })
        .dose(2, |d| d.cvx(allowed_cvx))
        .interval(1, 2, |i| {
            i.abs_min_interval("4m-4d")
                .min_interval("4m")
                .earliest_recommended_interval("4m")
        })
        .build()
}

pub fn men_b_4c_3_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("163"), cvx!("328")];

    CompiledSeries::builder("MEN_B_4_C_3_DOSE_SERIES")
        .code("MEN_B_4_C_3_DOSE_SERIES")
        .vaccine_group("MENB")
        .num_doses(3)
        .dose(1, |d| {
            d.abs_min_age("10y-4d")
                .min_age("10y")
                .earliest_recommended_age("10y")
                .cvx(allowed_cvx)
        })
        .dose(2, |d| {
            d.abs_min_age("10y-4d")
                .min_age("10y")
                .earliest_recommended_age("10y")
                .cvx(allowed_cvx)
        })
        .dose(3, |d| {
            d.abs_min_age("10y-4d")
                .min_age("10y")
                .earliest_recommended_age("10y")
                .cvx(allowed_cvx)
        })
        .interval(1, 2, |i| {
            i.abs_min_interval("4w-4d")
                .min_interval("4w")
                .earliest_recommended_interval("4w")
                .latest_recommended_interval("8w")
        })
        .interval(2, 3, |i| {
            i.abs_min_interval("4m-4d")
                .min_interval("4m")
                .earliest_recommended_interval("4m")
        })
        .build()
}

pub fn men_b_fhbp_2_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("162"), cvx!("316")];

    CompiledSeries::builder("MEN_BF_HBP_2_DOSE_SERIES")
        .code("MEN_BF_HBP_2_DOSE_SERIES")
        .vaccine_group("MENB")
        .num_doses(2)
        .dose(1, |d| {
            d.abs_min_age("16y-4d")
                .min_age("16y")
                .earliest_recommended_age("16y")
                .cvx(allowed_cvx)
        })
        .dose(2, |d| d.cvx(allowed_cvx))
        .interval(1, 2, |i| {
            i.abs_min_interval("6m-4d")
                .min_interval("6m")
                .earliest_recommended_interval("6m")
        })
        .build()
}

pub fn men_b_fhbp_3_dose_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("162"), cvx!("316")];

    CompiledSeries::builder("MEN_BF_HBP_3_DOSE_SERIES")
        .code("MEN_BF_HBP_3_DOSE_SERIES")
        .vaccine_group("MENB")
        .num_doses(3)
        .dose(1, |d| {
            d.abs_min_age("10y-4d")
                .min_age("10y")
                .earliest_recommended_age("10y")
                .cvx(allowed_cvx)
        })
        .dose(2, |d| d.cvx(allowed_cvx))
        .dose(3, |d| d.cvx(allowed_cvx))
        .interval(1, 2, |i| {
            i.abs_min_interval("4w-4d")
                .min_interval("4w")
                .earliest_recommended_interval("4w")
        })
        .interval(2, 3, |i| {
            i.abs_min_interval("4m-4d")
                .min_interval("4m")
                .earliest_recommended_interval("4m")
        })
        .build()
}
