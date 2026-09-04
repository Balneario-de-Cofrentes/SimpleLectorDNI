use std::ffi::OsString;
use std::fmt;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::engine_protocol::{
    DocumentData, ENGINE_PROTOCOL_VERSION, EngineRequest, EngineResponse, IntegrityResult,
};
use crate::reader::ReaderInfo;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineRead {
    pub document: DocumentData,
    pub integrity: IntegrityResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineFailure {
    pub code: String,
    pub retryable: bool,
}

impl EngineFailure {
    #[must_use]
    pub fn new(code: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: safe_code(code.into()),
            retryable,
        }
    }
}

impl fmt::Display for EngineFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "engine error ({})", self.code)
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
        Ok(Self::new(java, vec!["-jar".into(), jar.into()], timeout))
    }

    fn execute(&self, request: &EngineRequest) -> Result<String, EngineFailure> {
        let mut child = self.spawn()?;
        write_request(&mut child, request)?;
        let status = wait_for_child(&mut child, self.timeout)?;
        if !status.success() {
            return Err(EngineFailure::new("ENGINE_EXIT", true));
        }
        read_stdout(&mut child)
    }

    fn spawn(&self) -> Result<std::process::Child, EngineFailure> {
        Command::new(&self.program)
            .args(&self.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| EngineFailure::new("ENGINE_START_FAILED", true))
    }
}

impl DniEngine for ProcessEngine {
    fn read(&self, reader: &ReaderInfo) -> Result<EngineRead, EngineFailure> {
        let response = self.execute(&EngineRequest::read(&reader.name))?;
        parse_response(&response)
    }
}

fn write_request(
    child: &mut std::process::Child,
    request: &EngineRequest,
) -> Result<(), EngineFailure> {
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

fn wait_for_child(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, EngineFailure> {
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

fn read_stdout(child: &mut std::process::Child) -> Result<String, EngineFailure> {
    let mut output = String::new();
    child
        .stdout
        .take()
        .ok_or_else(|| EngineFailure::new("ENGINE_STDOUT", true))?
        .read_to_string(&mut output)
        .map_err(|_| EngineFailure::new("ENGINE_STDOUT", true))?;
    Ok(output)
}

fn parse_response(value: &str) -> Result<EngineRead, EngineFailure> {
    let response: EngineResponse = serde_json::from_str(value)
        .map_err(|_| EngineFailure::new("INVALID_ENGINE_RESPONSE", true))?;
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
        _ => Err(EngineFailure::new("INVALID_ENGINE_RESPONSE", true)),
    }
}

fn safe_code(value: String) -> String {
    if value.len() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_uppercase() || character == '_')
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
