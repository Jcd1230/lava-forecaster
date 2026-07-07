#![allow(dead_code)]

use chrono::NaiveDate;

/// COVID season boundaries used by Java ICE and mirrored by LAVA's COVID parity logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CovidSeason {
    Dec2020,
    Sep2023,
    Aug2024,
    Aug2025,
}

impl CovidSeason {
    pub fn start_date(self) -> NaiveDate {
        match self {
            Self::Dec2020 => NaiveDate::from_ymd_opt(2020, 12, 14).unwrap(),
            Self::Sep2023 => NaiveDate::from_ymd_opt(2023, 9, 12).unwrap(),
            Self::Aug2024 => NaiveDate::from_ymd_opt(2024, 8, 22).unwrap(),
            Self::Aug2025 => NaiveDate::from_ymd_opt(2025, 8, 27).unwrap(),
        }
    }

    pub fn ice_key(self) -> &'static str {
        match self {
            Self::Dec2020 => "COVID_19_DEC_2020_SEASON",
            Self::Sep2023 => "COVID_19_SEP_2023_SEASON",
            Self::Aug2024 => "COVID_19_AUG_2024_SEASON",
            Self::Aug2025 => "COVID_19_AUG_2025_SEASON",
        }
    }

    pub fn from_ice_key(key: &str) -> Option<Self> {
        match key {
            "COVID_19_DEC_2020_SEASON" => Some(Self::Dec2020),
            "COVID_19_SEP_2023_SEASON" => Some(Self::Sep2023),
            "COVID_19_AUG_2024_SEASON" => Some(Self::Aug2024),
            "COVID_19_AUG_2025_SEASON" => Some(Self::Aug2025),
            _ => None,
        }
    }

    pub fn for_date(date: NaiveDate) -> Self {
        if date < Self::Sep2023.start_date() {
            Self::Dec2020
        } else if date < Self::Aug2024.start_date() {
            Self::Sep2023
        } else if date < Self::Aug2025.start_date() {
            Self::Aug2024
        } else {
            Self::Aug2025
        }
    }
}
