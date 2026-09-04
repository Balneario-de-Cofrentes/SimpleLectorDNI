use serde::{Deserialize, Serialize};

pub const ENGINE_PROTOCOL_VERSION: u8 = 1;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrityResult {
    pub sod_signature: String,
    pub dg13_hash: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Unverified,
}

impl VerificationStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unverified => "unverified",
        }
    }
}

#[derive(Deserialize)]
struct IntegrityResultWire {
    sod_signature: VerificationStatus,
    dg13_hash: VerificationStatus,
}

impl<'de> Deserialize<'de> for IntegrityResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = IntegrityResultWire::deserialize(deserializer)?;
        Ok(Self {
            sod_signature: wire.sod_signature.as_str().to_owned(),
            dg13_hash: wire.dg13_hash.as_str().to_owned(),
        })
    }
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
