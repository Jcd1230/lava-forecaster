use chrono::NaiveDate;
use lava_forecaster::models::{
    Cvx, Dose, DoseEvaluation, DoseStatus, EvaluationReason, ExpectedResults, Gender, Patient,
    SeriesForecast, SeriesStatus, UnifiedTestCase,
};

pub fn generate_xml_payload(patient: &Patient, history: &[Dose]) -> String {
    let mut sae_templates = Vec::new();
    for (idx, dose) in history.iter().enumerate() {
        let dt_str = dose.date.format("%Y%m%d").to_string();
        let is_valid_xml = match dose.is_valid {
            Some(true) => "\n                        <isValid value=\"true\"/>",
            Some(false) => "\n                        <isValid value=\"false\"/>",
            None => "",
        };
        sae_templates.push(format!(
            r#"                    <substanceAdministrationEvent>
                        <templateId root="2.16.840.1.113883.3.795.11.9.1.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.10" extension="{}"/>
                        <substanceAdministrationGeneralPurpose code="384810002" codeSystem="2.16.840.1.113883.6.5"/>
                        <substance>
                            <id root="ab0c489e-782a-4c34-9e4e-9094cc2952d7"/>
                            <substanceCode code="{}" displayName="Vaccine" codeSystem="2.16.840.1.113883.12.292"/>
                        </substance>
                        <administrationTimeInterval high="{}" low="{}" />{}
                    </substanceAdministrationEvent>"#,
            1000 + idx, dose.cvx, dt_str, dt_str, is_valid_xml
        ));
    }

    let sae_str = sae_templates.join("\n");

    // Generate observations (immunities & contraindications)
    let mut obs_templates = Vec::new();
    let mut obs_idx = 0;

    for immunity in &patient.immunities {
        let (focus_code, focus_system) = match immunity.disease.to_lowercase().as_str() {
            "hepb" | "hep_b" | "hep b" | "hepatitis b" => {
                ("070.30".to_string(), "2.16.840.1.113883.6.103".to_string())
            }
            "varicella" | "chickenpox" => {
                ("052.9".to_string(), "2.16.840.1.113883.6.103".to_string())
            }
            "measles" | "rubeola" => ("055.9".to_string(), "2.16.840.1.113883.6.103".to_string()),
            "mumps" => ("072.9".to_string(), "2.16.840.1.113883.6.103".to_string()),
            "rubella" | "german measles" => {
                ("056.9".to_string(), "2.16.840.1.113883.6.103".to_string())
            }
            "hepa" | "hep_a" | "hep a" | "hepatitis a" => {
                ("070.1".to_string(), "2.16.840.1.113883.6.103".to_string())
            }
            _ => (
                immunity.disease.clone(),
                "2.16.840.1.113883.3.795.12.1.1".to_string(),
            ),
        };
        let reason_code = match immunity.reason.to_lowercase().as_str() {
            "disease documented" | "disease_documented" => "DISEASE_DOCUMENTED",
            _ => "PROOF_OF_IMMUNITY",
        };
        let dt_str = immunity.date.format("%Y%m%d").to_string();
        obs_templates.push(format!(
            r#"                    <observationResult>
                        <templateId root="2.16.840.1.113883.3.795.11.6.3.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.12" extension="{}"/>
                        <observationFocus code="{}" codeSystem="{}"/>
                        <observationEventTime low="{}" high="{}"/>
                        <observationValue>
                            <concept code="{}" codeSystem="2.16.840.1.113883.3.795.12.100.8"/>
                        </observationValue>
                        <interpretation code="IS_IMMUNE" codeSystem="2.16.840.1.113883.3.795.12.100.9"/>
                    </observationResult>"#,
            2000 + obs_idx, focus_code, focus_system, dt_str, dt_str, reason_code
        ));
        obs_idx += 1;
    }

    for contra in &patient.contraindications {
        let (focus_code, focus_system) = match contra.target.to_lowercase().as_str() {
            "dtp" | "dtap" | "pertussis" => {
                ("70654002".to_string(), "2.16.840.1.113883.6.96".to_string())
            }
            _ => {
                if contra.target.chars().all(|c| c.is_ascii_digit()) {
                    (contra.target.clone(), "2.16.840.1.113883.6.96".to_string())
                } else {
                    (
                        contra.target.clone(),
                        "2.16.840.1.113883.3.795.12.1.1".to_string(),
                    )
                }
            }
        };
        let reason_code = if contra.reason.is_empty() {
            "CONTRAINDICATION"
        } else {
            &contra.reason
        };
        let dt_str = contra.date.format("%Y%m%d").to_string();
        obs_templates.push(format!(
            r#"                    <observationResult>
                        <templateId root="2.16.840.1.113883.3.795.11.6.3.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.12" extension="{}"/>
                        <observationFocus code="{}" codeSystem="{}"/>
                        <observationEventTime low="{}" high="{}"/>
                        <observationValue>
                            <concept code="{}" codeSystem="2.16.840.1.113883.3.795.12.100.8"/>
                        </observationValue>
                        <interpretation code="CONTRAINDICATION" codeSystem="2.16.840.1.113883.3.795.12.100.9"/>
                    </observationResult>"#,
            2000 + obs_idx, focus_code, focus_system, dt_str, dt_str, reason_code
        ));
        obs_idx += 1;
    }

    let obs_str = if obs_templates.is_empty() {
        "".to_string()
    } else {
        format!(
            "                <observationResults>\n{}\n                </observationResults>\n",
            obs_templates.join("\n")
        )
    };

    let dob_str = patient.birth_date.format("%Y%m%d").to_string();
    let gender_code = match patient.gender {
        Gender::Female => "F",
        Gender::Male => "M",
        Gender::Unknown => "U",
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns3:cdsInput xmlns:ns2="org.opencds.vmr.v1_0.schema.cdsinput.specification"
        xmlns:ns3="org.opencds.vmr.v1_0.schema.cdsinput"
        xmlns:ns4="org.opencds.vmr.v1_0.schema.cdsoutput"
        xmlns:ns5="org.opencds.vmr.v1_0.schema.vmr">
    <templateId root="2.16.840.1.113883.3.795.11.1.1"/>
    <cdsContext>
        <cdsSystemUserPreferredLanguage code="en" codeSystem="2.16.840.1.113883.6.99" displayName="English"/>
    </cdsContext>
    <vmrInput>
        <templateId root="2.16.840.1.113883.3.795.11.1.1"/>
        <patient>
            <templateId root="2.16.840.1.113883.3.795.11.2.1.1"/>
            <id root="2.16.840.1.113883.3.795.12.100.11" extension="43299551" />
            <demographics>
                <birthTime value="{}"/>
                <gender code="{}" codeSystem="2.16.840.1.113883.5.1"/>
            </demographics>
            <clinicalStatements>
{}                <substanceAdministrationEvents>
{}                </substanceAdministrationEvents>
            </clinicalStatements>
        </patient>
    </vmrInput>
</ns3:cdsInput>"#,
        dob_str, gender_code, obs_str, sae_str
    )
}

