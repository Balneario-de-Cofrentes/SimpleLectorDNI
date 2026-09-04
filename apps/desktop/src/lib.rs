//! Desktop shell over the `simple-lector-dni` crate: one window that shows what the
//! session is doing and the last record read, plus an optional webhook. The reader,
//! the engine and the session loop are the crate's; this file only wires them to a
//! Tauri window and to the operating system keychain.

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use simple_lector_dni::app::{Progress, SessionMode, SessionOptions, describe, run_session};
use simple_lector_dni::engine::ProcessEngine;
use simple_lector_dni::model::ReadRecord;
use simple_lector_dni::output::{OutputError, Sink, WebhookSink};
use simple_lector_dni::reader::{PcscMonitor, ReaderMonitor};
use tauri::{AppHandle, Emitter, Manager, State};

const ENGINE_TIMEOUT: Duration = Duration::from_secs(20);
const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(10);
const SETTINGS_FILE: &str = "settings.json";
const KEYRING_SERVICE: &str = "es.cofrentes.simplelectordni";
const KEYRING_USER: &str = "webhook-token";
const MAX_ATTEMPTS: u8 = 3;

/// Persisted in the app config directory. The webhook token never lands here: it
/// lives in the operating system keychain.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    pub reader: String,
    pub webhook_url: String,
    pub attempts: u8,
}

impl Settings {
    fn attempts(&self) -> u8 {
        self.attempts.clamp(1, MAX_ATTEMPTS)
    }

    fn reader_pattern(&self) -> Option<&str> {
        let pattern = self.reader.trim();
        (!pattern.is_empty()).then_some(pattern)
    }

    fn webhook_url(&self) -> Option<&str> {
        let url = self.webhook_url.trim();
        (!url.is_empty()).then_some(url)
    }
}

#[derive(Serialize)]
pub struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    has_webhook_token: bool,
}

#[derive(Serialize)]
pub struct ReaderView {
    index: usize,
    name: String,
    presence: &'static str,
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

#[derive(Default)]
pub struct SessionState {
    running: Option<RunningSession>,
}

struct RunningSession {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

/// Delivers each record to the window. Nothing is written to disk.
#[derive(Debug)]
struct WindowSink {
    app: AppHandle,
}

impl Sink for WindowSink {
    fn name(&self) -> &'static str {
        "ventana"
    }

    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError> {
        self.app
            .emit("read", record)
            .map_err(|error| OutputError::Delivery(error.to_string()))
    }
}

#[tauri::command]
fn list_readers() -> Result<Vec<ReaderView>, String> {
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

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<SettingsView, String> {
    let settings = read_settings(&app)?;
    Ok(settings_view(settings))
}

/// `webhook_token`: `None` keeps the stored token, an empty string removes it and any
/// other value replaces it.
#[tauri::command]
fn save_settings(
    app: AppHandle,
    settings: Settings,
    webhook_token: Option<String>,
) -> Result<SettingsView, String> {
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
    std::fs::write(&path, json).map_err(|error| error.to_string())?;
    match webhook_token.as_deref().map(str::trim) {
        None => {}
        Some("") => {
            let _ = token_entry()?.delete_credential();
        }
        Some(token) => token_entry()?
            .set_password(token)
            .map_err(|error| error.to_string())?,
    }
    Ok(settings_view(settings))
}

#[tauri::command]
fn start_session(app: AppHandle, state: State<'_, Mutex<SessionState>>) -> Result<(), String> {
    let mut state = state
        .lock()
        .map_err(|_| "estado de sesión bloqueado".to_owned())?;
    if state
        .running
        .as_ref()
        .is_some_and(|run| !run.thread.is_finished())
    {
        return Err("ya hay una sesión en marcha".to_owned());
    }
    let settings = read_settings(&app)?;
    let token = if settings.webhook_url().is_some() {
        stored_token()?
    } else {
        None
    };
    let stop = Arc::new(AtomicBool::new(false));
    let thread = {
        let app = app.clone();
        let stop = Arc::clone(&stop);
        thread::Builder::new()
            .name("simple-lector-dni-session".to_owned())
            .spawn(move || {
                let outcome = watch(&app, &settings, token, &stop);
                let _ = app.emit(
                    "session_ended",
                    SessionEnded {
                        error: outcome.err(),
                    },
                );
            })
            .map_err(|error| error.to_string())?
    };
    state.running = Some(RunningSession { stop, thread });
    Ok(())
}

#[tauri::command]
fn stop_session(state: State<'_, Mutex<SessionState>>) -> Result<(), String> {
    let mut state = state
        .lock()
        .map_err(|_| "estado de sesión bloqueado".to_owned())?;
    if let Some(run) = state.running.take() {
        run.stop.store(true, Ordering::SeqCst);
        let _ = run.thread.join();
    }
    Ok(())
}

#[tauri::command]
fn is_running(state: State<'_, Mutex<SessionState>>) -> bool {
    state
        .lock()
        .ok()
        .and_then(|state| state.running.as_ref().map(|run| !run.thread.is_finished()))
        .unwrap_or(false)
}

fn watch(
    app: &AppHandle,
    settings: &Settings,
    token: Option<String>,
    stop: &AtomicBool,
) -> Result<(), String> {
    let engine = bundled_engine(app)?;
    let mut sinks: Vec<Box<dyn Sink>> = vec![Box::new(WindowSink { app: app.clone() })];
    if let Some(url) = settings.webhook_url() {
        let webhook = WebhookSink::new(url.to_owned(), token, WEBHOOK_TIMEOUT)
            .map_err(|error| error.to_string())?;
        sinks.push(Box::new(webhook));
    }
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
                    "progress",
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

/// The runtime and the worker travel as bundle resources under `resources/`
/// (`runtime/`, `engine/`), the same layout the CLI package uses next to its binary.
fn bundled_engine(app: &AppHandle) -> Result<ProcessEngine, String> {
    let root = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join("resources");
    let (java, jar) = ProcessEngine::bundled_layout(&root);
    ProcessEngine::at(java, jar, ENGINE_TIMEOUT).map_err(|error| error.to_string())
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(SETTINGS_FILE))
        .map_err(|error| error.to_string())
}

fn read_settings(app: &AppHandle) -> Result<Settings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let json = std::fs::read(&path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&json).map_err(|error| error.to_string())
}

fn settings_view(settings: Settings) -> SettingsView {
    let has_webhook_token = stored_token().ok().flatten().is_some();
    SettingsView {
        settings,
        has_webhook_token,
    }
}

fn token_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|error| error.to_string())
}

fn stored_token() -> Result<Option<String>, String> {
    match token_entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(SessionState::default()))
        .invoke_handler(tauri::generate_handler![
            list_readers,
            load_settings,
            save_settings,
            start_session,
            stop_session,
            is_running
        ])
        .run(tauri::generate_context!())
        .expect("SimpleLectorDNI no pudo arrancar la ventana");
}
