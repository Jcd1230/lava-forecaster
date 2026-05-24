use crate::schedule::CompiledSeries;

pub fn cholera_1_dose_risk_series() -> CompiledSeries {
    let allowed_cvx = &[174];

    CompiledSeries::builder("CHOLERA_1_DOSE_RISK_SERIES")
        .code("CHOLERA_1_DOSE_RISK_SERIES")
        .vaccine_group("CHOLERA")
        .num_doses(1)
        .dose(1, |d| d
            .abs_min_age("2y-4d")
            .min_age("2y")
            .earliest_recommended_age("2y")
            .cvx(allowed_cvx)
        )
        .build()
}
