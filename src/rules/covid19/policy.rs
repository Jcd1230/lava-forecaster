#![allow(dead_code)]

use crate::rules::covid19::products::CovidProductFamily;
use crate::rules::covid19::seasons::CovidSeason;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CovidAgeBand {
    Any,
    Under2AtEvaluation,
    Under5AtSeasonStart,
    Age2To64,
    Age65Plus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CovidSeriesId {
    Dec2020Primary,
    Sep2023PfizerLt5,
    Sep2023ModernaLt5,
    Sep2023MixedLt5,
    Sep2023Gte5,
    Sep2023Novavax,
    Aug2024PfizerLt5,
    Aug2024ModernaLt5,
    Aug2024MixedLt5,
    Aug2024Gte5,
    Aug2024Novavax,
    Aug2025Lt2,
    Aug2025Age2To64,
    Aug2025Age65Plus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CvxRelationship {
    MemberOfSelectedSeries,
    CovidButNotThisSeries,
    SupportedButOldProduct,
    UnsupportedIgnored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeriesSelectionPolicy {
    LegacyDefault,
    ProductSpecificUnder5,
    MixedProductUnder5,
    Gte5,
    Novavax,
    Aug2025Lt2ByEvalAgeOrDoseBefore2,
    Aug2025Age2To64,
    Aug2025Age65Plus,
    Aug2025Age65PlusWithinTwelveMonths,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoseIdentityPolicy {
    ChronologicalWithinCollapsedPriorLt5,
    SeasonLocal,
    ProductSeriesLocal,
    CurrentSeasonLocal,
    ResetOnNewSeason,
    PreserveTargetDoseFromIce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverflowPolicy {
    AcceptedKeepsDoseNumber,
    AcceptedAsExtraDose,
    PreserveValidForSeasonDoseOne,
    InvalidWhenBeyondSeries,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalAnchorPolicy {
    LastValidDoseInSelectedSeries,
    LastNotIgnoredCovidDose,
    LastValidOrAcceptedHistoricalDose,
    IgnoreInvalidNonSeries,
    IncludeInvalidNonSeriesForLt2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForecastAnchorPolicy {
    SeasonStart,
    LastCurrentSeasonDosePlus56,
    LastInvalidOldProductPlus56,
    LastInvalidLt2Plus28,
    PriorShotWindowClampedToInvalidAttempt,
    NoForecastAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalPolicy {
    pub anchor: IntervalAnchorPolicy,
    pub same_product_days: Option<i64>,
    pub mixed_product_days: Option<i64>,
    pub adult_days: Option<i64>,
}

impl IntervalPolicy {
    pub const fn unspecified(anchor: IntervalAnchorPolicy) -> Self {
        Self {
            anchor,
            same_product_days: None,
            mixed_product_days: None,
            adult_days: None,
        }
    }

    pub const fn aug2025_adult() -> Self {
        Self {
            anchor: IntervalAnchorPolicy::IgnoreInvalidNonSeries,
            same_product_days: None,
            mixed_product_days: None,
            adult_days: Some(52),
        }
    }

    pub const fn aug2025_lt2() -> Self {
        Self {
            anchor: IntervalAnchorPolicy::IncludeInvalidNonSeriesForLt2,
            same_product_days: Some(17),
            mixed_product_days: Some(24),
            adult_days: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvaluationPolicy {
    LegacyCovid,
    Sep2023OrAug2024Under5,
    Sep2023OrAug2024Gte5,
    Sep2023OrAug2024Novavax,
    Aug2025Lt2,
    Aug2025Adult,
    Aug2025Age65Plus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ForecastPolicy {
    pub anchor: ForecastAnchorPolicy,
    pub invalid_old_product_retry_days: Option<i64>,
    pub invalid_lt2_retry_days: Option<i64>,
}

impl ForecastPolicy {
    pub const fn legacy(anchor: ForecastAnchorPolicy) -> Self {
        Self {
            anchor,
            invalid_old_product_retry_days: None,
            invalid_lt2_retry_days: None,
        }
    }

    pub const fn aug2025_adult() -> Self {
        Self {
            anchor: ForecastAnchorPolicy::LastInvalidOldProductPlus56,
            invalid_old_product_retry_days: Some(56),
            invalid_lt2_retry_days: None,
        }
    }

    pub const fn aug2025_lt2() -> Self {
        Self {
            anchor: ForecastAnchorPolicy::LastInvalidLt2Plus28,
            invalid_old_product_retry_days: None,
            invalid_lt2_retry_days: Some(28),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IceSourceRefs {
    pub series_selection: &'static [&'static str],
    pub evaluation: &'static [&'static str],
    pub recommendation: &'static [&'static str],
    pub yaml: &'static [&'static str],
    pub notes: &'static [&'static str],
}

impl IceSourceRefs {
    pub const fn empty() -> Self {
        Self {
            series_selection: &[],
            evaluation: &[],
            recommendation: &[],
            yaml: &[],
            notes: &[],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CovidSeriesPolicy {
    pub id: CovidSeriesId,
    pub ice_name: &'static str,
    pub season: CovidSeason,
    pub product_family: CovidProductFamily,
    pub age_band: CovidAgeBand,
    pub cvx_members: &'static [u16],
    pub max_valid_doses: usize,
    pub selection: SeriesSelectionPolicy,
    pub dose_identity: DoseIdentityPolicy,
    pub overflow: OverflowPolicy,
    pub intervals: IntervalPolicy,
    pub evaluation: EvaluationPolicy,
    pub forecast: ForecastPolicy,
    pub sources: IceSourceRefs,
}

impl CovidSeriesPolicy {
    pub fn contains_cvx(self, cvx_code: crate::models::Cvx) -> bool {
        self.cvx_members.contains(&cvx_code.0)
    }
}
