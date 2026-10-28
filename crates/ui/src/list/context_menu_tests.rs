//! A list's context menu in the Reorder story: it opens at the pointer or at the lead row, and the
//! list keeps the focus.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::drag::show_flat;
use crate::direction::{self, Direction};
use crate::story::Args;
use crate::theme::{Base, install};

fn harness() -> Harness<'static, Args> {
    harness_in(Direction::Ltr)
}

fn harness_in(dir: Direction) -> Harness<'static, Args> {
    let mut harness = Harness::<Args>::builder()
        .with_size([420.0, 420.0])
        .with_step_dt(0.05)
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(show_flat, Args::new());
    harness.run();
    harness
}

fn row_rect(harness: &Harness<'_, Args>, label: &str) -> egui::Rect {
    harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(label)).rect()
}

fn menu_item_rect(harness: &Harness<'_, Args>, label: &str) -> Option<egui::Rect> {
    harness.query_by(|n| matches!(n.role(), Role::MenuItem | Role::MenuItemCheckBox) && n.label().as_deref() == Some(label)).map(|n| n.rect())
}

fn list_focused(harness: &Harness<'_, Args>) -> bool {
    harness.get_by_role(Role::ListBox).accesskit_node().active_descendant().is_some()
}

/// Whether egui's keyboard focus is on `list`; AccessKit's focus is on the open menu.
fn has_focus(harness: &Harness<'_, Args>, list: egui::Id) -> bool {
    harness.ctx.memory(|m| m.has_focus(list))
}

fn secondary_click(harness: &mut Harness<'_, Args>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        harness.run();
    }
    harness.run_steps(3);
}

fn key(harness: &mut Harness<'_, Args>, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        harness.event(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
    }
    harness.run_steps(3);
}

#[test]
fn a_row_menu_opens_at_the_pointer_or_the_lead_and_the_list_keeps_the_focus() {
    let mut harness = harness();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(list_focused(&harness), "Tab reaches the list");
    let list = harness.ctx.memory(|m| m.focused()).expect("the list's focus");
    let layer_2 = row_rect(&harness, "Layer 2");
    let pointer = Pos2::new(layer_2.left() + 30.0, layer_2.center().y);
    secondary_click(&mut harness, pointer);
    let rename = menu_item_rect(&harness, "Rename").expect("the menu opens on a secondary click");
    assert!((rename.left() - pointer.x).abs() < 24.0 && (rename.top() - pointer.y).abs() < 24.0, "at the pointer: {rename:?} for {pointer:?}");

    assert!(has_focus(&harness, list), "the list keeps the focus while the menu is open");
    key(&mut harness, Key::Escape, Modifiers::NONE);
    assert!(menu_item_rect(&harness, "Rename").is_none(), "Escape closes it");
    assert!(has_focus(&harness, list), "egui focus");
    assert!(list_focused(&harness), "AccessKit focus");

    let lead = harness.get_by_role(Role::ListBox).accesskit_node().active_descendant().and_then(|n| n.label()).expect("lead");
    let lead_rect = row_rect(&harness, &lead);
    key(&mut harness, Key::F10, Modifiers::SHIFT);
    let rename = menu_item_rect(&harness, "Rename").expect("Shift+F10 opens it");
    assert!(
        (rename.left() - lead_rect.left()).abs() < 24.0 && (rename.top() - lead_rect.bottom()).abs() < 24.0,
        "under the lead row: {rename:?} for {lead_rect:?}"
    );

    let duplicate = menu_item_rect(&harness, "Duplicate").expect("Duplicate").center();
    harness.event(Event::PointerMoved(duplicate));
    harness.run();
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos: duplicate, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        harness.run();
    }
    harness.run_steps(3);
    assert!(
        harness.query_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Layer 1 copy")).is_some(),
        "choosing Duplicate reports the command"
    );
    assert!(menu_item_rect(&harness, "Rename").is_none(), "and closes the menu");
}

#[allow(clippy::expect_used)]
fn lead(harness: &Harness<'_, Args>) -> String {
    harness.get_by_role(Role::ListBox).accesskit_node().active_descendant().and_then(|n| n.label()).expect("lead")
}

#[test]
fn an_open_menu_takes_the_arrows_and_enter_from_the_list() {
    let mut harness = harness();
    harness.key_press(Key::Tab);
    harness.run();
    let before = lead(&harness);
    key(&mut harness, Key::F10, Modifiers::SHIFT);
    assert!(menu_item_rect(&harness, "Rename").is_some(), "Shift+F10 opens the menu");
    key(&mut harness, Key::ArrowDown, Modifiers::NONE);
    key(&mut harness, Key::Enter, Modifiers::NONE);
    assert!(
        harness.query_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Layer 1 copy")).is_some(),
        "↓ and Enter chose the second item, Duplicate"
    );
    assert_eq!(lead(&harness), before, "the lead row stayed");
}

#[test]
fn a_keyboard_menu_opens_at_the_reading_start_of_the_row_right_to_left() {
    let mut harness = harness_in(Direction::Rtl);
    harness.key_press(Key::Tab);
    harness.run();
    let lead_rect = row_rect(&harness, &lead(&harness));
    key(&mut harness, Key::F10, Modifiers::SHIFT);
    let rename = menu_item_rect(&harness, "Rename").expect("Shift+F10 opens it");
    assert!((rename.right() - lead_rect.right()).abs() < 24.0, "at the row's right edge: {rename:?} for {lead_rect:?}");
}
