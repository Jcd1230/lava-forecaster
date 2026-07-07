#![allow(dead_code)]

use crate::rules::covid19::policy::{
    CovidAgeBand, CovidSeriesId, CovidSeriesPolicy, DoseIdentityPolicy, EvaluationPolicy,
    ForecastAnchorPolicy, ForecastPolicy, IceSourceRefs, IntervalAnchorPolicy, IntervalPolicy,
    OverflowPolicy, SeriesSelectionPolicy,
};
use crate::rules::covid19::products::CovidProductFamily;
use crate::rules::covid19::seasons::CovidSeason;
use lava_cvx_macro::cvx;

pub const AUG_2025_LT2_CVX: &[u16] = &[
    cvx!("213"),
    cvx!("309"),
    cvx!("310"),
    cvx!("311"),
    cvx!("312"),
    cvx!("313"),
    cvx!("334"),
];

pub const AUG_2025_2_TO_64_CVX: &[u16] = AUG_2025_LT2_CVX;

pub const AUG_2025_GTE65_CVX: &[u16] = &[
    cvx!("213"),
    cvx!("309"),
    cvx!("312"),
    cvx!("313"),
    cvx!("334"),
];

pub const SEP_2023_AUG_2024_PFIZER_LT5_CVX: &[u16] = &[
    cvx!("208"),
    cvx!("217"),
    cvx!("218"),
    cvx!("219"),
    cvx!("300"),
    cvx!("301"),
    cvx!("302"),
    cvx!("308"),
    cvx!("309"),
    cvx!("310"),
];

pub const SEP_2023_AUG_2024_MODERNA_LT5_CVX: &[u16] = &[cvx!("311"), cvx!("312")];

pub const SEP_2023_AUG_2024_NOVAVAX_CVX: &[u16] = &[cvx!("211"), cvx!("313")];

pub const SEP_2023_AUG_2024_GTE5_CVX: &[u16] = &[
    cvx!("208"),
    cvx!("211"),
    cvx!("212"),
    cvx!("213"),
    cvx!("217"),
    cvx!("218"),
    cvx!("219"),
    cvx!("221"),
    cvx!("228"),
    cvx!("229"),
    cvx!("300"),
    cvx!("301"),
    cvx!("302"),
    cvx!("308"),
    cvx!("309"),
    cvx!("310"),
    cvx!("311"),
    cvx!("312"),
    cvx!("313"),
    cvx!("502"),
    cvx!("519"),
];

pub const DEC_2020_PRIMARY_CVX: &[u16] = SEP_2023_AUG_2024_GTE5_CVX;

const SRC_SERIES_AUG2025_LT2: IceSourceRefs = IceSourceRefs {
    series_selection: &["SeriesSelection.drl:879-891"],
    evaluation: &[
        "Evaluation^COVID19^Aug2025Season.dslr:513-609",
        "Evaluation^COVID19^Aug2025Season.dslr:579-609",
    ],
    recommendation: &[
        "Recommendation^COVID19^Aug2025Season.dslr:255-305",
        "Recommendation^COVID19^Aug2025Season.dslr:308-505",
    ],
    yaml: &["covid_19_aug_2025_lt_2_series.yml"],
    notes: &[
        "LT2 selection depends on evaluation age or an in-season shot before age 2.",
        "LT2 forecast has distinct invalid-shot retry paths for below-min-age and prior invalid shots.",
    ],
};

const SRC_SERIES_AUG2025_2_TO_64: IceSourceRefs = IceSourceRefs {
    series_selection: &["SeriesSelection.drl:904-912", "SeriesSelection.drl:940-949"],
    evaluation: &[
        "Evaluation^COVID19^Aug2025Season.dslr:31-195",
        "Evaluation^COVID19^Aug2025Season.dslr:205-235",
    ],
    recommendation: &["Recommendation^COVID19^Aug2025Season.dslr:60-92"],
    yaml: &["covid_19_aug_2025_2_y_to_64_y_series.yml"],
    notes: &[
        "Target dose 1 can use the most recent non-ignored COVID shot plus 8 weeks.",
        "Invalid non-series shots can be forecast anchors while not necessarily being interval anchors.",
    ],
};

