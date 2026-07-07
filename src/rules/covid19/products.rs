#![allow(dead_code)]

use crate::models::Cvx;
use lava_cvx_macro::cvx;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CovidProductFamily {
    PfizerPediatric,
    PfizerAdult,
    ModernaPediatric,
    ModernaAdult,
    Novavax,
    Janssen,
    OldMonovalent,
    OldBivalent,
    Unspecified,
    OtherSupported,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CovidFormulationEra {
    Original,
    Bivalent,
    Seasonal2023,
    Seasonal2024,
    Seasonal2025,
    Other,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CovidProductInfo {
    pub family: CovidProductFamily,
    pub era: CovidFormulationEra,
    pub supported_by_java_covid: bool,
}

impl CovidProductInfo {
    pub const fn unsupported() -> Self {
        Self {
            family: CovidProductFamily::Unsupported,
            era: CovidFormulationEra::Unsupported,
            supported_by_java_covid: false,
        }
    }

    pub const fn supported(family: CovidProductFamily, era: CovidFormulationEra) -> Self {
        Self {
            family,
            era,
            supported_by_java_covid: true,
        }
    }
}

pub fn covid_product_info(cvx_code: Cvx) -> CovidProductInfo {
    match cvx_code.0 {
        cvx!("207") | cvx!("208") | cvx!("217") | cvx!("218") | cvx!("219")
        | cvx!("300") | cvx!("301") | cvx!("302") => CovidProductInfo::supported(
            CovidProductFamily::PfizerPediatric,
            CovidFormulationEra::Original,
        ),
        cvx!("229") => CovidProductInfo::supported(
            CovidProductFamily::PfizerAdult,
            CovidFormulationEra::Original,
        ),
        cvx!("308") | cvx!("309") | cvx!("310") => CovidProductInfo::supported(
            CovidProductFamily::PfizerPediatric,
            CovidFormulationEra::Seasonal2024,
        ),
        cvx!("311") | cvx!("312") => CovidProductInfo::supported(
            CovidProductFamily::ModernaPediatric,
            CovidFormulationEra::Seasonal2024,
        ),
        cvx!("313") => CovidProductInfo::supported(
            CovidProductFamily::Novavax,
            CovidFormulationEra::Seasonal2024,
        ),
        cvx!("334") => CovidProductInfo::supported(
            CovidProductFamily::ModernaAdult,
            CovidFormulationEra::Seasonal2025,
        ),
        cvx!("211") => CovidProductInfo::supported(
            CovidProductFamily::Novavax,
            CovidFormulationEra::Original,
        ),
        cvx!("212") => CovidProductInfo::supported(
            CovidProductFamily::Janssen,
            CovidFormulationEra::Original,
        ),
        cvx!("213") => CovidProductInfo::supported(
            CovidProductFamily::Unspecified,
            CovidFormulationEra::Other,
        ),
        cvx!("221") | cvx!("228") | cvx!("502") | cvx!("519") => {
            CovidProductInfo::supported(CovidProductFamily::OtherSupported, CovidFormulationEra::Other)
        }
        _ => CovidProductInfo::unsupported(),
    }
}

pub fn is_supported_by_java_covid(cvx_code: Cvx) -> bool {
    covid_product_info(cvx_code).supported_by_java_covid
}

pub fn is_aug2025_current_formulation(cvx_code: Cvx) -> bool {
    matches!(
        cvx_code.0,
        cvx!("213")
            | cvx!("308")
            | cvx!("309")
            | cvx!("310")
            | cvx!("311")
            | cvx!("312")
            | cvx!("313")
            | cvx!("334")
    )
}

pub fn is_legacy_pediatric_product(cvx_code: Cvx) -> bool {
    matches!(
        cvx_code.0,
        cvx!("218") | cvx!("219") | cvx!("228") | cvx!("301") | cvx!("302")
    )
}
