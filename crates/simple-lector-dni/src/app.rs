use std::thread;
use std::time::Duration;

use chrono::Local;
use thiserror::Error;
use uuid::Uuid;

use crate::cli::{Cli, Command, OutputOptions, RunOptions};
use crate::engine::{DniEngine, EngineFailure, EngineRead, ProcessEngine};
use crate::lifecycle::{CardLifecycle, LifecycleAction, LifecycleEvent, run_with_retries};
use crate::model::ReadRecord;
use crate::output::{
    CsvSink, JsonFileSink, JsonLinesSink, OutputError, Sink, StdoutSink, WebhookSink, deliver_all,
};
use crate::reader::{
    PcscMonitor, ReaderError, ReaderEvent, ReaderInfo, ReaderMonitor, ReaderPresence, select_reader,
};

const POLL_DELAY: Duration = Duration::from_millis(250);
const ENGINE_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Error)]
pub enum AppError {
    #[error("reader error: {0}")]
    Reader(#[from] ReaderError),
    #[error("output error: {0}")]
    Output(#[from] OutputError),
    #[error("could not initialise DNIe engine: {0}")]
    Engine(#[from] EngineFailure),
    #[error("DNIe read failed after {attempts} attempt(s), code {code}")]
    Read { attempts: u8, code: String },
    #[error("{0} configured output(s) failed")]
    Delivery(usize),
}

#[derive(Debug)]
pub struct WatchController {
    pattern: Option<String>,
    selected: Option<ReaderInfo>,
    lifecycle: CardLifecycle,
}

impl WatchController {
    pub fn new(readers: &[ReaderInfo], pattern: Option<&str>) -> Result<Self, AppError> {
        let selected = selectable_reader(readers, pattern)?;
        Ok(Self {
            pattern: pattern.map(str::to_owned),
            lifecycle: CardLifecycle::new(selected.is_some()),
            selected,
        })
    }

    #[must_use]
    pub fn initial_read(&mut self) -> Option<ReaderInfo> {
        let reader = self
            .selected
            .as_ref()
            .filter(|value| value.presence == ReaderPresence::Present)?
            .clone();
        self.start_read(reader)
    }

    #[must_use]
    pub fn handle(&mut self, event: ReaderEvent) -> Option<ReaderInfo> {
        match event {
            ReaderEvent::ReaderAttached(reader) => self.reader_attached(reader),
            ReaderEvent::ReaderDetached(reader) => self.reader_detached(&reader),
            ReaderEvent::CardInserted(reader) => self.card_inserted(reader),
            ReaderEvent::CardRemoved(reader) => self.card_removed(&reader),
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
        self.selected.as_ref().map(|reader| reader.name.as_str())
    }

    fn reader_attached(&mut self, reader: ReaderInfo) -> Option<ReaderInfo> {
        if self.selected.is_none() && matches_pattern(&reader, self.pattern.as_deref()) {
            self.selected = Some(reader);
            self.lifecycle.handle(LifecycleEvent::ReaderAttached);
        }
        None
    }

    fn reader_detached(&mut self, reader: &ReaderInfo) -> Option<ReaderInfo> {
        if self.is_selected(reader) {
            self.lifecycle.handle(LifecycleEvent::ReaderDetached);
            self.selected = None;
        }
        None
    }

    fn card_inserted(&mut self, reader: ReaderInfo) -> Option<ReaderInfo> {
        if self.is_selected(&reader) {
            return self.start_read(reader);
        }
        None
    }

    fn card_removed(&mut self, reader: &ReaderInfo) -> Option<ReaderInfo> {
        if self.is_selected(reader) {
            self.lifecycle.handle(LifecycleEvent::CardRemoved);
        }
        None
    }

    fn start_read(&mut self, reader: ReaderInfo) -> Option<ReaderInfo> {
        (self.lifecycle.handle(LifecycleEvent::CardInserted) == LifecycleAction::StartRead)
            .then_some(reader)
    }

    fn is_selected(&self, reader: &ReaderInfo) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|value| value.name == reader.name)
    }
}

pub fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Command::ListReaders => list_readers(),
        Command::Once(options) => run_once(&options),
        Command::Watch(options) => run_watch(&options),
    }
}

pub fn execute_read_cycle(
    engine: &dyn DniEngine,
    reader: &ReaderInfo,
    sinks: &[&dyn Sink],
    attempts: u8,
    retry_delay: Duration,
) -> Result<ReadRecord, AppError> {
    let result = read_with_retries(engine, reader, attempts, retry_delay)?;
    let record = ReadRecord::new(
        Uuid::new_v4(),
        Local::now().fixed_offset(),
        reader.name.clone(),
        result.document,
        result.integrity,
    );
    let report = deliver_all(sinks, &record);
    if report.failures.is_empty() {
        Ok(record)
    } else {
        Err(AppError::Delivery(report.failures.len()))
    }
}

