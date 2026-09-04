use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use clap::Parser;
use simple_lector_dni::app::execute_read_cycle;
use simple_lector_dni::cli::{Cli, Command};
use simple_lector_dni::engine::{DniEngine, EngineFailure, EngineRead};
use simple_lector_dni::engine_protocol::{DocumentData, IntegrityResult};
use simple_lector_dni::model::ReadRecord;
use simple_lector_dni::output::{OutputError, Sink};
use simple_lector_dni::reader::{ReaderInfo, ReaderPresence};

#[test]
fn parses_once_with_combined_outputs() {
    let cli = Cli::try_parse_from([
        "simple-lector-dni",
        "once",
        "--reader",
        "EMV",
        "--json",
        "latest.json",
        "--jsonl",
        "history.jsonl",
        "--csv",
        "history.csv",
        "--webhook",
        "https://example.test/dni",
    ])
    .unwrap();

    let Command::Once(options) = cli.command else {
        panic!("expected once command");
    };
    assert_eq!(options.reader.as_deref(), Some("EMV"));
    assert_eq!(options.outputs.json, Some(PathBuf::from("latest.json")));
    assert_eq!(options.outputs.csv, Some(PathBuf::from("history.csv")));
    assert_eq!(
        options.outputs.webhook.as_deref(),
        Some("https://example.test/dni")
    );
}

#[test]
fn parses_watch_and_list_readers_commands() {
    assert!(matches!(
        Cli::try_parse_from(["simple-lector-dni", "watch"])
            .unwrap()
            .command,
        Command::Watch(_)
    ));
    assert!(matches!(
        Cli::try_parse_from(["simple-lector-dni", "list-readers"])
            .unwrap()
            .command,
        Command::ListReaders
    ));
}

#[test]
fn read_cycle_retries_then_delivers_once() {
    let engine = SucceedsOnThird::default();
    let sink = CountsDeliveries::default();
    let reader = ReaderInfo {
        index: 0,
        name: "Synthetic reader".to_owned(),
        presence: ReaderPresence::Present,
        event_count: 0,
    };

    let record = execute_read_cycle(&engine, &reader, &[&sink], 3, Duration::ZERO).unwrap();

    assert_eq!(engine.attempts.load(Ordering::SeqCst), 3);
    assert_eq!(sink.deliveries.load(Ordering::SeqCst), 1);
    assert_eq!(record.document.nombre, "ANA");
}

#[derive(Debug, Default)]
struct SucceedsOnThird {
    attempts: AtomicUsize,
}

impl DniEngine for SucceedsOnThird {
    fn read(&self, _: &ReaderInfo) -> Result<EngineRead, EngineFailure> {
        if self.attempts.fetch_add(1, Ordering::SeqCst) < 2 {
            return Err(EngineFailure::new("TRANSIENT", true));
        }
        Ok(EngineRead {
            document: DocumentData {
                nombre: "ANA".to_owned(),
                ..DocumentData::default()
            },
            integrity: IntegrityResult {
                sod_signature: "verified".to_owned(),
                dg13_hash: "verified".to_owned(),
            },
        })
    }
}

#[derive(Debug, Default)]
struct CountsDeliveries {
    deliveries: AtomicUsize,
}

impl Sink for CountsDeliveries {
    fn name(&self) -> &'static str {
        "counter"
    }

    fn deliver(&self, _: &ReadRecord) -> Result<(), OutputError> {
        self.deliveries.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
