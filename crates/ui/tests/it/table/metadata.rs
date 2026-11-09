//! The metadata table: its filter, its check column and its rows.

use craft_ui::contract::{Bind, CollectionEvent};
use craft_ui::list::RowEvent;
use craft_ui::table::{FieldKey, MetadataGroup, MetadataRow, MetadataTable};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// What a fixture's table was given and reported, shared with its show closure.
#[derive(Default)]
struct Seen {
    filter: RefCell<String>,
    chosen: RefCell<Vec<String>>,
    checked: RefCell<usize>,
    events: RefCell<Vec<CollectionEvent<FieldKey, RowEvent>>>,
}

fn table(groups: Vec<MetadataGroup>, seen: &Rc<Seen>, with_filter: bool, with_checks: bool) -> Fixture {
    let seen = Rc::clone(seen);
    let mut fixture = Fixture::new(move |ui| {
        let mut filter = seen.filter.borrow().clone();
        let mut chosen = seen.chosen.borrow().clone();
        let mut table = MetadataTable::new("Metadata", &groups);
        if with_filter {
            table = table.filter(Bind::bound(&mut filter));
        }
        if with_checks {
            table = table.choose(Bind::bound(&mut chosen));
        }
        let out = table.show(ui);
        *seen.checked.borrow_mut() += out.checked.len();
        seen.events.borrow_mut().extend(out.table.events);
        *seen.filter.borrow_mut() = filter;
        *seen.chosen.borrow_mut() = chosen;
    });
    fixture.room(Vec2::new(360.0, 400.0)).settle();
    fixture
}

fn camera() -> Vec<MetadataGroup> {
    vec![MetadataGroup::new("Camera", vec![MetadataRow::new("Exposure", "1/250 s"), MetadataRow::new("ISO", "200")])]
}

/// Pressing the locked field's check and dragging down over the other rows leaves every field as it was: a locked
/// check starts no sweep.
#[trial]
fn dragging_from_the_locked_check_changes_no_field() {
    let rows = vec![MetadataRow::new("File", "IMG_0001").locked(), MetadataRow::new("Make", "Hasselblad"), MetadataRow::new("Model", "X2D")];
    let seen = Rc::new(Seen::default());
    *seen.chosen.borrow_mut() = vec!["Make".to_owned()];
    let mut fixture = table(vec![MetadataGroup::new("Camera", rows)], &seen, false, true);
    let locked = fixture.by_label("Show File").expect("the locked field has a check");
    let last = fixture.by_label("Show Model").expect("the last field has a check");
    fixture.drag(&locked, last.center() - locked.center());
    fixture.idle(2);
    assert_eq!(*seen.chosen.borrow(), ["Make"]);
    assert_eq!(*seen.checked.borrow(), 0);
}

/// A value that wraps in the value column makes its row taller than a row with a one-line value.
#[trial]
fn a_wrapping_value_makes_its_row_taller() {
    let long = "A free-text description that runs past the width of its column and wraps onto several lines";
    let groups = vec![MetadataGroup::new("Camera", vec![MetadataRow::new("Make", "Leica"), MetadataRow::new("Description", long)])];
    let fixture = table(groups, &Rc::new(Seen::default()), false, false);
    let short = fixture.by_label("Leica").expect("the one-line value is a cell");
    let wrapped = fixture.by_label(long).expect("the wrapping value is a cell");
    assert!(wrapped.rect.height() > short.rect.height(), "the wrapped row is taller than the one-line row");
}

/// ArrowDown in the filter field leads the first field in the rows while the filter keeps focus, and Enter activates
/// the lead.
#[trial]
fn arrow_down_from_the_filter_leads_into_the_rows() {
    let seen = Rc::new(Seen::default());
    let mut fixture = table(camera(), &seen, true, false);
    let filter = fixture.by_label("Filter fields").expect("the filter field");
    fixture.click(&filter);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(2);
    let focused = fixture.focused().and_then(|id| fixture.node_of(id)).expect("focus is somewhere");
    assert_eq!(focused.rect, filter.rect, "the filter keeps focus");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(1);
    let activated: Vec<_> =
        seen.events.borrow().iter().filter_map(|event| if let CollectionEvent::Activate(key) = event { Some(*key) } else { None }).collect();
    assert_eq!(activated, [FieldKey::Field(0, 0)]);
}