fn read_with_retries(
    engine: &dyn DniEngine,
    reader: &ReaderInfo,
    attempts: u8,
    delay: Duration,
) -> Result<EngineRead, AppError> {
    run_with_retries(
        attempts,
        |_| engine.read(reader),
        |error| error.retryable,
        |_, _| thread::sleep(delay),
    )
    .map_err(|failure| AppError::Read {
        attempts: failure.attempts,
        code: failure.last_error.code,
    })
}

fn list_readers() -> Result<(), AppError> {
    let mut monitor = PcscMonitor::new()?;
    let readers = monitor.initialise()?;
    if readers.is_empty() {
        println!("No se encontraron lectores PC/SC.");
    }
    for reader in readers {
        println!("{}\t{:?}\t{}", reader.index, reader.presence, reader.name);
    }
    Ok(())
}

fn run_once(options: &RunOptions) -> Result<(), AppError> {
    let mut monitor = PcscMonitor::new()?;
    let reader = wait_for_card(&mut monitor, options.reader.as_deref())?;
    let engine = ProcessEngine::from_bundle(ENGINE_TIMEOUT)?;
    let sinks = build_sinks(&options.outputs)?;
    let sink_refs = sink_references(&sinks);
    execute_read_cycle(
        &engine,
        &reader,
        &sink_refs,
        options.attempts,
        Duration::from_millis(options.retry_delay_ms),
    )?;
    Ok(())
}

fn run_watch(options: &RunOptions) -> Result<(), AppError> {
    let mut monitor = PcscMonitor::new()?;
    let readers = monitor.initialise()?;
    let mut controller = WatchController::new(&readers, options.reader.as_deref())?;
    let engine = ProcessEngine::from_bundle(ENGINE_TIMEOUT)?;
    let sinks = build_sinks(&options.outputs)?;
    if let Some(reader) = controller.initial_read() {
        process_watch_read(&reader, &mut controller, &engine, &sinks, options);
    }
    loop {
        let events = monitor.wait_for_events(POLL_DELAY)?;
        for event in events {
            if let Some(reader) = controller.handle(event) {
                process_watch_read(&reader, &mut controller, &engine, &sinks, options);
            }
        }
    }
}

fn wait_for_card(
    monitor: &mut dyn ReaderMonitor,
    pattern: Option<&str>,
) -> Result<ReaderInfo, AppError> {
    let readers = monitor.initialise()?;
    let mut selected = selectable_reader(&readers, pattern)?;
    if let Some(reader) = selected
        .as_ref()
        .filter(|value| value.presence == ReaderPresence::Present)
    {
        return Ok(reader.clone());
    }
    loop {
        for event in monitor.wait_for_events(POLL_DELAY)? {
            update_selection(&event, &mut selected, pattern);
            if let ReaderEvent::CardInserted(reader) = event
                && selected
                    .as_ref()
                    .is_some_and(|value| value.name == reader.name)
            {
                return Ok(reader);
            }
        }
    }
}

fn selectable_reader(
    readers: &[ReaderInfo],
    pattern: Option<&str>,
) -> Result<Option<ReaderInfo>, AppError> {
    match select_reader(readers, pattern) {
        Ok(reader) => Ok(Some(reader)),
        Err(ReaderError::NoReaders | ReaderError::NotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn update_selection(event: &ReaderEvent, selected: &mut Option<ReaderInfo>, pattern: Option<&str>) {
    match event {
        ReaderEvent::ReaderAttached(reader)
            if selected.is_none() && matches_pattern(reader, pattern) =>
        {
            *selected = Some(reader.clone());
        }
        ReaderEvent::ReaderDetached(reader)
            if selected
                .as_ref()
                .is_some_and(|value| value.name == reader.name) =>
        {
            *selected = None;
        }
        _ => {}
    }
}

fn matches_pattern(reader: &ReaderInfo, pattern: Option<&str>) -> bool {
    pattern.is_none_or(|value| reader.name.to_lowercase().contains(&value.to_lowercase()))
}

fn process_watch_read(
    reader: &ReaderInfo,
    controller: &mut WatchController,
    engine: &dyn DniEngine,
    sinks: &[Box<dyn Sink>],
    options: &RunOptions,
) {
    let sink_refs = sink_references(sinks);
    let result = execute_read_cycle(
        engine,
        reader,
        &sink_refs,
        options.attempts,
        Duration::from_millis(options.retry_delay_ms),
    );
    match result {
        Ok(_) => controller.read_succeeded(),
        Err(error) => {
            controller.read_failed();
            eprintln!("{error}");
        }
    }
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
        let token = std::env::var("SIMPLE_LECTOR_DNI_WEBHOOK_TOKEN").ok();
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