const SRC_SERIES_AUG2025_GTE65: IceSourceRefs = IceSourceRefs {
    series_selection: &["SeriesSelection.drl:922-930", "SeriesSelection.drl:959-986"],
    evaluation: &[
        "Evaluation^COVID19^Aug2025Season.dslr:31-235",
        "Evaluation^COVID19^Aug2025Season.dslr:205-235",
    ],
    recommendation: &[
        "Recommendation^COVID19^Aug2025Season.dslr:60-92",
        "Recommendation^COVID19^Aug2025Season.dslr:221-245",
    ],
    yaml: &["covid_19_aug_2025_gte_65_series.yml"],
    notes: &[
        "65+ can be selected when dose 1 is before age 65 but the patient turns 65 within 12 months of season start.",
    ],
};

const SRC_SEP2023_AUG2024_UNDER5: IceSourceRefs = IceSourceRefs {
    series_selection: &[
        "SeriesSelection.drl:1055-1072",
        "SeriesSelection.drl:1077-1117",
        "SeriesSelection.drl:1289-1335",
    ],
    evaluation: &[
        "Evaluation^COVID19^Sep2023Season.dslr:455-478",
        "Evaluation^COVID19^Sep2023Season.dslr:641-664",
    ],
    recommendation: &["Recommendation^COVID19^Sep2023Season.dslr"],
    yaml: &[
        "covid_19_sep_2023_pfizer_lt_5_y_series.yml",
        "covid_19_sep_2023_moderna_lt_5_y_series.yml",
        "covid_19_sep_2023_mixed_product_lt_5_y_series.yml",
    ],
    notes: &[
        "Sep2023/Aug2024 under-5 routing depends on product-family homogeneity and mixed-product rules.",
        "CVX 308 has special under-5 max-age behavior for Sep2023/Aug2024 Pfizer and mixed-product series.",
    ],
};

const SRC_SEP2023_AUG2024_GTE5: IceSourceRefs = IceSourceRefs {
    series_selection: &[
        "SeriesSelection.drl:997-1039",
        "SeriesSelection.drl:1055-1072",
    ],
    evaluation: &["Evaluation^COVID19^Sep2023Season.dslr"],
    recommendation: &["Recommendation^COVID19^Sep2023Season.dslr"],
    yaml: &["covid_19_sep_2023_gte_5_series.yml"],
    notes: &["Pre-Aug2025 completed-vs-incomplete selection remains a major parity target."],
};

const SRC_SEP2023_AUG2024_NOVAVAX: IceSourceRefs = IceSourceRefs {
    series_selection: &[
        "SeriesSelection.drl:997-1039",
        "SeriesSelection.drl:1077-1097",
    ],
    evaluation: &["Evaluation^COVID19^Sep2023Season.dslr"],
    recommendation: &["Recommendation^COVID19^Sep2023Season.dslr"],
    yaml: &["covid_19_sep_2023_novavax_series.yml"],
    notes: &["Novavax has distinct product-family routing in pre-Aug2025 seasons."],
};

