use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "simple-lector-dni",
    version,
    about = "Lee datos DG13 de un DNIe mediante un lector PC/SC"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Espera un DNI, lo lee una vez y termina.
    Once(RunOptions),
    /// Vigila inserciones y retiradas continuamente.
    Watch(RunOptions),
    /// Muestra los lectores PC/SC disponibles.
    ListReaders,
}

#[derive(Clone, Debug, Args)]
pub struct RunOptions {
    /// Selecciona un lector mediante una parte de su nombre.
    #[arg(long)]
    pub reader: Option<String>,

    /// Número máximo de intentos por inserción.
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=3))]
    pub attempts: u8,

    /// Pausa entre intentos fallidos.
    #[arg(long, default_value_t = 350)]
    pub retry_delay_ms: u64,

    #[command(flatten)]
    pub outputs: OutputOptions,
}

#[derive(Clone, Debug, Default, Args)]
pub struct OutputOptions {
    /// Emite cada lectura como JSON por la salida estándar.
    #[arg(long)]
    pub stdout: bool,

    /// Reemplaza atómicamente el último registro JSON.
    #[arg(long, value_name = "PATH")]
    pub json: Option<PathBuf>,

    /// Añade cada registro como una línea JSON.
    #[arg(long, value_name = "PATH")]
    pub jsonl: Option<PathBuf>,

    /// Añade una fila por lectura a un CSV.
    #[arg(long, value_name = "PATH")]
    pub csv: Option<PathBuf>,

    /// Envía cada registro con POST JSON.
    #[arg(long, value_name = "HTTPS_URL")]
    pub webhook: Option<String>,

    /// Tiempo máximo de cada webhook.
    #[arg(long, default_value_t = 10)]
    pub webhook_timeout_seconds: u64,
}

impl OutputOptions {
    #[must_use]
    pub fn has_explicit_sink(&self) -> bool {
        self.stdout
            || self.json.is_some()
            || self.jsonl.is_some()
            || self.csv.is_some()
            || self.webhook.is_some()
    }
}
