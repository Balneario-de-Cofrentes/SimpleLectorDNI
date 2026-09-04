#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleState {
    NoReader,
    Empty,
    Reading,
    Delivered,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleEvent {
    ReaderAttached,
    ReaderDetached,
    CardInserted,
    CardRemoved,
    ReadSucceeded,
    ReadFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleAction {
    None,
    StartRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CardLifecycle {
    state: LifecycleState,
}

impl CardLifecycle {
    #[must_use]
    pub fn new(reader_attached: bool) -> Self {
        let state = if reader_attached {
            LifecycleState::Empty
        } else {
            LifecycleState::NoReader
        };
        Self { state }
    }

    #[must_use]
    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn handle(&mut self, event: LifecycleEvent) -> LifecycleAction {
        let (state, action) = transition(self.state, event);
        self.state = state;
        action
    }
}

fn transition(state: LifecycleState, event: LifecycleEvent) -> (LifecycleState, LifecycleAction) {
    use LifecycleAction::{None, StartRead};
    use LifecycleEvent::{CardInserted, CardRemoved, ReadFailed, ReadSucceeded};
    use LifecycleState::{Delivered, Empty, Failed, NoReader, Reading};

    match (state, event) {
        (_, LifecycleEvent::ReaderDetached) => (NoReader, None),
        (NoReader, LifecycleEvent::ReaderAttached) => (Empty, None),
        (Empty, CardInserted) => (Reading, StartRead),
        (Reading, ReadSucceeded) => (Delivered, None),
        (Reading, ReadFailed) => (Failed, None),
        (Delivered | Failed | Reading, CardRemoved) => (Empty, None),
        _ => (state, None),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RetryFailure<E> {
    pub attempts: u8,
    pub last_error: E,
}

pub fn run_with_retries<T, E, Operation, Retryable, OnRetry>(
    max_attempts: u8,
    mut operation: Operation,
    is_retryable: Retryable,
    mut on_retry: OnRetry,
) -> Result<T, RetryFailure<E>>
where
    Operation: FnMut(u8) -> Result<T, E>,
    Retryable: Fn(&E) -> bool,
    OnRetry: FnMut(u8, &E),
{
    assert!(max_attempts > 0, "max_attempts must be greater than zero");
    let mut attempt = 1;
    loop {
        match operation(attempt) {
            Ok(value) => return Ok(value),
            Err(error) if attempt < max_attempts && is_retryable(&error) => {
                on_retry(attempt, &error);
                attempt += 1;
            }
            Err(last_error) => {
                return Err(RetryFailure {
                    attempts: attempt,
                    last_error,
                });
            }
        }
    }
}
