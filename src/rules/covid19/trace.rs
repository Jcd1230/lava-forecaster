#![allow(dead_code)]

use crate::models::{DoseEvaluation, SeriesForecast};
use crate::rules::covid19::facts::CovidDoseFact;
use crate::rules::covid19::policy::{CovidSeriesPolicy, CvxRelationship};

#[derive(Debug, Clone)]
pub struct CovidTraceDose {
    pub date: chrono::NaiveDate,
    pub cvx: crate::models::Cvx,
    pub season: &'static str,
    pub product_family: &'static str,
    pub relationship: CvxRelationship,
    pub evaluation: Option<DoseEvaluation>,
}

#[derive(Debug, Clone)]
pub struct CovidTrace<'a> {
    pub selected_series: &'a CovidSeriesPolicy,
    pub doses: Vec<CovidTraceDose>,
    pub forecast: Option<SeriesForecast>,
}

impl<'a> CovidTrace<'a> {
    pub fn from_policy_inputs(
        selected_series: &'a CovidSeriesPolicy,
        facts: &[CovidDoseFact<'_>],
        evaluations: &[DoseEvaluation],
        forecast: Option<SeriesForecast>,
    ) -> Self {
        let doses = facts
            .iter()
            .map(|fact| CovidTraceDose {
                date: fact.raw.date,
                cvx: fact.raw.cvx,
                season: fact.season.ice_key(),
                product_family: product_family_name(fact.product.family),
                relationship: fact.relationship_to(selected_series),
                evaluation: evaluations
                    .iter()
                    .find(|eval| eval.dose_date == fact.raw.date && eval.cvx == fact.raw.cvx)
                    .cloned(),
            })
            .collect();

        Self {
            selected_series,
            doses,
            forecast,
        }
    }
}

fn product_family_name(family: crate::rules::covid19::products::CovidProductFamily) -> &'static str {
    use crate::rules::covid19::products::CovidProductFamily;
    match family {
        CovidProductFamily::PfizerPediatric => "PfizerPediatric",
        CovidProductFamily::PfizerAdult => "PfizerAdult",
        CovidProductFamily::ModernaPediatric => "ModernaPediatric",
        CovidProductFamily::ModernaAdult => "ModernaAdult",
        CovidProductFamily::Novavax => "Novavax",
        CovidProductFamily::Janssen => "Janssen",
        CovidProductFamily::OldMonovalent => "OldMonovalent",
        CovidProductFamily::OldBivalent => "OldBivalent",
        CovidProductFamily::Unspecified => "Unspecified",
        CovidProductFamily::OtherSupported => "OtherSupported",
        CovidProductFamily::Unsupported => "Unsupported",
    }
}
