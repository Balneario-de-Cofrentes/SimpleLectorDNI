use std::ops::ControlFlow;
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
use thiserror::Error;
use uuid::Uuid;

use crate::cli::{Cli, Command, OnceOptions, OutputOptions, RunOptions, WEBHOOK_TOKEN_VARIABLE};
use crate::engine::{DniEngine, EngineFailure, EngineRead, ProcessEngine};
use crate::lifecycle::{
    CardLifecycle, LifecycleAction, LifecycleEvent, LifecycleState, run_with_retries,
};
use crate::model::ReadRecord;
use crate::output::{
    CsvSink, DeliveryFailure, JsonFileSink, JsonLinesSink, OutputError, Sink, StdoutSink,
    WebhookSink, deliver_all,
};
use crate::reader::{
    PcscMonitor, ReaderError, ReaderEvent, ReaderInfo, ReaderMonitor, ReaderPresence,
    ReaderSelection, SelectionChange,
};

const POLL_DELAY: Duration = Duration::from_millis(250);
const RECOVERY_DELAY: Duration = Duration::from_secs(1);
const ENGINE_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Error)]
pub enum AppError {
    #[error("error del lector: {0}")]
    Reader(#[from] ReaderError),
    #[error("error de salida: {0}")]
    Output(#[from] OutputError),
    #[error("no se pudo iniciar el motor DNIe: {0}")]
    Engine(#[from] EngineFailure),
    #[error(transparent)]
    Cycle(#[from] CycleError),
    #[error("no se insertó ningún DNIe en {0} segundos")]
    Timeout(u64),
}

/// Failure of one insertion cycle: the read itself or one or more outputs.
#[derive(Debug, Error)]
pub enum CycleError {
    #[error("la lectura del DNIe falló tras {attempts} intento(s), código {code}")]
    Read { attempts: u8, code: String },
    #[error("{} salida(s) configurada(s) fallaron", .0.len())]
    Delivery(Vec<DeliveryFailure>),
}

/// What the session is doing, emitted to the CLI (stderr) or to a GUI. Never carries
/// document fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Progress {
    WaitingForReader,
    WaitingForCard {
        reader: String,
    },
    Reading {
        reader: String,
        attempt: u8,
        attempts: u8,
    },
    Delivered {
        read_id: Uuid,
        delivered: usize,
    },
    WaitingForRemoval {
        reader: String,
    },
    ReadFailed {
        attempts: u8,
        code: String,
    },
    OutputFailed {
        sink: &'static str,
        message: String,
    },
    MonitorFailed {
        message: String,
    },
    MonitorRecovered,
}

/// Receives progress; returning `Break` stops the session cleanly.
pub type ProgressSink<'a> = dyn FnMut(Progress) -> ControlFlow<()> + 'a;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionMode {
    Once { timeout: Option<Duration> },
    Watch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionOptions {
    pub attempts: u8,
    pub retry_delay: Duration,
    pub poll_delay: Duration,
    pub recovery_delay: Duration,
}

impl SessionOptions {
    fn from_cli(options: &RunOptions) -> Self {
        Self {
            attempts: options.attempts,
            retry_delay: Duration::from_millis(options.retry_delay_ms),
            poll_delay: POLL_DELAY,
            recovery_delay: RECOVERY_DELAY,
        }
    }
}

#[derive(Debug)]
pub struct ReadOutcome {
    pub record: ReadRecord,
    pub delivered: usize,
}

#[derive(Debug)]
pub struct WatchController {
    selection: ReaderSelection,
    lifecycle: CardLifecycle,
}

impl WatchController {
    pub fn new(readers: &[ReaderInfo], pattern: Option<&str>) -> Result<Self, AppError> {
        let selection = ReaderSelection::new(readers, pattern)?;
        Ok(Self {
            lifecycle: CardLifecycle::new(selection.selected().is_some()),
            selection,
        })
    }

    /// Rebuilds the selection from a fresh snapshot after the PC/SC session was lost.
    pub fn resync(&mut self, readers: &[ReaderInfo]) -> Result<(), AppError> {
        self.selection.resync(readers)?;
        self.lifecycle = CardLifecycle::new(self.selection.selected().is_some());
        Ok(())
    }

    #[must_use]
    pub fn initial_read(&mut self) -> Option<ReaderInfo> {
        let reader = self
            .selection
            .selected()
            .filter(|value| value.presence == ReaderPresence::Present)?
            .clone();
        self.start_read(reader)
    }

    #[must_use]
    pub fn handle(&mut self, event: ReaderEvent) -> Option<ReaderInfo> {
        self.update_reader_lifecycle(&event);
        match event {
            ReaderEvent::CardInserted(reader) => self.card_inserted(reader),
            ReaderEvent::CardRemoved(reader) => self.card_removed(&reader),
            ReaderEvent::ReaderAttached(_) | ReaderEvent::ReaderDetached(_) => None,
        }
    }

    pub fn read_succeeded(&mut self) {
        self.lifecycle.handle(LifecycleEvent::ReadSucceeded);
    }

    pub fn read_failed(&mut self) {
        self.lifecycle.handle(LifecycleEvent::ReadFailed);
    }

    #[must_use]
    pub fn selected_name(&self) -> Option<&str> {
        self.selection.selected_name()
    }

    /// The idle state to show an operator: which reader, and whether a card is expected
    /// or must be removed first.
    #[must_use]
    pub fn status(&self) -> Progress {
        let Some(reader) = self.selection.selected() else {
            return Progress::WaitingForReader;
        };
        let reader = reader.name.clone();
        match self.lifecycle.state() {
            LifecycleState::NoReader | LifecycleState::Empty => Progress::WaitingForCard { reader },
            LifecycleState::Reading | LifecycleState::Delivered | LifecycleState::Failed => {
                Progress::WaitingForRemoval { reader }
            }
        }
    }

    fn update_reader_lifecycle(&mut self, event: &ReaderEvent) {
        match self.selection.update(event) {
            SelectionChange::Selected => {
                self.lifecycle.handle(LifecycleEvent::ReaderAttached);
            }
            SelectionChange::Deselected => {
                self.lifecycle.handle(LifecycleEvent::ReaderDetached);
            }
            SelectionChange::Unchanged => {}
        }
    }

    fn card_inserted(&mut self, reader: ReaderInfo) -> Option<ReaderInfo> {
        if self.selection.is_selected(&reader) {
            return self.start_read(reader);
        }
        None
    }

    fn card_removed(&mut self, reader: &ReaderInfo) -> Option<ReaderInfo> {
        if self.selection.is_selected(reader) {
            self.lifecycle.handle(LifecycleEvent::CardRemoved);
        }
        None
    }

    fn start_read(&mut self, reader: ReaderInfo) -> Option<ReaderInfo> {
        (self.lifecycle.handle(LifecycleEvent::CardInserted) == LifecycleAction::StartRead)
            .then_some(reader)
    }
}

pub fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Command::ListReaders => list_readers(),
        Command::Once(options) => run_once(&options),
        Command::Watch(options) => run_watch(&options),
    }
}

/// One loop for `once` and `watch`: selects the reader, reacts to PC/SC events, runs a
/// read cycle per insertion and survives PC/SC service failures.
pub fn run_session(
    monitor: &mut dyn ReaderMonitor,
    pattern: Option<&str>,
    engine: &dyn DniEngine,
    sinks: &[&dyn Sink],
    options: &SessionOptions,
    mode: SessionMode,
    report: &mut ProgressSink<'_>,
) -> Result<(), AppError> {
    let readers = monitor.initialise()?;
    let mut session = Session {
        controller: WatchController::new(&readers, pattern)?,
        engine,
        sinks,
        options,
        mode,
        report,
        started: Instant::now(),
        last_status: None,
    };
    session.run(monitor)
}

struct Session<'a> {
    controller: WatchController,
    engine: &'a dyn DniEngine,
    sinks: &'a [&'a dyn Sink],
    options: &'a SessionOptions,
    mode: SessionMode,
    report: &'a mut ProgressSink<'a>,
    started: Instant,
    last_status: Option<Progress>,
}

