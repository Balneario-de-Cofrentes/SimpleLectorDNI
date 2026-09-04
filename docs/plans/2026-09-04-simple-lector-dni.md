# SimpleLectorDNI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Publish a maintainable Rust CLI that monitors contact DNIe insertions and delivers verified DG13 text through JSON, JSONL, CSV, and webhook outputs on Windows and macOS.

**Architecture:** A Rust supervisor owns PC/SC events, lifecycle, retries, schema, and outputs. A separately spawned JMultiCard worker owns the cryptographic DNIe operation and communicates through a versioned one-request/one-response JSON protocol, allowing a native Rust engine to replace it later.

**Tech Stack:** Rust 2024, `pcsc`, `clap`, `serde`, `serde_json`, `csv`, `chrono`, `uuid`, `ureq`; Java 21 runtime with Java 11-compatible worker sources, JMultiCard 2.1, Bouncy Castle; Maven; GitHub Actions.

---

### Task 1: Bootstrap the repository and public contracts

**Files:**
- Create: `Cargo.toml`
- Create: `crates/simple-lector-dni/Cargo.toml`
- Create: `crates/simple-lector-dni/src/lib.rs`
- Create: `crates/simple-lector-dni/src/main.rs`
- Create: `protocol/engine-v1.schema.json`
- Create: `LICENSE`
- Create: `THIRD_PARTY_NOTICES.md`

**Step 1:** Create a failing Rust test that deserialises the documented engine response into the intended domain type.

**Step 2:** Run `cargo test -p simple-lector-dni protocol_contract -- --exact` and verify that it fails because the domain type does not exist.

**Step 3:** Add the workspace, minimal binary, engine protocol request/response types, and JSON Schema.

**Step 4:** Run the focused test and then `cargo test --workspace`; expect all tests to pass.

**Step 5:** Add the EUPL-1.2 text and notices for JMultiCard and other redistributed components.

**Step 6:** Commit with `chore: bootstrap Rust workspace and contracts`.

### Task 2: Define the stable read schema

**Files:**
- Create: `crates/simple-lector-dni/src/model.rs`
- Test: `crates/simple-lector-dni/src/model.rs`

**Step 1:** Write failing tests for a `ReadRecord` containing schema version 1, UUID, offset-aware timestamp, reader, source, integrity, and all documented DG13 fields.

**Step 2:** Run the focused model tests and verify missing-type failures.

**Step 3:** Implement serialisable domain types with deterministic field names and empty defaults for optional DG13 text.

**Step 4:** Run model tests and the complete workspace test suite.

**Step 5:** Commit with `feat: define versioned read record`.

### Task 3: Implement the insertion state machine and retries

**Files:**
- Create: `crates/simple-lector-dni/src/lifecycle.rs`
- Test: `crates/simple-lector-dni/src/lifecycle.rs`

**Step 1:** Write separate failing tests for first insertion, duplicate-present events, removal and reinsertion, three-attempt success, exhausted retries, and reader detachment recovery.

**Step 2:** Run each new test and confirm the expected missing-behaviour failure.

**Step 3:** Implement a small pure state machine with `NoReader`, `Empty`, `Reading`, `Delivered`, and `Failed` states, plus a reusable retry function capped at three attempts.

**Step 4:** Run lifecycle tests, refactor duplicated transitions, then run all tests.

**Step 5:** Commit with `feat: model card lifecycle and retries`.

### Task 4: Implement output sinks

**Files:**
- Create: `crates/simple-lector-dni/src/output/mod.rs`
- Create: `crates/simple-lector-dni/src/output/files.rs`
- Create: `crates/simple-lector-dni/src/output/csv_sink.rs`
- Create: `crates/simple-lector-dni/src/output/webhook.rs`
- Test: corresponding Rust module tests

**Step 1:** Write failing tests for atomic latest JSON, append-only JSONL, single CSV header, one row per record, RFC 4180 quoting, CSV formula protection, private file permissions, HTTPS enforcement, loopback HTTP allowance, timeout, idempotency header, bearer token, and sink failure isolation.

**Step 2:** Run focused tests and observe expected failures before each implementation slice.

**Step 3:** Implement a `Sink` trait and the minimal sink implementations, keeping serialisation centralised.

**Step 4:** Use a local test HTTP server for webhook behaviour and synthetic identity data for all fixtures.

**Step 5:** Run output tests and the complete suite.

**Step 6:** Commit with `feat: add composable output sinks`.

### Task 5: Implement PC/SC monitoring and reader selection

**Files:**
- Create: `crates/simple-lector-dni/src/reader.rs`
- Test: `crates/simple-lector-dni/src/reader.rs`

**Step 1:** Write failing tests for case-insensitive substring selection, ambiguity, missing readers, and mapping raw PC/SC flags to lifecycle events.

**Step 2:** Run tests and verify the expected failures.

**Step 3:** Implement reader listing and status-change monitoring with `pcsc`, including the PC/SC plug-and-play notification reader.

**Step 4:** Keep operating-system calls behind a `ReaderMonitor` trait so lifecycle integration tests use deterministic events.

