//! `FieldListPane`: ticks, field menus, drags between areas.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::{FieldArea, FieldListEdit, FieldListPane, FieldSort, PivotField};
use craft_ui::contract::Role;
use craft_ui::testing::access::name;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Event, Modifiers, PointerButton};

struct App {
    fields: Vec<PivotField>,
    areas: Vec<(FieldArea, Vec<String>)>,
    edits: Vec<FieldListEdit>,
}

fn rig() -> (Fixture, Rc<RefCell<App>>) {
    let field = |name: &str, numeric| PivotField::new(name, numeric);
    let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
    let app = Rc::new(RefCell::new(App {
        fields: vec![field("Region", false), field("Sales", true), field("Year", false), field("Product", false)],
        areas: vec![(FieldArea::Columns, names(&["Product"])), (FieldArea::Rows, names(&["Region", "Year"]))],
        edits: Vec::new(),
    }));
    let shown = app.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let out = FieldListPane::new(&app.fields, &app.areas).show(ui);
        for edit in out.edits {
            // The app applies a tick at once.
            match &edit {
                FieldListEdit::Add { field, .. } => app.areas[1].1.push(field.clone()),
                FieldListEdit::Remove { field, .. } => app.areas.iter_mut().for_each(|(_, fields)| fields.retain(|f| f != field)),
                _ => {}
            }
            app.edits.push(edit);
        }
    });
    fixture.settle();
    (fixture, app)
}

/// The nodes of `role` named `label`, top to bottom.
fn named(fixture: &Fixture, role: Role, label: &str) -> Vec<Target> {
    let mut found: Vec<Target> = fixture.by_role(role).into_iter().filter(|target| name(fixture.tree(), target.node) == Some(label)).collect();
    found.sort_by(|a, b| a.rect.top().total_cmp(&b.rect.top()));
    found
}

/// The field's row in an area: the lower of the rows named after it (the checklist's is above).
fn area_row(fixture: &Fixture, field: &str) -> Target {
    named(fixture, Role::LIST_ITEM, field).pop().expect("the field's row in an area")
}

fn secondary_click(fixture: &mut Fixture, at: egui::Pos2) {
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    fixture.frame(vec![Event::PointerMoved(at)]);
    fixture.frame(vec![button(true)]);
    fixture.frame(vec![button(false)]);
    fixture.idle(2);
}

/// Ticks report Add and Remove.
#[trial]
fn ticking_adds_and_unticking_removes() {
    let (mut fixture, app) = rig();
    let sales = named(&fixture, Role::CHECK_BOX, "Sales").pop().expect("a check for Sales");
    fixture.click(&sales);
    fixture.idle(2);
    let region = named(&fixture, Role::CHECK_BOX, "Region").pop().expect("a check for Region");
    fixture.click(&region);
    fixture.idle(2);
    let edits = &app.borrow().edits;
    let [FieldListEdit::Add { field: added, .. }, FieldListEdit::Remove { field: removed, from, .. }] = &edits[..] else {
        panic!("an add then a remove: {edits:?}")
    };
    assert_eq!((added.as_str(), removed.as_str(), *from), ("Sales", "Region", None));
}

/// Move to Columns reports one move of the field.
#[trial]
fn field_menus_move_between_areas() {
    let (mut fixture, app) = rig();
    let row = area_row(&fixture, "Region").center();
    secondary_click(&mut fixture, row);
    let item = fixture.by_label("Move to Columns").expect("the menu names the area");
    fixture.click(&item);
    fixture.idle(2);
    let app = app.borrow();
    let [FieldListEdit::Move { to, change, .. }] = &app.edits[..] else { panic!("one move: {:?}", app.edits) };
    assert_eq!(*to, FieldArea::Columns);
    let mv = change.value().expect("a one-shot move");
    assert_eq!(mv.fields, ["Region".to_string()]);
}

/// A drag from one area onto a field of another ends as a move into the second, before that field.
#[trial]
fn dragging_a_field_onto_another_area_moves_it() {
    let (mut fixture, app) = rig();
    let from = area_row(&fixture, "Region");
    let over = area_row(&fixture, "Product");
    fixture.drag(&from, over.center() - from.center());
    fixture.idle(2);
    let app = app.borrow();
    let moved = app.edits.iter().find_map(|edit| match edit {
        FieldListEdit::Move { to: FieldArea::Columns, change, .. } => change.value(),
        _ => None,
    });
    let mv = moved.expect("a move into Columns");
    assert_eq!((mv.fields.as_slice(), mv.position), (&["Region".to_string()][..], 0));
}

/// Remove Field from an area's menu names the area.
#[trial]
fn remove_field_names_its_area() {
    let (mut fixture, app) = rig();
    let row = area_row(&fixture, "Region").center();
    secondary_click(&mut fixture, row);
    let item = fixture.by_label("Remove Field").expect("the menu removes the field");
    fixture.click(&item);
    fixture.idle(2);
    let edits = &app.borrow().edits;
    let [FieldListEdit::Remove { field, from, .. }] = &edits[..] else { panic!("one removal: {edits:?}") };
    assert_eq!((field.as_str(), *from), ("Region", Some(FieldArea::Rows)));
}

/// The menu of a field outside Values has Sort and Group, and Sort reports the order.
#[trial]
fn the_field_menu_sorts_and_groups() {
    let (mut fixture, app) = rig();
    let row = area_row(&fixture, "Region").center();
    secondary_click(&mut fixture, row);
    assert!(fixture.by_label("Group by Quarters").is_some(), "Group is in the menu");
    let item = fixture.by_label("Sort Z to A").expect("Sort is in the menu");
    fixture.click(&item);
    fixture.idle(2);
    let edits = &app.borrow().edits;
    let [FieldListEdit::Sort { field, order, .. }] = &edits[..] else { panic!("one sort: {edits:?}") };
    assert_eq!((field.as_str(), *order), ("Region", FieldSort::Descending));
}
