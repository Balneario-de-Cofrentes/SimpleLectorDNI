use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use url::Url;

use super::{OutputError, Sink};
use crate::model::ReadRecord;

pub struct WebhookSink {
    url: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl WebhookSink {
    pub fn new(url: String, token: Option<String>, timeout: Duration) -> Result<Self, OutputError> {
        validate_webhook_url(&url)?;
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .build();
        Ok(Self {
            url,
            token,
            agent: config.into(),
        })
    }
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

    fn deliver(&self, record: &ReadRecord) -> Result<(), OutputError> {
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
        request
            .send_json(record)
            .map_err(|error| OutputError::Delivery(format!("webhook request failed: {error}")))?;
        Ok(())
    }
}

fn validate_webhook_url(value: &str) -> Result<(), OutputError> {
    let url = Url::parse(value)
        .map_err(|_| OutputError::Configuration("webhook URL is invalid".to_owned()))?;
    if url.scheme() == "https" || is_loopback_http(&url) {
        return Ok(());
    }
    Err(OutputError::Configuration(
        "webhook must use HTTPS, except for localhost".to_owned(),
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
