use simple_lector_dni::app::WatchController;
use simple_lector_dni::reader::{ReaderEvent, ReaderInfo, ReaderPresence};

fn reader(presence: ReaderPresence) -> ReaderInfo {
    ReaderInfo {
        index: 0,
        name: "Generic EMV Smartcard Reader".to_owned(),
        presence,
        event_count: 0,
    }
}

#[test]
fn present_card_is_read_once_until_removed_and_reinserted() {
    let present = reader(ReaderPresence::Present);
    let empty = reader(ReaderPresence::Empty);
    let mut controller = WatchController::new(std::slice::from_ref(&present), None).unwrap();

    assert_eq!(controller.initial_read(), Some(present.clone()));
    controller.read_succeeded();
    assert_eq!(
        controller.handle(ReaderEvent::CardInserted(present.clone())),
        None
    );
    assert_eq!(controller.handle(ReaderEvent::CardRemoved(empty)), None);
    assert_eq!(
        controller.handle(ReaderEvent::CardInserted(present.clone())),
        Some(present)
    );
}

#[test]
fn three_failed_attempts_wait_for_removal_before_a_new_cycle() {
    let present = reader(ReaderPresence::Present);
    let empty = reader(ReaderPresence::Empty);
    let mut controller = WatchController::new(std::slice::from_ref(&present), None).unwrap();

    assert!(controller.initial_read().is_some());
    controller.read_failed();
    assert_eq!(
        controller.handle(ReaderEvent::CardInserted(present.clone())),
        None
    );
    let _ = controller.handle(ReaderEvent::CardRemoved(empty));
    assert_eq!(
        controller.handle(ReaderEvent::CardInserted(present.clone())),
        Some(present)
    );
}

#[test]
fn reader_can_be_detached_and_reselected_after_reconnection() {
    let present = reader(ReaderPresence::Present);
    let mut controller = WatchController::new(std::slice::from_ref(&present), Some("emv")).unwrap();
    let _ = controller.initial_read();
    controller.read_succeeded();

    let _ = controller.handle(ReaderEvent::ReaderDetached(present.clone()));
    assert_eq!(controller.selected_name(), None);
    let _ = controller.handle(ReaderEvent::ReaderAttached(present.clone()));
    assert_eq!(
        controller.handle(ReaderEvent::CardInserted(present.clone())),
        Some(present)
    );
}
