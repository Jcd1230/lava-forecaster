use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::date_utils::TimePeriod;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlVaccine {
    pub code: String,
    #[serde(rename = "display-name")]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDoseVaccine {
    pub preferred: bool,
    pub vaccine: YamlVaccine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDoseRule {
    #[serde(rename = "absolute-minimum-age")]
    pub absolute_minimum_age: Option<String>,
    #[serde(rename = "minimum-age")]
    pub minimum_age: Option<String>,
    #[serde(rename = "earliest-recommended-age")]
    pub earliest_recommended_age: Option<String>,
    #[serde(rename = "latest-recommended-age")]
    pub latest_recommended_age: Option<String>,
    #[serde(rename = "dose-vaccines")]
    pub dose_vaccines: HashMap<String, YamlDoseVaccine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDoseInterval {
    #[serde(rename = "from-dose-number")]
    pub from_dose_number: usize,
    #[serde(rename = "to-dose-number")]
    pub to_dose_number: usize,
    #[serde(rename = "absolute-minimum-interval")]
    pub absolute_minimum_interval: Option<String>,
    #[serde(rename = "minimum-interval")]
    pub minimum_interval: Option<String>,
    #[serde(rename = "earliest-recommended-interval")]
    pub earliest_recommended_interval: Option<String>,
    #[serde(rename = "latest-recommended-interval")]
    pub latest_recommended_interval: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlSeriesDetails {
    #[serde(rename = "series-id")]
    pub series_id: String,
    pub series: HashMap<String, String>, // contains display-name, code, etc.
    #[serde(rename = "number-of-doses-in-series")]
    pub number_of_doses_in_series: usize,
    #[serde(rename = "dose-intervals")]
    pub dose_intervals: HashMap<String, YamlDoseInterval>,
    pub doses: HashMap<String, YamlDoseRule>,
    #[serde(rename = "vaccine-group")]
    pub vaccine_group: HashMap<String, HashMap<String, String>>,
}

// Wrapper structure to match ICE's YAML format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceSupportingData {
    #[serde(rename = "ice-supporting-data")]
    pub ice_supporting_data: KnowledgeModules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeModules {
    #[serde(rename = "knowledge-modules")]
    pub knowledge_modules: HashMap<String, SeriesMap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesMap {
    pub series: HashMap<String, YamlSeriesDetails>,
}

// Compiled static runtime representation of a series
#[derive(Debug, Clone)]
pub struct CompiledDoseRule {
    pub dose_number: usize,
    pub absolute_minimum_age: Option<TimePeriod>,
    pub minimum_age: Option<TimePeriod>,
    pub earliest_recommended_age: Option<TimePeriod>,
    pub latest_recommended_age: Option<TimePeriod>,
    pub allowed_cvx: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledDoseInterval {
    pub from_dose: usize,
    pub to_dose: usize,
    pub absolute_minimum_interval: Option<TimePeriod>,
    pub minimum_interval: Option<TimePeriod>,
    pub earliest_recommended_interval: Option<TimePeriod>,
    pub latest_recommended_interval: Option<TimePeriod>,
}

#[derive(Debug, Clone)]
pub struct CompiledSeries {
    pub name: String,
    pub code: String,
    pub vaccine_group: String,
    pub num_doses: usize,
    pub doses: Vec<CompiledDoseRule>,
    pub intervals: Vec<CompiledDoseInterval>,
}

impl CompiledSeries {
    pub fn from_yaml(name: &str, details: &YamlSeriesDetails) -> Result<Self, String> {
        let code = details.series.get("code").cloned().unwrap_or_else(|| name.to_string());
        
        let vaccine_group = details.vaccine_group
            .get("1")
            .and_then(|g| g.get("code"))
            .cloned()
            .ok_or_else(|| format!("Series {} lacks vaccine-group code", name))?;

        let mut doses = Vec::new();
        for i in 1..=details.number_of_doses_in_series {
            let key = i.to_string();
            let rule = details.doses.get(&key)
                .ok_or_else(|| format!("Series {} lacks dose {}", name, i))?;
            
            let abs_min_age = rule.absolute_minimum_age.as_deref()
                .map(TimePeriod::parse).transpose()?;
            let min_age = rule.minimum_age.as_deref()
                .map(TimePeriod::parse).transpose()?;
            let earliest_rec = rule.earliest_recommended_age.as_deref()
                .map(TimePeriod::parse).transpose()?;
            let latest_rec = rule.latest_recommended_age.as_deref()
                .map(TimePeriod::parse).transpose()?;

            let mut allowed_cvx = Vec::new();
            for dv in rule.dose_vaccines.values() {
                allowed_cvx.push(dv.vaccine.code.clone());
            }

            doses.push(CompiledDoseRule {
                dose_number: i,
                absolute_minimum_age: abs_min_age,
                minimum_age: min_age,
                earliest_recommended_age: earliest_rec,
                latest_recommended_age: latest_rec,
                allowed_cvx,
            });
        }

        let mut intervals = Vec::new();
        // Intervals are usually 1..num_doses-1
        for i in 1..details.number_of_doses_in_series {
            let key = i.to_string();
            if let Some(interval) = details.dose_intervals.get(&key) {
                let abs_min_int = interval.absolute_minimum_interval.as_deref()
                    .map(TimePeriod::parse).transpose()?;
                let min_int = interval.minimum_interval.as_deref()
                    .map(TimePeriod::parse).transpose()?;
                let earliest_rec_int = interval.earliest_recommended_interval.as_deref()
                    .map(TimePeriod::parse).transpose()?;
                let latest_rec_int = interval.latest_recommended_interval.as_deref()
                    .map(TimePeriod::parse).transpose()?;

                intervals.push(CompiledDoseInterval {
                    from_dose: interval.from_dose_number,
                    to_dose: interval.to_dose_number,
                    absolute_minimum_interval: abs_min_int,
                    minimum_interval: min_int,
                    earliest_recommended_interval: earliest_rec_int,
                    latest_recommended_interval: latest_rec_int,
                });
            }
        }

        Ok(CompiledSeries {
            name: name.to_string(),
            code,
            vaccine_group,
            num_doses: details.number_of_doses_in_series,
            doses,
            intervals,
        })
    }
}
