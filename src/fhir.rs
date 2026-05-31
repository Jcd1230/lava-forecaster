use serde::{Serialize, Deserialize};
use chrono::NaiveDate;
use crate::models::{Gender, Dose, DoseEvaluation, DoseStatus, EvaluationReason, SeriesStatus, VaccineGroupForecast, Patient as InternalPatient, Cvx};

// =========================================================================
// FHIR R4 Resource Models (Minimal subset for Immunization Decision Support)
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeableConcept {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<Coding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl CodeableConcept {
    pub fn new_simple(system: &str, code: &str, display: &str) -> Self {
        Self {
            coding: Some(vec![Coding {
                system: Some(system.to_string()),
                code: Some(code.to_string()),
                display: Some(display.to_string()),
            }]),
            text: Some(display.to_string()),
        }
    }

    pub fn new_text(text: &str) -> Self {
        Self {
            coding: None,
            text: Some(text.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub id: Option<String>,
    #[serde(rename = "birthDate")]
    pub birth_date: Option<String>,
    pub gender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Immunization {
    pub id: Option<String>,
    pub status: String,
    #[serde(rename = "vaccineCode")]
    pub vaccine_code: CodeableConcept,
    #[serde(rename = "occurrenceDateTime")]
    pub occurrence_date_time: Option<String>,
    #[serde(rename = "occurrenceString")]
    pub occurrence_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "resourceType")]
pub enum FhirResource {
    Patient(Patient),
    Immunization(Immunization),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleEntry {
    #[serde(rename = "fullUrl", skip_serializing_if = "Option::is_none")]
    pub full_url: Option<String>,
    pub resource: FhirResource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    #[serde(rename = "type")]
    pub bundle_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Vec<BundleEntry>>,
}

// Parameters model for $recommend operational endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<FhirResource>,
    #[serde(rename = "valueString", skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub parameter: Vec<ParameterEntry>,
}

// FHIR Evaluation & Recommendation Outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmunizationEvaluation {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub id: Option<String>,
    pub status: String, // "completed"
    pub patient: Reference,
    #[serde(rename = "targetDisease")]
    pub target_disease: CodeableConcept,
    #[serde(rename = "doseStatus")]
    pub dose_status: CodeableConcept,
    #[serde(rename = "doseStatusReason", skip_serializing_if = "Option::is_none")]
    pub dose_status_reason: Option<Vec<CodeableConcept>>,
    #[serde(rename = "immunizationEvent")]
    pub immunization_event: Reference,
    #[serde(rename = "doseNumberPositiveInt", skip_serializing_if = "Option::is_none")]
    pub dose_number_positive_int: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateCriterion {
    pub code: CodeableConcept,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationItem {
    #[serde(rename = "vaccineCode", skip_serializing_if = "Option::is_none")]
    pub vaccine_code: Option<CodeableConcept>,
    #[serde(rename = "targetDisease", skip_serializing_if = "Option::is_none")]
    pub target_disease: Option<CodeableConcept>,
    #[serde(rename = "forecastStatus")]
    pub forecast_status: CodeableConcept,
    #[serde(rename = "dateCriterion", skip_serializing_if = "Option::is_none")]
    pub date_criterion: Option<Vec<DateCriterion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "supportingImmunization", skip_serializing_if = "Option::is_none")]
    pub supporting_immunization: Option<Vec<Reference>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmunizationRecommendation {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub id: Option<String>,
    pub patient: Reference,
    pub date: String,
    pub recommendation: Vec<RecommendationItem>,
}

// Bundle wrapper for $recommend endpoint response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "resourceType")]
pub enum OutgoingFhirResource {
    ImmunizationRecommendation(ImmunizationRecommendation),
    ImmunizationEvaluation(ImmunizationEvaluation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingBundleEntry {
    pub resource: OutgoingFhirResource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingBundle {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    #[serde(rename = "type")]
    pub bundle_type: String,
    pub entry: Vec<OutgoingBundleEntry>,
}

// =========================================================================
// CDS Hooks Request/Response Models
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDSContext {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "patientId")]
    pub patient_id: String,
    #[serde(rename = "encounterId")]
    pub encounter_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDSRequest {
    pub hook: String,
    #[serde(rename = "hookInstance")]
    pub hook_instance: String,
    #[serde(rename = "fhirServer")]
    pub fhir_server: Option<String>,
    #[serde(rename = "fhirAuthorization")]
    pub fhir_authorization: Option<serde_json::Value>,
    pub context: CDSContext,
    pub prefetch: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub label: String,
    pub url: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub label: String,
    pub url: String,
    pub r#type: String, // "absolute" or "smart"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub r#type: String, // "create", "update", "delete"
    pub description: String,
    pub resource: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub label: String,
    pub uuid: Option<String>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub summary: String,
    pub detail: Option<String>,
    pub indicator: String, // "info", "warning", "critical"
    pub source: Source,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<Suggestion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<Link>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDSResponse {
    pub cards: Vec<Card>,
}

// =========================================================================
// Translators & Conversion Implementations
// =========================================================================

impl TryFrom<Patient> for InternalPatient {
    type Error = String;

    fn try_from(p: Patient) -> Result<Self, Self::Error> {
        let birth_date_str = p.birth_date.ok_or_else(|| "Missing birthDate".to_string())?;
        // Parse date (assumes YYYY-MM-DD or simple prefix)
        let clean_date = birth_date_str.chars().take(10).collect::<String>();
        let birth_date = NaiveDate::parse_from_str(&clean_date, "%Y-%m-%d")
            .map_err(|e| format!("Invalid birthDate format (expected YYYY-MM-DD): {}", e))?;

        let gender_str = p.gender.unwrap_or_default().to_lowercase();
        let gender = match gender_str.as_str() {
            "male" | "m" => Gender::Male,
            "female" | "f" => Gender::Female,
            _ => Gender::Unknown,
        };

        Ok(InternalPatient { birth_date, gender })
    }
}

impl TryFrom<Immunization> for Dose {
    type Error = String;

    fn try_from(imm: Immunization) -> Result<Self, Self::Error> {
        // Extract occurrence date
        let date_str = imm.occurrence_date_time
            .or(imm.occurrence_string)
            .ok_or_else(|| "Missing occurrenceDateTime or occurrenceString".to_string())?;
        
        let clean_date = date_str.chars().take(10).collect::<String>();
        let date = NaiveDate::parse_from_str(&clean_date, "%Y-%m-%d")
            .map_err(|e| format!("Invalid occurrence date format: {}", e))?;

        // Extract CVX code
        let codings = imm.vaccine_code.coding.ok_or_else(|| "Missing codings in vaccineCode".to_string())?;
        let cvx_code = codings.iter()
            .find(|c| c.system.as_deref() == Some("http://hl7.org/fhir/sid/cvx"))
            .and_then(|c| c.code.as_ref())
            .ok_or_else(|| "Missing CVX code in vaccineCode codings".to_string())?;

        let cvx_val = cvx_code.parse::<u16>()
            .map_err(|e| format!("Invalid CVX code (not a u16): {}", e))?;

        Ok(Dose { date, cvx: Cvx(cvx_val) })
    }
}

// Helpers to map internal models back to FHIR

pub fn map_evaluation_reason(reason: EvaluationReason) -> CodeableConcept {
    let display = format!("{:?}", reason);
    // Standard system for immunization evaluation reasons or custom text fallback
    CodeableConcept::new_simple("http://terminology.hl7.org/CodeSystem/immunization-evaluation-dose-status-reason", &display, &display)
}

pub fn map_target_disease(group_name: &str) -> CodeableConcept {
    // Map ICE target groups to SNOMED codes if possible
    let snomed = match group_name.to_lowercase().as_str() {
        "polio" | "ipv" | "opv" => Some(("89244002", "Poliomyelitis")),
        "hepa" | "hepatitis a" => Some(("22387003", "Hepatitis A")),
        "hepb" | "hepatitis b" => Some(("27836007", "Hepatitis B")),
        "mmr" => Some(("368305001", "Measles, mumps and rubella")),
        "measles" => Some(("14189004", "Measles")),
        "mumps" => Some(("36989005", "Mumps")),
        "rubella" => Some(("36653000", "Rubella")),
        "varicella" => Some(("38907003", "Varicella")),
        "pneumo" | "pneumococcal" => Some(("128601007", "Pneumococcal infectious disease")),
        "hpv" => Some(("240532009", "Human papillomavirus infection")),
        "meningococcal" | "mening" => Some(("16541001", "Meningococcal infectious disease")),
        "flu" | "influenza" => Some(("6142004", "Influenza")),
        "rotavirus" | "rota" => Some(("18624000", "Disease caused by Rotavirus")),
        "hib" => Some(("442438000", "Infection caused by Haemophilus influenzae type b")),
        _ => None,
    };

    if let Some((code, display)) = snomed {
        CodeableConcept::new_simple("http://snomed.info/sct", code, display)
    } else {
        CodeableConcept::new_text(group_name)
    }
}

pub fn make_immunization_evaluation(
    patient_id: &str,
    dose_idx: usize,
    dose_eval: &DoseEvaluation,
    target_group: &str,
) -> ImmunizationEvaluation {
    let eval_id = format!("eval-{}-{}", patient_id, dose_idx);
    
    let dose_status = match dose_eval.status {
        DoseStatus::Valid | DoseStatus::Accepted => {
            CodeableConcept::new_simple(
                "http://terminology.hl7.org/CodeSystem/immunization-evaluation-dose-status",
                "valid",
                "Valid"
            )
        }
        DoseStatus::Invalid | DoseStatus::Ignored => {
            CodeableConcept::new_simple(
                "http://terminology.hl7.org/CodeSystem/immunization-evaluation-dose-status",
                "invalid",
                "Invalid"
            )
        }
    };

    let dose_status_reason = if dose_eval.reasons.is_empty() {
        None
    } else {
        Some(dose_eval.reasons.iter().map(|&r| map_evaluation_reason(r)).collect())
    };

    let dose_number = dose_eval.dose_number.map(|n| n as i32);

    ImmunizationEvaluation {
        resource_type: "ImmunizationEvaluation".to_string(),
        id: Some(eval_id),
        status: "completed".to_string(),
        patient: Reference {
            reference: format!("Patient/{}", patient_id),
            display: None,
        },
        target_disease: map_target_disease(target_group),
        dose_status,
        dose_status_reason,
        immunization_event: Reference {
            reference: format!("Immunization/imm-{}", dose_idx),
            display: None,
        },
        dose_number_positive_int: dose_number,
    }
}

pub fn make_immunization_recommendation(
    patient_id: &str,
    exec_date: NaiveDate,
    forecasts: &[VaccineGroupForecast],
) -> ImmunizationRecommendation {
    let rec_items = forecasts.iter()
        .flat_map(|vg| {
            vg.forecasts.iter().map(|fc| {
                let status_code = match fc.status {
                    SeriesStatus::Complete => "complete",
                    SeriesStatus::NotRecommended => "not-recommended",
                    SeriesStatus::ConditionallyRecommended => "recommended",
                    SeriesStatus::NotComplete { .. } => {
                        if fc.status.overdue_date().map(|d| exec_date >= d).unwrap_or(false) {
                            "overdue"
                        } else {
                            "due"
                        }
                    }
                };

                let forecast_status = CodeableConcept::new_simple(
                    "http://terminology.hl7.org/CodeSystem/immunization-recommendation-status",
                    status_code,
                    status_code
                );

                let mut criteria = Vec::new();
                
                // Earliest: LOINC 30980-7
                if let Some(date) = fc.status.earliest_date() {
                    criteria.push(DateCriterion {
                        code: CodeableConcept::new_simple("http://loinc.org", "30980-7", "Earliest date"),
                        value: date.format("%Y-%m-%d").to_string(),
                    });
                }
                // Recommended: LOINC 30981-5
                if let Some(date) = fc.status.recommended_date() {
                    criteria.push(DateCriterion {
                        code: CodeableConcept::new_simple("http://loinc.org", "30981-5", "Recommended date"),
                        value: date.format("%Y-%m-%d").to_string(),
                    });
                }
                // Overdue: LOINC 30982-3
                if let Some(date) = fc.status.overdue_date() {
                    criteria.push(DateCriterion {
                        code: CodeableConcept::new_simple("http://loinc.org", "30982-3", "Past due date"),
                        value: date.format("%Y-%m-%d").to_string(),
                    });
                }
                // Latest: LOINC 30983-1
                if let Some(date) = fc.status.latest_date() {
                    criteria.push(DateCriterion {
                        code: CodeableConcept::new_simple("http://loinc.org", "30983-1", "Latest date"),
                        value: date.format("%Y-%m-%d").to_string(),
                    });
                }

                let date_criterion = if criteria.is_empty() { None } else { Some(criteria) };

                let reasons_text = if fc.reasons.is_empty() {
                    None
                } else {
                    Some(fc.reasons.join(", "))
                };

                RecommendationItem {
                    vaccine_code: None, // Can be mapped to specific vaccine group code if needed
                    target_disease: Some(map_target_disease(&vg.vaccine_group)),
                    forecast_status,
                    date_criterion,
                    description: reasons_text,
                    supporting_immunization: None,
                }
            })
        })
        .collect();

    ImmunizationRecommendation {
        resource_type: "ImmunizationRecommendation".to_string(),
        id: Some(format!("rec-{}", patient_id)),
        patient: Reference {
            reference: format!("Patient/{}", patient_id),
            display: None,
        },
        date: exec_date.format("%Y-%m-%d").to_string(),
        recommendation: rec_items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::models::SeriesForecast;

    #[test]
    fn test_patient_conversion() {
        let p_json = json!({
            "birthDate": "2022-04-15",
            "gender": "female"
        });
        let p: Patient = serde_json::from_value(p_json).unwrap();
        let internal_p = InternalPatient::try_from(p).unwrap();
        assert_eq!(internal_p.birth_date, NaiveDate::from_ymd_opt(2022, 4, 15).unwrap());
        assert_eq!(internal_p.gender, Gender::Female);
    }

    #[test]
    fn test_immunization_conversion() {
        let imm_json = json!({
            "status": "completed",
            "vaccineCode": {
                "coding": [
                    {
                        "system": "http://hl7.org/fhir/sid/cvx",
                        "code": "03",
                        "display": "MMR"
                    }
                ]
            },
            "occurrenceDateTime": "2023-05-20T00:00:00Z"
        });
        let imm: Immunization = serde_json::from_value(imm_json).unwrap();
        let dose = Dose::try_from(imm).unwrap();
        assert_eq!(dose.date, NaiveDate::from_ymd_opt(2023, 5, 20).unwrap());
        assert_eq!(dose.cvx.0, 3);
    }

    #[test]
    fn test_make_recommendation() {
        let forecasts = vec![
            VaccineGroupForecast {
                vaccine_group: std::borrow::Cow::Borrowed("Polio"),
                evaluations: crate::date_utils::TinyVec::new(),
                forecasts: {
                    let mut v = crate::date_utils::TinyVec::new();
                    v.push(SeriesForecast {
                        series_name: std::borrow::Cow::Borrowed("IPVs"),
                        status: SeriesStatus::NotComplete {
                            earliest_date: Some(NaiveDate::from_ymd_opt(2022, 6, 1).unwrap()),
                            recommended_date: Some(NaiveDate::from_ymd_opt(2022, 6, 15).unwrap()),
                            overdue_date: Some(NaiveDate::from_ymd_opt(2022, 7, 1).unwrap()),
                            latest_date: None,
                        },
                        reasons: crate::date_utils::TinyVec::new(),
                    });
                    v
                },
                selected_series: None,
            }
        ];

        let rec = make_immunization_recommendation("test-pat", NaiveDate::from_ymd_opt(2022, 6, 20).unwrap(), &forecasts);
        assert_eq!(rec.resource_type, "ImmunizationRecommendation");
        assert_eq!(rec.patient.reference, "Patient/test-pat");
        assert_eq!(rec.recommendation.len(), 1);
        
        let item = &rec.recommendation[0];
        assert_eq!(item.forecast_status.coding.as_ref().unwrap()[0].code.as_deref(), Some("due"));
        
        let criteria = item.date_criterion.as_ref().unwrap();
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0].value, "2022-06-01");
    }
}