pub fn build_evaluate_payload(
    patient: &Patient,
    history: &[Dose],
    eval_date: NaiveDate,
) -> serde_json::Value {
    let xml_content = generate_xml_payload(patient, history);
    let b64_xml = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, xml_content.as_bytes());

    let eval_datetime = eval_date.and_hms_opt(23, 59, 59).unwrap();
    let ts_ms = eval_datetime.and_utc().timestamp_millis();

    serde_json::json!({
        "interactionId": {
            "scopingEntityId": "org.nyc.cir",
            "interactionId": "123456",
            "submissionTime": ts_ms
        },
        "specifiedTime": ts_ms,
        "evaluationRequest": {
            "clientLanguage": "en",
            "clientTimeZoneOffset": "+0000",
            "kmEvaluationRequest": [{
                "kmId": {
                    "scopingEntityId": "org.nyc.cir",
                    "businessId": "ICE",
                    "version": "1.0.0"
                }
            }],
            "dataRequirementItemData": [{
                "driId": {
                    "containingEntityId": {
                        "scopingEntityId": "org.nyc.cir",
                        "businessId": "ICEData",
                        "version": "1.0.0"
                    },
                    "itemId": "cdsPayload"
                },
                "data": {
                    "informationModelSSId": {
                        "scopingEntityId": "org.opencds.vmr",
                        "businessId": "VMR",
                        "version": "1.0"
                    },
                    "base64EncodedPayload": [b64_xml]
                }
            }]
        }
    })
}

