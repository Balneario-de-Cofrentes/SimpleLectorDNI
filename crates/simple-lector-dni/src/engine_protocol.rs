use serde::{Deserialize, Serialize};

pub const ENGINE_PROTOCOL_VERSION: u8 = 1;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentData {
    pub nombre: String,
    pub primer_apellido: String,
    pub segundo_apellido: String,
    pub apellidos: String,
    pub dni: String,
    pub dni_formateado: String,
    pub fecha_nacimiento: String,
    pub nacionalidad: String,
    pub fecha_caducidad: String,
    pub numero_soporte: String,
    pub sexo: String,
    pub ciudad_nacimiento: String,
    pub provincia_nacimiento: String,
    pub pais_nacimiento: String,
    pub nombres_progenitores: String,
    pub direccion: String,
    pub localidad: String,
    pub provincia: String,
    pub pais: String,
    pub version_dnie: String,
    pub serial_chip: String,
}

/// The worker aborts any read whose SOD signature or DG13 hash fails, so a delivered
/// record can only carry this value. It means the DG13 bytes match the hash signed in
/// the SOD and the SOD signature matches the certificate the SOD itself carries. It
/// does not mean that certificate was validated against the CSCA.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
}

impl VerificationStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "verified"
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntegrityResult {
    pub sod_signature: VerificationStatus,
    pub dg13_hash: VerificationStatus,
}

impl IntegrityResult {
    pub const VERIFIED: Self = Self {
        sod_signature: VerificationStatus::Verified,
        dg13_hash: VerificationStatus::Verified,
    };
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EngineError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EngineResponse {
    Ok {
        protocol: u8,
        document: Box<DocumentData>,
        integrity: IntegrityResult,
    },
    Error {
        protocol: u8,
        error: EngineError,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EngineRequest {
    pub protocol: u8,
    pub command: String,
    pub reader_name: String,
}

impl EngineRequest {
    #[must_use]
    pub fn read(reader_name: impl Into<String>) -> Self {
        Self {
            protocol: ENGINE_PROTOCOL_VERSION,
            command: "read".to_owned(),
            reader_name: reader_name.into(),
        }
    }
}
