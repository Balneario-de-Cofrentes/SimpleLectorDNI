use std::io::Write;
use std::path::PathBuf;

use atomic_write_file::AtomicWriteFile;

use super::{OutputError, Sink, open_private_append, set_private_permissions};
use crate::model::ReadRecord;

#[derive(Debug)]
pub struct JsonFileSink {
    path: PathBuf,
}

impl JsonFileSink {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Sink for JsonFileSink {
    fn name(&self) -> &'static str {
        "json"
    }

    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError> {
        let mut file = AtomicWriteFile::open(&self.path)?;
        set_private_permissions(file.as_file())?;
        serde_json::to_writer_pretty(&mut file, record)?;
        file.write_all(b"\n")?;
        file.commit()?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct JsonLinesSink {
    path: PathBuf,
}

impl JsonLinesSink {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Sink for JsonLinesSink {
    fn name(&self) -> &'static str {
        "jsonl"
    }

    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError> {
        let mut file = open_private_append(&self.path)?;
        serde_json::to_writer(&mut file, record)?;
        file.write_all(b"\n")?;
        file.flush()?;
        Ok(())
    }
}
