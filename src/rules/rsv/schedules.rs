use crate::schedule::CompiledSeries;

pub fn rsv_infant_series() -> CompiledSeries {
	let allowed_cvx = &["304", "306", "307", "315", "332"];

	CompiledSeries::builder("RSV_INFANT_SERIES")
		.code("RSV_INFANT_SERIES")
		.vaccine_group("RSV")
		.num_doses(1)
		.dose(1, |d| {
			d.abs_min_age("0d")
				.min_age("0d")
				.earliest_recommended_age("0d")
				.cvx(allowed_cvx)
		})
		.build()
}

pub fn rsv_adult_series() -> CompiledSeries {
	let allowed_cvx = &["303", "304", "305", "314", "326"];

	CompiledSeries::builder("RSV_ADULT_SERIES")
		.code("RSV_ADULT_SERIES")
		.vaccine_group("RSV")
		.num_doses(1)
		.dose(1, |d| {
			d.abs_min_age("50y")
				.min_age("75y")
				.earliest_recommended_age("75y")
				.cvx(allowed_cvx)
		})
		.build()
}
