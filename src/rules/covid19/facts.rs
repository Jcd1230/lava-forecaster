
#![allow(dead_code)]

use crate::date_utils::compare_elapsed;
use crate::models::{Dose, Patient};
use crate::rules::covid19::policy::{CovidSeriesPolicy, CvxRelationship};
use crate::rules::covid19::products::{
    covid_product_info, is_aug2025_current_formulation, CovidProductInfo,
};
use crate::rules::covid19::seasons::CovidSeason;
use chrono::NaiveDate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CovidAgeFacts {
    pub at_least_6_months: bool,
    pub under_2_years: bool,
    pub under_5_years: bool,
    pub under_12_years: bool,
    pub at_least_65_years: bool,
}

impl CovidAgeFacts {
    pub fn at(patient: &Patient, date: NaiveDate) -> Self {
        Self {
            at_least_6_months: age_ge(patient, date, crate::time_period!("6m")),
            under_2_years: age_lt(patient, date, crate::time_period!("2y")),
            under_5_years: age_lt(patient, date, crate::time_period!("5y")),
            under_12_years: age_lt(patient, date, crate::time_period!("12y")),
            at_least_65_years: age_ge(patient, date, crate::time_period!("65y")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CovidDoseFact<'a> {
    pub raw: &'a Dose,
    pub season: CovidSeason,
    pub product: CovidProductInfo,
    pub age_at_dose: CovidAgeFacts,
    pub age_at_season_start: CovidAgeFacts,
    pub supported_by_java_covid: bool,
}

impl<'a> CovidDoseFact<'a> {
    pub fn from_dose(patient: &Patient, dose: &'a Dose) -> Self {
        let season = CovidSeason::for_date(dose.date);
        let product = covid_product_info(dose.cvx);
        Self {
            raw: dose,
            season,
            product,
            age_at_dose: CovidAgeFacts::at(patient, dose.date),
            age_at_season_start: CovidAgeFacts::at(patient, season.start_date()),
            supported_by_java_covid: product.supported_by_java_covid,
        }
    }

    pub fn relationship_to(self, selected: &CovidSeriesPolicy) -> CvxRelationship {
        if !self.supported_by_java_covid {
            return CvxRelationship::UnsupportedIgnored;
        }
        if selected.contains_cvx(self.raw.cvx) {
            return CvxRelationship::MemberOfSelectedSeries;
        }
        if selected.season == CovidSeason::Aug2025 && !is_aug2025_current_formulation(self.raw.cvx) {
            return CvxRelationship::SupportedButOldProduct;
        }
        CvxRelationship::CovidButNotThisSeries
    }
}

pub fn normalize_covid_history<'a>(patient: &Patient, history: &'a [Dose]) -> Vec<CovidDoseFact<'a>> {
    history
        .iter()
        .map(|dose| CovidDoseFact::from_dose(patient, dose))
        .collect()
}

fn age_ge(patient: &Patient, date: NaiveDate, age: crate::date_utils::TimePeriod) -> bool {
    compare_elapsed(patient.birth_date, date, &age) != std::cmp::Ordering::Less
}

fn age_lt(patient: &Patient, date: NaiveDate, age: crate::date_utils::TimePeriod) -> bool {
    compare_elapsed(patient.birth_date, date, &age) == std::cmp::Ordering::Less
}
