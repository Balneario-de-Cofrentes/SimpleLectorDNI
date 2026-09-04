use simple_lector_dni::engine_protocol::{DocumentData, EngineRequest, EngineResponse};

#[test]
fn read_request_identifies_the_reader_by_stable_name() {
    let request = EngineRequest::read("Synthetic reader");
    let value = serde_json::to_value(request).expect("request serializes");

    assert_eq!(value["reader_name"], "Synthetic reader");
    assert!(value.get("reader_index").is_none());
}

#[test]
fn protocol_contract() {
    let json = r#"{
        "protocol": 1,
        "status": "ok",
        "document": {
            "nombre": "ANA",
            "primer_apellido": "EJEMPLO",
            "segundo_apellido": "PRUEBA",
            "apellidos": "EJEMPLO PRUEBA",
            "dni": "00000000T",
            "dni_formateado": "00000000-T",
            "fecha_nacimiento": "1990-01-01",
            "nacionalidad": "ESP",
            "fecha_caducidad": "2030-01-01",
            "numero_soporte": "AAA000000",
            "sexo": "F",
            "ciudad_nacimiento": "MADRID",
            "provincia_nacimiento": "MADRID",
            "pais_nacimiento": "ESPANA",
            "nombres_progenitores": "PERSONA UNO / PERSONA DOS",
            "direccion": "CALLE DE EJEMPLO 1",
            "localidad": "MADRID",
            "provincia": "MADRID",
            "pais": "ESPANA",
            "version_dnie": "4.0",
            "serial_chip": "01020304"
        },
        "integrity": {
            "sod_signature": "verified",
            "dg13_hash": "verified"
        }
    }"#;

    let response: EngineResponse = serde_json::from_str(json).expect("valid response");
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
    assert_eq!(integrity.dg13_hash, "verified");

    let empty = DocumentData::default();
    assert!(empty.direccion.is_empty());
}
