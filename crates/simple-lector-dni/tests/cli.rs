use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use clap::Parser;
use simple_lector_dni::app::{Progress, execute_read_cycle};
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
        "--timeout-seconds",
        "30",
    ])
    .unwrap();

    let Command::Once(options) = cli.command else {
        panic!("expected once command");
    };
    assert_eq!(options.run.reader.as_deref(), Some("EMV"));
    assert_eq!(options.run.outputs.json, Some(PathBuf::from("latest.json")));
    assert_eq!(options.run.outputs.csv, Some(PathBuf::from("history.csv")));
    assert_eq!(
        options.run.outputs.webhook.as_deref(),
        Some("https://example.test/dni")
    );
    assert_eq!(options.timeout_seconds, Some(30));
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

    let mut attempts_reported = Vec::new();
    let outcome = execute_read_cycle(
        &engine,
        &reader,
        &[&sink],
        3,
        Duration::ZERO,
        &mut |progress| {
            if let Progress::Reading { attempt, .. } = progress {
                attempts_reported.push(attempt);
            }
            ControlFlow::Continue(())
        },
    )
    .unwrap();

    assert_eq!(engine.attempts.load(Ordering::SeqCst), 3);
    assert_eq!(sink.deliveries.load(Ordering::SeqCst), 1);
    assert_eq!(outcome.record.document.nombre, "ANA");
    assert_eq!(outcome.delivered, 1);
    assert_eq!(attempts_reported, vec![1, 2, 3]);
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
            integrity: IntegrityResult::VERIFIED,
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
