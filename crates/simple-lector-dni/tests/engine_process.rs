use std::time::Duration;

use simple_lector_dni::engine::{DniEngine, ProcessEngine};
use simple_lector_dni::reader::{ReaderInfo, ReaderPresence};

const NORMAL_PROCESS_TIMEOUT: Duration = Duration::from_secs(10);

fn reader() -> ReaderInfo {
    ReaderInfo {
        index: 2,
        name: "Synthetic reader".to_owned(),
        presence: ReaderPresence::Present,
        event_count: 0,
    }
}

#[test]
fn process_engine_parses_a_successful_response() {
    let engine = fake_engine(
        FakeBehavior::Stdout(
            "{\"protocol\":1,\"status\":\"ok\",\"document\":{\"nombre\":\"ANA\",\"primer_apellido\":\"\",\"segundo_apellido\":\"\",\"apellidos\":\"\",\"dni\":\"00000000T\",\"dni_formateado\":\"\",\"fecha_nacimiento\":\"\",\"nacionalidad\":\"\",\"fecha_caducidad\":\"\",\"numero_soporte\":\"\",\"sexo\":\"\",\"ciudad_nacimiento\":\"\",\"provincia_nacimiento\":\"\",\"pais_nacimiento\":\"\",\"nombres_progenitores\":\"\",\"direccion\":\"\",\"localidad\":\"\",\"provincia\":\"\",\"pais\":\"\",\"version_dnie\":\"\",\"serial_chip\":\"\"},\"integrity\":{\"sod_signature\":\"verified\",\"dg13_hash\":\"verified\"}}",
        ),
        NORMAL_PROCESS_TIMEOUT,
    );

    let result = engine.read(&reader()).unwrap();
    assert_eq!(result.document.nombre, "ANA");
    assert_eq!(result.document.dni, "00000000T");
}

#[test]
fn process_engine_rejects_invalid_json() {
    let error = fake_engine(FakeBehavior::Stdout("not-json"), NORMAL_PROCESS_TIMEOUT)
        .read(&reader())
        .unwrap_err();

    assert_eq!(error.code, "INVALID_ENGINE_RESPONSE");
    assert!(error.retryable);
}

#[test]
fn process_engine_reports_nonzero_exit_without_stderr_contents() {
    let error = fake_engine(
        FakeBehavior::StderrAndExit("sensitive DNI 00000000T"),
        NORMAL_PROCESS_TIMEOUT,
    )
    .read(&reader())
    .unwrap_err();

    assert_eq!(error.code, "ENGINE_EXIT");
    assert!(!error.to_string().contains("00000000T"));
}

#[test]
fn process_engine_has_a_bounded_timeout() {
    let error = fake_engine(FakeBehavior::Sleep, Duration::from_millis(30))
        .read(&reader())
        .unwrap_err();

    assert_eq!(error.code, "ENGINE_TIMEOUT");
    assert!(error.retryable);
}

#[test]
fn worker_error_messages_are_not_trusted_or_exposed() {
    let engine = fake_engine(
        FakeBehavior::Stdout(
            "{\"protocol\":1,\"status\":\"error\",\"error\":{\"code\":\"CARD_READ_FAILED\",\"message\":\"DNI 00000000T\",\"retryable\":true}}",
        ),
        NORMAL_PROCESS_TIMEOUT,
    );

    let error = engine.read(&reader()).unwrap_err();
    assert_eq!(error.code, "CARD_READ_FAILED");
    assert!(!error.to_string().contains("00000000T"));
}

enum FakeBehavior {
    Stdout(&'static str),
    StderrAndExit(&'static str),
    Sleep,
}

#[cfg(unix)]
fn fake_engine(behavior: FakeBehavior, timeout: Duration) -> ProcessEngine {
    let script = match behavior {
        FakeBehavior::Stdout(value) => format!("printf '%s' '{value}'"),
        FakeBehavior::StderrAndExit(value) => format!("printf '%s' '{value}' >&2; exit 7"),
        FakeBehavior::Sleep => "sleep 2".to_owned(),
    };
    ProcessEngine::new("/bin/sh".into(), vec!["-c".into(), script.into()], timeout)
}

#[cfg(windows)]
fn fake_engine(behavior: FakeBehavior, timeout: Duration) -> ProcessEngine {
    let script = match behavior {
        FakeBehavior::Stdout(value) => {
            format!("[Console]::Out.Write('{}')", value.replace('\'', "''"))
        }
        FakeBehavior::StderrAndExit(value) => format!(
            "[Console]::Error.Write('{}'); exit 7",
            value.replace('\'', "''")
        ),
        FakeBehavior::Sleep => "Start-Sleep -Seconds 2".to_owned(),
    };
    ProcessEngine::new(
        "powershell.exe".into(),
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            script.into(),
        ],
        timeout,
    )
}
