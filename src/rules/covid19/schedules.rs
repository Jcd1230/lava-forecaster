use crate::schedule::CompiledSeries;

const ALL_COVID_CVX: &[&str] = &[
	"207", "208", "211", "212", "213", "217", "218", "219", "221", "228", "229", "272", "300",
	"301", "302", "308", "309", "310", "311", "312", "313", "334", "502", "519",
];

pub fn covid19_aug2025_lt2_series() -> CompiledSeries {
	CompiledSeries::builder("COVID_19_AUG_2025_LT_2_SERIES")
		.code("COVID_19_AUG_2025_LT_2_SERIES")
		.vaccine_group("COVID19")
		.num_doses(2)
		.dose(1, |d| {
			d.abs_min_age("6m-4d")
				.min_age("6m")
				.earliest_recommended_age("6m")
				.cvx(ALL_COVID_CVX)
		})
		.dose(2, |d| d.cvx(ALL_COVID_CVX))
		.interval(1, 2, |i| {
			i.abs_min_interval("24d")
				.min_interval("28d")
				.earliest_recommended_interval("28d")
				.latest_recommended_interval("8w")
		})
		.build()
}

pub fn covid19_aug2025_2y_to_64y_series() -> CompiledSeries {
	CompiledSeries::builder("COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES")
		.code("COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES")
		.vaccine_group("COVID19")
		.num_doses(1)
		.dose(1, |d| d.abs_min_age("2y").cvx(ALL_COVID_CVX))
		.build()
}

pub fn covid19_aug2025_gte65_series() -> CompiledSeries {
	CompiledSeries::builder("COVID_19_AUG_2025_GTE_65_SERIES")
		.code("COVID_19_AUG_2025_GTE_65_SERIES")
		.vaccine_group("COVID19")
		.num_doses(2)
		.dose(1, |d| d.abs_min_age("65y").cvx(ALL_COVID_CVX))
		.dose(2, |d| d.cvx(ALL_COVID_CVX))
		.interval(1, 2, |i| {
			i.abs_min_interval("8w-4d")
				.min_interval("8w")
				.earliest_recommended_interval("6m")
		})
		.build()
}
