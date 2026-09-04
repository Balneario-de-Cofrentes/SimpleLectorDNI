use chrono::DateTime;
use simple_lector_dni::engine_protocol::{DocumentData, IntegrityResult, VerificationStatus};
use simple_lector_dni::model::{READ_SCHEMA_VERSION, ReadRecord};
use uuid::Uuid;

#[test]
fn read_record_has_stable_versioned_envelope() {
    let read_id = Uuid::parse_str("9f142e2c-f4ec-47e5-b8cc-1bbfe49118a7").unwrap();
    let read_at = DateTime::parse_from_rfc3339("2026-09-04T12:34:56+02:00").unwrap();
    let document = DocumentData {
        nombre: "ANA".to_owned(),
        dni: "00000000T".to_owned(),
        ..DocumentData::default()
    };
    let integrity = IntegrityResult {
        sod_signature: VerificationStatus::Verified,
        dg13_hash: VerificationStatus::Verified,
    };

    let record = ReadRecord::new(
        read_id,
        read_at,
        "Generic EMV Smartcard Reader".to_owned(),
        document,
        integrity,
    );
    let json = serde_json::to_value(record).unwrap();

    assert_eq!(json["schema_version"], READ_SCHEMA_VERSION);
    assert_eq!(json["read_id"], read_id.to_string());
    assert_eq!(json["read_at"], "2026-09-04T12:34:56+02:00");
    assert_eq!(json["reader"], "Generic EMV Smartcard Reader");
    assert_eq!(json["source"], "DNIe_DG13");
    assert_eq!(json["document"]["nombre"], "ANA");
    assert_eq!(json["integrity"]["dg13_hash"], "verified");
}

#[test]
fn absent_document_values_stay_empty_for_csv_compatibility() {
    let record = ReadRecord::new(
        Uuid::nil(),
        DateTime::parse_from_rfc3339("2026-09-04T00:00:00Z").unwrap(),
        "reader".to_owned(),
        DocumentData::default(),
        IntegrityResult {
            sod_signature: VerificationStatus::Unverified,
            dg13_hash: VerificationStatus::Unverified,
        },
    );

    assert!(record.document.direccion.is_empty());
    assert!(record.document.segundo_apellido.is_empty());
}
