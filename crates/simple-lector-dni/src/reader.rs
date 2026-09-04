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
    pub event_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReaderEvent {
    ReaderAttached(ReaderInfo),
    ReaderDetached(ReaderInfo),
    CardInserted(ReaderInfo),
    CardRemoved(ReaderInfo),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionChange {
    Unchanged,
    Selected,
    Deselected,
}

#[derive(Debug)]
pub(crate) struct ReaderSelection {
    pattern: Option<String>,
    selected: Option<ReaderInfo>,
}

impl ReaderSelection {
    pub(crate) fn new(readers: &[ReaderInfo], pattern: Option<&str>) -> Result<Self, ReaderError> {
        let selected = optional_reader(readers, pattern)?;
        Ok(Self {
            pattern: pattern.map(str::to_lowercase),
            selected,
        })
    }

    pub(crate) fn selected(&self) -> Option<&ReaderInfo> {
        self.selected.as_ref()
    }

    pub(crate) fn selected_name(&self) -> Option<&str> {
        self.selected().map(|reader| reader.name.as_str())
    }

    pub(crate) fn is_selected(&self, reader: &ReaderInfo) -> bool {
        self.selected_name() == Some(reader.name.as_str())
    }

    pub(crate) fn update(&mut self, event: &ReaderEvent) -> SelectionChange {
        match event {
            ReaderEvent::ReaderAttached(reader)
                if self.selected.is_none() && self.matches(reader) =>
            {
                self.selected = Some(reader.clone());
                SelectionChange::Selected
            }
            ReaderEvent::ReaderDetached(reader) if self.is_selected(reader) => {
                self.selected = None;
                SelectionChange::Deselected
            }
            _ => SelectionChange::Unchanged,
        }
    }

    fn matches(&self, reader: &ReaderInfo) -> bool {
        self.pattern
            .as_ref()
            .is_none_or(|pattern| reader.name.to_lowercase().contains(pattern))
    }
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
            event_count: state.event_count(),
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

fn optional_reader(
    readers: &[ReaderInfo],
    pattern: Option<&str>,
) -> Result<Option<ReaderInfo>, ReaderError> {
    match select_reader(readers, pattern) {
        Ok(reader) => Ok(Some(reader)),
        Err(ReaderError::NoReaders | ReaderError::NotFound(_)) => Ok(None),
        Err(error) => Err(error),
    }
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
    if old.presence == ReaderPresence::Present && new.presence != ReaderPresence::Present {
        events.push(ReaderEvent::CardRemoved(new.clone()));
    } else if old.presence != ReaderPresence::Present && new.presence == ReaderPresence::Present {
        events.push(ReaderEvent::CardInserted(new.clone()));
    } else if old.presence == ReaderPresence::Present && old.event_count != new.event_count {
        events.push(ReaderEvent::CardRemoved(new.clone()));
        events.push(ReaderEvent::CardInserted(new.clone()));
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
