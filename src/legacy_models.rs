use serde::Deserialize;
use chrono::NaiveDate;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use crate::models::{Patient, Gender, Dose, ForecastRequest, Cvx};

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

fn parse_xml_date(ds: &str) -> Result<NaiveDate, Box<dyn std::error::Error>> {
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

/// Helper function to parse elements in XML, matching local name (ignoring prefixes)
fn handle_element(
    e: &quick_xml::events::BytesStart,
    in_event: bool,
    birth_date: &mut Option<NaiveDate>,
    gender: &mut Gender,
    current_cvx: &mut Option<Cvx>,
    current_date: &mut Option<NaiveDate>,
) -> Result<(), Box<dyn std::error::Error>> {
    match e.local_name().as_ref() {
        b"birthTime" => {
            for attr in e.attributes() {
                let attr = attr?;
                if attr.key.as_ref() == b"value" {
                    let val_str = std::str::from_utf8(&attr.value)?;
                    *birth_date = Some(parse_xml_date(val_str)?);
                }
            }
        }
        b"gender" => {
            for attr in e.attributes() {
                let attr = attr?;
                if attr.key.as_ref() == b"code" {
                    let code_str = std::str::from_utf8(&attr.value)?;
                    *gender = match code_str {
                        "M" | "m" => Gender::Male,
                        "F" | "f" => Gender::Female,
                        _ => Gender::Unknown,
                    };
                }
            }
        }
        b"substanceCode" if in_event => {
            for attr in e.attributes() {
                let attr = attr?;
                if attr.key.as_ref() == b"code" {
                    let code_str = std::str::from_utf8(&attr.value)?;
                    let code_num = code_str.parse::<u16>()?;
                    *current_cvx = Some(Cvx(code_num));
                }
            }
        }
        b"administrationTimeInterval" if in_event => {
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
                *current_date = Some(parse_xml_date(&ds)?);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parses the base64-encoded vMR XML content and extracts Patient demographics and immunization history.
pub fn parse_vmr_xml(xml: &str) -> Result<(NaiveDate, Gender, Vec<Dose>), Box<dyn std::error::Error>> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut birth_date = None;
    let mut gender = Gender::Unknown;
    let mut doses = Vec::new();
    let mut buf = Vec::new();

    let mut in_event = false;
    let mut current_cvx = None;
    let mut current_date = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"substanceAdministrationEvent" {
                    in_event = true;
                    current_cvx = None;
                    current_date = None;
                } else {
                    handle_element(e, in_event, &mut birth_date, &mut gender, &mut current_cvx, &mut current_date)?;
                }
            }
            Ok(Event::Empty(ref e)) => {
                handle_element(e, in_event, &mut birth_date, &mut gender, &mut current_cvx, &mut current_date)?;
                if e.local_name().as_ref() == b"substanceAdministrationEvent" {
                    // Though unexpected to be empty, reset just in case
                    in_event = false;
                }
            }
            Ok(Event::End(ref e)) => {
                if e.local_name().as_ref() == b"substanceAdministrationEvent" {
                    in_event = false;
                    if let (Some(date), Some(cvx)) = (current_date, current_cvx.take()) {
                        doses.push(Dose { date, cvx });
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Box::new(e)),
            _ => {}
        }
        buf.clear();
    }

    let birth_date = birth_date.ok_or("Missing birthTime in XML payload")?;
    Ok((birth_date, gender, doses))
}

impl LegacyEvaluateRequest {
    /// Translates the legacy REST payload into the internal ForecastRequest.
    pub fn translate(self) -> Result<ForecastRequest, Box<dyn std::error::Error>> {
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
        let (birth_date, gender, history) = parse_vmr_xml(xml_str)?;

        Ok(ForecastRequest {
            patient: Patient { birth_date, gender },
            history,
            execution_date: eval_date,
        })
    }
}
