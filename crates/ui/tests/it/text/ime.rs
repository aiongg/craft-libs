//! The IME engine and the text adapter: preedit, one commit, reset at pass end, middle-click paste.

use super::kit::Rig;
use super::kit::TEXT;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::Frames;
use craft_ui::testing::pointer::{click, middle, moved, press, release};
use craft_ui::testing::text::{ime_preedit, widget_id};
use craft_ui::testing::trial;
use craft_ui::text::TextField;
use egui::{Event, ImeEvent};

fn preedit(text: &str) -> Event {
    ime_preedit(text)
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

/// Pressing another field while composing appends the preedit to the old field's text once, interrupts the OS
/// composition for the old owner and leaves the new owner's text alone.
#[trial]
fn clicking_elsewhere_while_composing_commits_the_preedit_once_and_interrupts_the_old_owner() {
    let mut frames = Frames::new();
    let (first, second) = (std::cell::RefCell::new("hello".to_owned()), std::cell::RefCell::new(String::new()));
    let seen = std::cell::Cell::new((egui::Rect::NOTHING, egui::Rect::NOTHING, egui::Id::NULL));
    let mut show = |ui: &mut egui::Ui| {
        let a = TextField::new("A", Bind::bound(&mut *first.borrow_mut())).id_salt("a").show(ui);
        let b = TextField::new("B", Bind::bound(&mut *second.borrow_mut())).id_salt("b").show(ui);
        seen.set((a.response.rect, b.response.rect, a.response.id));
    };
    frames.idle(3, &mut show);
    click(&mut frames, seen.get().0.center(), &mut show);
    frames.frame(vec![preedit("ni")], &mut show);
    frames.frame(vec![], &mut show);
    let old = widget_id(seen.get().2, TEXT);
    assert_eq!(frames.ime().map(|ime| ime.owner), Some(old), "the first field owns the IME while composing");
    assert_eq!(*first.borrow(), "hello", "the preedit is not text yet");

    let elsewhere = seen.get().1.center();
    let mut interrupted = false;
    frames.frame(vec![moved(elsewhere)], &mut show);
    for events in [vec![press(elsewhere)], vec![release(elsewhere)], vec![], vec![]] {
        frames.frame(events, &mut show);
        interrupted |= frames.ime().is_some_and(|ime| ime.interrupt && ime.owner == old);
    }
    assert!(interrupted, "the old owner's output interrupts the OS composition");
    assert_eq!(*first.borrow(), "helloni", "the composed text was committed once, into the composing field");
    assert!(second.borrow().is_empty(), "the new owner receives none of it");
}
