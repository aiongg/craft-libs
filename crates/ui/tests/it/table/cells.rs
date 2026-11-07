//! The cells of a table row: nodes for the label, toggle and text columns, and the alignment of text cells.

use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, ItemSource};
use craft_i18n::Direction;
use craft_ui::contract::{ColumnKind, ColumnSpec, Role, Show as _, ShowCollection as _, ToggleColumn};
use craft_ui::list::{ListRow, ToggleCell, ToggleState};
use craft_ui::table::Table;
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Vec2};

const KEEP: ColumnKey = ColumnKey::new("keep");
const NAME: ColumnKey = ColumnKey::new("name");
const PAGES: ColumnKey = ColumnKey::new("pages");

struct One;

impl DisplayOrder<usize> for One {
    fn len(&self) -> usize {
        1
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key == 0).then_some(0)
    }
}

impl ItemSource<usize> for One {}

fn spec(key: &ColumnKey) -> ColumnSpec {
    let spec = ColumnSpec::new(key.clone(), "Column", ColumnWidth::Fixed(100.0));
    if *key == KEEP { spec.kind(ColumnKind::Toggle(ToggleColumn::sweepable())) } else { spec }
}

fn table(order: [ColumnKey; 3]) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let columns: Vec<ColumnSpec> = order.iter().map(spec).collect();
        let _ = Table::<usize>::new("Files", columns).show_items(ui, &One, &mut |_, icx| {
            ListRow::new("Row").cells(icx.cells()).cell(PAGES, "3").toggle(ToggleCell::check(KEEP, ToggleState::Off, "Keep {row}"))
        });
    });
    fixture.settle();
    fixture
}

/// The column index of the cell node whose name is `name`'s one, and the position of `key` in `order`.
fn check_cells(order: [ColumnKey; 3]) {
    let position = |key: &ColumnKey| order.iter().position(|column| column == key);
    let fixture = table(order.clone());
    let mut indexes: Vec<_> = fixture.by_role(Role::CELL).iter().map(|cell| fixture.tree().grid_of(cell.node).column_index).collect();
    indexes.sort();
    assert_eq!(indexes, [Some(0), Some(1), Some(2)], "one cell per column: label, toggle and text");
    let text = fixture.by_label("3").expect("the text cell has a node");
    assert_eq!(fixture.tree().grid_of(text.node).column_index, position(&PAGES));
    let check = &fixture.by_role(Role::CHECK_BOX)[0];
    let cell = fixture.tree().parent_of_key(check.node).expect("the check box sits in a cell");
    assert_eq!(fixture.tree().grid_of(cell).column_index, position(&KEEP));
}

#[trial]
fn a_row_has_a_cell_node_per_column_with_its_display_position_after_a_move() {
    check_cells([KEEP, NAME, PAGES]);
    check_cells([NAME, PAGES, KEEP]);
}

/// A numeric column's text ends at the cell's end: the right edge in LTR, the left edge in RTL.
#[trial]
fn an_end_aligned_numeric_column_ends_at_the_cells_end_and_mirrors() {
    for direction in [Direction::Ltr, Direction::Rtl] {
        let mut fixture = Fixture::new(|ui| {
            let origin = ui.cursor().min;
            let cells = [
                (NAME, Rect::from_min_size(origin, Vec2::new(150.0, 20.0))),
                (PAGES, Rect::from_min_size(origin + Vec2::new(150.0, 0.0), Vec2::new(150.0, 20.0))),
            ];
            let columns =
                [ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(150.0)), ColumnSpec::new(PAGES, "Pages", ColumnWidth::Fixed(150.0)).numeric()];
            let _ = ListRow::new("Row").cells(&cells).columns(&columns).cell(PAGES, "42").show(ui);
        });
        fixture.direction(direction).settle();
        let text = fixture.texts().iter().find(|text| text.text == "42").expect("the cell's text is painted").rect;
        let shown = fixture.shown();
        let (before, after) = (text.min.x - (shown.min.x + 150.0), shown.min.x + 300.0 - text.max.x);
        match direction {
            Direction::Rtl => assert!(before < after, "RTL: the text hangs at the cell's left edge"),
            _ => assert!(after < before, "LTR: the text hangs at the cell's right edge"),
        }
    }
}
