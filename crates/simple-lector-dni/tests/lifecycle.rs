use simple_lector_dni::lifecycle::{
    CardLifecycle, LifecycleAction, LifecycleEvent, LifecycleState, run_with_retries,
};

#[test]
fn first_insertion_starts_exactly_one_read() {
    let mut lifecycle = CardLifecycle::new(true);

    assert_eq!(
        lifecycle.handle(LifecycleEvent::CardInserted),
        LifecycleAction::StartRead
    );
    assert_eq!(lifecycle.state(), LifecycleState::Reading);
    assert_eq!(
        lifecycle.handle(LifecycleEvent::CardInserted),
        LifecycleAction::None
    );
}

#[test]
fn removal_and_reinsertion_starts_a_new_read() {
    let mut lifecycle = CardLifecycle::new(true);
    lifecycle.handle(LifecycleEvent::CardInserted);
    lifecycle.handle(LifecycleEvent::ReadSucceeded);

    assert_eq!(lifecycle.state(), LifecycleState::Delivered);
    lifecycle.handle(LifecycleEvent::CardRemoved);
    assert_eq!(lifecycle.state(), LifecycleState::Empty);
    assert_eq!(
        lifecycle.handle(LifecycleEvent::CardInserted),
        LifecycleAction::StartRead
    );
}

#[test]
fn exhausted_read_waits_for_card_removal() {
    let mut lifecycle = CardLifecycle::new(true);
    lifecycle.handle(LifecycleEvent::CardInserted);
    lifecycle.handle(LifecycleEvent::ReadFailed);

    assert_eq!(lifecycle.state(), LifecycleState::Failed);
    assert_eq!(
        lifecycle.handle(LifecycleEvent::CardInserted),
        LifecycleAction::None
    );
    lifecycle.handle(LifecycleEvent::CardRemoved);
    assert_eq!(lifecycle.state(), LifecycleState::Empty);
}

#[test]
fn reader_detachment_and_attachment_recovers_to_empty() {
    let mut lifecycle = CardLifecycle::new(true);
    lifecycle.handle(LifecycleEvent::ReaderDetached);

    assert_eq!(lifecycle.state(), LifecycleState::NoReader);
    assert_eq!(
        lifecycle.handle(LifecycleEvent::ReaderAttached),
        LifecycleAction::None
    );
    assert_eq!(lifecycle.state(), LifecycleState::Empty);
}

#[test]
fn retry_succeeds_on_the_third_attempt() {
    let mut attempts = 0;
    let mut retry_notifications = Vec::new();

    let result = run_with_retries(
        3,
        |attempt| {
            attempts += 1;
            if attempt < 3 {
                Err("transient")
            } else {
                Ok("ok")
            }
        },
        |_| true,
        |attempt, _| retry_notifications.push(attempt),
    );

    assert_eq!(result.unwrap(), "ok");
    assert_eq!(attempts, 3);
    assert_eq!(retry_notifications, vec![1, 2]);
}

#[test]
fn retry_returns_the_last_error_after_three_attempts() {
    let result = run_with_retries(3, |_| Err::<(), _>("still failing"), |_| true, |_, _| {});
    let failure = result.unwrap_err();

    assert_eq!(failure.attempts, 3);
    assert_eq!(failure.last_error, "still failing");
}

#[test]
fn retry_stops_after_a_non_retryable_error() {
    let mut attempts = 0;
    let result = run_with_retries(
        3,
        |_| {
            attempts += 1;
            Err::<(), _>("unsupported card")
        },
        |_| false,
        |_, _| panic!("must not announce a retry"),
    );

    assert_eq!(attempts, 1);
    assert_eq!(result.unwrap_err().attempts, 1);
}
