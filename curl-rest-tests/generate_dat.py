import base64
import json

dob = "19990610"
gender_code = "F"
doses = [
    ("19991016", "10"),
    ("20060411", "110"),
    ("20090721", "110"),
    ("20090721", "120"),
    ("20160315", "10")
]

xml_template = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
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
{sae_list}
                </substanceAdministrationEvents>
            </clinicalStatements>
        </patient>
    </vmrInput>
</ns3:cdsInput>"""

sae_template = """                    <substanceAdministrationEvent>
                        <templateId root="2.16.840.1.113883.3.795.11.9.1.1"/>
                        <id root="2.16.840.1.113883.3.795.12.100.10" extension="{ext}"/>
                        <substanceAdministrationGeneralPurpose code="384810002" codeSystem="2.16.840.1.113883.6.5"/>
                        <substance>
                            <id root="ab0c489e-782a-4c34-9e4e-9094cc2952d7"/>
                            <substanceCode code="{cvx}" displayName="Vaccine" codeSystem="2.16.840.1.113883.12.292"/>
                        </substance>
                        <administrationTimeInterval high="{dt}" low="{dt}"/>
                    </substanceAdministrationEvent>"""

sae_elements = []
for idx, (dt, cvx) in enumerate(doses):
    sae_elements.append(sae_template.format(ext=1000 + idx, cvx=cvx, dt=dt))

sae_str = "\n".join(sae_elements)
xml_content = xml_template.format(dob=dob, gender=gender_code, sae_list=sae_str)

b64_xml = base64.b64encode(xml_content.encode('utf-8')).decode('utf-8')

ts_ms = 1458086399000

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

with open("rest-test-json-evalue.dat", "w") as f:
    json.dump(payload, f)
print("rest-test-json-evalue.dat written successfully.")
