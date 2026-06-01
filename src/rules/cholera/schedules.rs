use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn cholera_1_dose_risk_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("174")];

    CompiledSeries::builder("CHOLERA_1_DOSE_RISK_SERIES")
        .code("CHOLERA_1_DOSE_RISK_SERIES")
        .vaccine_group("CHOLERA")
        .num_doses(1)
        .dose(1, |d| d
            .abs_min_age(crate::time_period!("2y-4d"))
            .min_age(crate::time_period!("2y"))
            .earliest_recommended_age(crate::time_period!("2y"))
            .cvx(allowed_cvx)
        )
        .build()
}
