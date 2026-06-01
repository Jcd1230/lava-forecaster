use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use crate::date_utils::TinyVec;

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

    // Named constants for commonly used CVX codes to eliminate magic numbers
    pub const MMR: u16 = 3;
    pub const MEASLES_RUBELLA: u16 = 4;
    pub const MEASLES: u16 = 5;
    pub const RUBELLA: u16 = 6;
    pub const MUMPS: u16 = 7;
    pub const PNEUMOCOCCAL_PPV23: u16 = 33;
    pub const YELLOW_FEVER: u16 = 37;
    pub const RUBELLA_MUMPS: u16 = 38;
    pub const MMRV: u16 = 94;
    pub const YELLOW_FEVER_UNSPECIFIED: u16 = 183;
    pub const YELLOW_FEVER_UNKNOWN: u16 = 184;
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

use lava_cvx_macro::generate_cvx_registry;

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
pub struct DiseaseImmunity {
    pub disease: String,
    pub date: NaiveDate,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contraindication {
    pub date: NaiveDate,
    pub target: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub birth_date: NaiveDate,
    pub gender: Gender,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub immunities: Vec<DiseaseImmunity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contraindications: Vec<Contraindication>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Dose {
    pub date: NaiveDate,
    pub cvx: Cvx,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_valid: Option<bool>,
}

impl Default for Dose {
    fn default() -> Self {
        Self {
            date: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            cvx: Cvx::default(),
            is_valid: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DoseStatus {
    #[default]
    Valid,
    Invalid,
    Accepted,
    Ignored,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum EvaluationReason {
    #[default]
    VaccineNotPartOfSeries,
    PriorToDOB,
    BelowMinimumAge,
    BelowMinimumAgeFinalDose,
    BelowMinimumInterval,
    TooEarlyLiveVirus,
    DuplicateShotSameDay,
    MissingAntigen,
    BoosterDose,
    VaccineNotCountedBasedOnMostRecentVaccineGiven,
    OutsideRoutineSeries,
    InsufficientAntigen,
    VaccineNotLicensedForMales,
    AboveRecommendedAgeSeries,
    OutsideFluVacSeason,
    VaccineNotAllowedInUs,
    DoseOverrideValid,
    DoseOverrideInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoseEvaluation {
    pub dose_date: NaiveDate,
    pub cvx: Cvx,
    pub status: DoseStatus,
    pub reasons: TinyVec<EvaluationReason, 4>,
    // The designated target dose number in the series
    pub dose_number: Option<usize>,
}

impl Default for DoseEvaluation {
    fn default() -> Self {
        Self {
            dose_date: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            cvx: Cvx::default(),
            status: DoseStatus::Valid,
            reasons: TinyVec::new(),
            dose_number: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SeriesStatus {
    NotComplete {
        earliest_date: Option<NaiveDate>,
        recommended_date: Option<NaiveDate>,
        overdue_date: Option<NaiveDate>,
        latest_date: Option<NaiveDate>,
    },
    Complete,
    NotRecommended,
    ConditionallyRecommended,
}

impl Default for SeriesStatus {
    fn default() -> Self {
        Self::NotComplete {
            earliest_date: None,
            recommended_date: None,
            overdue_date: None,
            latest_date: None,
        }
    }
}

impl SeriesStatus {
    pub fn earliest_date(&self) -> Option<NaiveDate> {
        if let Self::NotComplete { earliest_date, .. } = self {
            *earliest_date
        } else {
            None
        }
    }

    pub fn recommended_date(&self) -> Option<NaiveDate> {
        if let Self::NotComplete { recommended_date, .. } = self {
            *recommended_date
        } else {
            None
        }
    }

    pub fn overdue_date(&self) -> Option<NaiveDate> {
        if let Self::NotComplete { overdue_date, .. } = self {
            *overdue_date
        } else {
            None
        }
    }

    pub fn latest_date(&self) -> Option<NaiveDate> {
        if let Self::NotComplete { latest_date, .. } = self {
            *latest_date
        } else {
            None
        }
    }

    pub fn with_earliest_date(self, date: Option<NaiveDate>) -> Self {
        if let Self::NotComplete { recommended_date, overdue_date, latest_date, .. } = self {
            Self::NotComplete { earliest_date: date, recommended_date, overdue_date, latest_date }
        } else {
            self
        }
    }

    pub fn with_recommended_date(self, date: Option<NaiveDate>) -> Self {
        if let Self::NotComplete { earliest_date, overdue_date, latest_date, .. } = self {
            Self::NotComplete { earliest_date, recommended_date: date, overdue_date, latest_date }
        } else {
            self
        }
    }

    pub fn with_overdue_date(self, date: Option<NaiveDate>) -> Self {
        if let Self::NotComplete { earliest_date, recommended_date, latest_date, .. } = self {
            Self::NotComplete { earliest_date, recommended_date, overdue_date: date, latest_date }
        } else {
            self
        }
    }

    pub fn with_latest_date(self, date: Option<NaiveDate>) -> Self {
        if let Self::NotComplete { earliest_date, recommended_date, overdue_date, .. } = self {
            Self::NotComplete { earliest_date, recommended_date, overdue_date, latest_date: date }
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "FlatSeriesForecast", into = "FlatSeriesForecast")]
pub struct SeriesForecast {
    pub series_name: std::borrow::Cow<'static, str>,
    pub status: SeriesStatus,
    pub reasons: TinyVec<std::borrow::Cow<'static, str>, 2>,
}

#[derive(Serialize, Deserialize)]
struct FlatSeriesForecast {
    pub series_name: std::borrow::Cow<'static, str>,
    pub earliest_date: Option<NaiveDate>,
    pub recommended_date: Option<NaiveDate>,
    pub overdue_date: Option<NaiveDate>,
    pub latest_date: Option<NaiveDate>,
    pub status: FlatSeriesStatus,
    pub reasons: TinyVec<std::borrow::Cow<'static, str>, 2>,
}

#[derive(Serialize, Deserialize)]
enum FlatSeriesStatus {
    NotComplete,
    Complete,
    NotRecommended,
    ConditionallyRecommended,
}

impl From<SeriesForecast> for FlatSeriesForecast {
    fn from(f: SeriesForecast) -> Self {
        let status = match f.status {
            SeriesStatus::NotComplete { .. } => FlatSeriesStatus::NotComplete,
            SeriesStatus::Complete => FlatSeriesStatus::Complete,
            SeriesStatus::NotRecommended => FlatSeriesStatus::NotRecommended,
            SeriesStatus::ConditionallyRecommended => FlatSeriesStatus::ConditionallyRecommended,
        };
        Self {
            series_name: f.series_name,
            earliest_date: f.status.earliest_date(),
            recommended_date: f.status.recommended_date(),
            overdue_date: f.status.overdue_date(),
            latest_date: f.status.latest_date(),
            status,
            reasons: f.reasons,
        }
    }
}

impl From<FlatSeriesForecast> for SeriesForecast {
    fn from(f: FlatSeriesForecast) -> Self {
        let status = match f.status {
            FlatSeriesStatus::NotComplete => SeriesStatus::NotComplete {
                earliest_date: f.earliest_date,
                recommended_date: f.recommended_date,
                overdue_date: f.overdue_date,
                latest_date: f.latest_date,
            },
            FlatSeriesStatus::Complete => SeriesStatus::Complete,
            FlatSeriesStatus::NotRecommended => SeriesStatus::NotRecommended,
            FlatSeriesStatus::ConditionallyRecommended => SeriesStatus::ConditionallyRecommended,
        };
        Self {
            series_name: f.series_name,
            status,
            reasons: f.reasons,
        }
    }
}

impl Default for SeriesForecast {
    fn default() -> Self {
        Self {
            series_name: std::borrow::Cow::Borrowed(""),
            status: SeriesStatus::default(),
            reasons: TinyVec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaccineGroupForecast {
    pub vaccine_group: std::borrow::Cow<'static, str>,
    pub evaluations: TinyVec<DoseEvaluation, 8>,
    pub forecasts: TinyVec<SeriesForecast, 2>,
    pub selected_series: Option<std::borrow::Cow<'static, str>>,
}

impl Default for VaccineGroupForecast {
    fn default() -> Self {
        Self {
            vaccine_group: std::borrow::Cow::Borrowed(""),
            evaluations: TinyVec::new(),
            forecasts: TinyVec::new(),
            selected_series: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastRequest {
    pub patient: Patient,
    pub history: Vec<Dose>,
    pub execution_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResponse {
    pub vaccine_groups: TinyVec<VaccineGroupForecast, 24>,
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