pub fn parse_date_only(date_str: &str) -> Option<NaiveDate> {
    if date_str.len() < 8 {
        return None;
    }
    let clean: String = date_str.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean.len() >= 8 {
        NaiveDate::parse_from_str(&clean[0..8], "%Y%m%d").ok()
    } else {
        None
    }
}

pub fn map_legacy_status(legacy_status: &str, reasons: &[String]) -> SeriesStatus {
    if reasons
        .iter()
        .any(|r| r == "PROOF_OF_IMMUNITY" || r == "DISEASE_DOCUMENTED")
    {
        return SeriesStatus::Complete;
    }
    if legacy_status == "COMPLETE" || reasons.iter().any(|r| r.contains("COMPLETE")) {
        return SeriesStatus::Complete;
    }
    if legacy_status == "CONDITIONAL" {
        return SeriesStatus::ConditionallyRecommended;
    }
    if legacy_status == "NOT_RECOMMENDED" {
        return SeriesStatus::NotRecommended;
    }
    if legacy_status == "RECOMMENDED" || legacy_status == "FUTURE_RECOMMENDED" {
        return SeriesStatus::default();
    }
    SeriesStatus::default()
}

pub fn is_immune(patient: &Patient, group: &str, eval_date: NaiveDate) -> bool {
    let group_lower = group.to_lowercase();
    patient.immunities.iter().any(|imm| {
        let imm_disease_lower = imm.disease.to_lowercase();
        let matches_group = if group_lower.contains("hepb")
            || group_lower.contains("hep_b")
            || group_lower.contains("hep b")
            || group_lower.contains("hepatitis b")
        {
            imm_disease_lower.contains("hepb")
                || imm_disease_lower.contains("hep_b")
                || imm_disease_lower.contains("hep b")
                || imm_disease_lower.contains("hepatitis b")
        } else if group_lower.contains("varicella") {
            imm_disease_lower.contains("varicella") || imm_disease_lower.contains("chickenpox")
        } else if group_lower.contains("measles") {
            imm_disease_lower.contains("measles") || imm_disease_lower.contains("rubeola")
        } else if group_lower.contains("mumps") {
            imm_disease_lower.contains("mumps")
        } else if group_lower.contains("rubella") {
            imm_disease_lower.contains("rubella") || imm_disease_lower.contains("german measles")
        } else if group_lower.contains("mmr") {
            imm_disease_lower.contains("mmr")
                || imm_disease_lower.contains("measles")
                || imm_disease_lower.contains("mumps")
                || imm_disease_lower.contains("rubella")
        } else {
            imm_disease_lower == group_lower
        };
        matches_group && eval_date >= imm.date
    })
}

pub fn is_contraindicated(patient: &Patient, group: &str, eval_date: NaiveDate) -> bool {
    let group_lower = group.to_lowercase();
    patient.contraindications.iter().any(|c| {
        if c.cvx.is_some() {
            return false;
        }
        let target_lower = c.target.to_lowercase();
        let matches_group = if group_lower.contains("dtp")
            || group_lower.contains("dtap")
            || group_lower.contains("dt")
            || group_lower.contains("tdap")
            || group_lower.contains("td")
            || group_lower.contains("diphtheria")
            || group_lower.contains("tetanus")
            || group_lower.contains("pertussis")
        {
            target_lower.contains("dtp")
                || target_lower.contains("dtap")
                || target_lower.contains("dt")
                || target_lower.contains("tdap")
                || target_lower.contains("td")
                || target_lower.contains("diphtheria")
                || target_lower.contains("tetanus")
                || target_lower.contains("pertussis")
        } else {
            target_lower == group_lower
        };
        let active = eval_date >= c.date && c.valid_until.map_or(true, |until| eval_date < until);
        matches_group && active
    })
}

pub fn map_legacy_dose_status(status: &str) -> DoseStatus {
    match status {
        "VALID" => DoseStatus::Valid,
        "INVALID" => DoseStatus::Invalid,
        "ACCEPTED" => DoseStatus::Accepted,
        _ => DoseStatus::Invalid,
    }
}

