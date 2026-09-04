use std::collections::VecDeque;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use simple_lector_dni::app::{
    AppError, Progress, SessionMode, SessionOptions, describe, run_session,
};
use simple_lector_dni::engine::{DniEngine, EngineFailure, EngineRead};
use simple_lector_dni::engine_protocol::{DocumentData, IntegrityResult};
use simple_lector_dni::model::ReadRecord;
use simple_lector_dni::output::{OutputError, Sink};
use simple_lector_dni::reader::{
    ReaderError, ReaderEvent, ReaderInfo, ReaderMonitor, ReaderPresence,
};

const READER: &str = "Generic EMV Smartcard Reader";

fn reader(presence: ReaderPresence) -> ReaderInfo {
    ReaderInfo {
        index: 0,
        name: READER.to_owned(),
        presence,
        event_count: 0,
    }
}

fn options() -> SessionOptions {
    SessionOptions {
        attempts: 3,
        retry_delay: Duration::ZERO,
        poll_delay: Duration::ZERO,
        recovery_delay: Duration::ZERO,
    }
}

/// Plays back scripted PC/SC ticks; an exhausted script fails loudly so a test can
/// never spin forever.
struct ScriptedMonitor {
    initial: Vec<ReaderInfo>,
    ticks: VecDeque<Result<Vec<ReaderEvent>, ReaderError>>,
    recoveries: VecDeque<Result<Vec<ReaderInfo>, ReaderError>>,
}

impl ScriptedMonitor {
    fn new(initial: Vec<ReaderInfo>, ticks: Vec<Result<Vec<ReaderEvent>, ReaderError>>) -> Self {
        Self {
            initial,
            ticks: ticks.into(),
            recoveries: VecDeque::new(),
        }
    }

    fn recovering_with(mut self, readers: Vec<ReaderInfo>) -> Self {
        self.recoveries.push_back(Ok(readers));
        self
    }
}

impl ReaderMonitor for ScriptedMonitor {
    fn initialise(&mut self) -> Result<Vec<ReaderInfo>, ReaderError> {
        Ok(self.initial.clone())
    }

    fn wait_for_events(&mut self, _: Duration) -> Result<Vec<ReaderEvent>, ReaderError> {
        self.ticks
            .pop_front()
            .expect("script exhausted: the session did not stop")
    }

    fn recover(&mut self) -> Result<Vec<ReaderInfo>, ReaderError> {
        self.recoveries
            .pop_front()
            .unwrap_or(Err(ReaderError::NoReaders))
    }
}

#[derive(Debug, Default)]
struct CountingEngine {
    reads: AtomicUsize,
    failures_before_success: usize,
}

impl DniEngine for CountingEngine {
    fn read(&self, _: &ReaderInfo) -> Result<EngineRead, EngineFailure> {
        let attempt = self.reads.fetch_add(1, Ordering::SeqCst);
        if attempt < self.failures_before_success {
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
struct CountingSink(AtomicUsize);

impl Sink for CountingSink {
    fn name(&self) -> &'static str {
        "counter"
    }

    fn deliver(&self, _: &ReadRecord) -> Result<(), OutputError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Debug)]
struct FailingSink;

impl Sink for FailingSink {
    fn name(&self) -> &'static str {
        "webhook"
    }

    fn deliver(&self, _: &ReadRecord) -> Result<(), OutputError> {
        Err(OutputError::Delivery(
            "el webhook devolvió HTTP 503".to_owned(),
        ))
    }
}

fn collect(events: &mut Vec<Progress>) -> impl FnMut(Progress) -> ControlFlow<()> + '_ {
    move |progress| {
        if progress != Progress::Idle {
            events.push(progress);
        }
        ControlFlow::Continue(())
    }
}

#[test]
fn once_waits_for_reader_and_card_then_reads_and_stops() {
    let mut monitor = ScriptedMonitor::new(
        vec![],
        vec![
            Ok(vec![]),
            Ok(vec![ReaderEvent::ReaderAttached(reader(
                ReaderPresence::Empty,
            ))]),
            Ok(vec![ReaderEvent::CardInserted(reader(
                ReaderPresence::Present,
            ))]),
        ],
    );
    let engine = CountingEngine::default();
    let sink = CountingSink::default();
    let mut events = Vec::new();

    run_session(
        &mut monitor,
        None,
        &engine,
        &[&sink],
        &options(),
        SessionMode::Once { timeout: None },
        &mut collect(&mut events),
    )
    .unwrap();

    assert_eq!(sink.0.load(Ordering::SeqCst), 1);
    assert_eq!(events[0], Progress::WaitingForReader);
    assert_eq!(
        events[1],
        Progress::WaitingForCard {
            reader: READER.to_owned()
        }
    );
    assert!(matches!(
        events[2],
        Progress::Reading {
            attempt: 1,
            attempts: 3,
            ..
        }
    ));
    assert!(matches!(
        events[3],
        Progress::Delivered { delivered: 1, .. }
    ));
    assert_eq!(
        events[4],
        Progress::WaitingForRemoval {
            reader: READER.to_owned()
        }
    );
}

#[test]
fn once_times_out_without_a_card() {
    let mut monitor = ScriptedMonitor::new(
        vec![reader(ReaderPresence::Empty)],
        vec![Ok(vec![]), Ok(vec![]), Ok(vec![])],
    );
    let engine = CountingEngine::default();
    let sink = CountingSink::default();

    let error = run_session(
        &mut monitor,
        None,
        &engine,
        &[&sink],
        &options(),
        SessionMode::Once {
            timeout: Some(Duration::ZERO),
        },
        &mut |_| ControlFlow::Continue(()),
    )
    .unwrap_err();

    assert!(matches!(error, AppError::Timeout(0)));
    assert_eq!(sink.0.load(Ordering::SeqCst), 0);
}

