use std::collections::HashMap;
use std::fs;
use std::path::Path;
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use ice_rust_forecaster_poc::{
    evaluate_patient_all_groups,
    models::{
        Dose, DoseEvaluation, DoseStatus, EvaluationReason, ExpectedResults,
        Gender, Patient, SeriesForecast, SeriesStatus, UnifiedTestCase, Cvx,
    },
};

// Helper for relative date resolution
struct RelativeDateResolver {
    dob: NaiveDate,
    dose_dates: Vec<NaiveDate>,
}

impl RelativeDateResolver {
    fn new(dob: NaiveDate) -> Self {
        Self {
            dob,
            dose_dates: Vec::new(),
        }
    }

    fn add_months(d: NaiveDate, months: i32) -> NaiveDate {
        let mut year = d.year();
        let mut month = d.month() as i32 + months - 1;
        year += month / 12;
        month = month % 12 + 1;
        if month <= 0 {
            month += 12;
            year -= 1;
        }
        
        // Handle end-of-month clamping (e.g. Oct 31 + 1 month -> Nov 30)
        let mut day = d.day();
        loop {
            if let Some(date) = NaiveDate::from_ymd_opt(year, month as u32, day) {
                return date;
            }
            day -= 1;
            if day == 0 {
                panic!("Invalid day reached in add_months");
            }
        }
    }

    fn add_offset(d: NaiveDate, val: i32, unit: char) -> NaiveDate {
        match unit {
            'y' => Self::add_months(d, val * 12),
            'm' => Self::add_months(d, val),
            'w' => d + chrono::Duration::days(val as i64 * 7),
            'd' => d + chrono::Duration::days(val as i64),
            _ => d,
        }
    }

    fn resolve(&mut self, expr: &str) -> NaiveDate {
        let expr = expr.trim();
        if expr == "birth" {
            return self.dob;
        }

        let parts: Vec<&str> = expr.split('+').collect();
        let base_ref = parts[0].trim();

        let base_date = if base_ref == "birth" {
            self.dob
        } else if base_ref == "prev" {
            *self.dose_dates.last().unwrap_or(&self.dob)
        } else if base_ref.starts_with("dose") {
            let idx: usize = base_ref[4..].parse::<usize>().unwrap() - 1;
            self.dose_dates[idx]
        } else {
            match NaiveDate::parse_from_str(base_ref, "%Y-%m-%d") {
                Ok(d) => return d,
                Err(_) => panic!("Unknown date base reference: {}", base_ref),
            }
        };

        if parts.len() == 1 {
            return base_date;
        }

        let offset_str = parts[1].trim();
        // Simple manual parsing of terms like "2m" or "15m" or "-4d"
        let mut current_date = base_date;
        let mut chars = offset_str.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
                continue;
            }
            let mut sign = 1;
            if c == '+' {
                chars.next();
            } else if c == '-' {
                sign = -1;
                chars.next();
            }

            let mut digits = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_digit() {
                    digits.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }

            let val: i32 = digits.parse().unwrap_or(0) * sign;
            if let Some(unit) = chars.next() {
                current_date = Self::add_offset(current_date, val, unit);
            }
        }

        current_date
    }
}

