//! Tests of the Properties panel session (task AD-5).

use super::AdjustSession;

// Tracked, edited, Reset: the start settings; Reset again: the defaults; a pass without tracking or another target takes the committed settings anew.
#[test]
fn adjust_session_reset_returns_to_the_values_found_when_edited_else_defaults() {
    let mut session = AdjustSession::new();
    session.track(7, 1, &10);
    session.track(7, 2, &20);
    assert_eq!(session.start(), None, "nothing is offered before an edit");
    session.edited();
    session.track(7, 3, &30);
    assert_eq!(session.start(), Some(&10), "following passes keep the settings found first");
    assert_eq!(session.reset(|| 0), 10, "edited: the start settings");
    assert!(!session.is_edited());
    assert_eq!(session.reset(|| 0), 0, "a second Reset gives the defaults");

    // A pass without tracking: the committed settings are taken anew and `edited` clears.
    session.edited();
    session.track(7, 9, &90);
    assert!(!session.is_edited());
    session.edited();
    assert_eq!(session.start(), Some(&90));

    // Another target does the same.
    session.track(8, 10, &5);
    assert!(!session.is_edited());
    session.edited();
    assert_eq!(session.start(), Some(&5));
}
