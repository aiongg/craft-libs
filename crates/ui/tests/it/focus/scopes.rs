//! Focus scopes: initial focus, restore, trapping, panes, programmatic requests and focus after removal.

use super::{control, focus, focused, id, tap};
use craft_behavior::focus::EscapeStep;
use craft_behavior::{CancelRule, DocKey, Input, PartKey};
use craft_ui::runtime::focus::{FocusPolicy, FocusTarget, KeyUse, request};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::focus::{Stop, begin_body, pop_scope, push_scope, register_doc, register_stop, scope_escape, take_reveal, widget};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Ui};
use std::cell::Cell;
use std::sync::Arc;

fn dialog(ui: &mut Ui, fields: &[&str]) {
    push_scope(ui.ctx(), id("dialog"), FocusPolicy::DIALOG, None);
    for field in fields {
        control(ui, field);
    }
    pop_scope(ui.ctx());
}

/// Tab visits a container's body stops, then its footer's, then its header's; initial focus takes the first body stop.
#[trial]
fn tab_visits_the_body_then_the_footer_then_the_header() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        push_scope(ui.ctx(), id("dialog"), FocusPolicy::DIALOG, None);
        control(ui, "close");
        begin_body(ui.ctx());
        control(ui, "first");
        control(ui, "last");
        control(ui, "ok");
        pop_scope(ui.ctx());
    };
    frames.idle(2, &mut show);
    frames.frame(Vec::new(), &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("first")), "initial focus takes the first body stop");
    for next in ["last", "ok", "close", "first"] {
        tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
        assert_eq!(focused(&frames.ctx), Some(id(next)));
    }
}

/// Spec test row "a dialog opened by a key gets initial focus on its first visible frame", and the restore
/// rule: focus returns to the opener when the scope disappears.
#[trial]
fn a_dialog_takes_initial_focus_on_its_first_frame_and_returns_focus_when_it_closes() {
    let mut frames = Frames::new();
    let open = Cell::new(false);
    let mut show = |ui: &mut Ui| {
        control(ui, "opener");
        if open.get() {
            dialog(ui, &["first", "second"]);
        }
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "opener", &mut show);
    open.set(true);
    frames.frame(Vec::new(), &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("first")), "initial focus on the first frame the scope is pushed");
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("second")));
    frames.idle(3, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("second")), "initial focus runs once");
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("first")), "a trapping scope wraps and never reaches the opener");
    open.set(false);
    frames.idle(2, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("opener")), "restore returns focus to the opener");
}

/// 9.4: Escape goes to the scope's rule only when no component took it, and one scope takes each Escape.
#[trial]
fn an_untaken_escape_goes_to_the_scope_rule_once() {
    let mut frames = Frames::new();
    let steps = Cell::new((None, None));
    let mut show = |ui: &mut Ui| {
        dialog(ui, &["field"]);
        steps.set((scope_escape(ui.ctx(), CancelRule::Close, false, false), scope_escape(ui.ctx(), CancelRule::Close, false, false)));
    };
    frames.idle(2, &mut show);
    frames.frame(Vec::new(), &mut show);
    assert_eq!(steps.get(), (None, None), "no Escape, no step");
    frames.frame(vec![craft_ui::testing::input::key_press(Key::Escape, Modifiers::NONE)], &mut show);
    assert_eq!(steps.get(), (Some(EscapeStep::CloseScope), None));
}

/// 9.4: F6 cycles the pane scopes and announces each pane's label.
#[trial]
fn f6_cycles_panes_and_announces_each() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        for (pane, field, label) in [("left", "a", "Layers"), ("right", "b", "Properties")] {
            push_scope(ui.ctx(), id(pane), FocusPolicy::PANE, Some(Arc::from(label)));
            control(ui, field);
            pop_scope(ui.ctx());
        }
    };
    frames.idle(2, &mut show);
    tap(&mut frames, Key::F6, Modifiers::NONE, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("a")));
    tap(&mut frames, Key::F6, Modifiers::NONE, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("b")));
    tap(&mut frames, Key::F6, Modifiers::NONE, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("a")), "F6 wraps");
    tap(&mut frames, Key::F6, Modifiers::SHIFT, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("b")), "Shift+F6 goes back");
    assert_eq!(take_output(&frames.ctx).announcements, ["Layers", "Properties", "Layers", "Properties"]);
}

/// Spec test row "`runtime::focus::request` focuses an id, a part and a doc key in the next pass and reveals
/// it".
#[trial]
fn request_focuses_an_id_a_part_and_a_doc_key_and_reveals_it() {
    const KNOB: PartKey = PartKey::new("knob");
    const FIND: DocKey = DocKey::new("craft-ui.find");
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        control(ui, "plain");
        widget(ui, id("pair"));
        for part in 0..2 {
            register_stop(ui.ctx(), Stop::new(id("pair"), KeyUse::of(Input::Pointer)).part(KNOB.at(part)));
        }
        control(ui, "find");
        register_doc(ui.ctx(), FIND, id("find"));
    };
    frames.idle(2, &mut show);
    request(&frames.ctx, FocusTarget::Id(id("plain")));
    assert_eq!(focused(&frames.ctx), None, "the request applies at the next pass's begin hook");
    frames.frame(Vec::new(), &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("plain")));
    assert_eq!(take_reveal(&frames.ctx), Some((id("plain"), None)), "the target is revealed");
    request(&frames.ctx, FocusTarget::Part(id("pair"), KNOB.at(1)));
    frames.frame(Vec::new(), &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("pair")));
    assert_eq!(craft_ui::testing::focus::active_part(&frames.ctx, id("pair")), Some(KNOB.at(1)));
    request(&frames.ctx, FocusTarget::Doc(FIND));
    frames.frame(Vec::new(), &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("find")));
}

/// Spec test row "when the focused component is not shown, focus moves to the next stop in its scope, else
/// the previous".
#[trial]
fn focus_moves_to_the_next_stop_when_the_focused_one_is_not_shown_else_the_previous() {
    let mut frames = Frames::new();
    let hidden = Cell::new(None);
    let mut show = |ui: &mut Ui| {
        for name in ["one", "two", "three"] {
            if hidden.get() != Some(name) {
                control(ui, name);
            }
        }
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "two", &mut show);
    frames.idle(1, &mut show);
    hidden.set(Some("two"));
    frames.idle(2, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("three")), "the stop after the removed one");
    hidden.set(Some("three"));
    frames.idle(2, &mut show);
    assert_eq!(focused(&frames.ctx), Some(id("one")), "no stop after it: the one before");
}
