//! A list's context menu in the Reorder story: it opens at the pointer or at the lead row, and the
//! list keeps the focus.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::drag::show_flat;
use super::{ContextMenuRequest, List, ListEvent, ListRow, RowCommand, ToggleColumn};
use crate::behavior::RowInfo;
use crate::direction::{self, Direction};
use crate::menu::MenuItem;
use crate::story::NoArgs;
use crate::theme::{Base, install};

fn harness() -> Harness<'static, NoArgs> {
    harness_in(Direction::Ltr)
}

fn harness_in(dir: Direction) -> Harness<'static, NoArgs> {
    let mut harness = Harness::<NoArgs>::builder()
        .with_size([420.0, 420.0])
        .with_step_dt(0.05)
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(show_flat, NoArgs {});
    harness.run();
    harness
}

fn row_rect(harness: &Harness<'_, NoArgs>, label: &str) -> egui::Rect {
    harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(label)).rect()
}

fn menu_item_rect(harness: &Harness<'_, NoArgs>, label: &str) -> Option<egui::Rect> {
    harness.query_by(|n| matches!(n.role(), Role::MenuItem | Role::MenuItemCheckBox) && n.label().as_deref() == Some(label)).map(|n| n.rect())
}

fn list_focused(harness: &Harness<'_, NoArgs>) -> bool {
    harness.get_by_role(Role::ListBox).accesskit_node().active_descendant().is_some()
}

/// Whether egui's keyboard focus is on `list`; AccessKit's focus is on the open menu.
fn has_focus(harness: &Harness<'_, NoArgs>, list: egui::Id) -> bool {
    harness.ctx.memory(|m| m.has_focus(list))
}

fn secondary_click(harness: &mut Harness<'_, NoArgs>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        harness.run();
    }
    harness.run_steps(3);
}

fn key<S>(harness: &mut Harness<'_, S>, key: Key, modifiers: Modifiers) {
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
fn lead(harness: &Harness<'_, NoArgs>) -> String {
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

type Requests = (Vec<RowInfo<usize>>, Vec<ContextMenuRequest<usize>>);

// A secondary click on a row's second toggle cell asks for the menu with `toggle: Some(1)`, the
// row and its rect; on the label `toggle: None`; Shift+F10 on the lead `toggle: None`.
#[test]
fn a_secondary_click_on_a_toggle_cell_names_its_column() {
    let mut harness = Harness::<Requests>::builder()
        .with_size([420.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, (rows, requests): &mut Requests| {
                let out = List::new("list", "Layers", rows.as_slice()).show(ui, |_, &key| {
                    ListRow::new(key, format!("Row {key}"))
                        .toggle(ToggleColumn::eye("Visibility {row}", true))
                        .toggle(ToggleColumn::lock("Lock {row}", false))
                });
                for event in out.events {
                    if let ListEvent::ContextMenu(request) = event {
                        requests.push(request);
                    }
                }
            },
            ((0..3).map(RowInfo::leaf).collect(), Vec::new()),
        );
    harness.run();
    let lock = harness.get_by_label("Lock Row 1").rect();
    let asked = |harness: &mut Harness<'_, Requests>, pos| {
        harness.state_mut().1.clear();
        harness.event(Event::PointerMoved(pos));
        for pressed in [true, false] {
            harness.event(Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
            harness.run();
        }
        harness.run_steps(3);
        harness.state().1.clone()
    };
    let [request] = &asked(&mut harness, lock.center())[..] else { panic!("one request") };
    assert_eq!((request.row, request.toggle), (Some(1), Some(1)));
    assert!(request.anchor.contains(lock.center()), "the anchor is the row's rect");

    let label = harness.get_by_label("Row 1").rect().center();
    let [request] = &asked(&mut harness, label)[..] else { panic!("one request") };
    assert_eq!((request.row, request.toggle), (Some(1), None));

    harness.key_press(Key::Escape);
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    harness.state_mut().1.clear();
    key(&mut harness, Key::F10, Modifiers::SHIFT);
    let [request] = &harness.state().1[..] else { panic!("one request") };
    assert_eq!(request.toggle, None);
}

type Menus = (Vec<RowInfo<usize>>, Vec<ListEvent<usize>>);

fn menu_harness() -> Harness<'static, Menus> {
    let mut harness = Harness::<Menus>::builder()
        .with_size([420.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, (rows, log): &mut Menus| {
                let out = List::new("list", "Layers", rows.as_slice())
                    .can_rename(|_| true)
                    .row_menu(|_| vec![MenuItem::new("Rename", RowCommand::Rename).into(), MenuItem::new("Remove", RowCommand::Other(7)).into()])
                    .show(ui, |_, &key| ListRow::new(key, format!("Row {key}")));
                log.extend(out.events);
            },
            ((0..3).map(RowInfo::leaf).collect(), Vec::new()),
        );
    harness.run();
    harness
}

fn click(harness: &mut Harness<'_, Menus>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.run();
    harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
}

#[allow(clippy::expect_used)]
fn menu_item(harness: &Harness<'_, Menus>, label: &str) -> Pos2 {
    let item = harness.query_by(|n| n.role() == Role::MenuItem && n.label().as_deref() == Some(label));
    item.expect("the menu shows the entry").rect().center()
}

// A Rename pick starts the rename field in the pass of the click: the pass that ran the command
// already draws the field, so the host sees no event for it and no frame passes without it.
#[test]
fn choosing_rename_in_a_row_menu_shows_the_field_in_the_same_pass() {
    let mut harness = menu_harness();
    let more = harness.get_by_label("More (Row 1)").rect().center();
    click(&mut harness, more);
    harness.run();
    let rename = menu_item(&harness, "Rename");
    assert!(harness.query_by_role(Role::TextInput).is_none());
    click(&mut harness, rename);
    harness.step();
    assert!(harness.query_by_role(Role::TextInput).is_some(), "the pass of the release draws the field");
    assert!(harness.state().1.iter().all(|e| !matches!(e, ListEvent::RowMenu { .. })), "Rename is the list's: {:?}", harness.state().1);
}

#[test]
fn another_row_menu_entry_is_reported_with_its_row() {
    let mut harness = menu_harness();
    let more = harness.get_by_label("More (Row 2)").rect().center();
    click(&mut harness, more);
    harness.run();
    let remove = menu_item(&harness, "Remove");
    click(&mut harness, remove);
    harness.run_steps(3);
    assert!(harness.state().1.contains(&ListEvent::RowMenu { key: 2, command: RowCommand::Other(7) }), "{:?}", harness.state().1);
    assert!(harness.query_by_role(Role::TextInput).is_none());
}
