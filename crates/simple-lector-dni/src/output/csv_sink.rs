use std::path::PathBuf;

use super::{OutputError, Sink, open_private_append};
use crate::model::ReadRecord;

const HEADERS: [&str; 28] = [
    "schema_version",
    "read_id",
    "read_at",
    "reader",
    "source",
    "integrity_sod_signature",
    "nombre",
    "primer_apellido",
    "segundo_apellido",
    "apellidos",
    "dni",
    "dni_formateado",
    "fecha_nacimiento",
    "nacionalidad",
    "fecha_caducidad",
    "numero_soporte",
    "sexo",
    "ciudad_nacimiento",
    "provincia_nacimiento",
    "pais_nacimiento",
    "nombres_progenitores",
    "direccion",
    "localidad",
    "provincia",
    "pais",
    "version_dnie",
    "serial_chip",
    "integrity_dg13_hash",
];

#[derive(Debug)]
pub struct CsvSink {
    path: PathBuf,
}

impl CsvSink {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Sink for CsvSink {
    fn name(&self) -> &'static str {
        "csv"
    }

    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError> {
        let needs_header = self
            .path
            .metadata()
            .map_or(true, |metadata| metadata.len() == 0);
        let file = open_private_append(&self.path)?;
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);
        if needs_header {
            writer.write_record(HEADERS)?;
        }
        writer.write_record(csv_row(record))?;
        writer.flush()?;
        Ok(())
    }
}

fn csv_row(record: &ReadRecord) -> Vec<String> {
    let document = &record.document;
    [
        record.schema_version.to_string(),
        record.read_id.to_string(),
        record.read_at.to_rfc3339(),
        protect_csv(&record.reader),
        record.source.clone(),
        record.integrity.sod_signature.clone(),
        protect_csv(&document.nombre),
        protect_csv(&document.primer_apellido),
        protect_csv(&document.segundo_apellido),
        protect_csv(&document.apellidos),
        protect_csv(&document.dni),
        protect_csv(&document.dni_formateado),
        protect_csv(&document.fecha_nacimiento),
        protect_csv(&document.nacionalidad),
        protect_csv(&document.fecha_caducidad),
        protect_csv(&document.numero_soporte),
        protect_csv(&document.sexo),
        protect_csv(&document.ciudad_nacimiento),
        protect_csv(&document.provincia_nacimiento),
        protect_csv(&document.pais_nacimiento),
        protect_csv(&document.nombres_progenitores),
        protect_csv(&document.direccion),
        protect_csv(&document.localidad),
        protect_csv(&document.provincia),
        protect_csv(&document.pais),
        protect_csv(&document.version_dnie),
        protect_csv(&document.serial_chip),
        record.integrity.dg13_hash.clone(),
    ]
    .into_iter()
    .collect()
}

fn protect_csv(value: &str) -> String {
    match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{value}"),
        _ => value.to_owned(),
    }
}