pub fn process_element(
    name: &[u8],
    attributes: quick_xml::events::attributes::Attributes,
    tag_stack: &[String],
    focus_code: &str,
    outer_date: &mut Option<NaiveDate>,
    outer_cvx: &mut Option<Cvx>,
    inner_focus_matched: &mut bool,
    inner_status: &mut Option<String>,
    inner_dose_number: &mut Option<usize>,
    inner_reasons: &mut Vec<String>,
    prop_focus_matched: &mut bool,
    prop_earliest: &mut Option<NaiveDate>,
    prop_recommended: &mut Option<NaiveDate>,
    prop_overdue: &mut Option<NaiveDate>,
    prop_status: &mut Option<String>,
    prop_reasons: &mut Vec<String>,
) {
    let event_depth = tag_stack
        .iter()
        .filter(|&t| t == "substanceAdministrationEvent")
        .count();
    let in_proposal = tag_stack
        .iter()
        .any(|t| t == "substanceAdministrationProposal");

    match name {
        b"substanceAdministrationEvent" => {
            if event_depth == 0 {
                *outer_date = None;
                *outer_cvx = None;
            } else if event_depth == 1 {
                *inner_focus_matched = false;
                *inner_status = None;
                *inner_dose_number = None;
                inner_reasons.clear();
            }
        }
        b"substanceAdministrationProposal" => {
            *prop_focus_matched = false;
            *prop_earliest = None;
            *prop_recommended = None;
            *prop_overdue = None;
            *prop_status = None;
            prop_reasons.clear();
        }
        b"administrationTimeInterval" => {
            if event_depth == 1 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        let key = a.key.as_ref();
                        if key == b"low" || key == b"high" {
                            let val = String::from_utf8_lossy(a.value.as_ref());
                            if let Some(d) = parse_date_only(&val) {
                                *outer_date = Some(d);
                            }
                        }
                    }
                }
            }
        }
        b"substanceCode" => {
            if event_depth == 1 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            let raw_cvx = String::from_utf8_lossy(a.value.as_ref());
                            if let Ok(n) = raw_cvx.parse::<u16>() {
                                *outer_cvx = Some(Cvx(n));
                            }
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            let code = String::from_utf8_lossy(a.value.as_ref());
                            if code == focus_code {
                                *prop_focus_matched = true;
                            }
                        }
                    }
                }
            }
        }
        b"observationFocus" => {
            if event_depth == 2 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            let code = String::from_utf8_lossy(a.value.as_ref());
                            if code == focus_code {
                                *inner_focus_matched = true;
                            }
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            let code = String::from_utf8_lossy(a.value.as_ref());
                            if code == focus_code {
                                *prop_focus_matched = true;
                            }
                        }
                    }
                }
            }
        }
        b"concept" => {
            if event_depth == 2 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            *inner_status =
                                Some(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            *prop_status =
                                Some(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            }
        }
        b"interpretation" => {
            if event_depth == 2 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            inner_reasons
                                .push(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            prop_reasons
                                .push(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            }
        }
        b"doseNumber" => {
            if event_depth == 2 {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"value" {
                            let val_str = String::from_utf8_lossy(a.value.as_ref());
                            if let Ok(val) = val_str.parse::<usize>() {
                                *inner_dose_number = Some(val);
                            }
                        }
                    }
                }
            }
        }
        b"validAdministrationTimeInterval" => {
            if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"low" {
                            let val = String::from_utf8_lossy(a.value.as_ref());
                            *prop_earliest = parse_date_only(&val);
                        }
                    }
                }
            }
        }
        b"proposedAdministrationTimeInterval" => {
            if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        let key = a.key.as_ref();
                        let val = String::from_utf8_lossy(a.value.as_ref());
                        if key == b"low" {
                            *prop_recommended = parse_date_only(&val);
                        } else if key == b"high" {
                            *prop_overdue = parse_date_only(&val);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn parse_legacy_xml(xml_content: &str, focus_code: &str) -> ExpectedResults {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut evaluations = Vec::new();
    let mut forecasts = Vec::new();

    let mut tag_stack: Vec<String> = Vec::new();

    let mut outer_date = None;
    let mut outer_cvx = None;

    let mut inner_focus_matched = false;
    let mut inner_status = None;
    let mut inner_dose_number = None;
    let mut inner_reasons = Vec::new();

    let mut prop_focus_matched = false;
    let mut prop_earliest = None;
    let mut prop_recommended = None;
    let mut prop_overdue = None;
    let mut prop_status = None;
    let mut prop_reasons = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!(
                "Error parsing XML at position {}: {:?}",
                reader.buffer_position(),
                e
            ),
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) => {
                let name = e.local_name();
                let name_bytes = name.as_ref();
                let name_str = String::from_utf8_lossy(name_bytes).into_owned();

                process_element(
                    name_bytes,
                    e.attributes(),
                    &tag_stack,
                    focus_code,
                    &mut outer_date,
                    &mut outer_cvx,
                    &mut inner_focus_matched,
                    &mut inner_status,
                    &mut inner_dose_number,
                    &mut inner_reasons,
                    &mut prop_focus_matched,
                    &mut prop_earliest,
                    &mut prop_recommended,
                    &mut prop_overdue,
                    &mut prop_status,
                    &mut prop_reasons,
                );

                tag_stack.push(name_str);
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.local_name();
                let name_bytes = name.as_ref();

                process_element(
                    name_bytes,
                    e.attributes(),
                    &tag_stack,
                    focus_code,
                    &mut outer_date,
                    &mut outer_cvx,
                    &mut inner_focus_matched,
                    &mut inner_status,
                    &mut inner_dose_number,
                    &mut inner_reasons,
                    &mut prop_focus_matched,
                    &mut prop_earliest,
                    &mut prop_recommended,
                    &mut prop_overdue,
                    &mut prop_status,
                    &mut prop_reasons,
                );
            }
            Ok(Event::End(ref e)) => {
                let name = e.local_name();
                let name_bytes = name.as_ref();
                let name_str = String::from_utf8_lossy(name_bytes);

                if name_bytes == b"substanceAdministrationEvent" {
                    let event_depth = tag_stack
                        .iter()
                        .filter(|&t| t == "substanceAdministrationEvent")
                        .count();
                    if event_depth == 2 {
                        if inner_focus_matched {
                            if let (Some(dt), Some(cvx), Some(ref st)) =
                                (outer_date, outer_cvx.as_ref(), inner_status.as_ref())
                            {
                                let mapped_reasons: Vec<EvaluationReason> = inner_reasons
                                    .iter()
                                    .filter_map(|r| match r.as_str() {
                                        "BelowMinimumAge" => {
                                            Some(EvaluationReason::BelowMinimumAge)
                                        }
                                        "BelowMinimumInterval" => {
                                            Some(EvaluationReason::BelowMinimumInterval)
                                        }
                                        "TooEarlyLiveVirus" => {
                                            Some(EvaluationReason::TooEarlyLiveVirus)
                                        }
                                        "DuplicateShotSameDay" => {
                                            Some(EvaluationReason::DuplicateShotSameDay)
                                        }
                                        "VaccineNotPartOfSeries" => {
                                            Some(EvaluationReason::VaccineNotPartOfSeries)
                                        }
                                        "AboveRecommendedAgeSeries" => {
                                            Some(EvaluationReason::AboveRecommendedAgeSeries)
                                        }
                                        _ => None,
                                    })
                                    .collect();

                                evaluations.push(DoseEvaluation {
                                    dose_date: dt,
                                    cvx: *cvx,
                                    status: map_legacy_dose_status(st),
                                    dose_number: inner_dose_number,
                                    reasons: mapped_reasons.into(),
                                    sources: std::collections::HashMap::new(),
                                });
                            }
                        }
                    }
                } else if name_bytes == b"substanceAdministrationProposal" {
                    if prop_focus_matched {
                        let st = prop_status.as_deref().unwrap_or("RECOMMENDED");
                        let legacy_status = map_legacy_status(st, &prop_reasons);
                        forecasts.push(SeriesForecast {
                            series_name: "".into(),
                            status: if matches!(legacy_status, SeriesStatus::NotComplete { .. }) {
                                SeriesStatus::NotComplete {
                                    earliest_date: prop_earliest,
                                    recommended_date: prop_recommended,
                                    overdue_date: prop_overdue,
                                    latest_date: None,
                                }
                            } else {
                                legacy_status
                            },
                            reasons: prop_reasons.iter().map(|r| r.clone().into()).collect(),
                            sources: std::collections::HashMap::new(),
                        });
                    }
                }

                if let Some(last) = tag_stack.last() {
                    if last == name_str.as_ref() {
                        tag_stack.pop();
                    }
                }
            }
            _ => {}
        }
        buf.clear();
    }

    ExpectedResults {
        evaluations,
        forecasts,
    }
}

