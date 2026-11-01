//! The page range field and the From/To row.

use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::range::*;
use crate::range::{PageRange, Subset};

struct FieldApp {
    state: PageRangeFieldState,
    selected: Vec<usize>,
    menu: Vec<MenuEntry<Vec<usize>>>,
}

fn field(selected: Vec<usize>) -> Harness<'static, FieldApp> {
    let menu: Vec<MenuEntry<Vec<usize>>> = vec![MenuItem::new("Odd pages", vec![0, 2, 4]).into()];
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(400.0, 200.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut FieldApp| {
                let out = PageRangeField::new("range", &mut app.state, &app.selected, 6, 0).menu(&app.menu).show(ui);
                if let Some(pages) = out.picked {
                    app.selected = pages;
                }
            },
            FieldApp { state: PageRangeFieldState::default(), selected, menu },
        )
}

fn text(harness: &Harness<'_, FieldApp>) -> Option<String> {
    harness.get_by_role_and_label(Role::TextInput, "Selected pages").accesskit_node().value()
}

// Typing "5,1-3" and Enter returns [0, 1, 2, 4] and shows "1-3, 5"; "1-x" keeps the text with the error under it; Esc restores the selection's text.
#[test]
fn typed_pages_apply_on_enter_and_show_back_formatted() {
    let mut harness = field(vec![5]);
    harness.run();
    assert_eq!(text(&harness).as_deref(), Some("6"));

    harness.get_by_role_and_label(Role::TextInput, "Selected pages").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_role_and_label(Role::TextInput, "Selected pages").type_text("5,1-3");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().selected, vec![0, 1, 2, 4]);
    assert_eq!(text(&harness).as_deref(), Some("1-3, 5"));

    harness.get_by_role_and_label(Role::TextInput, "Selected pages").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_role_and_label(Role::TextInput, "Selected pages").type_text("1-x");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().selected, vec![0, 1, 2, 4], "the selection stays");
    assert_eq!(text(&harness).as_deref(), Some("1-x"));
    assert!(harness.query_by_label_contains("is not a page number").is_some(), "the error shows under the field");

    harness.get_by_role_and_label(Role::TextInput, "Selected pages").focus();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(text(&harness).as_deref(), Some("1-3, 5"));
    assert!(harness.query_by_label_contains("is not a page number").is_none());
}

// A pick from the chevron menu returns its pages and shows them formatted.
#[test]
fn menu_pick_returns_like_typed_pages() {
    let mut harness = field(Vec::new());
    harness.run();
    harness.get_by_label("Select pages").click();
    harness.run();
    harness.get_by_label("Odd pages").click();
    harness.run();
    assert_eq!(harness.state().selected, vec![0, 2, 4]);
    assert_eq!(text(&harness).as_deref(), Some("1, 3, 5"));
}

struct RowApp {
    range: PageRange,
}

fn row() -> Harness<'static, RowApp> {
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(500.0, 200.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut RowApp| {
                PageRangeOptions::new("row", &mut app.range, 8).show(ui);
            },
            RowApp { range: PageRange::default() },
        )
}

// With All pages the From and To fields are disabled; choosing Pages from enables them, clamped to the count.
#[test]
fn from_to_row_disables_with_all_pages() {
    let mut harness = row();
    harness.run();
    assert!(harness.get_by_role_and_label(Role::SpinButton, "Pages from").accesskit_node().is_disabled());
    harness.get_by_role_and_label(Role::RadioButton, "Pages from").click();
    harness.run();
    assert!(!harness.state().range.all);
    let to = harness.get_by_role_and_label(Role::SpinButton, "to");
    assert!(!to.accesskit_node().is_disabled());
    assert_eq!(to.accesskit_node().numeric_value(), Some(8.0), "the open end shows the last page");
    assert_eq!(harness.state().range.subset, Subset::All);
}
