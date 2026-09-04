use std::time::Duration;

use simple_lector_dni::engine::{DniEngine, ProcessEngine};
use simple_lector_dni::reader::{ReaderInfo, ReaderPresence};

fn reader() -> ReaderInfo {
    ReaderInfo {
        index: 2,
        name: "Synthetic reader".to_owned(),
        presence: ReaderPresence::Present,
    }
}

#[test]
fn process_engine_parses_a_successful_response() {
    let engine = shell_engine(
        "printf '%s' '{\"protocol\":1,\"status\":\"ok\",\"document\":{\"nombre\":\"ANA\",\"primer_apellido\":\"\",\"segundo_apellido\":\"\",\"apellidos\":\"\",\"dni\":\"00000000T\",\"dni_formateado\":\"\",\"fecha_nacimiento\":\"\",\"nacionalidad\":\"\",\"fecha_caducidad\":\"\",\"numero_soporte\":\"\",\"sexo\":\"\",\"ciudad_nacimiento\":\"\",\"provincia_nacimiento\":\"\",\"pais_nacimiento\":\"\",\"nombres_progenitores\":\"\",\"direccion\":\"\",\"localidad\":\"\",\"provincia\":\"\",\"pais\":\"\",\"version_dnie\":\"\",\"serial_chip\":\"\"},\"integrity\":{\"sod_signature\":\"verified\",\"dg13_hash\":\"verified\"}}'",
        Duration::from_secs(1),
    );

    let result = engine.read(&reader()).unwrap();
    assert_eq!(result.document.nombre, "ANA");
    assert_eq!(result.document.dni, "00000000T");
}

#[test]
fn process_engine_rejects_invalid_json() {
    let error = shell_engine("printf '%s' 'not-json'", Duration::from_secs(1))
        .read(&reader())
        .unwrap_err();

    assert_eq!(error.code, "INVALID_ENGINE_RESPONSE");
    assert!(error.retryable);
}

#[test]
fn process_engine_reports_nonzero_exit_without_stderr_contents() {
    let error = shell_engine(
        "printf '%s' 'sensitive DNI 00000000T' >&2; exit 7",
        Duration::from_secs(1),
    )
    .read(&reader())
    .unwrap_err();

    assert_eq!(error.code, "ENGINE_EXIT");
    assert!(!error.to_string().contains("00000000T"));
}

#[test]
fn process_engine_has_a_bounded_timeout() {
    let error = shell_engine("sleep 2", Duration::from_millis(30))
        .read(&reader())
        .unwrap_err();

    assert_eq!(error.code, "ENGINE_TIMEOUT");
    assert!(error.retryable);
}

#[test]
fn worker_error_messages_are_not_trusted_or_exposed() {
    let engine = shell_engine(
        "printf '%s' '{\"protocol\":1,\"status\":\"error\",\"error\":{\"code\":\"CARD_READ_FAILED\",\"message\":\"DNI 00000000T\",\"retryable\":true}}'",
        Duration::from_secs(1),
    );

    let error = engine.read(&reader()).unwrap_err();
    assert_eq!(error.code, "CARD_READ_FAILED");
    assert!(!error.to_string().contains("00000000T"));
}

#[cfg(unix)]
fn shell_engine(script: &str, timeout: Duration) -> ProcessEngine {
    ProcessEngine::new("/bin/sh".into(), vec!["-c".into(), script.into()], timeout)
}

#[cfg(windows)]
fn shell_engine(script: &str, timeout: Duration) -> ProcessEngine {
    ProcessEngine::new("cmd.exe".into(), vec!["/C".into(), script.into()], timeout)
}