**Step 5:** Run reader tests, all tests, and `cargo clippy --workspace --all-targets -- -D warnings`.

**Step 6:** Commit with `feat: monitor PCSC readers and cards`.

### Task 6: Build the isolated JMultiCard worker

**Files:**
- Create: `engine/jmulticard-worker/pom.xml`
- Create: `engine/jmulticard-worker/src/main/java/es/cofrentes/simplelectordni/Worker.java`
- Create: `engine/jmulticard-worker/src/main/java/es/cofrentes/simplelectordni/Dg13Reader.java`
- Create: `engine/jmulticard-worker/src/test/java/es/cofrentes/simplelectordni/WorkerProtocolTest.java`
- Add: `vendor/jmulticard` pinned Git submodule

**Step 1:** Write a failing worker protocol test for valid synthetic JSON and structured, non-sensitive errors.

**Step 2:** Run the Maven test and verify the expected missing-class failure.

**Step 3:** Implement one-request/one-response JSON on standard input/output, explicit T=0, reader selection by index, user CWA channel, and DG13-only extraction.

**Step 4:** Add focused SOD code that validates its signature and compares only the DG13 hash without loading DG2 or DG7.

**Step 5:** Run worker tests and use source inspection to assert that production code contains no `getDg2()` or `getDg7()` call.

**Step 6:** Commit with `feat: add isolated JMultiCard DG13 engine`.

### Task 7: Connect Rust to the engine and expose the CLI

**Files:**
- Create: `crates/simple-lector-dni/src/engine.rs`
- Create: `crates/simple-lector-dni/src/cli.rs`
- Modify: `crates/simple-lector-dni/src/main.rs`
- Test: Rust integration and module tests

**Step 1:** Write failing tests for engine process success, timeout, invalid JSON, nonzero exit, secret-safe errors, `once`, `watch`, `list-readers`, reader selection, and combined sinks.

**Step 2:** Run focused tests and verify each failure is caused by missing behaviour.

**Step 3:** Implement engine discovery relative to the executable with an explicit development override, bounded execution, and redacted error mapping.

**Step 4:** Implement `clap` commands and connect monitor events to lifecycle, engine reads, retries, record creation, and independent sink delivery.

**Step 5:** Run all tests, formatter, and clippy.

**Step 6:** Commit with `feat: expose complete reader CLI`.

### Task 8: Package cross-platform releases

**Files:**
- Create: `scripts/build-worker.sh`
- Create: `scripts/package-release.sh`
- Create: `scripts/package-release.ps1`
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`

**Step 1:** Add a failing packaging smoke check that requires the expected binary, worker JARs, licenses, and reduced runtime layout.

**Step 2:** Implement reproducible JMultiCard and worker builds from the pinned source.

**Step 3:** Use `jlink` to create the smallest practical runtime containing the smart-card, crypto, HTTP, logging, and base modules required by the worker.

**Step 4:** Build ZIP artifacts for Windows x64, macOS x64, and macOS ARM64 in GitHub Actions.

**Step 5:** Run CI locally where possible, push, and verify all GitHub Actions jobs rather than assuming cross-platform success.

**Step 6:** Commit with `ci: package self-contained releases`.

### Task 9: Document operation, integration, and research

**Files:**
- Create: `README.md`
- Create: `docs/INTEGRATION.md`
- Create: `docs/RESEARCH.md`
- Create: `docs/PRIVACY.md`
- Create: `docs/COMPATIBILITY.md`
- Create: `CONTRIBUTING.md`
- Create: `SECURITY.md`
- Create: `CHANGELOG.md`

**Step 1:** Add executable documentation examples to a shell smoke script using synthetic engine output.

**Step 2:** Document quick start, commands, exact schema, CSV behaviour, webhook examples, exit codes, event semantics, extension points, and native Rust engine replacement.

**Step 3:** Document the verified reader and DNIe combination separately from unverified compatibility claims.

**Step 4:** Document consent, minimisation, access control, retention, TLS, and incident handling without presenting the text as legal advice.

**Step 5:** Run link, secret, and PII scans plus the documentation smoke test.

**Step 6:** Commit with `docs: add usage integration and research guides`.

### Task 10: Perform physical and independent review

**Files:**
- Create: `docs/MANUAL_TESTS.md`
- Modify only as required by verified findings

**Step 1:** Run the complete Rust and Java tests, formatter, clippy, build, packaging smoke checks, secret scan, and PII scan.

**Step 2:** With the authorised DNI inserted, run `list-readers`, `once`, JSON, JSONL, CSV, webhook-to-loopback, and a short `watch` removal/reinsertion test.

**Step 3:** Verify that no DG2 or DG7 access occurred, no PIN was requested, no APDU or identity data reached logs, and all generated files remain ignored.

**Step 4:** Request independent code and test review. Fix every critical and important issue with a failing regression test first.

**Step 5:** Create the public `balneariodecofrentes/SimpleLectorDNI` repository, push the reviewed branch, and verify the public page and CI status.

**Step 6:** Tag the release only after cross-platform jobs pass and attach the generated ZIPs.
