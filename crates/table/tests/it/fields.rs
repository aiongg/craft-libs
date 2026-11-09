//! `FieldListPane`: ticks, field menus, drags between areas.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::{FieldArea, FieldListEdit, FieldListPane, FieldSort, PivotField};
use craft_ui::contract::Role;
use craft_ui::testing::access::name;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Event, Modifiers, PointerButton, Vec2};

/// The room the pane is shown in: tall enough for all four areas.
const ROOM: Vec2 = Vec2::new(420.0, 900.0);

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
    fixture.room(ROOM).settle();
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

fn moves_into(app: &Rc<RefCell<App>>, area: FieldArea) -> Option<(Vec<String>, usize)> {
    app.borrow().edits.iter().find_map(|edit| match edit {
        FieldListEdit::Move { to, change, .. } if *to == area => change.value().map(|mv| (mv.fields.clone(), mv.position)),
        _ => None,
    })
}

/// A field dropped into an empty area lands first in it.
#[trial]
fn dragging_a_field_into_an_empty_area_moves_it_there() {
    let (mut fixture, app) = rig();
    let from = area_row(&fixture, "Product");
    let area = named(&fixture, Role::LIST, "Filters").pop().expect("the empty Filters list");
    fixture.drag(&from, area.center() - from.center());
    fixture.idle(2);
    assert_eq!(moves_into(&app, FieldArea::Filters), Some((vec!["Product".to_string()], 0)));
}

/// A field dropped between two fields of an area lands between them.
#[trial]
fn dragging_a_field_between_two_fields_lands_between_them() {
    let (mut fixture, app) = rig();
    let from = area_row(&fixture, "Product");
    let (above, below) = (area_row(&fixture, "Region"), area_row(&fixture, "Year"));
    let gap = (above.rect.bottom() + below.rect.top()) / 2.0;
    let target = egui::pos2(above.rect.center().x, gap);
    fixture.drag(&from, target - from.center());
    fixture.idle(2);
    assert_eq!(moves_into(&app, FieldArea::Rows), Some((vec!["Product".to_string()], 1)));
}
