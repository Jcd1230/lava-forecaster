use crate::schedule::CompiledSeries;

pub fn yellow_fever_risk_series() -> CompiledSeries {
    let allowed_cvx = &["37", "183", "184"];

    CompiledSeries::builder("YELLOW_FEVER_RISK_SERIES")
        .code("YELLOW_FEVER_RISK_SERIES")
        .vaccine_group("YELLOW_FEVER")
        .num_doses(1)
        .dose(1, |d| {
            d.abs_min_age("6m")
                .min_age("6m")
                .earliest_recommended_age("9m")
                .cvx(allowed_cvx)
        })
        .build()
}
