use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::DateTime;
use simple_lector_dni::engine_protocol::{DocumentData, IntegrityResult};
use simple_lector_dni::model::ReadRecord;
use simple_lector_dni::output::{
    CsvSink, JsonFileSink, JsonLinesSink, OutputError, Sink, WebhookSink, deliver_all,
};
use tempfile::tempdir;
use uuid::Uuid;

fn record(name: &str) -> ReadRecord {
    ReadRecord::new(
        Uuid::parse_str("9f142e2c-f4ec-47e5-b8cc-1bbfe49118a7").unwrap(),
        DateTime::parse_from_rfc3339("2026-09-04T12:34:56+02:00").unwrap(),
        "Synthetic reader".to_owned(),
        DocumentData {
            nombre: name.to_owned(),
            dni: "00000000T".to_owned(),
            direccion: "=CMD(\"synthetic\")".to_owned(),
            ..DocumentData::default()
        },
        IntegrityResult {
            sod_signature: "verified".to_owned(),
            dg13_hash: "verified".to_owned(),
        },
    )
}

#[test]
fn latest_json_is_replaced_atomically() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("latest.json");
    let sink = JsonFileSink::new(path.clone());

    sink.deliver(&record("FIRST")).unwrap();
    sink.deliver(&record("SECOND")).unwrap();

    let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["document"]["nombre"], "SECOND");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn json_lines_appends_one_complete_object_per_read() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("history.jsonl");
    let sink = JsonLinesSink::new(path.clone());

    sink.deliver(&record("FIRST")).unwrap();
    sink.deliver(&record("SECOND")).unwrap();

    let text = fs::read_to_string(path).unwrap();
    let values: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(values.len(), 2);
    assert_eq!(values[1]["document"]["nombre"], "SECOND");
}

#[test]
fn csv_creates_one_header_and_appends_protected_rows() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("history.csv");
    let sink = CsvSink::new(path.clone());

    sink.deliver(&record("FIRST")).unwrap();
    sink.deliver(&record("SECOND")).unwrap();

    let mut reader = csv::Reader::from_path(path).unwrap();
    let headers = reader.headers().unwrap().clone();
    let address_index = headers
        .iter()
        .position(|value| value == "direccion")
        .unwrap();
    let rows: Vec<csv::StringRecord> = reader.records().map(Result::unwrap).collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].get(6), Some("SECOND"));
    assert_eq!(rows[0].get(address_index), Some("'=CMD(\"synthetic\")"));
}

#[cfg(unix)]
#[test]
fn identity_files_are_private_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let path = directory.path().join("history.jsonl");
    JsonLinesSink::new(path.clone())
        .deliver(&record("PRIVATE"))
        .unwrap();

    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn webhook_rejects_non_loopback_plain_http() {
    let error = WebhookSink::new(
        "http://example.com/hook".to_owned(),
        None,
        Duration::from_secs(1),
    )
    .unwrap_err();

    assert!(error.to_string().contains("HTTPS"));
}

#[test]
fn webhook_sends_json_auth_and_idempotency_to_loopback() {
    let (url, request_rx) = capture_one_request();
    let sink = WebhookSink::new(
        url,
        Some("synthetic-secret".to_owned()),
        Duration::from_secs(2),
    )
    .unwrap();

    sink.deliver(&record("WEBHOOK")).unwrap();
    let request = request_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let lowercase = request.to_ascii_lowercase();
    assert!(lowercase.contains("authorization: bearer synthetic-secret"));
    assert!(lowercase.contains("idempotency-key: 9f142e2c-f4ec-47e5-b8cc-1bbfe49118a7"));
    let body = request.split("\r\n\r\n").nth(1).unwrap();
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(json["document"]["nombre"], "WEBHOOK");
}

#[test]
fn webhook_never_redirects_identity_data() {
    let redirected_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    redirected_listener.set_nonblocking(true).unwrap();
    let redirected_address = redirected_listener.local_addr().unwrap();
    let redirect_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let redirect_address = redirect_listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = redirect_listener.accept().unwrap();
        let _ = read_http_request(&mut stream);
        write!(
            stream,
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{redirected_address}/capture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
    });
    let sink = WebhookSink::new(
        format!("http://{redirect_address}/hook"),
        None,
        Duration::from_millis(250),
    )
    .unwrap();

    assert!(sink.deliver(&record("DO NOT REDIRECT")).is_err());
    assert!(redirected_listener.accept().is_err());
}

#[test]
fn sink_failures_do_not_stop_other_sinks() {
    let delivered = AtomicUsize::new(0);
    let counter = CountsDeliveries(&delivered);
    let sinks: Vec<&dyn Sink> = vec![&AlwaysFails, &counter];

    let report = deliver_all(&sinks, &record("ISOLATED"));

    assert_eq!(delivered.load(Ordering::SeqCst), 1);
    assert_eq!(report.delivered, 1);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].sink, "always-fails");
}

#[derive(Debug)]
struct AlwaysFails;

impl Sink for AlwaysFails {
    fn name(&self) -> &'static str {
        "always-fails"
    }

    fn deliver(&self, _: &ReadRecord) -> Result<(), OutputError> {
        Err(OutputError::Delivery("synthetic failure".to_owned()))
    }
}

#[derive(Debug)]
struct CountsDeliveries<'a>(&'a AtomicUsize);

impl Sink for CountsDeliveries<'_> {
    fn name(&self) -> &'static str {
        "counter"
    }

    fn deliver(&self, _: &ReadRecord) -> Result<(), OutputError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn capture_one_request() -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let request = read_http_request(&mut stream);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .unwrap();
        sender.send(request).unwrap();
    });
    (format!("http://{address}/hook"), receiver)
}

fn read_http_request(stream: &mut impl Read) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream.read(&mut chunk).unwrap();
        bytes.extend_from_slice(&chunk[..read]);
        let text = String::from_utf8_lossy(&bytes);
        if request_is_complete(&text) {
            return text.into_owned();
        }
    }
}

fn request_is_complete(request: &str) -> bool {
    let Some((headers, body)) = request.split_once("\r\n\r\n") else {
        return false;
    };
    let length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .map(str::to_owned)
        })
        .and_then(|value| value.parse::<usize>().ok());
    length.is_some_and(|length| body.len() >= length)
}
