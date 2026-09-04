use std::fmt;
use std::net::IpAddr;
use std::thread;
use std::time::Duration;

use ureq::tls::{RootCerts, TlsConfig};
use url::Url;

use super::{OutputError, Sink};
use crate::lifecycle::run_with_retries;
use crate::model::ReadRecord;

const ATTEMPTS: u8 = 3;
const RETRY_DELAY: Duration = Duration::from_millis(500);

pub struct WebhookSink {
    url: String,
    token: Option<String>,
    agent: ureq::Agent,
    retry_delay: Duration,
}

impl WebhookSink {
    /// HTTPS only, except loopback. Certificates are validated with the operating
    /// system trust store so a PMS behind a corporate CA is reachable.
    pub fn new(url: String, token: Option<String>, timeout: Duration) -> Result<Self, OutputError> {
        validate_webhook_url(&url)?;
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .max_redirects(0)
            .http_status_as_error(false)
            .tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            .build();
        Ok(Self {
            url,
            token,
            agent: config.into(),
            retry_delay: RETRY_DELAY,
        })
    }

    #[must_use]
    pub fn with_retry_delay(mut self, retry_delay: Duration) -> Self {
        self.retry_delay = retry_delay;
        self
    }

    fn post(&self, record: &ReadRecord) -> Result<(), Attempt> {
        let request = self
            .agent
            .post(&self.url)
            .header("Idempotency-Key", record.read_id.to_string())
            .header(
                "User-Agent",
                concat!("SimpleLectorDNI/", env!("CARGO_PKG_VERSION")),
            );
        let request = match &self.token {
            Some(token) => request.header("Authorization", format!("Bearer {token}")),
            None => request,
        };
        let response = request.send_json(record).map_err(|error| Attempt {
            message: format!("la petición del webhook falló: {error}"),
            retryable: transport_retryable(&error),
        })?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        Err(Attempt {
            message: format!("el webhook devolvió HTTP {}", status.as_u16()),
            retryable: status.is_server_error() || matches!(status.as_u16(), 408 | 429),
        })
    }
}

struct Attempt {
    message: String,
    retryable: bool,
}

/// TLS and URL failures are configuration; everything else may be a blip.
fn transport_retryable(error: &ureq::Error) -> bool {
    !matches!(error, ureq::Error::Tls(_) | ureq::Error::BadUri(_))
}

impl fmt::Debug for WebhookSink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WebhookSink")
            .field("url", &self.url)
            .field("has_token", &self.token.is_some())
            .finish_non_exhaustive()
    }
}

impl Sink for WebhookSink {
    fn name(&self) -> &'static str {
        "webhook"
    }

    /// Retries transport errors and 5xx with the same `Idempotency-Key`, so the
    /// receiver sees one logical read.
    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError> {
        run_with_retries(
            ATTEMPTS,
            |_| self.post(record),
            |attempt| attempt.retryable,
            |_, _| thread::sleep(self.retry_delay),
        )
        .map_err(|failure| OutputError::Delivery(failure.last_error.message))
    }
}

fn validate_webhook_url(value: &str) -> Result<(), OutputError> {
    let url = Url::parse(value)
        .map_err(|_| OutputError::Configuration("la URL del webhook no es válida".to_owned()))?;
    if url.scheme() == "https" || is_loopback_http(&url) {
        return Ok(());
    }
    Err(OutputError::Configuration(
        "el webhook debe usar HTTPS, salvo en localhost".to_owned(),
    ))
}

fn is_loopback_http(url: &Url) -> bool {
    if url.scheme() != "http" {
        return false;
    }
    match url.host_str() {
        Some("localhost") => true,
        Some(host) => host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback()),
        None => false,
    }
}
