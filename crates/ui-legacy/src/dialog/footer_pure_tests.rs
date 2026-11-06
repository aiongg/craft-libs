//! The button order, the key rules and the Apply gesture handling, without a frame.

use egui::Context;

use super::*;
use crate::edit::{GestureHost, UndoMarks};

fn labels(roles: &[ButtonRole], platform: Platform) -> Vec<ButtonRole> {
    let listed: Vec<(ButtonRole, ())> = roles.iter().map(|r| (*r, ())).collect();
    order(&listed, platform).into_iter().map(|i| roles[i]).collect()
}

#[test]
fn buttons_follow_the_platform_order() {
    use ButtonRole::{Alternate, Apply, Cancel, Default};
    let listed = [Apply, Cancel, Alternate, Default];
    assert_eq!(labels(&listed, Platform::Other), [Default, Alternate, Cancel, Apply]);
    assert_eq!(labels(&listed, Platform::Mac), [Alternate, Cancel, Apply, Default]);
    let two = [(Alternate, "No"), (Default, "Yes"), (Alternate, "Maybe")];
    let names = |p| order(&two, p).into_iter().map(|i| two[i].1).collect::<Vec<_>>();
    assert_eq!(names(Platform::Other), ["Yes", "No", "Maybe"], "alternates keep their listed order");
    assert_eq!(names(Platform::Mac), ["No", "Maybe", "Yes"]);
}

fn keys() -> KeyInput {
    KeyInput {
        enter: false,
        escape: false,
        modal: true,
        active: true,
        footer: true,
        popup_open: false,
        button_focused: false,
        default_enabled: true,
        enter_rule: EnterRule::Default,
    }
}

#[test]
fn enter_and_esc_resolve_by_the_key_table() {
    let enter = KeyInput { enter: true, ..keys() };
    let esc = KeyInput { escape: true, ..keys() };
    assert_eq!(resolve_keys(enter), Some(Outcome::Ok));
    assert_eq!(resolve_keys(esc), Some(Outcome::Cancel));
    assert_eq!(resolve_keys(KeyInput { enter: true, escape: true, ..keys() }), Some(Outcome::Cancel), "Esc wins");

    for (what, input) in [
        ("a popup is open", KeyInput { popup_open: true, ..enter }),
        ("a footer button has the focus", KeyInput { button_focused: true, ..enter }),
        ("the default is disabled", KeyInput { default_enabled: false, ..enter }),
        ("the body owns Enter", KeyInput { enter_rule: EnterRule::Body, ..enter }),
        ("the dialog is not on top", KeyInput { active: false, ..enter }),
    ] {
        assert_eq!(resolve_keys(input), None, "{what}");
    }
    assert_eq!(resolve_keys(KeyInput { popup_open: true, ..esc }), None, "Esc closes the popup, not the dialog");
    assert_eq!(resolve_keys(KeyInput { button_focused: true, ..esc }), Some(Outcome::Cancel));
}

#[test]
fn a_modeless_dialog_closes_on_esc_only_with_a_footer() {
    let esc = KeyInput { modal: false, escape: true, ..keys() };
    assert_eq!(resolve_keys(esc), Some(Outcome::Close));
    assert_eq!(resolve_keys(KeyInput { footer: false, ..esc }), None);
}

#[derive(Default)]
struct Doc {
    value: i32,
    undo: Vec<i32>,
}

impl Doc {
    fn set(&mut self, value: i32) {
        self.undo.push(self.value);
        self.value = value;
    }
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&v) = self.undo.get(mark) {
            self.value = v;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

#[test]
fn cancel_after_apply_keeps_what_was_applied() {
    let ctx = Context::default();
    let (mut doc, mut host, mut state) = (Doc::default(), GestureHost::new(), DialogState::<()>::new());
    state.begin(&ctx, (), true);
    let first = state.outer().expect("an outer gesture");
    host.begin_outer(&ctx, &mut doc, first);
    doc.set(1);

    state.apply(&ctx, &mut host, &mut doc);
    let second = state.outer().expect("a new gesture");
    assert_ne!(first, second);
    assert_eq!(doc.undo, [0], "the applied edit is one undo step");
    doc.set(2);
    assert!(host.cancel(&mut doc, second.id()));
    assert_eq!(doc.value, 1);
}