pub fn query_java_service(
    client: &reqwest::blocking::Client,
    java_endpoint: &str,
    payload: serde_json::Value,
) -> String {
    let resp = client
        .post(java_endpoint)
        .json(&payload)
        .send()
        .expect("Failed to send request to Java service");

    let resp_json: serde_json::Value = resp.json().expect("Failed to parse Java JSON response");

    let b64_payload = resp_json["finalKMEvaluationResponse"][0]["kmEvaluationResultData"][0]
        ["data"]["base64EncodedPayload"][0]
        .as_str()
        .expect("Failed to extract base64 payload from Java response");

    let clean_b64: String = b64_payload.chars().filter(|c| !c.is_whitespace()).collect();
    let xml_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, clean_b64.as_bytes())
        .expect("Failed to decode base64 XML payload");

    String::from_utf8(xml_bytes).expect("Failed to decode UTF-8 XML string")
}

pub fn get_java_expected_results(
    client: &reqwest::blocking::Client,
    java_url: &str,
    tc: &UnifiedTestCase,
) -> Result<ExpectedResults, String> {
    let java_endpoint = format!(
        "{}/opencds-decision-support-service/api/resources/evaluateAtSpecifiedTime",
        java_url
    );
    let payload = build_evaluate_payload(&tc.patient, &tc.history, tc.execution_date);
    let resp = client
        .post(&java_endpoint)
        .json(&payload)
        .send()
        .map_err(|e| {
            format!(
                "Failed to connect to Java ICE service at {}: {}",
                java_url, e
            )
        })?;

    if !resp.status().is_success() {
        return Err(format!(
            "Java service returned HTTP error: {}",
            resp.status()
        ));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Failed to parse Java JSON response: {}", e))?;

    let b64_payload = resp_json["finalKMEvaluationResponse"][0]["kmEvaluationResultData"][0]
        ["data"]["base64EncodedPayload"][0]
        .as_str()
        .ok_or_else(|| "Failed to extract base64 payload from Java response".to_string())?;

    let clean_b64: String = b64_payload.chars().filter(|c| !c.is_whitespace()).collect();
    let xml_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, clean_b64.as_bytes())
        .map_err(|e| format!("Failed to decode base64 XML payload: {}", e))?;

    let xml_content = String::from_utf8(xml_bytes)
        .map_err(|e| format!("Failed to decode UTF-8 XML string: {}", e))?;

    let mut java_res = parse_legacy_xml(&xml_content, &tc.focus_code);

    for f in &mut java_res.forecasts {
        f.series_name = tc.group.clone().into();
    }

    Ok(java_res)
}

