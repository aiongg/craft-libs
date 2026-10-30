//! The bars on a kittest harness: context menu, rename bar, toolbar, footer and empty states.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::theme::{Base, install};

struct Host {
    rows: Vec<PresetRow<u32>>,
    state: PresetState<u32>,
    events: Vec<PresetEvent<u32>>,
    footer: Vec<FooterEvent>,
    time: f64,
}

/// Group 100 with presets 1 to 3, group 200 with preset 4.
fn library() -> Vec<PresetRow<u32>> {
    let mut rows = vec![PresetRow::group(100, "Round", 0)];
    rows.extend((1..=3).map(|n| PresetRow::item(n, format!("Brush {n}"), 1)));
    rows.push(PresetRow::group(200, "Square", 0));
    rows.push(PresetRow::item(4, "Hard", 1));
    rows
}

fn show(ui: &mut Ui, host: &mut Host) {
    let Host { rows, state, events, footer, .. } = host;
    let out = PresetBrowser::new("presets", "Presets", state, rows).toolbar(true).show(ui, |painter, rect, _| {
        painter.rect_filled(rect, 0.0, egui::Color32::RED);
    });
    events.extend(out.events);
    footer.extend(PresetFooter::new(state, &PresetLabels::default()).count(4).show(ui));
}

fn harness(rows: Vec<PresetRow<u32>>) -> Harness<'static, Host> {
    let host = Host { rows, state: PresetState::new(), events: Vec::new(), footer: Vec::new(), time: 0.0 };
    let mut harness = Harness::<Host>::builder()
        .with_size([320.0, 360.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn frame(harness: &mut Harness<'_, Host>, events: Vec<Event>) {
    harness.state_mut().time += 1.0;
    let time = harness.state().time;
    harness.input_mut().time = Some(time);
    harness.input_mut().events.extend(events);
    harness.run();
}

fn click_with(harness: &mut Harness<'_, Host>, pos: Pos2, button: PointerButton) {
    let at = |pressed| Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE };
    frame(harness, vec![Event::PointerMoved(pos), at(true), at(false)]);
}

fn click(harness: &mut Harness<'_, Host>, pos: Pos2) {
    click_with(harness, pos, PointerButton::Primary);
}

fn press(harness: &mut Harness<'_, Host>, key: Key) {
    let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    frame(harness, vec![event(true), event(false)]);
}

fn type_text(harness: &mut Harness<'_, Host>, text: &str) {
    frame(harness, vec![Event::Text(text.to_owned())]);
}

fn center(harness: &Harness<'_, Host>, label: &str) -> Pos2 {
    let label = label.to_owned();
    harness.get_by(move |n| n.label().as_deref() == Some(label.as_str())).rect().center()
}

fn menu_item(harness: &Harness<'_, Host>, label: &str) -> Pos2 {
    let label = label.to_owned();
    harness.get_by(move |n| n.role() == Role::MenuItem && n.label().as_deref() == Some(label.as_str())).rect().center()
}

fn present(harness: &Harness<'_, Host>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn take(harness: &mut Harness<'_, Host>) -> Vec<PresetEvent<u32>> {
    std::mem::take(&mut harness.state_mut().events)
}

fn cell(harness: &Harness<'_, Host>, label: &str) -> Pos2 {
    let label = label.to_owned();
    harness.get_by(move |n| matches!(n.role(), Role::TreeItem | Role::ListBoxOption) && n.label().as_deref() == Some(label.as_str())).rect().center()
}

#[test]
fn the_context_menu_deletes_and_moves_a_preset() {
    let mut h = harness(library());
    let at = cell(&h, "Brush 2");
    click_with(&mut h, at, PointerButton::Secondary);
    assert!(present(&h, "Rename…") && present(&h, "Delete") && present(&h, "Move to"));
    let pos = menu_item(&h, "Delete");
    click(&mut h, pos);
    assert_eq!(take(&mut h), vec![PresetEvent::Delete(2)]);

    click_with(&mut h, at, PointerButton::Secondary);
    let pos = menu_item(&h, "Move to");
    click(&mut h, pos);
    assert!(h.query_all_by_role_and_label(Role::MenuItem, "Round").next().is_none(), "the item's own group is not a target");
    let pos = menu_item(&h, "Square");
    click(&mut h, pos);
    assert_eq!(take(&mut h), vec![PresetEvent::MoveTo { key: 2, group: 200 }]);
}

#[test]
fn rename_from_the_menu_commits_with_enter() {
    let mut h = harness(library());
    let at = cell(&h, "Brush 2");
    click_with(&mut h, at, PointerButton::Secondary);
    let pos = menu_item(&h, "Rename…");
    click(&mut h, pos);
    assert!(h.query_by_role(Role::TextInput).is_some(), "the rename bar is open");
    type_text(&mut h, "Soft");
    press(&mut h, Key::Enter);
    assert_eq!(take(&mut h), vec![PresetEvent::Rename { key: 2, text: "Soft".to_owned() }]);
    assert!(h.query_by_role(Role::TextInput).is_none(), "the bar closed");
}

#[test]
fn f2_renames_the_lead_and_escape_cancels() {
    let mut h = harness(library());
    let at = cell(&h, "Brush 1");
    click(&mut h, at);
    take(&mut h);
    press(&mut h, Key::F2);
    type_text(&mut h, "Gone");
    press(&mut h, Key::Escape);
    assert!(take(&mut h).is_empty());
    assert!(h.query_by_role(Role::TextInput).is_none());

    press(&mut h, Key::F2);
    type_text(&mut h, "Fresh");
    press(&mut h, Key::Enter);
    assert_eq!(take(&mut h), vec![PresetEvent::Rename { key: 1, text: "Fresh".to_owned() }]);
}

#[test]
fn the_search_field_filters_and_the_view_buttons_switch_the_view() {
    let mut h = harness(library());
    let search = h.get_by_role(Role::SearchInput).rect().center();
    click(&mut h, search);
    type_text(&mut h, "hard");
    assert!(present(&h, "Hard") && !present(&h, "Brush 1"));
    assert_eq!(h.state().state.filter, "hard");
    type_text(&mut h, "x");
    assert!(present(&h, "No presets match"));

    let list = center(&h, "List View");
    click(&mut h, list);
    assert_eq!(h.state().state.view, PresetView::List);
    let grid = center(&h, "Grid View");
    click(&mut h, grid);
    assert_eq!(h.state().state.view, PresetView::Grid);
}

#[test]
fn footer_buttons_report_and_the_empty_library_says_so() {
    let mut h = harness(Vec::new());
    assert!(present(&h, "No presets"));
    for (label, event) in [("New", FooterEvent::New), ("New Group", FooterEvent::NewGroup), ("Delete", FooterEvent::Delete)] {
        let pos = center(&h, label);
        click(&mut h, pos);
        assert_eq!(std::mem::take(&mut h.state_mut().footer), vec![event], "{label}");
    }
}