// Java ICE XML Payload Builder
fn generate_xml_payload(dob: NaiveDate, gender: Gender, doses: &[(NaiveDate, Cvx)]) -> String {
    let mut sae_templates = Vec::new();
    for (idx, &(dt, cvx)) in doses.iter().enumerate() {
        let dt_str = dt.format("%Y%m%d").to_string();
        sae_templates.push(format!(
            r#"                    <substanceAdministrationEvent>
                        <templateId root="2.16.840.1.113883.3.795.11.9.1.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.10" extension="{}"/>
                        <substanceAdministrationGeneralPurpose code="384810002" codeSystem="2.16.840.1.113883.6.5"/>
                        <substance>
                            <id root="ab0c489e-782a-4c34-9e4e-9094cc2952d7"/>
                            <substanceCode code="{}" displayName="Vaccine" codeSystem="2.16.840.1.113883.12.292"/>
                        </substance>
                        <administrationTimeInterval high="{}" low="{}"/>
                    </substanceAdministrationEvent>"#,
            1000 + idx, cvx.0, dt_str, dt_str
        ));
    }

    let sae_str = sae_templates.join("\n");
    let dob_str = dob.format("%Y%m%d").to_string();
    let gender_code = match gender {
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
                <substanceAdministrationEvents>
{}
                </substanceAdministrationEvents>
            </clinicalStatements>
        </patient>
    </vmrInput>
</ns3:cdsInput>"#,
        dob_str, gender_code, sae_str
    )
}

fn build_evaluate_payload(
    dob: NaiveDate,
    gender: Gender,
    doses: &[(NaiveDate, Cvx)],
    eval_date: NaiveDate,
) -> serde_json::Value {
    let xml_content = generate_xml_payload(dob, gender, doses);
    let b64_xml = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, xml_content.as_bytes());

    let eval_datetime = eval_date.and_hms_opt(0, 0, 0).unwrap();
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

// Minimal XML parser to extract expected evaluations & forecast from Java response
fn parse_date_only(date_str: &str) -> Option<NaiveDate> {
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

fn map_legacy_status(legacy_status: &str, reasons: &[String]) -> SeriesStatus {
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
        return SeriesStatus::NotComplete;
    }
    SeriesStatus::NotComplete
}

fn map_legacy_dose_status(status: &str) -> DoseStatus {
    match status {
        "VALID" => DoseStatus::Valid,
        "INVALID" => DoseStatus::Invalid,
        "ACCEPTED" => DoseStatus::Accepted,
        _ => DoseStatus::Invalid,
    }
}

