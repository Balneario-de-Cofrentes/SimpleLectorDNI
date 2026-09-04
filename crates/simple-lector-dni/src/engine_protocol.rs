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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntegrityResult {
    pub sod_signature: String,
    pub dg13_hash: String,
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
        document: DocumentData,
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
    pub reader_index: usize,
}

impl EngineRequest {
    #[must_use]
    pub fn read(reader_index: usize) -> Self {
        Self {
            protocol: ENGINE_PROTOCOL_VERSION,
            command: "read".to_owned(),
            reader_index,
        }
    }
}
