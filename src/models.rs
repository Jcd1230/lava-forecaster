use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Gender {
    Female,
    Male,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub birth_date: NaiveDate,
    pub gender: Gender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dose {
    pub date: NaiveDate,
    pub cvx: String,
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
    pub cvx: String,
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
    pub series_name: String,
    pub earliest_date: Option<NaiveDate>,
    pub recommended_date: Option<NaiveDate>,
    pub overdue_date: Option<NaiveDate>,
    pub latest_date: Option<NaiveDate>,
    pub status: SeriesStatus,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaccineGroupForecast {
    pub vaccine_group: String,
    pub evaluations: Vec<DoseEvaluation>,
    pub forecasts: Vec<SeriesForecast>,
    pub selected_series: Option<String>,
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