fn process_element(
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
    let event_depth = tag_stack.iter().filter(|&t| t == "substanceAdministrationEvent").count();
    let in_proposal = tag_stack.iter().any(|t| t == "substanceAdministrationProposal");

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
                            *inner_status = Some(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            *prop_status = Some(String::from_utf8_lossy(a.value.as_ref()).into_owned());
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
                            inner_reasons.push(String::from_utf8_lossy(a.value.as_ref()).into_owned());
                        }
                    }
                }
            } else if in_proposal {
                for attr in attributes {
                    if let Ok(a) = attr {
                        if a.key.as_ref() == b"code" {
                            prop_reasons.push(String::from_utf8_lossy(a.value.as_ref()).into_owned());
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

fn parse_legacy_xml(xml_content: &str, focus_code: &str) -> ExpectedResults {
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
            Err(e) => panic!("Error parsing XML at position {}: {:?}", reader.buffer_position(), e),
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
                    let event_depth = tag_stack.iter().filter(|&t| t == "substanceAdministrationEvent").count();
                    if event_depth == 2 {
                        if inner_focus_matched {
                            if let (Some(dt), Some(cvx), Some(ref st)) = (outer_date, outer_cvx.as_ref(), inner_status.as_ref()) {
                                let mapped_reasons: Vec<EvaluationReason> = inner_reasons
                                    .iter()
                                    .filter_map(|r| match r.as_str() {
                                        "BelowMinimumAge" => Some(EvaluationReason::BelowMinimumAge),
                                        "BelowMinimumInterval" => Some(EvaluationReason::BelowMinimumInterval),
                                        "TooEarlyLiveVirus" => Some(EvaluationReason::TooEarlyLiveVirus),
                                        "DuplicateShotSameDay" => Some(EvaluationReason::DuplicateShotSameDay),
                                        "VaccineNotPartOfSeries" => Some(EvaluationReason::VaccineNotPartOfSeries),
                                        "AboveRecommendedAgeSeries" => Some(EvaluationReason::AboveRecommendedAgeSeries),
                                        _ => None,
                                    })
                                    .collect();

                                evaluations.push(DoseEvaluation {
                                    dose_date: dt,
                                    cvx: *cvx,
                                    status: map_legacy_dose_status(st),
                                    dose_number: inner_dose_number,
                                    reasons: mapped_reasons,
                                });
                            }
                        }
                    }
                } else if name_bytes == b"substanceAdministrationProposal" {
                    if prop_focus_matched {
                        let st = prop_status.as_deref().unwrap_or("RECOMMENDED");
                        forecasts.push(SeriesForecast {
                            series_name: "".to_string(),
                            earliest_date: prop_earliest,
                            recommended_date: prop_recommended,
                            overdue_date: prop_overdue,
                            latest_date: None,
                            status: map_legacy_status(st, &prop_reasons),
                            reasons: prop_reasons.clone(),
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

// REST call to Java ICE
fn query_java_service(java_endpoint: &str, payload: serde_json::Value) -> String {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(java_endpoint)
        .json(&payload)
        .send()
        .expect("Failed to send request to Java service");
    
    let resp_json: serde_json::Value = resp.json().expect("Failed to parse Java JSON response");
    
    let b64_payload = resp_json["finalKMEvaluationResponse"][0]["kmEvaluationResultData"][0]["data"]["base64EncodedPayload"][0]
        .as_str()
        .expect("Failed to extract base64 payload from Java response");

    let clean_b64: String = b64_payload.chars().filter(|c| !c.is_whitespace()).collect();
    let xml_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, clean_b64.as_bytes())
        .expect("Failed to decode base64 XML payload");

    String::from_utf8(xml_bytes).expect("Failed to decode UTF-8 XML string")
}

// Relative parsing structure from Python json suites
#[derive(Debug, Deserialize)]
struct RawSuiteCase {
    name: String,
    dob: String,
    gender: String,
    eval_date: String,
    doses: Vec<String>,
    group: String,
    focus: String,
}

#[derive(Debug, Deserialize)]
struct RawTestSuite {
    #[serde(default)]
    histories: HashMap<String, serde_json::Value>,
    test_cases: Vec<RawSuiteCase>,
}

fn import_python_cases(input_file: &Path, output_dir: &Path) {
    let content = fs::read_to_string(input_file).expect("Failed to read raw suite file");
    let suite: RawTestSuite = serde_json::from_str(&content).expect("Failed to parse raw suite JSON");

    let mut imported = 0;
    for tc in suite.test_cases {
        let dob = NaiveDate::parse_from_str(&tc.dob, "%Y-%m-%d")
            .expect("Failed to parse patient birth_date");
        let gender = match tc.gender.as_str() {
            "F" => Gender::Female,
            "M" => Gender::Male,
            _ => Gender::Unknown,
        };

        let mut resolver = RelativeDateResolver::new(dob);
        let mut resolved_doses = Vec::new();
        for dose_str in tc.doses {
            let parts: Vec<&str> = dose_str.split(':').collect();
            let date_expr = parts[0];
            let cvx_num = parts[1].parse::<u16>().unwrap();
            let resolved_date = resolver.resolve(date_expr);
            resolver.dose_dates.push(resolved_date);
            resolved_doses.push(Dose {
                date: resolved_date,
                cvx: Cvx(cvx_num),
            });
        }

        let resolved_eval_date = resolver.resolve(&tc.eval_date);

        let mut focus_code = tc.focus.clone();
        if focus_code.len() == 1 && focus_code.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            focus_code = format!("0{}", focus_code);
        }

        let unified_case = UnifiedTestCase {
            name: tc.name.clone(),
            group: tc.group,
            focus_code,
            patient: Patient {
                birth_date: dob,
                gender,
            },
            history: resolved_doses,
            execution_date: resolved_eval_date,
            expected: None,
        };

        let out_path = output_dir.join(format!("{}.json", tc.name));
        let out_json = serde_json::to_string_pretty(&unified_case).unwrap();
        fs::write(out_path, out_json).expect("Failed to write unified case JSON");
        imported += 1;
    }

    println!("Imported {} test cases from {:?}", imported, input_file);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("=== Rust-Native Centralized Test Runner ===");
        println!("Usage:");
        println!("  test-runner --import-cdsi <raw_json_suite> <output_dir>");
        println!("  test-runner --record <cases_dir> [java_url]");
        println!("  test-runner --run <cases_dir> [--group <group_name>] [--case <case_name>]");
        return;
    }

    let mode = &args[1];
    if mode == "--import-cdsi" {
        let input_suite = Path::new(&args[2]);
        let output_dir = Path::new(&args[3]);
        fs::create_dir_all(output_dir).unwrap();
        import_python_cases(input_suite, output_dir);
    } else if mode == "--record" {
        let cases_dir = Path::new(&args[2]);
        let java_uri = args.get(3).map(|s| s.as_str()).unwrap_or("http://localhost:8080");
        let java_endpoint = format!("{}/opencds-decision-support-service/api/resources/evaluateAtSpecifiedTime", java_uri);

        let entries = fs::read_dir(cases_dir).expect("Failed to read cases dir");
        let mut count = 0;
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                let content = fs::read_to_string(&path).unwrap();
                if let Ok(mut tc) = serde_json::from_str::<UnifiedTestCase>(&content) {
                    println!("Recording Java snapshot for: {}", tc.name);
                    let doses_tuples: Vec<(NaiveDate, Cvx)> = tc.history
                        .iter()
                        .map(|d| (d.date, d.cvx))
                        .collect();
                    let payload = build_evaluate_payload(
                        tc.patient.birth_date,
                        tc.patient.gender,
                        &doses_tuples,
                        tc.execution_date,
                    );
                    let xml_out = query_java_service(&java_endpoint, payload);
                    let mut java_res = parse_legacy_xml(&xml_out, &tc.focus_code);
                    
                    // Fill in series names for expected forecasts to match Rust
                    for f in &mut java_res.forecasts {
                        f.series_name = tc.group.clone(); // Fallback focus series
                    }

                    tc.expected = Some(java_res);

                    let out_json = serde_json::to_string_pretty(&tc).unwrap();
                    fs::write(&path, out_json).unwrap();
                    count += 1;
                }
            }
        }
        println!("Successfully recorded snapshots for {} cases.", count);
    } else if mode == "--run" {
        let cases_dir = Path::new(&args[2]);
        
        let mut filter_group = None;
        let mut filter_case = None;
        let mut idx = 3;
        while idx < args.len() {
            if args[idx] == "--group" {
                filter_group = Some(args[idx + 1].to_uppercase());
                idx += 2;
            } else if args[idx] == "--case" {
                filter_case = Some(args[idx + 1].clone());
                idx += 2;
            } else {
                idx += 1;
            }
        }

        let entries = fs::read_dir(cases_dir).expect("Failed to read cases dir");
        let mut total = 0;
        let mut passed = 0;
        let mut failed = 0;
        
        let mut failed_details = Vec::new();

        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                let content = fs::read_to_string(&path).unwrap();
                if let Ok(tc) = serde_json::from_str::<UnifiedTestCase>(&content) {
                    if let Some(ref fg) = filter_group {
                        if tc.group.to_uppercase() != *fg {
                            continue;
                        }
                    }
                    if let Some(ref fc) = filter_case {
                        if tc.name != *fc {
                            continue;
                        }
                    }

                    total += 1;
                    
                    // Run the Rust forecaster core in-process!
                    let rust_results = evaluate_patient_all_groups(&tc.patient, &tc.history, tc.execution_date);
                    
                    // Retrieve matching group forecast
                    let rust_group = rust_results.iter().find(|rg| rg.vaccine_group == tc.group);

                    if filter_case.is_some() {
                        println!("DEBUG: patient: {:?}", tc.patient);
                        println!("DEBUG: history: {:?}", tc.history);
                        println!("DEBUG: execution_date: {:?}", tc.execution_date);
                        println!("DEBUG: rust_group: {:#?}", rust_group);
                        println!("DEBUG: expected: {:#?}", tc.expected);
                    }
                    
                    let mut is_ok = true;
                    let mut errors = Vec::new();

                    if let Some(expected) = &tc.expected {
                        if let Some(rg) = rust_group {
                            // 1. Verify evaluations
                            for ee in &expected.evaluations {
                                let re = rg.evaluations.iter().find(|r| r.dose_date == ee.dose_date && r.cvx == ee.cvx);
                                match re {
                                    Some(re) => {
                                        if re.status != ee.status {
                                            is_ok = false;
                                            errors.push(format!(
                                                "Evaluation status mismatch for dose ({:?}, {}): Rust={:?} (reasons={:?}), Expected={:?}",
                                                re.dose_date, re.cvx, re.status, re.reasons, ee.status
                                            ));
                                        }
                                    }
                                    None => {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Evaluation missing in Rust for dose ({:?}, {})",
                                            ee.dose_date, ee.cvx
                                        ));
                                    }
                                }
                            }
                            for re in &rg.evaluations {
                                let ee = expected.evaluations.iter().find(|e| e.dose_date == re.dose_date && e.cvx == re.cvx);
                                if ee.is_none() {
                                    is_ok = false;
                                    errors.push(format!(
                                        "Evaluation missing in Expected for dose ({:?}, {})",
                                        re.dose_date, re.cvx
                                    ));
                                }
                            }

                            // 2. Verify forecasts
                            // Extract primary series forecast from Rust
                            let rust_fc = rg.forecasts.first();
                            let exp_fc = expected.forecasts.first();

                            match (rust_fc, exp_fc) {
                                (Some(rf), Some(ef)) => {
                                    if rf.status != ef.status {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast status mismatch: Rust={:?}, Expected={:?}",
                                            rf.status, ef.status
                                        ));
                                    }
                                    if rf.earliest_date != ef.earliest_date {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast earliest date mismatch: Rust={:?}, Expected={:?}",
                                            rf.earliest_date, ef.earliest_date
                                        ));
                                    }
                                    if rf.recommended_date != ef.recommended_date {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast recommended date mismatch: Rust={:?}, Expected={:?}",
                                            rf.recommended_date, ef.recommended_date
                                        ));
                                    }
                                    if rf.overdue_date != ef.overdue_date {
                                        is_ok = false;
                                        errors.push(format!(
                                            "Forecast overdue date mismatch: Rust={:?}, Expected={:?}",
                                            rf.overdue_date, ef.overdue_date
                                        ));
                                    }
                                }
                                (None, None) => {}
                                _ => {
                                    is_ok = false;
                                    errors.push("Forecast availability mismatch (one is missing)".to_string());
                                }
                            }
                        } else {
                            is_ok = false;
                            errors.push(format!("Vaccine group {} not found in Rust forecast results", tc.group));
                        }
                    } else {
                        // No expected snapshot stored in case file
                        is_ok = false;
                        errors.push("No expected snapshots recorded in test case file. Run with --record first.".to_string());
                    }

                    if is_ok {
                        passed += 1;
                    } else {
                        failed += 1;
                        failed_details.push((tc.name.clone(), errors));
                    }
                }
            }
        }

        println!("\n--- Rust-Native Test Runner Summary ---");
        println!("Executed: {}", total);
        println!("Passed  : {}", passed);
        println!("Failed  : {}", failed);

        if failed > 0 {
            println!("\nFailed Cases:");
            for (name, errs) in failed_details {
                println!("  FAIL: {}", name);
                for err in errs {
                    println!("    - {}", err);
                }
            }
            std::process::exit(1);
        } else {
            println!("All checked tests passed successfully!");
            std::process::exit(0);
        }
    }
}
