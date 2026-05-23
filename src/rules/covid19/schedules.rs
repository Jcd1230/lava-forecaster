use crate::schedule::CompiledSeries;

pub fn covid19_aug2025_lt2_series() -> CompiledSeries {
	let allowed_cvx = &["213", "309", "310", "311", "312", "313", "334"];

	CompiledSeries::builder("COVID_19_AUG_2025_LT_2_SERIES")
		.code("COVID_19_AUG_2025_LT_2_SERIES")
		.vaccine_group("COVID19")
		.num_doses(2)
		.dose(1, |d| {
			d.abs_min_age("6m-4d")
				.min_age("6m")
				.earliest_recommended_age("6m")
				.cvx(allowed_cvx)
		})
		.dose(2, |d| d.cvx(allowed_cvx))
		.interval(1, 2, |i| {
			i.abs_min_interval("24d")
				.min_interval("28d")
				.earliest_recommended_interval("28d")
				.latest_recommended_interval("8w")
		})
		.build()
}

pub fn covid19_aug2025_2y_to_64y_series() -> CompiledSeries {
	let allowed_cvx = &["213", "309", "310", "311", "312", "313", "334"];

	CompiledSeries::builder("COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES")
		.code("COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES")
		.vaccine_group("COVID19")
		.num_doses(1)
		.dose(1, |d| d.abs_min_age("2y").cvx(allowed_cvx))
		.build()
}

pub fn covid19_aug2025_gte65_series() -> CompiledSeries {
	let allowed_cvx = &["213", "309", "312", "313", "334"];

	CompiledSeries::builder("COVID_19_AUG_2025_GTE_65_SERIES")
		.code("COVID_19_AUG_2025_GTE_65_SERIES")
		.vaccine_group("COVID19")
		.num_doses(2)
		.dose(1, |d| d.abs_min_age("65y").cvx(allowed_cvx))
		.dose(2, |d| d.cvx(allowed_cvx))
		.interval(1, 2, |i| {
			i.abs_min_interval("8w-4d")
				.min_interval("8w")
				.earliest_recommended_interval("6m")
		})
		.build()
}
