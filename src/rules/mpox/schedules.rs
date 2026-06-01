use lava_cvx_macro::cvx;
use crate::schedule::CompiledSeries;

pub fn mpox_1_dose_series() -> CompiledSeries {
	CompiledSeries::builder("MPOX_1_DOSE_SERIES")
		.code("MPOX_1_DOSE_SERIES")
		.vaccine_group("MPOX")
		.num_doses(1)
		.dose(1, |dose| dose.abs_min_age(crate::time_period!("1y-4d")).cvx(&[cvx!("75"), cvx!("105")]))
		.build()
}

pub fn mpox_2_dose_series() -> CompiledSeries {
	CompiledSeries::builder("MPOX_2_DOSE_SERIES")
		.code("MPOX_2_DOSE_SERIES")
		.vaccine_group("MPOX")
		.num_doses(2)
		.dose(1, |dose| dose.abs_min_age(crate::time_period!("0d")).cvx(&[cvx!("206"), cvx!("325")]))
		.dose(2, |dose| dose.cvx(&[cvx!("206"), cvx!("75"), cvx!("105"), cvx!("325")]))
		.interval(1, 2, |interval| {
			interval
				.abs_min_interval(crate::time_period!("1d"))
				.min_interval(crate::time_period!("28d"))
				.earliest_recommended_interval(crate::time_period!("28d"))
		})
		.build()
}
