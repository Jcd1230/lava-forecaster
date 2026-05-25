use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Gender {
    Female,
    Male,
    Unknown,
}

/// A CDC CVX code stored as a compact integer.
/// Serializes/deserializes as a zero-padded string ("03") for API compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Cvx(pub u16);

impl Cvx {
    pub fn new(n: u16) -> Self { Self(n) }
    pub fn as_u16(self) -> u16 { self.0 }
}

impl std::fmt::Display for Cvx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 10 {
            write!(f, "0{}", self.0)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

use ice_cvx_macro::generate_cvx_registry;

// Generates `cvx_to_id` and `id_to_cvx` helpers
generate_cvx_registry!();

impl Serialize for Cvx {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let raw = id_to_cvx(self.0).unwrap_or_else(|| self.0.to_string());
        s.serialize_str(&raw)
    }
}

impl<'de> Deserialize<'de> for Cvx {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct CvxVisitor;
        impl<'de> serde::de::Visitor<'de> for CvxVisitor {
            type Value = Cvx;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CVX string (e.g. \"03\") or integer")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                cvx_to_id(value)
                    .map(Cvx)
                    .ok_or_else(|| serde::de::Error::custom("invalid CVX string"))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value <= u16::MAX as u64 {
                    Ok(Cvx(value as u16))
                } else {
                    Err(serde::de::Error::custom("CVX integer out of bounds"))
                }
            }
        }
        d.deserialize_any(CvxVisitor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub birth_date: NaiveDate,
    pub gender: Gender,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Dose {
    pub date: NaiveDate,
    pub cvx: Cvx,
}

impl Default for Dose {
    fn default() -> Self {
        Self {
            date: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            cvx: Cvx::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DoseStatus {
    Valid,
    Invalid,
    Accepted,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvaluationReason {
    PriorToDOB,
    BelowMinimumAge,
    BelowMinimumAgeFinalDose,
    BelowMinimumInterval,
    TooEarlyLiveVirus,
    DuplicateShotSameDay,
    VaccineNotPartOfSeries,
    MissingAntigen,
    BoosterDose,
    VaccineNotCountedBasedOnMostRecentVaccineGiven,
    OutsideRoutineSeries,
    InsufficientAntigen,
    VaccineNotLicensedForMales,
    AboveRecommendedAgeSeries,
    OutsideFluVacSeason,
    VaccineNotAllowedInUs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoseEvaluation {
    pub dose_date: NaiveDate,
    pub cvx: Cvx,
    pub status: DoseStatus,
    pub reasons: Vec<EvaluationReason>,
    // The designated target dose number in the series
    pub dose_number: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeriesStatus {
    NotComplete,
    Complete,
    NotRecommended,
    ConditionallyRecommended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesForecast {
    pub series_name: std::borrow::Cow<'static, str>,
    pub earliest_date: Option<NaiveDate>,
    pub recommended_date: Option<NaiveDate>,
    pub overdue_date: Option<NaiveDate>,
    pub latest_date: Option<NaiveDate>,
    pub status: SeriesStatus,
    pub reasons: Vec<std::borrow::Cow<'static, str>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaccineGroupForecast {
    pub vaccine_group: std::borrow::Cow<'static, str>,
    pub evaluations: Vec<DoseEvaluation>,
    pub forecasts: Vec<SeriesForecast>,
    pub selected_series: Option<std::borrow::Cow<'static, str>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastRequest {
    pub patient: Patient,
    pub history: Vec<Dose>,
    pub execution_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResponse {
    pub vaccine_groups: Vec<VaccineGroupForecast>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkForecastRequest {
    pub requests: Vec<ForecastRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkForecastResponse {
    pub responses: Vec<ForecastResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedResults {
    pub evaluations: Vec<DoseEvaluation>,
    pub forecasts: Vec<SeriesForecast>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTestCase {
    pub name: String,
    pub group: String,
    pub focus_code: String,
    pub patient: Patient,
    pub history: Vec<Dose>,
    pub execution_date: NaiveDate,
    pub expected: Option<ExpectedResults>,
}

