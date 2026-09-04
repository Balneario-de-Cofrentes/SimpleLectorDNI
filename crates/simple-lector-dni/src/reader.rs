use std::collections::BTreeMap;
use std::thread;
use std::time::Duration;

use pcsc::{Context, ReaderState, Scope, State};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderPresence {
    Empty,
    Present,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderInfo {
    pub index: usize,
    pub name: String,
    pub presence: ReaderPresence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReaderEvent {
    ReaderAttached(ReaderInfo),
    ReaderDetached(ReaderInfo),
    CardInserted(ReaderInfo),
    CardRemoved(ReaderInfo),
}

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("PC/SC error: {0}")]
    Pcsc(#[from] pcsc::Error),
    #[error("no smart-card readers are connected")]
    NoReaders,
    #[error("reader matching '{0}' was not found")]
    NotFound(String),
    #[error("reader pattern '{0}' is ambiguous")]
    Ambiguous(String),
}

pub trait ReaderMonitor {
    fn initialise(&mut self) -> Result<Vec<ReaderInfo>, ReaderError>;
    fn wait_for_events(&mut self, delay: Duration) -> Result<Vec<ReaderEvent>, ReaderError>;
}

pub struct PcscMonitor {
    context: Context,
    previous: Vec<ReaderInfo>,
}

impl PcscMonitor {
    pub fn new() -> Result<Self, ReaderError> {
        Ok(Self {
            context: Context::establish(Scope::User)?,
            previous: Vec::new(),
        })
    }

    fn read_snapshot(&self) -> Result<Vec<ReaderInfo>, ReaderError> {
        let names = match self.context.list_readers_owned() {
            Ok(names) => names,
            Err(pcsc::Error::NoReadersAvailable) => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut states: Vec<_> = names
            .iter()
            .map(|name| ReaderState::new(name.as_c_str(), State::UNAWARE))
            .collect();
        self.context
            .get_status_change(Duration::ZERO, &mut states)?;
        Ok(to_reader_info(&states))
    }
}

impl ReaderMonitor for PcscMonitor {
    fn initialise(&mut self) -> Result<Vec<ReaderInfo>, ReaderError> {
        let current = self.read_snapshot()?;
        self.previous.clone_from(&current);
        Ok(current)
    }

    fn wait_for_events(&mut self, delay: Duration) -> Result<Vec<ReaderEvent>, ReaderError> {
        thread::sleep(delay);
        let current = self.read_snapshot()?;
        let events = diff_snapshots(&self.previous, &current);
        self.previous = current;
        Ok(events)
    }
}

fn to_reader_info(states: &[ReaderState]) -> Vec<ReaderInfo> {
    states
        .iter()
        .enumerate()
        .map(|(index, state)| ReaderInfo {
            index,
            name: state.name().to_string_lossy().into_owned(),
            presence: classify_state(state.event_state()),
        })
        .collect()
}

#[must_use]
pub fn classify_state(state: State) -> ReaderPresence {
    if state.intersects(State::UNAVAILABLE | State::UNKNOWN | State::IGNORE) {
        ReaderPresence::Unavailable
    } else if state.contains(State::PRESENT) {
        ReaderPresence::Present
    } else {
        ReaderPresence::Empty
    }
}

pub fn select_reader(
    readers: &[ReaderInfo],
    pattern: Option<&str>,
) -> Result<ReaderInfo, ReaderError> {
    let Some(pattern) = pattern else {
        return readers.first().cloned().ok_or(ReaderError::NoReaders);
    };
    let pattern_lower = pattern.to_lowercase();
    let matches: Vec<_> = readers
        .iter()
        .filter(|reader| reader.name.to_lowercase().contains(&pattern_lower))
        .cloned()
        .collect();
    selected_match(matches, pattern)
}

fn selected_match(mut matches: Vec<ReaderInfo>, pattern: &str) -> Result<ReaderInfo, ReaderError> {
    match matches.len() {
        0 => Err(ReaderError::NotFound(pattern.to_owned())),
        1 => Ok(matches.remove(0)),
        _ => Err(ReaderError::Ambiguous(pattern.to_owned())),
    }
}

#[must_use]
pub fn diff_snapshots(previous: &[ReaderInfo], current: &[ReaderInfo]) -> Vec<ReaderEvent> {
    let previous_by_name = by_name(previous);
    let current_by_name = by_name(current);
    let mut events = attached_and_changed(&previous_by_name, current);
    events.extend(detached(&current_by_name, previous));
    events
}

fn by_name(readers: &[ReaderInfo]) -> BTreeMap<&str, &ReaderInfo> {
    readers
        .iter()
        .map(|reader| (reader.name.as_str(), reader))
        .collect()
}

fn attached_and_changed(
    previous: &BTreeMap<&str, &ReaderInfo>,
    current: &[ReaderInfo],
) -> Vec<ReaderEvent> {
    let mut events = Vec::new();
    for reader in current {
        match previous.get(reader.name.as_str()) {
            None => push_attached(&mut events, reader),
            Some(old) => push_presence_change(&mut events, old, reader),
        }
    }
    events
}

fn push_attached(events: &mut Vec<ReaderEvent>, reader: &ReaderInfo) {
    events.push(ReaderEvent::ReaderAttached(reader.clone()));
    if reader.presence == ReaderPresence::Present {
        events.push(ReaderEvent::CardInserted(reader.clone()));
    }
}

fn push_presence_change(events: &mut Vec<ReaderEvent>, old: &ReaderInfo, new: &ReaderInfo) {
    match (old.presence, new.presence) {
        (ReaderPresence::Present, ReaderPresence::Empty) => {
            events.push(ReaderEvent::CardRemoved(new.clone()));
        }
        (ReaderPresence::Empty, ReaderPresence::Present) => {
            events.push(ReaderEvent::CardInserted(new.clone()));
        }
        _ => {}
    }
}

fn detached(current: &BTreeMap<&str, &ReaderInfo>, previous: &[ReaderInfo]) -> Vec<ReaderEvent> {
    previous
        .iter()
        .filter(|reader| !current.contains_key(reader.name.as_str()))
        .cloned()
        .map(ReaderEvent::ReaderDetached)
        .collect()
}
