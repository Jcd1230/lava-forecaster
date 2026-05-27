use ice_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

const ALL_COVID_CVX: &[u16] = &[cvx!("207"), cvx!("208"), cvx!("211"), cvx!("212"), cvx!("213"), cvx!("217"), cvx!("218"), cvx!("219"), cvx!("221"), cvx!("228"), cvx!("229"), cvx!("272"), cvx!("300"), cvx!("301"), cvx!("302"), cvx!("308"), cvx!("309"), cvx!("310"), cvx!("311"), cvx!("312"), cvx!("313"), cvx!("334"), cvx!("502"), cvx!("519")];

pub fn covid19_aug2025_lt2_series() -> CompiledSeries {
	CompiledSeries::builder("COVID_19_AUG_2025_LT_2_SERIES")
		.code("COVID_19_AUG_2025_LT_2_SERIES")
		.vaccine_group("COVID19")
		.num_doses(2)
		.dose(1, |d| {
			d.abs_min_age(crate::time_period!("6m-4d"))
				.min_age(crate::time_period!("6m"))
				.earliest_recommended_age(crate::time_period!("6m"))
				.cvx(ALL_COVID_CVX)
		})
		.dose(2, |d| d.cvx(ALL_COVID_CVX))
		.interval(1, 2, |i| {
			i.abs_min_interval(crate::time_period!("24d"))
				.min_interval(crate::time_period!("28d"))
				.earliest_recommended_interval(crate::time_period!("28d"))
				.latest_recommended_interval(crate::time_period!("8w"))
		})
		.build()
}

pub fn covid19_aug2025_2y_to_64y_series() -> CompiledSeries {
	CompiledSeries::builder("COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES")
		.code("COVID_19_AUG_2025_2_Y_TO_64_Y_SERIES")
		.vaccine_group("COVID19")
		.num_doses(1)
		.dose(1, |d| d.abs_min_age(crate::time_period!("2y")).cvx(ALL_COVID_CVX))
		.build()
}

pub fn covid19_aug2025_gte65_series() -> CompiledSeries {
	CompiledSeries::builder("COVID_19_AUG_2025_GTE_65_SERIES")
		.code("COVID_19_AUG_2025_GTE_65_SERIES")
		.vaccine_group("COVID19")
		.num_doses(2)
		.dose(1, |d| d.abs_min_age(crate::time_period!("65y")).cvx(ALL_COVID_CVX))
		.dose(2, |d| d.cvx(ALL_COVID_CVX))
		.interval(1, 2, |i| {
			i.abs_min_interval(crate::time_period!("8w-4d"))
				.min_interval(crate::time_period!("8w"))
				.earliest_recommended_interval(crate::time_period!("6m"))
		})
		.build()
}