impl Session<'_> {
    fn run(&mut self, monitor: &mut dyn ReaderMonitor) -> Result<(), AppError> {
        if self.announce_status().is_break() {
            return Ok(());
        }
        if let Some(reader) = self.controller.initial_read()
            && self.cycle(&reader)?.is_break()
        {
            return Ok(());
        }
        loop {
            self.check_deadline()?;
            let step = match monitor.wait_for_events(self.options.poll_delay) {
                Ok(events) => self.dispatch(events)?,
                Err(error) => self.recover(monitor, &error)?,
            };
            if step.is_break() {
                return Ok(());
            }
        }
    }

    fn dispatch(&mut self, events: Vec<ReaderEvent>) -> Result<ControlFlow<()>, AppError> {
        for event in events {
            if let Some(reader) = self.controller.handle(event)
                && self.cycle(&reader)?.is_break()
            {
                return Ok(ControlFlow::Break(()));
            }
        }
        Ok(self.announce_status())
    }

    fn recover(
        &mut self,
        monitor: &mut dyn ReaderMonitor,
        error: &ReaderError,
    ) -> Result<ControlFlow<()>, AppError> {
        let mut message = error.to_string();
        loop {
            if self.emit(Progress::MonitorFailed { message }).is_break() {
                return Ok(ControlFlow::Break(()));
            }
            thread::sleep(self.options.recovery_delay);
            self.check_deadline()?;
            match monitor.recover() {
                Ok(readers) => {
                    self.controller.resync(&readers)?;
                    self.last_status = None;
                    if self.emit(Progress::MonitorRecovered).is_break()
                        || self.announce_status().is_break()
                    {
                        return Ok(ControlFlow::Break(()));
                    }
                    return match self.controller.initial_read() {
                        Some(reader) => self.cycle(&reader),
                        None => Ok(ControlFlow::Continue(())),
                    };
                }
                Err(next) => message = next.to_string(),
            }
        }
    }

    fn cycle(&mut self, reader: &ReaderInfo) -> Result<ControlFlow<()>, AppError> {
        let result = execute_read_cycle(
            self.engine,
            reader,
            self.sinks,
            self.options.attempts,
            self.options.retry_delay,
            &mut *self.report,
        );
        let (outcome, flow) = match result {
            Ok(outcome) => {
                self.controller.read_succeeded();
                let flow = self.emit(Progress::Delivered {
                    read_id: outcome.record.read_id,
                    delivered: outcome.delivered,
                });
                (Ok(()), flow)
            }
            Err(error) => {
                self.controller.read_failed();
                let flow = self.report_cycle_failure(&error);
                (Err(error), flow)
            }
        };
        self.last_status = None;
        let flow = if flow.is_break() {
            flow
        } else {
            self.announce_status()
        };
        match self.mode {
            SessionMode::Once { .. } => {
                outcome?;
                Ok(ControlFlow::Break(()))
            }
            SessionMode::Watch => Ok(flow),
        }
    }

    fn report_cycle_failure(&mut self, error: &CycleError) -> ControlFlow<()> {
        match error {
            CycleError::Read { attempts, code } => self.emit(Progress::ReadFailed {
                attempts: *attempts,
                code: code.clone(),
            }),
            CycleError::Delivery(failures) => {
                for failure in failures {
                    let flow = self.emit(Progress::OutputFailed {
                        sink: failure.sink,
                        message: failure.message.clone(),
                    });
                    if flow.is_break() {
                        return flow;
                    }
                }
                ControlFlow::Continue(())
            }
        }
    }

    fn announce_status(&mut self) -> ControlFlow<()> {
        let status = self.controller.status();
        if self.last_status.as_ref() == Some(&status) {
            return ControlFlow::Continue(());
        }
        self.last_status = Some(status.clone());
        self.emit(status)
    }

    fn emit(&mut self, progress: Progress) -> ControlFlow<()> {
        (self.report)(progress)
    }

    fn check_deadline(&self) -> Result<(), AppError> {
        if let SessionMode::Once {
            timeout: Some(timeout),
        } = self.mode
            && self.started.elapsed() >= timeout
        {
            return Err(AppError::Timeout(timeout.as_secs()));
        }
        Ok(())
    }
}

