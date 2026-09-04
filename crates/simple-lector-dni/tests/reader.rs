use pcsc::State;
use simple_lector_dni::reader::{
    ReaderEvent, ReaderInfo, ReaderPresence, classify_state, diff_snapshots, select_reader,
};

fn reader(index: usize, name: &str, presence: ReaderPresence) -> ReaderInfo {
    ReaderInfo {
        index,
        name: name.to_owned(),
        presence,
    }
}

#[test]
fn selection_uses_first_reader_by_default() {
    let readers = vec![
        reader(0, "Reader Alpha", ReaderPresence::Empty),
        reader(1, "Reader Beta", ReaderPresence::Present),
    ];

    assert_eq!(select_reader(&readers, None).unwrap().index, 0);
}

#[test]
fn selection_matches_case_insensitive_substring() {
    let readers = vec![
        reader(0, "Reader Alpha", ReaderPresence::Empty),
        reader(1, "Generic EMV Smartcard Reader", ReaderPresence::Present),
    ];

    let selected = select_reader(&readers, Some("emv SMARTCARD")).unwrap();
    assert_eq!(selected.index, 1);
}

#[test]
fn selection_rejects_ambiguous_and_missing_patterns() {
    let readers = vec![
        reader(0, "Acme Reader A", ReaderPresence::Empty),
        reader(1, "Acme Reader B", ReaderPresence::Empty),
    ];

    assert!(
        select_reader(&readers, Some("acme"))
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
    assert!(
        select_reader(&readers, Some("missing"))
            .unwrap_err()
            .to_string()
            .contains("not found")
    );
}

#[test]
fn raw_pcsc_flags_are_classified_without_connecting_to_card() {
    assert_eq!(classify_state(State::PRESENT), ReaderPresence::Present);
    assert_eq!(classify_state(State::EMPTY), ReaderPresence::Empty);
    assert_eq!(
        classify_state(State::UNAVAILABLE | State::UNKNOWN),
        ReaderPresence::Unavailable
    );
}

#[test]
fn snapshot_diff_maps_reader_and_card_lifecycle_events() {
    let empty = reader(0, "Reader", ReaderPresence::Empty);
    let present = reader(0, "Reader", ReaderPresence::Present);

    assert_eq!(
        diff_snapshots(&[], std::slice::from_ref(&empty)),
        vec![ReaderEvent::ReaderAttached(empty.clone())]
    );
    assert_eq!(
        diff_snapshots(std::slice::from_ref(&empty), std::slice::from_ref(&present)),
        vec![ReaderEvent::CardInserted(present.clone())]
    );
    assert_eq!(
        diff_snapshots(std::slice::from_ref(&present), std::slice::from_ref(&empty)),
        vec![ReaderEvent::CardRemoved(empty.clone())]
    );
    assert_eq!(
        diff_snapshots(std::slice::from_ref(&empty), &[]),
        vec![ReaderEvent::ReaderDetached(empty)]
    );
}
