use std::ffi::OsString;
use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::cli::WEBHOOK_TOKEN_VARIABLE;
use crate::engine_protocol::{
    DocumentData, ENGINE_PROTOCOL_VERSION, EngineRequest, EngineResponse, IntegrityResult,
};
use crate::reader::ReaderInfo;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineRead {
    pub document: DocumentData,
    pub integrity: IntegrityResult,
}

/// A sanitised failure code plus an optional detail that never carries card data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineFailure {
    pub code: String,
    pub retryable: bool,
    pub detail: Option<String>,
}

impl EngineFailure {
    #[must_use]
    pub fn new(code: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: safe_code(code.into()),
            retryable,
            detail: None,
        }
    }

    #[must_use]
    fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl fmt::Display for EngineFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "error del motor ({})", self.code)?;
        if let Some(detail) = &self.detail {
            write!(formatter, ": {detail}")?;
        }
        Ok(())
    }
}

impl std::error::Error for EngineFailure {}

pub trait DniEngine: fmt::Debug + Send + Sync {
    fn read(&self, reader: &ReaderInfo) -> Result<EngineRead, EngineFailure>;
}

#[derive(Debug)]
pub struct ProcessEngine {
    program: PathBuf,
    arguments: Vec<OsString>,
    timeout: Duration,
}

impl ProcessEngine {
    #[must_use]
    pub fn new(program: PathBuf, arguments: Vec<OsString>, timeout: Duration) -> Self {
        Self {
            program,
            arguments,
            timeout,
        }
    }

    /// Resolves the bundled runtime and worker next to the executable and checks that
    /// both exist, so a broken installation fails before any card is inserted.
    pub fn from_bundle(timeout: Duration) -> Result<Self, EngineFailure> {
        let executable =
            std::env::current_exe().map_err(|_| EngineFailure::new("ENGINE_NOT_FOUND", false))?;
        let root = executable
            .parent()
            .ok_or_else(|| EngineFailure::new("ENGINE_NOT_FOUND", false))?;
        let java = std::env::var_os("SIMPLE_LECTOR_DNI_JAVA")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(java_relative_path()));
        let jar = std::env::var_os("SIMPLE_LECTOR_DNI_ENGINE_JAR")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("engine/simple-lector-dni-engine.jar"));
        require_file(&java)?;
        require_file(&jar)?;
        Ok(Self::new(java, vec!["-jar".into(), jar.into()], timeout))
    }

    fn execute(&self, request: &EngineRequest) -> Result<String, EngineFailure> {
        let mut child = self.spawn()?;
        write_request(&mut child, request)?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineFailure::new("ENGINE_STDOUT", false))?;
        let output = thread::spawn(move || {
            let mut buffer = String::new();
            stdout.read_to_string(&mut buffer).map(|_| buffer)
        });
        let status = wait_for_child(&mut child, self.timeout)?;
        let output = output
            .join()
            .map_err(|_| EngineFailure::new("ENGINE_STDOUT", false))?
            .map_err(|_| EngineFailure::new("ENGINE_STDOUT", false))?;
        if !status.success() {
            return Err(EngineFailure::new(exit_code(status), true));
        }
        Ok(output)
    }

    fn spawn(&self) -> Result<Child, EngineFailure> {
        Command::new(&self.program)
            .args(&self.arguments)
            .env_remove(WEBHOOK_TOKEN_VARIABLE)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                EngineFailure::new("ENGINE_START_FAILED", false)
                    .with_detail(format!("{}: {error}", self.program.display()))
            })
    }
}

impl DniEngine for ProcessEngine {
    fn read(&self, reader: &ReaderInfo) -> Result<EngineRead, EngineFailure> {
        let response = self.execute(&EngineRequest::read(&reader.name))?;
        parse_response(&response)
    }
}

fn require_file(path: &Path) -> Result<(), EngineFailure> {
    if path.is_file() {
        Ok(())
    } else {
        Err(EngineFailure::new("ENGINE_NOT_FOUND", false)
            .with_detail(format!("no existe {}", path.display())))
    }
}

fn write_request(child: &mut Child, request: &EngineRequest) -> Result<(), EngineFailure> {
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| EngineFailure::new("ENGINE_STDIN", true))?;
    serde_json::to_writer(&mut input, request)
        .map_err(|_| EngineFailure::new("ENGINE_STDIN", true))?;
    input
        .write_all(b"\n")
        .map_err(|_| EngineFailure::new("ENGINE_STDIN", true))
}

fn wait_for_child(child: &mut Child, timeout: Duration) -> Result<ExitStatus, EngineFailure> {
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => Ok(status),
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(EngineFailure::new("ENGINE_TIMEOUT", true))
        }
        Err(_) => Err(EngineFailure::new("ENGINE_WAIT_FAILED", true)),
    }
}

fn exit_code(status: ExitStatus) -> String {
    match status.code() {
        Some(code) if code >= 0 => format!("ENGINE_EXIT_{code}"),
        _ => "ENGINE_EXIT".to_owned(),
    }
}

fn parse_response(value: &str) -> Result<EngineRead, EngineFailure> {
    let response: EngineResponse = serde_json::from_str(value)
        .map_err(|_| EngineFailure::new("INVALID_ENGINE_RESPONSE", false))?;
    match response {
        EngineResponse::Ok {
            protocol,
            document,
            integrity,
        } if protocol == ENGINE_PROTOCOL_VERSION => Ok(EngineRead {
            document: *document,
            integrity,
        }),
        EngineResponse::Error { protocol, error } if protocol == ENGINE_PROTOCOL_VERSION => {
            Err(EngineFailure::new(error.code, error.retryable))
        }
        _ => Err(EngineFailure::new("INVALID_ENGINE_RESPONSE", false)),
    }
}

fn safe_code(value: String) -> String {
    if value.len() <= 64
        && value.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
    {
        value
    } else {
        "ENGINE_ERROR".to_owned()
    }
}

#[cfg(windows)]
fn java_relative_path() -> &'static str {
    "runtime/bin/java.exe"
}

#[cfg(not(windows))]
fn java_relative_path() -> &'static str {
    "runtime/bin/java"
}
