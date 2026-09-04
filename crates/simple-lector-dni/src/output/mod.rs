mod csv_sink;
mod files;
mod webhook;

use std::fmt::Debug;
use std::path::Path;

use thiserror::Error;

use crate::model::ReadRecord;

pub use csv_sink::{CSV_HEADERS, CsvSink};
pub use files::{JsonFileSink, JsonLinesSink, StdoutSink};
pub use webhook::WebhookSink;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("fallo al serializar JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("fallo al serializar CSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("configuración de salida inválida: {0}")]
    Configuration(String),
    #[error("entrega fallida: {0}")]
    Delivery(String),
}

pub trait Sink: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError>;
}

/// Which sink failed and why. Messages come from I/O, HTTP or CSV errors and never
/// include document fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryFailure {
    pub sink: &'static str,
    pub message: String,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct DeliveryReport {
    pub delivered: usize,
    pub failures: Vec<DeliveryFailure>,
}

#[must_use]
pub fn deliver_all(sinks: &[&dyn Sink], record: &ReadRecord) -> DeliveryReport {
    let mut report = DeliveryReport::default();
    for sink in sinks {
        match sink.deliver(record) {
            Ok(()) => report.delivered += 1,
            Err(error) => report.failures.push(DeliveryFailure {
                sink: sink.name(),
                message: error.to_string(),
            }),
        }
    }
    report
}

/// Opens an append-only file readable by the current account only.
pub(crate) fn open_private_append(path: &Path) -> std::io::Result<std::fs::File> {
    let created = !path.exists();
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    set_private_creation_mode(&mut options);
    let file = options.open(path)?;
    set_private_permissions(&file)?;
    if created {
        restrict_to_owner(path)?;
    }
    Ok(file)
}

#[cfg(unix)]
fn set_private_creation_mode(options: &mut std::fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

#[cfg(not(unix))]
fn set_private_creation_mode(_: &mut std::fs::OpenOptions) {}

#[cfg(unix)]
pub(crate) fn set_private_permissions(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
pub(crate) fn set_private_permissions(_: &std::fs::File) -> std::io::Result<()> {
    Ok(())
}

/// Windows has no mode bits: replace the inherited DACL with one entry for the
/// current account, the equivalent of 0600.
#[cfg(windows)]
pub(crate) fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
    let user = std::env::var("USERNAME")
        .map_err(|_| std::io::Error::other("USERNAME no está definida"))?;
    let account = match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{user}"),
        _ => user,
    };
    let output = std::process::Command::new("icacls")
        .arg(path)
        .args(["/inheritance:r", "/grant:r"])
        .arg(format!("{account}:F"))
        .arg("/q")
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "icacls devolvió {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(not(windows))]
pub(crate) fn restrict_to_owner(_: &Path) -> std::io::Result<()> {
    Ok(())
}
