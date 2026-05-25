use ice_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn typhoid_risk_series() -> CompiledSeries {
    let allowed_cvx = &[cvx!("25"), cvx!("101")];

    CompiledSeries::builder("TYPHOID_RISK_SERIES")
        .code("TYPHOID_RISK_SERIES")
        .vaccine_group("TYPHOID")
        .num_doses(1)
        .dose(1, |d| {
            d.abs_min_age("2y-4d")
                .min_age("2y")
                .earliest_recommended_age("2y")
                .cvx(allowed_cvx)
        })
        .build()
}
