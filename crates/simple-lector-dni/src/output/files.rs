use std::io::Write;
use std::path::PathBuf;

use atomic_write_file::AtomicWriteFile;

use super::{OutputError, Sink, open_private_append, restrict_to_owner, set_private_permissions};
use crate::model::ReadRecord;

#[derive(Debug)]
pub struct StdoutSink;

impl Sink for StdoutSink {
    fn name(&self) -> &'static str {
        "stdout"
    }

    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError> {
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer(&mut output, record)?;
        output.write_all(b"\n")?;
        output.flush()?;
        Ok(())
    }
}

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
        restrict_to_owner(&self.path)?;
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
