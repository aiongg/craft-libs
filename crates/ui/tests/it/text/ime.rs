//! The IME engine and the text adapter: preedit, one commit, reset at pass end, middle-click paste.

use super::kit::Rig;
use craft_ui::testing::pointer::middle;
use craft_ui::testing::trial;
use egui::{Event, ImeEvent};

fn preedit(text: &str) -> Event {
    Event::Ime(ImeEvent::Preedit { text: text.into(), active_range_chars: None })
}

/// The preedit shows inline, commits once as typed text, and the pass writes the engine's IME output.
#[trial]
fn ime_preedit_shows_commits_once_and_the_pass_writes_the_ime_output() {
    let mut rig = Rig::new("hello");
    rig.focus();
    rig.frame(vec![preedit("ni")]);
    rig.idle(1);
    assert_eq!(rig.text, "hello", "the preedit is not text yet");
    let owner = rig.frames.ime().map(|ime| ime.owner);
    assert_eq!(owner, Some(rig.widget()), "the engine's output names the composing part");
    let events_before = rig.seen.events.len();
    rig.frame(vec![Event::Ime(ImeEvent::Commit("ni".into()))]);
    rig.idle(3);
    assert_eq!(rig.text, "helloni", "the commit became text once");
    assert!(rig.seen.events.len() > events_before, "and edited the value");
    rig.idle(3);
    assert_eq!(rig.text, "helloni", "a later pass adds nothing");
}

/// A middle click pastes the primary selection once.
#[trial]
fn middle_click_paste_pastes_once() {
    let mut rig = Rig::new("ab");
    rig.focus();
    let at = rig.seen.rect.left_center() + egui::vec2(1.0, 0.0);
    rig.frame(vec![Event::PointerMoved(at)]);
    rig.frame(vec![middle(at, true), middle(at, false), Event::MiddleClickPaste { pos: at, text: "X".into() }]);
    rig.idle(4);
    assert_eq!(rig.text.matches('X').count(), 1, "pasted once: {:?}", rig.text);
}
