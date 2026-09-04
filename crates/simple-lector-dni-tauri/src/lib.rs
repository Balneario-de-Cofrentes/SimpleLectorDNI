//! Reusable Tauri-side pieces over the `simple-lector-dni` crate, shared by the public
//! desktop window and by private apps built on it: a session that runs on its own thread
//! and reports to the window, JSON settings in the app config directory, and secrets in
//! the operating system keychain.

pub mod secrets;
pub mod session;
pub mod settings;
