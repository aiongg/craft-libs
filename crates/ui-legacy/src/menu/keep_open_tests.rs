//! Keep-open rows in a harness.
use egui::{Id, Key, Popup, SetOpenCommand};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, install};

#[derive(Default)]
struct State {
    on: bool,
    open: bool,
    chosen: Vec<&'static str>,
}

/// A button "Open" with a menu of a keep-open check row, a plain row, a keep-open plain row and
/// a submenu with a keep-open check row.
fn harness() -> Harness<'static, State> {
    let mut harness = Harness::builder().with_size([400.0, 300.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, state: &mut State| {
            let entries: Vec<MenuEntry<&'static str>> = vec![
                MenuItem::new("Show Cropped Area", "crop").check(state.on).keep_open(true).into(),
                MenuItem::new("Plain", "plain").into(),
                MenuItem::new("Keep open but plain", "stay").keep_open(true).into(),
                MenuItem::submenu("More", vec![MenuItem::new("Show Grid", "grid").check(false).keep_open(true).into()]).into(),
            ];
            let button = ui.add(crate::controls::Button::new("Open"));
            let toggle = button.clicked_by(egui::PointerButton::Primary).then_some(SetOpenCommand::Toggle);
            let shown = Menu::new(&entries).show(ui, Popup::menu(&button).open_memory(toggle));
            state.open = shown.open;
            if let Some(command) = shown.chosen {
                state.chosen.push(command);
                if command == "crop" {
                    state.on = !state.on;
                }
            }
        },
        State::default(),
    );
    harness.run();
    harness
}

#[test]
fn a_keep_open_check_row_runs_and_leaves_the_menu_open() {
    let mut harness = harness();
    harness.get_by_label("Open").click();
    harness.run();
    harness.get_by_label("Show Cropped Area").click();
    harness.run();
    harness.get_by_label("Show Cropped Area").click();
    harness.run();
    let state = harness.state();
    assert_eq!(state.chosen, ["crop", "crop"], "each click runs the command");
    assert!(state.open, "the menu stays open");
    assert!(!state.on, "the host's setting follows the clicks");
}

#[test]
fn keys_on_a_keep_open_row_leave_the_menu_open() {
    let mut harness = harness();
    harness.get_by_label("Open").click();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().chosen, ["crop"]);
    assert!(harness.state().open, "Enter keeps it open");
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(harness.state().chosen, ["crop", "crop"]);
    assert!(harness.state().open, "Space keeps it open");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().open, "Esc closes it");
}

#[test]
fn other_rows_still_close_the_menu() {
    let mut harness = harness();
    harness.get_by_label("Open").click();
    harness.run();
    harness.get_by_label("Keep open but plain").click();
    harness.run();
    assert_eq!(harness.state().chosen, ["stay"]);
    assert!(!harness.state().open, "keep_open on a plain row does nothing");
    harness.get_by_label("Open").click();
    harness.run();
    harness.get_by_label("Plain").click();
    harness.run();
    assert!(!harness.state().open, "a plain row closes the menu");
}

#[test]
fn a_mnemonic_on_a_keep_open_row_runs_it_and_leaves_the_menu_open() {
    let mut harness = harness();
    harness.get_by_label("Open").click();
    harness.run();
    harness.key_press(Key::S);
    harness.run();
    assert_eq!(harness.state().chosen, ["crop"], "S is the first row's letter");
    assert!(harness.state().open, "the mnemonic keeps the menu open");
}

#[test]
fn a_keep_open_row_in_a_submenu_leaves_both_levels_open() {
    let mut harness = harness();
    harness.get_by_label("Open").click();
    harness.run();
    harness.get_by_label("More").click();
    harness.run();
    harness.get_by_label("Show Grid").click();
    harness.run();
    assert_eq!(harness.state().chosen, ["grid"]);
    assert!(harness.state().open, "the menu stays open");
    assert!(harness.query_by_label("Show Grid").is_some(), "the submenu stays open");
}

#[derive(Default)]
struct Rename {
    /// The pass in which the Rename row ran, once it has.
    asked: bool,
    /// Whether the field held the focus at the start of every pass after the one that asked.
    kept: Vec<bool>,
    width: f32,
}

/// A press menu whose Rename row asks a text field, drawn after the menu, for the focus in the
/// pass the row runs.
fn rename_harness() -> Harness<'static, Rename> {
    let mut harness = Harness::builder()
        .with_size([400.0, 300.0])
        .with_accessibility_check(false)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, state: &mut Rename| {
                let field = Id::unique("rename-field");
                if state.asked {
                    state.kept.push(ui.memory(|m| m.focused()) == Some(field));
                }
                let entries: Vec<MenuEntry<&'static str>> = vec![MenuItem::new("Rename", "rename").into(), MenuItem::new("Delete", "delete").into()];
                let button = ui.add(crate::controls::Button::new("Gear"));
                let chosen = PressMenu::new(&entries).min_width(300.0).show(ui, &button).chosen;
                let mut text = String::new();
                let edit = ui.add(egui::TextEdit::singleline(&mut text).id(field));
                if chosen == Some("rename") {
                    state.asked = true;
                    edit.request_focus();
                }
            },
            Rename::default(),
        );
    harness.run();
    harness
}

#[test]
fn a_row_that_asks_for_the_focus_keeps_it_in_the_pass_it_runs_and_after() {
    let mut harness = rename_harness();
    harness.get_by_label("Gear").click();
    harness.run();
    state_width(&mut harness);
    harness.get_by_label("Rename").click();
    harness.run();
    let state = harness.state();
    assert!(state.asked);
    assert!(!state.kept.is_empty() && state.kept.iter().all(|&k| k), "the field keeps the focus after the row ran: {:?}", state.kept);
}

fn state_width(harness: &mut Harness<'_, Rename>) {
    let width = harness.get_by_label("Rename").rect().width();
    harness.state_mut().width = width;
}

#[test]
fn a_press_menu_takes_its_least_width() {
    let mut harness = rename_harness();
    harness.get_by_label("Gear").click();
    harness.run();
    state_width(&mut harness);
    assert_eq!(harness.state().width, 300.0);
}
