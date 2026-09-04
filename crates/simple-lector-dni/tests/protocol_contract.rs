use simple_lector_dni::engine_protocol::{
    DocumentData, EngineRequest, EngineResponse, VerificationStatus,
};

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
fn protocol_rejects_unknown_integrity_status() {
    let invalid = SUCCESS_RESPONSE.replacen("verified", "unknown", 1);

    assert!(serde_json::from_str::<EngineResponse>(&invalid).is_err());
}

#[test]
fn document_fields_match_schema_and_shared_fixture() {
    let schema: serde_json::Value = serde_json::from_str(PROTOCOL_SCHEMA).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(SUCCESS_RESPONSE).unwrap();
    let rust_model = serde_json::to_value(DocumentData::default()).unwrap();

    let expected = sorted_strings(&schema["$defs"]["document"]["required"]);
    assert_eq!(sorted_keys(&fixture["document"]), expected);
    assert_eq!(sorted_keys(&rust_model), expected);
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