pub fn get_java_expected_results_bulk(
    client: &reqwest::blocking::Client,
    java_url: &str,
    cases: &[&UnifiedTestCase],
) -> Result<Vec<Result<ExpectedResults, String>>, String> {
    let java_endpoint = format!(
        "{}/opencds-decision-support-service/api/resources/bulkEvaluateAtSpecifiedTime",
        java_url
    );
    let mut payloads = Vec::new();
    for tc in cases {
        let payload = build_evaluate_payload(&tc.patient, &tc.history, tc.execution_date);
        payloads.push(payload);
    }
    let resp = client
        .post(&java_endpoint)
        .json(&payloads)
        .send()
        .map_err(|e| {
            format!(
                "Failed to connect to Java ICE service at {}: {}",
                java_url, e
            )
        })?;

    if !resp.status().is_success() {
        return Err(format!(
            "Java bulk service returned HTTP error: {}",
            resp.status()
        ));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Failed to parse Java JSON response: {}", e))?;

    let resp_array = resp_json
        .as_array()
        .ok_or_else(|| "Java bulk service response is not a JSON array".to_string())?;

    if resp_array.len() != cases.len() {
        return Err(format!(
            "Java bulk service returned {} responses, but expected {}",
            resp_array.len(),
            cases.len()
        ));
    }

    let mut results = Vec::new();
    for (i, resp_item) in resp_array.iter().enumerate() {
        let tc = cases[i];
        let parse_res = (|| {
            let b64_payload = resp_item["finalKMEvaluationResponse"][0]["kmEvaluationResultData"]
                [0]["data"]["base64EncodedPayload"][0]
                .as_str()
                .ok_or_else(|| "Failed to extract base64 payload from Java response".to_string())?;

            let clean_b64: String = b64_payload.chars().filter(|c| !c.is_whitespace()).collect();
            let xml_bytes =
                base64::Engine::decode(&base64::prelude::BASE64_STANDARD, clean_b64.as_bytes())
                    .map_err(|e| format!("Failed to decode base64 XML payload: {}", e))?;

            let xml_content = String::from_utf8(xml_bytes)
                .map_err(|e| format!("Failed to decode UTF-8 XML string: {}", e))?;

            let mut java_res = parse_legacy_xml(&xml_content, &tc.focus_code);

            for f in &mut java_res.forecasts {
                f.series_name = tc.group.clone().into();
            }
            Ok(java_res)
        })();
        results.push(parse_res);
    }

    Ok(results)
}

pub fn is_group_supported_by_java(group: &str) -> bool {
    let g = group.to_uppercase();
    g != "CHOLERA" && g != "JEV" && g != "TYPHOID" && g != "YELLOW_FEVER" && g != "YELLOWFEVER"
}

pub fn query_rust_rest_service(
    client: &reqwest::blocking::Client,
    rust_url: &str,
    tc: &UnifiedTestCase,
) -> Result<lava_forecaster::models::ForecastResponse, String> {
    let req_payload = lava_forecaster::models::ForecastRequest {
        patient: tc.patient.clone(),
        history: tc.history.clone(),
        execution_date: tc.execution_date,
    };
    let endpoint = format!("{}/evaluate", rust_url);
    let resp = client
        .post(&endpoint)
        .json(&req_payload)
        .send()
        .map_err(|e| format!("Failed to connect to Rust REST server: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Rust REST server returned error: {}",
            resp.status()
        ));
    }

    let resp_data: lava_forecaster::models::ForecastResponse = resp
        .json()
        .map_err(|e| format!("Failed to parse Rust REST response: {}", e))?;

    Ok(resp_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_legacy_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ns4:cdsOutput xmlns:ns4="org.opencds.vmr.v1_0.schema.cdsoutput">
    <substanceAdministrationEvent>
        <administrationTimeInterval low="20210115"/>
        <substance>
            <substanceCode code="03"/>
        </substance>
        <substanceAdministrationEvent>
            <observationFocus code="03"/>
            <observationValue>
                <concept code="VALID"/>
            </observationValue>
            <interpretation code="OK"/>
            <doseNumber value="1"/>
        </substanceAdministrationEvent>
    </substanceAdministrationEvent>
    <substanceAdministrationProposal>
        <substanceCode code="03"/>
        <observationValue>
            <concept code="RECOMMENDED"/>
        </observationValue>
        <interpretation code="DUE"/>
        <validAdministrationTimeInterval low="20210215"/>
        <proposedAdministrationTimeInterval low="20210315" high="20210615"/>
    </substanceAdministrationProposal>
</ns4:cdsOutput>"#;

        let res = parse_legacy_xml(xml, "03");
        assert_eq!(res.evaluations.len(), 1);
        let eval = &res.evaluations[0];
        assert_eq!(
            eval.dose_date,
            NaiveDate::from_ymd_opt(2021, 1, 15).unwrap()
        );
        assert_eq!(eval.cvx, Cvx(3));
        assert_eq!(eval.status, DoseStatus::Valid);
        assert_eq!(eval.dose_number, Some(1));

        assert_eq!(res.forecasts.len(), 1);
        let fc = &res.forecasts[0];
        assert!(matches!(fc.status, SeriesStatus::NotComplete { .. }));
        if let SeriesStatus::NotComplete {
            earliest_date,
            recommended_date,
            overdue_date,
            latest_date,
        } = fc.status
        {
            assert_eq!(
                earliest_date,
                Some(NaiveDate::from_ymd_opt(2021, 2, 15).unwrap())
            );
            assert_eq!(
                recommended_date,
                Some(NaiveDate::from_ymd_opt(2021, 3, 15).unwrap())
            );
            assert_eq!(
                overdue_date,
                Some(NaiveDate::from_ymd_opt(2021, 6, 15).unwrap())
            );
            assert_eq!(latest_date, None);
        }
    }
}