pub const COVID_SERIES_POLICIES: &[CovidSeriesPolicy] = &[
    CovidSeriesPolicy {
        id: CovidSeriesId::Dec2020Primary,
        ice_name: "COVID-19 Dec 2020 primary series",
        season: CovidSeason::Dec2020,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Any,
        cvx_members: DEC_2020_PRIMARY_CVX,
        max_valid_doses: 2,
        selection: SeriesSelectionPolicy::LegacyDefault,
        dose_identity: DoseIdentityPolicy::ChronologicalWithinCollapsedPriorLt5,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(
            IntervalAnchorPolicy::LastValidOrAcceptedHistoricalDose,
        ),
        evaluation: EvaluationPolicy::LegacyCovid,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: IceSourceRefs::empty(),
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Sep2023PfizerLt5,
        ice_name: "COVID-19 Sep 2023 Pfizer <5 series",
        season: CovidSeason::Sep2023,
        product_family: CovidProductFamily::PfizerPediatric,
        age_band: CovidAgeBand::Under5AtSeasonStart,
        cvx_members: SEP_2023_AUG_2024_PFIZER_LT5_CVX,
        max_valid_doses: 3,
        selection: SeriesSelectionPolicy::ProductSpecificUnder5,
        dose_identity: DoseIdentityPolicy::ProductSeriesLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(IntervalAnchorPolicy::LastValidDoseInSelectedSeries),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Under5,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_UNDER5,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Sep2023ModernaLt5,
        ice_name: "COVID-19 Sep 2023 Moderna <5 series",
        season: CovidSeason::Sep2023,
        product_family: CovidProductFamily::ModernaPediatric,
        age_band: CovidAgeBand::Under5AtSeasonStart,
        cvx_members: SEP_2023_AUG_2024_MODERNA_LT5_CVX,
        max_valid_doses: 2,
        selection: SeriesSelectionPolicy::ProductSpecificUnder5,
        dose_identity: DoseIdentityPolicy::ProductSeriesLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(IntervalAnchorPolicy::LastValidDoseInSelectedSeries),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Under5,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_UNDER5,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Sep2023MixedLt5,
        ice_name: "COVID-19 Sep 2023 mixed product <5 series",
        season: CovidSeason::Sep2023,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Under5AtSeasonStart,
        cvx_members: SEP_2023_AUG_2024_GTE5_CVX,
        max_valid_doses: 3,
        selection: SeriesSelectionPolicy::MixedProductUnder5,
        dose_identity: DoseIdentityPolicy::ProductSeriesLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(IntervalAnchorPolicy::LastValidDoseInSelectedSeries),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Under5,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_UNDER5,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Sep2023Gte5,
        ice_name: "COVID-19 Sep 2023 >=5 series",
        season: CovidSeason::Sep2023,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Any,
        cvx_members: SEP_2023_AUG_2024_GTE5_CVX,
        max_valid_doses: 1,
        selection: SeriesSelectionPolicy::Gte5,
        dose_identity: DoseIdentityPolicy::SeasonLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(
            IntervalAnchorPolicy::LastValidOrAcceptedHistoricalDose,
        ),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Gte5,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_GTE5,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Sep2023Novavax,
        ice_name: "COVID-19 Sep 2023 Novavax series",
        season: CovidSeason::Sep2023,
        product_family: CovidProductFamily::Novavax,
        age_band: CovidAgeBand::Any,
        cvx_members: SEP_2023_AUG_2024_NOVAVAX_CVX,
        max_valid_doses: 2,
        selection: SeriesSelectionPolicy::Novavax,
        dose_identity: DoseIdentityPolicy::ProductSeriesLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(IntervalAnchorPolicy::LastValidDoseInSelectedSeries),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Novavax,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_NOVAVAX,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2024PfizerLt5,
        ice_name: "COVID-19 Aug 2024 Pfizer <5 series",
        season: CovidSeason::Aug2024,
        product_family: CovidProductFamily::PfizerPediatric,
        age_band: CovidAgeBand::Under5AtSeasonStart,
        cvx_members: SEP_2023_AUG_2024_PFIZER_LT5_CVX,
        max_valid_doses: 3,
        selection: SeriesSelectionPolicy::ProductSpecificUnder5,
        dose_identity: DoseIdentityPolicy::SeasonLocal,
        overflow: OverflowPolicy::PreserveValidForSeasonDoseOne,
        intervals: IntervalPolicy::unspecified(IntervalAnchorPolicy::LastValidDoseInSelectedSeries),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Under5,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_UNDER5,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2024ModernaLt5,
        ice_name: "COVID-19 Aug 2024 Moderna <5 series",
        season: CovidSeason::Aug2024,
        product_family: CovidProductFamily::ModernaPediatric,
        age_band: CovidAgeBand::Under5AtSeasonStart,
        cvx_members: SEP_2023_AUG_2024_MODERNA_LT5_CVX,
        max_valid_doses: 2,
        selection: SeriesSelectionPolicy::ProductSpecificUnder5,
        dose_identity: DoseIdentityPolicy::ProductSeriesLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(IntervalAnchorPolicy::LastValidDoseInSelectedSeries),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Under5,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_UNDER5,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2024MixedLt5,
        ice_name: "COVID-19 Aug 2024 mixed product <5 series",
        season: CovidSeason::Aug2024,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Under5AtSeasonStart,
        cvx_members: SEP_2023_AUG_2024_GTE5_CVX,
        max_valid_doses: 3,
        selection: SeriesSelectionPolicy::MixedProductUnder5,
        dose_identity: DoseIdentityPolicy::ProductSeriesLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(IntervalAnchorPolicy::LastValidDoseInSelectedSeries),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Under5,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_UNDER5,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2024Gte5,
        ice_name: "COVID-19 Aug 2024 >=5 series",
        season: CovidSeason::Aug2024,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Any,
        cvx_members: SEP_2023_AUG_2024_GTE5_CVX,
        max_valid_doses: 1,
        selection: SeriesSelectionPolicy::Gte5,
        dose_identity: DoseIdentityPolicy::SeasonLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(
            IntervalAnchorPolicy::LastValidOrAcceptedHistoricalDose,
        ),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Gte5,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_GTE5,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2024Novavax,
        ice_name: "COVID-19 Aug 2024 Novavax series",
        season: CovidSeason::Aug2024,
        product_family: CovidProductFamily::Novavax,
        age_band: CovidAgeBand::Any,
        cvx_members: SEP_2023_AUG_2024_NOVAVAX_CVX,
        max_valid_doses: 2,
        selection: SeriesSelectionPolicy::Novavax,
        dose_identity: DoseIdentityPolicy::ProductSeriesLocal,
        overflow: OverflowPolicy::AcceptedKeepsDoseNumber,
        intervals: IntervalPolicy::unspecified(IntervalAnchorPolicy::LastValidDoseInSelectedSeries),
        evaluation: EvaluationPolicy::Sep2023OrAug2024Novavax,
        forecast: ForecastPolicy::legacy(ForecastAnchorPolicy::SeasonStart),
        sources: SRC_SEP2023_AUG2024_NOVAVAX,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2025Lt2,
        ice_name: "COVID-19 Aug 2025 <2 series",
        season: CovidSeason::Aug2025,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Under2AtEvaluation,
        cvx_members: AUG_2025_LT2_CVX,
        max_valid_doses: 2,
        selection: SeriesSelectionPolicy::Aug2025Lt2ByEvalAgeOrDoseBefore2,
        dose_identity: DoseIdentityPolicy::CurrentSeasonLocal,
        overflow: OverflowPolicy::AcceptedAsExtraDose,
        intervals: IntervalPolicy::aug2025_lt2(),
        evaluation: EvaluationPolicy::Aug2025Lt2,
        forecast: ForecastPolicy::aug2025_lt2(),
        sources: SRC_SERIES_AUG2025_LT2,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2025Age2To64,
        ice_name: "COVID-19 Aug 2025 2y-64y series",
        season: CovidSeason::Aug2025,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Age2To64,
        cvx_members: AUG_2025_2_TO_64_CVX,
        max_valid_doses: 1,
        selection: SeriesSelectionPolicy::Aug2025Age2To64,
        dose_identity: DoseIdentityPolicy::CurrentSeasonLocal,
        overflow: OverflowPolicy::AcceptedAsExtraDose,
        intervals: IntervalPolicy::aug2025_adult(),
        evaluation: EvaluationPolicy::Aug2025Adult,
        forecast: ForecastPolicy::aug2025_adult(),
        sources: SRC_SERIES_AUG2025_2_TO_64,
    },
    CovidSeriesPolicy {
        id: CovidSeriesId::Aug2025Age65Plus,
        ice_name: "COVID-19 Aug 2025 >=65 series",
        season: CovidSeason::Aug2025,
        product_family: CovidProductFamily::OtherSupported,
        age_band: CovidAgeBand::Age65Plus,
        cvx_members: AUG_2025_GTE65_CVX,
        max_valid_doses: 2,
        selection: SeriesSelectionPolicy::Aug2025Age65Plus,
        dose_identity: DoseIdentityPolicy::CurrentSeasonLocal,
        overflow: OverflowPolicy::AcceptedAsExtraDose,
        intervals: IntervalPolicy::aug2025_adult(),
        evaluation: EvaluationPolicy::Aug2025Age65Plus,
        forecast: ForecastPolicy::aug2025_adult(),
        sources: SRC_SERIES_AUG2025_GTE65,
    },
];

pub fn policy_by_id(id: CovidSeriesId) -> Option<&'static CovidSeriesPolicy> {
    COVID_SERIES_POLICIES.iter().find(|policy| policy.id == id)
}

pub fn policies_for_season(
    season: CovidSeason,
) -> impl Iterator<Item = &'static CovidSeriesPolicy> {
    COVID_SERIES_POLICIES
        .iter()
        .filter(move |policy| policy.season == season)
}
