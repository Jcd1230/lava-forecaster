use serde::Deserialize;
use chrono::NaiveDate;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use crate::models::{Patient, Gender, Dose, ForecastRequest, Cvx, DiseaseImmunity, Contraindication};

#[derive(Debug, Deserialize)]
pub struct LegacyInteractionId {
    #[serde(rename = "submissionTime")]
    pub submission_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyDataPayload {
    #[serde(rename = "base64EncodedPayload")]
    pub base64_encoded_payload: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyDataRequirementItem {
    pub data: LegacyDataPayload,
}

#[derive(Debug, Deserialize)]
pub struct LegacyEvaluationRequest {
    #[serde(rename = "dataRequirementItemData")]
    pub data_requirement_item_data: Vec<LegacyDataRequirementItem>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyEvaluateRequest {
    #[serde(rename = "interactionId")]
    pub interaction_id: Option<LegacyInteractionId>,
    #[serde(rename = "specifiedTime")]
    pub specified_time: Option<i64>,
    #[serde(rename = "evaluationRequest")]
    pub evaluation_request: LegacyEvaluationRequest,
}

fn parse_xml_date(ds: &str) -> Result<NaiveDate, crate::errors::ForecasterError> {
    let clean: String = ds.chars().filter(|c| c.is_ascii_digit() || *c == '-').collect();
    if clean.contains('-') {
        if clean.len() >= 10 {
            Ok(NaiveDate::parse_from_str(&clean[0..10], "%Y-%m-%d")?)
        } else {
            Err(format!("Invalid hyphenated date: {}", ds).into())
        }
    } else {
        if clean.len() >= 8 {
            Ok(NaiveDate::parse_from_str(&clean[0..8], "%Y%m%d")?)
        } else {
            Err(format!("Invalid unhyphenated date: {}", ds).into())
        }
    }
}

enum EitherObservation {
    Immunity(DiseaseImmunity),
    Contraindication(Contraindication),
}

fn map_observation(
    focus: Option<String>,
    value: Option<String>,
    interpretations: &[String],
    date: Option<NaiveDate>,
    dob: NaiveDate,
) -> Option<EitherObservation> {
    let focus_str = focus.as_deref().unwrap_or("");
    let is_immunity = interpretations.iter().any(|i| {
        let il = i.to_lowercase();
        il.contains("immune") || il.contains("immunity") || il.contains("documented")
    });

    let date = date.unwrap_or(dob);

    if is_immunity {
        let disease = if focus_str.contains("HEP_B") || focus_str.contains("27836007") || focus_str.contains("22322")
            || focus_str == "070.30" || focus_str == "B19.10" || focus_str.contains("271511000") || focus_str.contains("161467007") {
            "HepB"
        } else if focus_str.contains("VARICELLA") || focus_str.contains("38907003") || focus_str.contains("15410")
            || focus_str == "052.9" || focus_str == "B01.9" || focus_str.contains("371113008") || focus_str.contains("161719005") {
            "Varicella"
        } else if focus_str.contains("MEASLES") || focus_str.contains("14189004")
            || focus_str == "055.9" || focus_str == "B05.9" || focus_str.contains("371111005") || focus_str.contains("161278002") {
            "Measles"
        } else if focus_str.contains("MUMPS") || focus_str.contains("36989005")
            || focus_str == "072.9" || focus_str == "B26.9" || focus_str.contains("371112003") {
            "Mumps"
        } else if focus_str.contains("RUBELLA") || focus_str.contains("36653000")
            || focus_str == "056.9" || focus_str == "B06.9" || focus_str.contains("278968001") || focus_str.contains("161280008") {
            "Rubella"
        } else if focus_str.contains("HEP_A") || focus_str.contains("40468003")
            || focus_str == "070.1" || focus_str == "B15.9" || focus_str.contains("278971009") || focus_str.contains("161466003") {
            "HepA"
        } else {
            focus_str
        };
        let reason = interpretations.first().cloned().unwrap_or_else(|| "PROOF_OF_IMMUNITY".to_string());
        Some(EitherObservation::Immunity(DiseaseImmunity {
            disease: disease.to_string(),
            date,
            reason,
        }))
    } else {
        // Contraindication
        let target = if focus_str.contains("PERTUSSIS") || focus_str.contains("70654002") {
            "DTP"
        } else {
            focus_str
        };
        let reason = interpretations.first().or(value.as_ref()).cloned().unwrap_or_else(|| "CONTRAINDICATION".to_string());
        Some(EitherObservation::Contraindication(Contraindication {
            date,
            target: target.to_string(),
            reason,
            valid_until: None,
            cvx: None,
        }))
    }
}

/// Parses the base64-encoded vMR XML content and extracts Patient demographics and immunization history.
pub fn parse_vmr_xml(xml: &str) -> Result<(Patient, Vec<Dose>), crate::errors::ForecasterError> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut birth_date = None;
    let mut gender = Gender::Unknown;
    let mut doses = Vec::new();
    let mut immunities = Vec::new();
    let mut contraindications = Vec::new();
    let mut buf = Vec::new();

    let mut in_event = false;
    let mut current_cvx = None;
    let mut current_date = None;
    let mut current_is_valid = None;

    let mut in_obs = false;
    let mut obs_focus = None;
    let mut obs_value = None;
    let mut obs_interpretations = Vec::new();
    let mut obs_date = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local_name = e.local_name();
                let tag = local_name.as_ref();
                if tag == b"substanceAdministrationEvent" {
                    in_event = true;
                    current_cvx = None;
                    current_date = None;
                    current_is_valid = None;
                } else if tag == b"observationResult" {
                    in_obs = true;
                    obs_focus = None;
                    obs_value = None;
                    obs_interpretations.clear();
                    obs_date = None;
                } else if in_event {
                    if tag == b"substanceCode" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                let code_str = std::str::from_utf8(&attr.value)?;
                                let code_num = code_str.parse::<u16>()?;
                                current_cvx = Some(Cvx(code_num));
                            }
                        }
                    } else if tag == b"administrationTimeInterval" {
                        let mut low = None;
                        let mut high = None;
                        for attr in e.attributes() {
                            let attr = attr?;
                            match attr.key.as_ref() {
                                b"low" => low = Some(std::str::from_utf8(&attr.value)?.to_string()),
                                b"high" => high = Some(std::str::from_utf8(&attr.value)?.to_string()),
                                _ => {}
                            }
                        }
                        let date_str = low.or(high);
                        if let Some(ds) = date_str {
                            current_date = Some(parse_xml_date(&ds)?);
                        }
                    } else if tag == b"isValid" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"value" {
                                let val_str = std::str::from_utf8(&attr.value)?;
                                current_is_valid = Some(val_str == "true" || val_str == "1");
                            }
                        }
                    }
                } else if in_obs {
                    if tag == b"observationFocus" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                obs_focus = Some(std::str::from_utf8(&attr.value)?.to_string());
                            }
                        }
                    } else if tag == b"concept" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                obs_value = Some(std::str::from_utf8(&attr.value)?.to_string());
                            }
                        }
                    } else if tag == b"interpretation" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                obs_interpretations.push(std::str::from_utf8(&attr.value)?.to_string());
                            }
                        }
                    } else if tag == b"observationEventTime" {
                        let mut val = None;
                        let mut low = None;
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"value" {
                                val = Some(std::str::from_utf8(&attr.value)?.to_string());
                            } else if attr.key.as_ref() == b"low" {
                                low = Some(std::str::from_utf8(&attr.value)?.to_string());
                            }
                        }
                        let date_str = val.or(low);
                        if let Some(ds) = date_str {
                            obs_date = Some(parse_xml_date(&ds)?);
                        }
                    }
                } else {
                    if tag == b"birthTime" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"value" {
                                let val_str = std::str::from_utf8(&attr.value)?;
                                birth_date = Some(parse_xml_date(val_str)?);
                            }
                        }
                    } else if tag == b"gender" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                let code_str = std::str::from_utf8(&attr.value)?;
                                gender = match code_str {
                                    "M" | "m" => Gender::Male,
                                    "F" | "f" => Gender::Female,
                                    _ => Gender::Unknown,
                                };
                            }
                        }
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local_name = e.local_name();
                let tag = local_name.as_ref();
                if in_event {
                    if tag == b"isValid" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"value" {
                                let val_str = std::str::from_utf8(&attr.value)?;
                                current_is_valid = Some(val_str == "true" || val_str == "1");
                            }
                        }
                    }
                } else if in_obs {
                    if tag == b"observationFocus" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                obs_focus = Some(std::str::from_utf8(&attr.value)?.to_string());
                            }
                        }
                    } else if tag == b"concept" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                obs_value = Some(std::str::from_utf8(&attr.value)?.to_string());
                            }
                        }
                    } else if tag == b"interpretation" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                obs_interpretations.push(std::str::from_utf8(&attr.value)?.to_string());
                            }
                        }
                    }
                } else {
                    if tag == b"birthTime" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"value" {
                                let val_str = std::str::from_utf8(&attr.value)?;
                                birth_date = Some(parse_xml_date(val_str)?);
                            }
                        }
                    } else if tag == b"gender" {
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"code" {
                                let code_str = std::str::from_utf8(&attr.value)?;
                                gender = match code_str {
                                    "M" | "m" => Gender::Male,
                                    "F" | "f" => Gender::Female,
                                    _ => Gender::Unknown,
                                };
                            }
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local_name = e.local_name();
                let tag = local_name.as_ref();
                if tag == b"substanceAdministrationEvent" {
                    in_event = false;
                    if let (Some(date), Some(cvx)) = (current_date, current_cvx.take()) {
                        doses.push(Dose { date, cvx, is_valid: current_is_valid });
                    }
                } else if tag == b"observationResult" {
                    in_obs = false;
                    let dob_fallback = birth_date.unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                    if let Some(mapped) = map_observation(obs_focus.take(), obs_value.take(), &obs_interpretations, obs_date.take(), dob_fallback) {
                        match mapped {
                            EitherObservation::Immunity(imm) => immunities.push(imm),
                            EitherObservation::Contraindication(c) => contraindications.push(c),
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    let birth_date = birth_date.ok_or("Missing birthTime in XML payload")?;
    Ok((
        Patient {
            birth_date,
            gender,
            immunities,
            contraindications,
        },
        doses,
    ))
}

impl LegacyEvaluateRequest {
    /// Translates the legacy REST payload into the internal ForecastRequest.
    pub fn translate(self) -> Result<ForecastRequest, crate::errors::ForecasterError> {
        // 1. Determine execution date (specifiedTime or submissionTime fallback)
        let eval_timestamp_ms = self.specified_time
            .or_else(|| self.interaction_id.as_ref().and_then(|id| id.submission_time))
            .ok_or("Missing evaluation execution date (specifiedTime or submissionTime)")?;
        
        let eval_secs = eval_timestamp_ms / 1000;
        let eval_date = chrono::DateTime::from_timestamp(eval_secs, 0)
            .ok_or("Invalid evaluation execution date timestamp")?
            .date_naive();

        // 2. Locate base64 encoded XML payload
        let dri = self.evaluation_request.data_requirement_item_data.first()
            .ok_or("Missing dataRequirementItemData item")?;
        
        let b64_payload = dri.data.base64_encoded_payload.first()
            .ok_or("Missing base64EncodedPayload string")?;
        
        // Remove whitespace/newlines from base64 string
        let clean_b64: String = b64_payload.chars().filter(|c| !c.is_whitespace()).collect();
        let xml_bytes = STANDARD.decode(clean_b64)?;
        let xml_str = std::str::from_utf8(&xml_bytes)?;

        // 3. Parse XML
        let (patient, history) = parse_vmr_xml(xml_str)?;

        Ok(ForecastRequest {
            patient,
            history,
            execution_date: eval_date,
        })
    }
}
