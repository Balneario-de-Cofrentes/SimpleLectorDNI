use simple_lector_dni::engine_protocol::{
    DocumentData, EngineRequest, EngineResponse, VerificationStatus,
};
use simple_lector_dni::output::CSV_HEADERS;

const SUCCESS_RESPONSE: &str = include_str!("../../../protocol/examples/success.json");
const PROTOCOL_SCHEMA: &str = include_str!("../../../protocol/engine-v1.schema.json");

#[test]
fn read_request_identifies_the_reader_by_stable_name() {
    let request = EngineRequest::read("Synthetic reader");
    let value = serde_json::to_value(request).expect("request serializes");

    assert_eq!(value["reader_name"], "Synthetic reader");
    assert!(value.get("reader_index").is_none());
}

#[test]
fn protocol_contract() {
    let response: EngineResponse = serde_json::from_str(SUCCESS_RESPONSE).expect("valid response");
    let EngineResponse::Ok {
        protocol,
        document,
        integrity,
    } = response
    else {
        panic!("expected an ok response");
    };

    assert_eq!(protocol, 1);
    assert_eq!(document.nombre, "ANA");
    assert_eq!(document.dni, "00000000T");
    assert_eq!(integrity.dg13_hash, VerificationStatus::Verified);

    let empty = DocumentData::default();
    assert!(empty.direccion.is_empty());
}

#[test]
fn protocol_rejects_unknown_integrity_status_and_unknown_fields() {
    let unknown_status = SUCCESS_RESPONSE.replacen("verified", "unverified", 1);
    let unknown_field = SUCCESS_RESPONSE.replacen("\"nombre\"", "\"foto\": \"x\", \"nombre\"", 1);

    assert!(serde_json::from_str::<EngineResponse>(&unknown_status).is_err());
    assert!(serde_json::from_str::<EngineResponse>(&unknown_field).is_err());
}

#[test]
fn csv_columns_follow_the_schema_and_the_integration_guide() {
    let schema: serde_json::Value = serde_json::from_str(PROTOCOL_SCHEMA).unwrap();
    let mut document_columns = sorted_strings(&schema["$defs"]["document"]["required"]);
    let mut csv_document_columns: Vec<String> =
        CSV_HEADERS[6..27].iter().map(|c| (*c).to_owned()).collect();
    document_columns.sort();
    csv_document_columns.sort();
    assert_eq!(csv_document_columns, document_columns);

    let guide = include_str!("../../../docs/INTEGRATION.md");
    assert!(
        guide.contains(&CSV_HEADERS.join(",")),
        "INTEGRATION.md must list the CSV header line"
    );
}

#[test]
fn document_fields_match_schema_and_shared_fixture() {
    let schema: serde_json::Value = serde_json::from_str(PROTOCOL_SCHEMA).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(SUCCESS_RESPONSE).unwrap();
    let rust_model = serde_json::to_value(DocumentData::default()).unwrap();

    let document = &schema["$defs"]["document"];
    assert_object_contract(&fixture["document"], document);
    assert_string_properties(&fixture["document"], document);

    let expected = sorted_strings(&document["required"]);
    assert_eq!(sorted_keys(&fixture["document"]), expected);
    assert_eq!(sorted_keys(&rust_model), expected);
}

#[test]
fn shared_fixture_matches_success_and_integrity_schema() {
    let schema: serde_json::Value = serde_json::from_str(PROTOCOL_SCHEMA).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(SUCCESS_RESPONSE).unwrap();
    let success = &schema["$defs"]["success"];
    let integrity = &schema["$defs"]["integrity"];

    assert_object_contract(&fixture, success);
    assert_eq!(
        fixture["protocol"],
        success["properties"]["protocol"]["const"]
    );
    assert_eq!(fixture["status"], success["properties"]["status"]["const"]);
    assert_object_contract(&fixture["integrity"], integrity);
    assert_enum_properties(&fixture["integrity"], integrity);
}

fn assert_object_contract(value: &serde_json::Value, definition: &serde_json::Value) {
    let required = sorted_strings(&definition["required"]);
    assert_eq!(sorted_keys(value), required);
    assert_eq!(sorted_keys(&definition["properties"]), required);
    assert_eq!(definition["additionalProperties"], false);
}

fn assert_string_properties(value: &serde_json::Value, definition: &serde_json::Value) {
    for (name, property) in definition["properties"].as_object().unwrap() {
        assert_eq!(property["type"], "string", "schema type for {name}");
        assert!(value[name].is_string(), "fixture type for {name}");
    }
}

fn assert_enum_properties(value: &serde_json::Value, definition: &serde_json::Value) {
    for (name, property) in definition["properties"].as_object().unwrap() {
        let allowed = property["enum"].as_array().unwrap();
        assert!(allowed.contains(&value[name]), "fixture enum for {name}");
    }
}

fn sorted_strings(value: &serde_json::Value) -> Vec<String> {
    let mut values = value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    values.sort();
    values
}

fn sorted_keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys = value
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}
