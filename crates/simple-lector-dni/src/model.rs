use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::engine_protocol::{DocumentData, IntegrityResult};

pub const READ_SCHEMA_VERSION: u8 = 1;
pub const DNI_SOURCE: &str = "DNIe_DG13";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReadRecord {
    pub schema_version: u8,
    pub read_id: Uuid,
    pub read_at: DateTime<FixedOffset>,
    pub reader: String,
    pub source: String,
    pub integrity: IntegrityResult,
    pub document: DocumentData,
}

impl ReadRecord {
    #[must_use]
    pub fn new(
        read_id: Uuid,
        read_at: DateTime<FixedOffset>,
        reader: String,
        document: DocumentData,
        integrity: IntegrityResult,
    ) -> Self {
        Self {
            schema_version: READ_SCHEMA_VERSION,
            read_id,
            read_at,
            reader,
            source: DNI_SOURCE.to_owned(),
            integrity,
            document,
        }
    }
}
