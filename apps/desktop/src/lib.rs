//! Public desktop window: session status, last read on screen and an optional webhook.
//! Everything reusable lives in `simple-lector-dni-tauri`; this crate only defines the
//! window's settings and commands.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use simple_lector_dni::output::{Sink, WebhookSink};
use simple_lector_dni_tauri::secrets::Secret;
use simple_lector_dni_tauri::session::{
    MAX_ATTEMPTS, ReaderView, SessionController, SessionSettings,
};
use simple_lector_dni_tauri::settings;
use tauri::{AppHandle, Manager, State};

const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(10);
const SETTINGS_FILE: &str = "settings.json";
const WEBHOOK_TOKEN: Secret = Secret::new("es.cofrentes.simplelectordni", "webhook-token");

/// Persisted in the app config directory. The webhook token never lands here.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    pub reader: String,
    pub webhook_url: String,
    pub attempts: u8,
}

impl Settings {
    fn webhook_url(&self) -> Option<&str> {
        let url = self.webhook_url.trim();
        (!url.is_empty()).then_some(url)
    }

    fn session(&self) -> SessionSettings {
        SessionSettings {
            reader: self.reader.clone(),
            attempts: if self.attempts == 0 {
                MAX_ATTEMPTS
            } else {
                self.attempts
            },
        }
    }
}

#[derive(Serialize)]
pub struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    has_webhook_token: bool,
}

#[tauri::command]
fn list_readers() -> Result<Vec<ReaderView>, String> {
    simple_lector_dni_tauri::session::list_readers()
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<SettingsView, String> {
    Ok(settings_view(read_settings(&app)?))
}

/// `webhook_token`: `None` keeps the stored token, an empty string removes it and any
/// other value replaces it.
#[tauri::command]
fn save_settings(
    app: AppHandle,
    settings: Settings,
    webhook_token: Option<String>,
) -> Result<SettingsView, String> {
    settings::save(&config_dir(&app)?, SETTINGS_FILE, &settings)?;
    WEBHOOK_TOKEN.apply(webhook_token.as_deref())?;
    Ok(settings_view(settings))
}

#[tauri::command]
fn start_session(app: AppHandle, controller: State<'_, SessionController>) -> Result<(), String> {
    let settings = read_settings(&app)?;
    let webhook = settings.webhook_url().map(str::to_owned);
    let token = if webhook.is_some() {
        WEBHOOK_TOKEN.get()?
    } else {
        None
    };
    controller.start(
        &app,
        settings.session(),
        Box::new(move || {
            let mut sinks: Vec<Box<dyn Sink>> = Vec::new();
            if let Some(url) = webhook {
                let sink = WebhookSink::new(url, token, WEBHOOK_TIMEOUT)
                    .map_err(|error| error.to_string())?;
                sinks.push(Box::new(sink));
            }
            Ok(sinks)
        }),
    )
}

#[tauri::command]
fn stop_session(controller: State<'_, SessionController>) -> Result<(), String> {
    controller.stop()
}

#[tauri::command]
fn is_running(controller: State<'_, SessionController>) -> bool {
    controller.is_running()
}

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|error| error.to_string())
}

fn read_settings(app: &AppHandle) -> Result<Settings, String> {
    settings::load(&config_dir(app)?, SETTINGS_FILE)
}

fn settings_view(settings: Settings) -> SettingsView {
    let has_webhook_token = WEBHOOK_TOKEN.get().ok().flatten().is_some();
    SettingsView {
        settings,
        has_webhook_token,
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(SessionController::default())
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
