//! `TitleRow`: lead and action parts wear faces, report a value and an action, and form one roving stop.

use craft_behavior::PartKey;
use craft_ui::contract::{Bind, Props as _, Role, Show as _};
use craft_ui::sections::{HeaderAction, HeaderStrength, TitleOutput, TitleRow};
use craft_ui::testing::focus::{active_part, egui_focused, key_release};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Frames, trial};
use egui::{Id, Key, Modifiers, Pos2, Rect, Ui};
use std::cell::RefCell;

const ROW: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 40.0) };
const ACTION: PartKey = PartKey::new("action");
const LEAD: PartKey = PartKey::new("lead");

#[derive(Default)]
struct Out {
    id: Option<Id>,
    action: Option<HeaderAction>,
    changed: bool,
}

fn frame(ui: &mut Ui, value: &mut bool, gate: bool, out: &RefCell<Out>) {
    let row = TitleRow::new("Options").strength(HeaderStrength::Strong).actions([HeaderAction::Menu, HeaderAction::Close]).id_salt("row");
    let row = if gate { row.gate(Bind::bound(value)) } else { row.disclosure(Bind::bound(value)) };
    let TitleOutput { edit, action, .. } = row.show_at(ui, ROW);
    let mut out = out.borrow_mut();
    out.id = Some(edit.response.id);
    out.changed |= edit.event.is_some();
    out.action = out.action.or(action);
}

fn side() -> f32 {
    ROW.height()
}

#[trial]
fn a_title_rows_lead_toggles_its_value_and_an_action_part_is_reported() {
    for gate in [false, true] {
        let mut frames = Frames::new();
        let (mut value, out) = (false, RefCell::new(Out::default()));
        frames.idle(2, |ui| frame(ui, &mut value, gate, &out));
        let lead = Pos2::new(ROW.left() + side() / 2.0, ROW.center().y);
        click(&mut frames, lead, &mut |ui| frame(ui, &mut value, gate, &out));
        assert!(value && out.borrow().changed, "a click on the lead toggles the value and reports the edit (gate: {gate})");
        assert_eq!(out.borrow().action, None, "the lead is not an action");
        let close = Pos2::new(ROW.right() - side() / 2.0, ROW.center().y);
        click(&mut frames, close, &mut |ui| frame(ui, &mut value, gate, &out));
        assert_eq!(out.borrow().action, Some(HeaderAction::Close), "the last action part is Close (gate: {gate})");
        assert!(value, "an action leaves the value");
    }
}

#[trial]
fn a_title_rows_lead_and_actions_are_one_roving_stop() {
    let mut frames = Frames::new();
    let (mut value, out) = (false, RefCell::new(Out::default()));
    let mut show = |ui: &mut Ui, value: &mut bool| {
        frame(ui, value, false, &out);
        ui.add_space(4.0);
    };
    frames.idle(2, |ui| show(ui, &mut value));
    let tap = |frames: &mut Frames, key: Key, value: &mut bool| {
        frames.frame(vec![key_press(key, Modifiers::NONE)], |ui| show(ui, value));
        frames.frame(vec![key_release(key, Modifiers::NONE)], |ui| show(ui, value));
    };
    tap(&mut frames, Key::Tab, &mut value);
    let id = out.borrow().id.expect("the row showed");
    assert!(egui_focused(&frames.ctx).is_some(), "Tab reaches the row");
    assert_eq!(active_part(&frames.ctx, id), Some(LEAD.into()), "Tab arrives at the lead");
    tap(&mut frames, Key::ArrowRight, &mut value);
    assert_eq!(active_part(&frames.ctx, id), Some(ACTION.at(0)), "an arrow moves to the first action");
    tap(&mut frames, Key::Enter, &mut value);
    assert_eq!(out.borrow().action, Some(HeaderAction::Menu), "Enter takes the focused action");
    tap(&mut frames, Key::Tab, &mut value);
    assert_ne!(active_part(&frames.ctx, id), Some(ACTION.at(1)), "Tab leaves the row instead of moving to the next part");
}

#[trial]
fn a_title_row_is_a_heading_with_a_level_and_its_title_as_name() {
    let mut frames = Frames::new();
    let (mut value, out) = (false, RefCell::new(Out::default()));
    frames.idle(2, |ui| frame(ui, &mut value, false, &out));
    let id = out.borrow().id.expect("the row showed");
    let tree = frames.tree();
    assert_eq!((tree.role(id), tree.name(id)), (Some(Role::HEADING), Some("Options")));
}
