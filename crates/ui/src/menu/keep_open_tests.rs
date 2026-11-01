//! Keep-open rows in a harness.
use egui::{Key, Popup, SetOpenCommand};
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
