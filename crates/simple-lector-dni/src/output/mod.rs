mod csv_sink;
mod files;
mod webhook;

use std::fmt::Debug;

use thiserror::Error;

use crate::model::ReadRecord;

pub use csv_sink::CsvSink;
pub use files::{JsonFileSink, JsonLinesSink, StdoutSink};
pub use webhook::WebhookSink;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CSV serialization failed: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid output configuration: {0}")]
    Configuration(String),
    #[error("delivery failed: {0}")]
    Delivery(String),
}

pub trait Sink: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError>;
}

#[derive(Debug, Eq, PartialEq)]
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

pub(crate) fn open_private_append(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    set_private_creation_mode(&mut options);
    let file = options.open(path)?;
    set_private_permissions(&file)?;
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
