#!/usr/bin/env python3
import argparse
import base64
import json
from datetime import datetime, timezone

def generate_xml(dob, gender, doses):
    sae_templates = []
    for idx, dose in enumerate(doses):
        dt, cvx = dose.split(":")
        sae_templates.append(f"""                    <substanceAdministrationEvent>
                        <templateId root="2.16.840.1.113883.3.795.11.9.1.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.10" extension="{1000 + idx}"/>
                        <substanceAdministrationGeneralPurpose code="384810002" codeSystem="2.16.840.1.113883.6.5"/>
                        <substance>
                            <id root="ab0c489e-782a-4c34-9e4e-9094cc2952d7"/>
                            <substanceCode code="{cvx}" displayName="Vaccine" codeSystem="2.16.840.1.113883.12.292"/>
                        </substance>
                        <administrationTimeInterval high="{dt}" low="{dt}"/>
                    </substanceAdministrationEvent>""")
    
    sae_str = "\n".join(sae_templates)
    
    xml = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
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
                <birthTime value="{dob}"/>
                <gender code="{gender}" codeSystem="2.16.840.1.113883.5.1"/>
            </demographics>
            <clinicalStatements>
                <substanceAdministrationEvents>
{sae_str}
                </substanceAdministrationEvents>
            </clinicalStatements>
        </patient>
    </vmrInput>
</ns3:cdsInput>"""
    return xml

def main():
    parser = argparse.ArgumentParser(description="Generate ICE rest test json payload")
    parser.add_argument("--dob", required=True, help="Patient DOB in YYYY-MM-DD")
    parser.add_argument("--gender", default="F", help="F or M")
    parser.add_argument("--doses", default="", help="Comma-separated list of date:cvx pairs (e.g. 2020-03-01:83,2020-09-01:83)")
    parser.add_argument("--eval-date", required=True, help="Evaluation/Execution date in YYYY-MM-DD")
    parser.add_argument("--output", required=True, help="Output JSON path")
    args = parser.parse_args()

    doses_list = [d for d in args.doses.split(",") if d]
    
    xml_content = generate_xml(args.dob, args.gender, doses_list)
    b64_xml = base64.b64encode(xml_content.encode("utf-8")).decode("utf-8")
    
    dt = datetime.strptime(args.eval_date, "%Y-%m-%d").replace(tzinfo=timezone.utc)
    ts_ms = int(dt.timestamp() * 1000)
    
    payload = {
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
    }
    
    with open(args.output, "w") as f:
        json.dump(payload, f, indent=2)
        
    print(f"Generated test payload: {args.output}")

if __name__ == "__main__":
    main()
