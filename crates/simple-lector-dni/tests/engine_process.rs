use std::time::Duration;

use simple_lector_dni::cli::WEBHOOK_TOKEN_VARIABLE;
use simple_lector_dni::engine::{DniEngine, ProcessEngine};
use simple_lector_dni::reader::{ReaderInfo, ReaderPresence};

const NORMAL_PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const SUCCESS_RESPONSE: &str = include_str!("../../../protocol/examples/success.json");

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
        FakeBehavior::Stdout(SUCCESS_RESPONSE),
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
    assert!(!error.retryable);
}

#[test]
fn process_engine_reports_nonzero_exit_without_stderr_contents() {
    let error = fake_engine(
        FakeBehavior::StderrAndExit("sensitive DNI 00000000T"),
        NORMAL_PROCESS_TIMEOUT,
    )
    .read(&reader())
    .unwrap_err();

    assert!(error.code.starts_with("ENGINE_EXIT"), "{}", error.code);
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
fn missing_engine_program_fails_once_with_its_path() {
    let engine = ProcessEngine::new(
        "/nonexistent/simple-lector-dni-java".into(),
        vec![],
        NORMAL_PROCESS_TIMEOUT,
    );

    let error = engine.read(&reader()).unwrap_err();

    assert_eq!(error.code, "ENGINE_START_FAILED");
    assert!(!error.retryable);
    assert!(
        error.to_string().contains("simple-lector-dni-java"),
        "{error}"
    );
}

#[test]
fn bundled_layout_places_runtime_and_worker_under_the_root() {
    let (java, jar) = ProcessEngine::bundled_layout(std::path::Path::new("/opt/slector"));

    assert!(java.starts_with("/opt/slector/runtime/bin"));
    assert_eq!(
        jar,
        std::path::PathBuf::from("/opt/slector/engine/simple-lector-dni-engine.jar")
    );
    let error = ProcessEngine::at(java, jar, NORMAL_PROCESS_TIMEOUT).unwrap_err();
    assert_eq!(error.code, "ENGINE_NOT_FOUND");
    assert!(!error.retryable);
}

#[test]
fn nonzero_exit_reports_the_exit_code() {
    let error = fake_engine(FakeBehavior::StderrAndExit("boom"), NORMAL_PROCESS_TIMEOUT)
        .read(&reader())
        .unwrap_err();

    assert_eq!(error.code, "ENGINE_EXIT_7");
}

#[test]
fn engine_process_does_not_inherit_the_webhook_token() {
    // SAFETY: tests in this binary that read the variable run after this write or not at all.
    unsafe { std::env::set_var(WEBHOOK_TOKEN_VARIABLE, "synthetic-secret") };
    let engine = fake_engine(FakeBehavior::ExitIfTokenVisible, NORMAL_PROCESS_TIMEOUT);

    let error = engine.read(&reader()).unwrap_err();

    assert_eq!(
        error.code, "INVALID_ENGINE_RESPONSE",
        "token was visible or output differed"
    );
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
    /// Exits 3 when the token is visible; otherwise prints an empty response.
    ExitIfTokenVisible,
}

#[cfg(unix)]
fn fake_engine(behavior: FakeBehavior, timeout: Duration) -> ProcessEngine {
    let script = match behavior {
        FakeBehavior::Stdout(value) => format!("printf '%s' '{value}'"),
        FakeBehavior::StderrAndExit(value) => format!("printf '%s' '{value}' >&2; exit 7"),
        FakeBehavior::Sleep => "sleep 2".to_owned(),
        FakeBehavior::ExitIfTokenVisible => {
            format!("test -z \"${WEBHOOK_TOKEN_VARIABLE}\" || exit 3; printf ''")
        }
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
        FakeBehavior::ExitIfTokenVisible => {
            format!("if ($env:{WEBHOOK_TOKEN_VARIABLE}) {{ exit 3 }}")
        }
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