/// Reads the inserted card with retries and delivers the record to every sink.
pub fn execute_read_cycle(
    engine: &dyn DniEngine,
    reader: &ReaderInfo,
    sinks: &[&dyn Sink],
    attempts: u8,
    retry_delay: Duration,
    report: &mut ProgressSink<'_>,
) -> Result<ReadOutcome, CycleError> {
    let reading = |attempt: u8| Progress::Reading {
        reader: reader.name.clone(),
        attempt,
        attempts,
    };
    let _ = report(reading(1));
    let result = run_with_retries(
        attempts,
        |_| engine.read(reader),
        |error| error.retryable,
        |attempt, _| {
            thread::sleep(retry_delay);
            let _ = report(reading(attempt + 1));
        },
    )
    .map_err(|failure| CycleError::Read {
        attempts: failure.attempts,
        code: failure.last_error.code,
    })?;
    let record = new_record(reader, result);
    let delivery = deliver_all(sinks, &record);
    if delivery.failures.is_empty() {
        Ok(ReadOutcome {
            record,
            delivered: delivery.delivered,
        })
    } else {
        Err(CycleError::Delivery(delivery.failures))
    }
}

fn new_record(reader: &ReaderInfo, read: EngineRead) -> ReadRecord {
    ReadRecord::new(
        Uuid::new_v4(),
        Local::now().fixed_offset(),
        reader.name.clone(),
        read.document,
        read.integrity,
    )
}

