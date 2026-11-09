//! A toggle cell's node takes the cell's own name, and a toggle-column sweep flips each switch as the pointer crosses
//! it.

use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, EditEvent, ItemSource, Mixed};
use craft_ui::contract::{CollectionEvent, ColumnKind, ColumnSpec, Role, ShowCollection as _, ToggleColumn};
use craft_ui::list::{List, ListRow, ToggleCell, ToggleState};
use craft_ui::testing::{Fixture, trial};
use egui::Vec2;
use std::cell::RefCell;
use std::rc::Rc;

const KEEP: ColumnKey = ColumnKey::new("keep");
const EYE: ColumnKey = ColumnKey::new("eye");
const PIN: ColumnKey = ColumnKey::new("pin");
const NAME: ColumnKey = ColumnKey::new("name");

/// Three rows with the state of the `KEEP` column.
struct Rows(Rc<RefCell<Vec<bool>>>);

impl DisplayOrder<usize> for Rows {
    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Rows {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        *column == KEEP && self.0.borrow()[at]
    }
}

fn columns() -> Vec<ColumnSpec> {
    let toggle = |key: ColumnKey, header: &'static str| {
        ColumnSpec::new(key, header, ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable()))
    };
    vec![toggle(KEEP, ""), toggle(EYE, ""), toggle(PIN, "Pinned"), ColumnSpec::new(NAME, "", ColumnWidth::fill(1.0))]
}

/// A list of three rows with a named check column, an eye cell and a check cell without a name under a header word.
/// The app applies the toggle edits to `on`, as an app does.
fn rig(on: &Rc<RefCell<Vec<bool>>>, seen: &Rc<RefCell<Vec<String>>>) -> Fixture {
    let (on, seen) = (on.clone(), seen.clone());
    let mut fixture = Fixture::new(move |ui| {
        let rows = Rows(on.clone());
        let out = List::new("Items").columns(columns()).show_items(ui, &rows, &mut |key, item| {
            let state = ToggleState::of(on.borrow()[*key]);
            ListRow::new(format!("Row {key}"))
                .cells(item.cells())
                .toggle(ToggleCell::check(KEEP, state, "Keep {row}"))
                .toggle(ToggleCell::eye(EYE, false))
                .toggle(ToggleCell::check(PIN, ToggleState::Off, ""))
        });
        for event in out.events {
            let CollectionEvent::Toggle(edit) = event else { continue };
            match edit {
                EditEvent::Update { value, .. } => {
                    on.borrow_mut()[value.key] = value.on;
                    seen.borrow_mut().push(format!("update {} {}", value.key, value.on));
                }
                EditEvent::Cancel { before: Some(Mixed::Same(before)), .. } => {
                    on.borrow_mut()[before.key] = before.on;
                    seen.borrow_mut().push(format!("cancel {} {}", before.key, before.on));
                }
                _ => {}
            }
        }
    });
    fixture.room(Vec2::new(400.0, 200.0));
    fixture.settle();
    fixture
}

fn switch_names(fixture: &Fixture) -> Vec<String> {
    let mut names: Vec<String> =
        fixture.by_role(Role::SWITCH).iter().map(|target| fixture.tree().name_of(target.node).unwrap_or("").to_owned()).collect();
    names.sort();
    names
}

/// Each switch reads its cell's name with the row's label, whether or not the column has a header word. A header word
/// names the cells that give no name.
#[trial]
fn a_toggle_cell_node_takes_the_cells_own_name() {
    let fixture = rig(&Rc::new(RefCell::new(vec![false; 3])), &Rc::default());
    let mut wanted: Vec<String> = (0..3).flat_map(|row| [format!("Keep Row {row}"), format!("Visibility Row {row}"), "Pinned".to_owned()]).collect();
    wanted.sort();
    assert_eq!(switch_names(&fixture), wanted);
}