#[test]
fn once_reports_every_failed_output_by_name_and_exits_with_error() {
    let mut monitor = ScriptedMonitor::new(vec![reader(ReaderPresence::Present)], vec![]);
    let engine = CountingEngine::default();
    let counter = CountingSink::default();
    let mut events = Vec::new();

    let error = run_session(
        &mut monitor,
        None,
        &engine,
        &[&FailingSink, &counter],
        &options(),
        SessionMode::Once { timeout: None },
        &mut collect(&mut events),
    )
    .unwrap_err();

    assert_eq!(counter.0.load(Ordering::SeqCst), 1);
    assert!(error.to_string().contains("1 salida(s)"), "{error}");
    let failed = events
        .iter()
        .find(|event| matches!(event, Progress::OutputFailed { .. }))
        .unwrap();
    assert_eq!(
        failed,
        &Progress::OutputFailed {
            sink: "webhook",
            message: "entrega fallida: el webhook devolvió HTTP 503".to_owned()
        }
    );
    assert!(!describe(failed).contains("ANA"));
}

#[test]
fn watch_reads_once_per_insertion_and_keeps_going_after_failures() {
    let present = reader(ReaderPresence::Present);
    let empty = reader(ReaderPresence::Empty);
    let mut monitor = ScriptedMonitor::new(
        vec![present.clone()],
        vec![
            Ok(vec![ReaderEvent::CardInserted(present.clone())]),
            Ok(vec![ReaderEvent::CardRemoved(empty.clone())]),
            Ok(vec![ReaderEvent::CardInserted(present.clone())]),
            Ok(vec![ReaderEvent::CardRemoved(empty)]),
            Ok(vec![ReaderEvent::CardInserted(present)]),
        ],
    );
    let engine = CountingEngine {
        reads: AtomicUsize::new(0),
        failures_before_success: 3,
    };
    let sink = CountingSink::default();
    let mut events = Vec::new();
    let mut deliveries = 0;

    run_session(
        &mut monitor,
        None,
        &engine,
        &[&sink],
        &options(),
        SessionMode::Watch,
        &mut |progress| {
            if matches!(progress, Progress::Delivered { .. }) {
                deliveries += 1;
            }
            events.push(progress);
            if deliveries == 2 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .unwrap();

    assert_eq!(sink.0.load(Ordering::SeqCst), 2);
    assert_eq!(engine.reads.load(Ordering::SeqCst), 5);
    assert!(events.contains(&Progress::ReadFailed {
        attempts: 3,
        code: "TRANSIENT".to_owned()
    }));
}

#[test]
fn watch_survives_a_pcsc_service_failure_and_reads_the_card_found_after_recovery() {
    let present = reader(ReaderPresence::Present);
    let mut monitor = ScriptedMonitor::new(
        vec![reader(ReaderPresence::Empty)],
        vec![Err(ReaderError::Pcsc(pcsc::Error::ServiceStopped))],
    )
    .recovering_with(vec![present]);
    let engine = CountingEngine::default();
    let sink = CountingSink::default();
    let mut events = Vec::new();

    run_session(
        &mut monitor,
        None,
        &engine,
        &[&sink],
        &options(),
        SessionMode::Watch,
        &mut |progress| {
            let done = matches!(progress, Progress::Delivered { .. });
            events.push(progress);
            if done {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .unwrap();

    assert_eq!(sink.0.load(Ordering::SeqCst), 1);
    assert!(events.iter().any(
        |event| matches!(event, Progress::MonitorFailed { message } if message.contains("PC/SC"))
    ));
    assert!(events.contains(&Progress::MonitorRecovered));
}

#[test]
fn an_idle_tick_lets_the_caller_stop_a_watch_session() {
    let mut monitor = ScriptedMonitor::new(
        vec![reader(ReaderPresence::Empty)],
        vec![Ok(vec![]), Ok(vec![]), Ok(vec![])],
    );
    let engine = CountingEngine::default();
    let sink = CountingSink::default();
    let mut idle_ticks = 0;

    run_session(
        &mut monitor,
        None,
        &engine,
        &[&sink],
        &options(),
        SessionMode::Watch,
        &mut |progress| {
            if progress == Progress::Idle {
                idle_ticks += 1;
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        },
    )
    .unwrap();

    assert_eq!(idle_ticks, 1);
    assert_eq!(sink.0.load(Ordering::SeqCst), 0);
}

#[test]
fn progress_serialises_with_a_kind_tag_for_windows_and_sockets() {
    let json = serde_json::to_value(Progress::Reading {
        reader: READER.to_owned(),
        attempt: 2,
        attempts: 3,
    })
    .unwrap();

    assert_eq!(json["kind"], "reading");
    assert_eq!(json["attempt"], 2);
    assert_eq!(
        serde_json::to_value(Progress::Idle).unwrap()["kind"],
        "idle"
    );
}

#[test]
fn progress_wording_is_spanish_and_never_carries_document_fields() {
    let text = describe(&Progress::Reading {
        reader: READER.to_owned(),
        attempt: 2,
        attempts: 3,
    });

    assert_eq!(
        text,
        "Leyendo el DNIe en «Generic EMV Smartcard Reader» (intento 2 de 3)..."
    );
}
