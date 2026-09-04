//! Secrets live in the operating system keychain (macOS Keychain, Windows Credential
//! Manager), never in a settings file.

/// One keychain entry, identified by service and user.
pub struct Secret {
    service: &'static str,
    user: &'static str,
}

impl Secret {
    #[must_use]
    pub const fn new(service: &'static str, user: &'static str) -> Self {
        Self { service, user }
    }

    pub fn get(&self) -> Result<Option<String>, String> {
        match self.entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn set(&self, value: &str) -> Result<(), String> {
        self.entry()?
            .set_password(value)
            .map_err(|error| error.to_string())
    }

    pub fn delete(&self) -> Result<(), String> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Applies a value coming from a form: `None` keeps the stored secret, an empty or
    /// blank string removes it, anything else replaces it.
    pub fn apply(&self, value: Option<&str>) -> Result<(), String> {
        match value.map(str::trim) {
            None => Ok(()),
            Some("") => self.delete(),
            Some(value) => self.set(value),
        }
    }

    fn entry(&self) -> Result<keyring::Entry, String> {
        keyring::Entry::new(self.service, self.user).map_err(|error| error.to_string())
    }
}