/// Operator-facing wording for a progress event.
#[must_use]
pub fn describe(progress: &Progress) -> String {
    match progress {
        Progress::WaitingForReader => "Esperando un lector PC/SC...".to_owned(),
        Progress::WaitingForCard { reader } => format!("Esperando un DNIe en «{reader}»..."),
        Progress::Reading {
            reader,
            attempt,
            attempts,
        } => format!("Leyendo el DNIe en «{reader}» (intento {attempt} de {attempts})..."),
        Progress::Delivered { read_id, delivered } => {
            format!("Lectura {read_id} entregada a {delivered} salida(s).")
        }
        Progress::WaitingForRemoval { reader } => {
            format!("Retire el DNIe de «{reader}» para permitir otra lectura.")
        }
        Progress::ReadFailed { attempts, code } => {
            format!("Lectura fallida tras {attempts} intento(s): {code}.")
        }
        Progress::OutputFailed { sink, message } => format!("Salida {sink} fallida: {message}."),
        Progress::MonitorFailed { message } => {
            format!("Servicio PC/SC no disponible: {message}. Reintentando...")
        }
        Progress::MonitorRecovered => "Servicio PC/SC recuperado.".to_owned(),
    }
}

fn print_progress(progress: Progress) -> ControlFlow<()> {
    eprintln!("{}", describe(&progress));
    ControlFlow::Continue(())
}

fn list_readers() -> Result<(), AppError> {
    let mut monitor = PcscMonitor::new()?;
    let readers = monitor.initialise()?;
    if readers.is_empty() {
        println!("No se encontraron lectores PC/SC.");
    }
    for reader in readers {
        println!(
            "{}\t{}\t{}",
            reader.index,
            reader.presence.describe(),
            reader.name
        );
    }
    Ok(())
}

fn run_once(options: &OnceOptions) -> Result<(), AppError> {
    let engine = ProcessEngine::from_bundle(ENGINE_TIMEOUT)?;
    let sinks = build_sinks(&options.run.outputs)?;
    let mut monitor = PcscMonitor::new()?;
    run_session(
        &mut monitor,
        options.run.reader.as_deref(),
        &engine,
        &sink_references(&sinks),
        &SessionOptions::from_cli(&options.run),
        SessionMode::Once {
            timeout: options.timeout_seconds.map(Duration::from_secs),
        },
        &mut print_progress,
    )
}

fn run_watch(options: &RunOptions) -> Result<(), AppError> {
    let engine = ProcessEngine::from_bundle(ENGINE_TIMEOUT)?;
    let sinks = build_sinks(&options.outputs)?;
    let mut monitor = PcscMonitor::new()?;
    run_session(
        &mut monitor,
        options.reader.as_deref(),
        &engine,
        &sink_references(&sinks),
        &SessionOptions::from_cli(options),
        SessionMode::Watch,
        &mut print_progress,
    )
}

fn build_sinks(options: &OutputOptions) -> Result<Vec<Box<dyn Sink>>, OutputError> {
    let mut sinks: Vec<Box<dyn Sink>> = Vec::new();
    if options.stdout || !options.has_explicit_sink() {
        sinks.push(Box::new(StdoutSink));
    }
    if let Some(path) = &options.json {
        sinks.push(Box::new(JsonFileSink::new(path.clone())));
    }
    if let Some(path) = &options.jsonl {
        sinks.push(Box::new(JsonLinesSink::new(path.clone())));
    }
    if let Some(path) = &options.csv {
        sinks.push(Box::new(CsvSink::new(path.clone())));
    }
    add_webhook(&mut sinks, options)?;
    Ok(sinks)
}

fn add_webhook(sinks: &mut Vec<Box<dyn Sink>>, options: &OutputOptions) -> Result<(), OutputError> {
    if let Some(url) = &options.webhook {
        let token = std::env::var(WEBHOOK_TOKEN_VARIABLE).ok();
        sinks.push(Box::new(WebhookSink::new(
            url.clone(),
            token,
            Duration::from_secs(options.webhook_timeout_seconds),
        )?));
    }
    Ok(())
}

fn sink_references(sinks: &[Box<dyn Sink>]) -> Vec<&dyn Sink> {
    sinks.iter().map(Box::as_ref).collect()
}
