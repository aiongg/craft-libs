//! The field list.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::behavior::DropPosition;
use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Modifiers, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::fields::{FieldArea, FieldListEdit, FieldListPane, PivotField};

struct App {
    fields: Vec<PivotField>,
    areas: Vec<(FieldArea, Vec<String>)>,
    edits: Vec<FieldListEdit>,
}

fn show(ui: &mut Ui, app: &mut App) {
    let out = FieldListPane::new("fields", &app.fields, &app.areas).show(ui);
    for edit in out.edits {
        // The app applies a tick at once; a gesture's later events then find the state as it asks.
        match &edit {
            FieldListEdit::Add { field } => app.areas[0].1.push(field.clone()),
            FieldListEdit::Remove { field } => app.areas.iter_mut().for_each(|(_, fields)| fields.retain(|f| f != field)),
            _ => {}
        }
        app.edits.push(edit);
    }
}

fn harness() -> Harness<'static, App> {
    let field = |name: &str, numeric| PivotField { name: name.into(), numeric };
    let app = App {
        fields: vec![field("Region", false), field("Sales", true), field("Year", false)],
        areas: vec![(FieldArea::Rows, vec!["Region".into(), "Year".into()])],
        edits: Vec::new(),
    };
    let mut harness = Harness::<App>::builder()
        .with_size([420.0, 640.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, app);
    harness.run_steps(10);
    harness
}

// ticks report Add and Remove.
#[test]
fn ticking_adds_and_unticking_removes() {
    let mut harness = harness();
    harness.get_by_role_and_label(Role::CheckBox, "Sales").click();
    harness.run_steps(10);
    harness.get_by_role_and_label(Role::CheckBox, "Region").click();
    harness.run_steps(10);
    assert_eq!(harness.state().edits, [FieldListEdit::Add { field: "Sales".into() }, FieldListEdit::Remove { field: "Region".into() }]);
}

// Move to Columns reports Move.
#[test]
fn field_menus_move_between_areas() {
    let mut harness = harness();
    // The field's row in the Rows area is the lower of the two rows named after it (the checklist's is above).
    let rows = harness.get_all_by_role_and_label(Role::ListBoxOption, "Region");
    let row = rows.max_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).expect("the field's row in an area");
    row.click_secondary();
    harness.run_steps(10);
    harness.get_by_label("Move to Columns").click();
    harness.run_steps(10);
    let [FieldListEdit::Move { to, change }] = &harness.state().edits[..] else { panic!("one move: {:?}", harness.state().edits) };
    assert_eq!(*to, FieldArea::Columns);
    let mv = change.value().expect("a one-shot move");
    assert_eq!((mv.items.as_slice(), mv.position, mv.copy), (&["Region".to_string()][..], DropPosition::After, false));
}

// a drag from one area onto another is one gesture that ends as a move into the second.
#[test]
fn dragging_a_field_onto_another_area_moves_it() {
    let mut harness = harness();
    let rows = harness.get_all_by_role_and_label(Role::ListBoxOption, "Region");
    let from = rows.max_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).expect("the field's row in an area").rect().center();
    let to = harness.get_by_role_and_label(Role::ListBox, "Columns").rect().center();
    harness.drag(from, to, Modifiers::NONE);
    harness.run_steps(10);
    let moves: Vec<_> = harness
        .state()
        .edits
        .iter()
        .filter_map(|edit| match edit {
            FieldListEdit::Move { to, change } => Some((*to, change.clone())),
            _ => None,
        })
        .collect();
    let [(began, EditEvent::Begin { id, .. }), (area, EditEvent::Commit { id: ended, value, .. })] = &moves[..] else {
        panic!("begin, commit: {moves:?}")
    };
    assert_eq!((*began, *area, id), (FieldArea::Rows, FieldArea::Columns, ended));
    assert_eq!(value.items, ["Region".to_string()]);
}
