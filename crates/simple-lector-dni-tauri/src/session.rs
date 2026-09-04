//! A reader session on its own thread that reports to the Tauri window: `progress`
//! events (kind-tagged, with the operator wording), one `read` event per record and a
//! final `session_ended` event. Extra sinks (a webhook, a PMS client) are supplied by
//! the app.

use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::Serialize;
use simple_lector_dni::app::{Progress, SessionMode, SessionOptions, describe, run_session};
use simple_lector_dni::engine::ProcessEngine;
use simple_lector_dni::model::ReadRecord;
use simple_lector_dni::output::{OutputError, Sink};
use simple_lector_dni::reader::{PcscMonitor, ReaderMonitor};
use tauri::{AppHandle, Emitter, Manager};

const ENGINE_TIMEOUT: Duration = Duration::from_secs(20);
pub const MAX_ATTEMPTS: u8 = 3;

/// Event names the window can listen to.
pub const PROGRESS_EVENT: &str = "progress";
pub const READ_EVENT: &str = "read";
pub const SESSION_ENDED_EVENT: &str = "session_ended";

#[derive(Clone, Debug, Default)]
pub struct SessionSettings {
    /// Case-insensitive substring of the reader name; empty selects the first reader.
    pub reader: String,
    pub attempts: u8,
}

impl SessionSettings {
    fn attempts(&self) -> u8 {
        self.attempts.clamp(1, MAX_ATTEMPTS)
    }

    fn reader_pattern(&self) -> Option<&str> {
        let pattern = self.reader.trim();
        (!pattern.is_empty()).then_some(pattern)
    }
}

#[derive(Serialize)]
pub struct ReaderView {
    pub index: usize,
    pub name: String,
    pub presence: &'static str,
}

#[derive(Clone, Serialize)]
struct ProgressEvent {
    text: String,
    #[serde(flatten)]
    progress: Progress,
}

#[derive(Clone, Serialize)]
struct SessionEnded {
    error: Option<String>,
}

/// Owns the running session, if any. Keep one in Tauri managed state.
#[derive(Default)]
pub struct SessionController {
    running: Mutex<Option<RunningSession>>,
}

struct RunningSession {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

/// Builds the app-specific sinks for a session, on the session thread.
pub type SinkFactory = Box<dyn FnOnce() -> Result<Vec<Box<dyn Sink>>, String> + Send>;

impl SessionController {
    /// Starts a watch session; the window sink is always first, the app's sinks follow.
    pub fn start(
        &self,
        app: &AppHandle,
        settings: SessionSettings,
        extra_sinks: SinkFactory,
    ) -> Result<(), String> {
        let mut running = self.lock()?;
        if running
            .as_ref()
            .is_some_and(|run| !run.thread.is_finished())
        {
            return Err("ya hay una sesión en marcha".to_owned());
        }
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let app = app.clone();
            let stop = Arc::clone(&stop);
            thread::Builder::new()
                .name("simple-lector-dni-session".to_owned())
                .spawn(move || {
                    let outcome = watch(&app, &settings, extra_sinks, &stop);
                    let _ = app.emit(
                        SESSION_ENDED_EVENT,
                        SessionEnded {
                            error: outcome.err(),
                        },
                    );
                })
                .map_err(|error| error.to_string())?
        };
        *running = Some(RunningSession { stop, thread });
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let run = self.lock()?.take();
        if let Some(run) = run {
            run.stop.store(true, Ordering::SeqCst);
            let _ = run.thread.join();
        }
        Ok(())
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
            .lock()
            .ok()
            .and_then(|running| running.as_ref().map(|run| !run.thread.is_finished()))
            .unwrap_or(false)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Option<RunningSession>>, String> {
        self.running
            .lock()
            .map_err(|_| "estado de sesión bloqueado".to_owned())
    }
}

/// Delivers each record to the window. Nothing is written to disk.
#[derive(Debug)]
pub struct WindowSink {
    app: AppHandle,
}

impl WindowSink {
    #[must_use]
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl Sink for WindowSink {
    fn name(&self) -> &'static str {
        "ventana"
    }

    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError> {
        self.app
            .emit(READ_EVENT, record)
            .map_err(|error| OutputError::Delivery(error.to_string()))
    }
}

pub fn list_readers() -> Result<Vec<ReaderView>, String> {
    let mut monitor = PcscMonitor::new().map_err(|error| error.to_string())?;
    let readers = monitor.initialise().map_err(|error| error.to_string())?;
    Ok(readers
        .into_iter()
        .map(|reader| ReaderView {
            index: reader.index,
            name: reader.name,
            presence: reader.presence.describe(),
        })
        .collect())
}

/// The runtime and the worker travel as bundle resources under `resources/`
/// (`runtime/`, `engine/`), the same layout the CLI package uses next to its binary.
pub fn bundled_engine(app: &AppHandle) -> Result<ProcessEngine, String> {
    let root = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join("resources");
    let (java, jar) = ProcessEngine::bundled_layout(&root);
    ProcessEngine::at(java, jar, ENGINE_TIMEOUT).map_err(|error| error.to_string())
}

fn watch(
    app: &AppHandle,
    settings: &SessionSettings,
    extra_sinks: SinkFactory,
    stop: &AtomicBool,
) -> Result<(), String> {
    let engine = bundled_engine(app)?;
    let mut sinks: Vec<Box<dyn Sink>> = vec![Box::new(WindowSink::new(app.clone()))];
    sinks.extend(extra_sinks()?);
    let sink_refs: Vec<&dyn Sink> = sinks.iter().map(Box::as_ref).collect();
    let mut monitor = PcscMonitor::new().map_err(|error| error.to_string())?;
    let options = SessionOptions {
        attempts: settings.attempts(),
        retry_delay: Duration::from_millis(350),
        poll_delay: Duration::from_millis(250),
        recovery_delay: Duration::from_secs(1),
    };
    run_session(
        &mut monitor,
        settings.reader_pattern(),
        &engine,
        &sink_refs,
        &options,
        SessionMode::Watch,
        &mut |progress| {
            if progress != Progress::Idle {
                let _ = app.emit(
                    PROGRESS_EVENT,
                    ProgressEvent {
                        text: describe(&progress),
                        progress,
                    },
                );
            }
            if stop.load(Ordering::SeqCst) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .map_err(|error| error.to_string())
}
