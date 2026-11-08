//! The shared cell-selection rules.

use craft_behavior::{
    CellAdvance, CellAt, CellDirection, CellRange, CellSelection, CellSpace, DisplayOrder, Positions, RowAt, SelectionChange, SelectionModel,
};

fn c(row: u32, column: u32) -> CellAt<u32> {
    CellAt::new(row, column)
}

fn r(a: (u32, u32), b: (u32, u32)) -> CellRange<u32> {
    CellRange::cells(c(a.0, a.1), c(b.0, b.1))
}

fn active(selection: &CellSelection<u32>) -> CellAt<u32> {
    *selection.active().expect("the selection has an active cell")
}

fn current(selection: &CellSelection<u32>) -> CellRange<u32> {
    selection.current().expect("the selection has a current range").clone()
}

// A range grows over a merge it crosses and again over a merge that one reaches; whole columns
// stay as they are in `CellSpace::cover`.
#[test]
fn ranges_grow_over_merged_cells_but_whole_lines_stay() {
    let merges = [r((1, 1), (2, 2)), r((3, 3), (3, 5))];
    let rows = Positions::new(10);
    let space = CellSpace::new(&rows, 8).merges(&merges);
    // B2:C3 holds the first merge; D4:F4 touches it only through the second merge.
    assert_eq!(space.cover(r((0, 0), (1, 1))), r((0, 0), (2, 2)));
    assert_eq!(space.cover(r((2, 2), (3, 3))), r((1, 1), (3, 5)));
    let whole = space.whole_columns(1, 1);
    assert_eq!(space.cover(whole.clone()), whole);
    assert_eq!(space.cover(space.whole_rows(1, 1)), space.whole_rows(1, 1));
}

// `neighbour` leaves a merge from its far edge, skips hidden rows and columns, lands on a merge's
// top-left, steps over unselectable cells and is None at the edge.
#[test]
fn neighbour_skips_hidden_lines_and_merges() {
    let merges = [r((2, 2), (3, 3))];
    let hidden_rows = |row: u32| row == 5;
    let hidden_columns = |column: u32| column == 1;
    let locked = |cell: CellAt<u32>| cell == c(0, 3);
    let rows = Positions::new(8);
    let space = CellSpace::new(&rows, 6).merges(&merges).hidden_rows(&hidden_rows).hidden_columns(&hidden_columns);
    assert_eq!(space.neighbour(c(2, 2), CellDirection::Down), Some(c(4, 2)));
    assert_eq!(space.neighbour(c(4, 2), CellDirection::Up), Some(c(2, 2)));
    assert_eq!(space.neighbour(c(2, 2), CellDirection::Right), Some(c(2, 4)));
    assert_eq!(space.neighbour(c(2, 0), CellDirection::Right), Some(c(2, 2)));
    assert_eq!(space.neighbour(c(4, 0), CellDirection::Right), Some(c(4, 2)));
    assert_eq!(space.neighbour(c(4, 3), CellDirection::Down), Some(c(6, 3)));
    assert_eq!(space.neighbour(c(0, 0), CellDirection::Up), None);
    assert_eq!(space.neighbour(c(7, 5), CellDirection::Right), None);
    let only_one = CellSpace::new(&rows, 6).selectable(&locked);
    assert_eq!(only_one.neighbour(c(0, 0), CellDirection::Right), Some(c(0, 3)));
    assert_eq!(only_one.neighbour(c(0, 3), CellDirection::Right), None);
}

// A click selects a merge whole with its top-left active; Shift+click extends from the anchor and
// keeps the active cell; Cmd+click adds a range that becomes active.
#[test]
fn click_shift_click_and_command_click_follow_excel() {
    let merges = [r((2, 2), (3, 3))];
    let rows = Positions::new(10);
    let space = CellSpace::new(&rows, 10).merges(&merges);
    let mut sel = CellSelection::<u32>::default();
    assert!(sel.select(c(3, 3), &space));
    assert_eq!(sel.ranges(), [merges[0].clone()]);
    assert_eq!(active(&sel), c(2, 2));
    assert!(!sel.select(c(2, 3), &space));

    let mut sel = CellSelection::cell(c(5, 5));
    assert!(sel.extend_to(c(3, 3), &space));
    assert_eq!(current(&sel), r((2, 2), (5, 5)));
    assert_eq!(active(&sel), c(5, 5));

    assert!(sel.add(c(8, 8), &space));
    assert_eq!(sel.ranges().len(), 2);
    assert_eq!((active(&sel), sel.anchor_cell().copied()), (c(8, 8), Some(c(8, 8))));
    assert!(sel.extend_to(c(9, 9), &space));
    assert_eq!(sel.ranges()[1], r((8, 8), (9, 9)));
}

// Shift+arrows move the head (the corner opposite the anchor), not the active cell; a plain arrow
// collapses to the active cell's neighbour.
#[test]
fn shift_arrows_move_the_head_and_arrows_collapse() {
    let rows = Positions::new(10);
    let space = CellSpace::new(&rows, 10);
    let mut sel = CellSelection::cell(c(4, 4));
    assert!(sel.extend(CellDirection::Down, &space));
    assert!(sel.extend(CellDirection::Right, &space));
    assert!(sel.extend(CellDirection::Right, &space));
    assert_eq!(current(&sel), r((4, 4), (5, 6)));
    assert_eq!(sel.head(&space), Some(c(5, 6)));
    assert_eq!(active(&sel), c(4, 4));
    assert!(sel.extend(CellDirection::Up, &space));
    assert_eq!(current(&sel), r((4, 4), (4, 6)));

    assert!(sel.step(CellDirection::Down, &space));
    assert_eq!(sel.ranges(), [CellRange::cell(c(5, 4))]);
    let mut edge = CellSelection::cell(c(0, 0));
    assert!(!edge.extend(CellDirection::Up, &space));
    assert!(!edge.step(CellDirection::Left, &space));
}

// Enter and Tab cycle the active cell through every range of a multi-range selection, both ways,
// and keep the selection; on a single merged cell they move the selection.
#[test]
fn enter_and_tab_cycle_inside_the_selection() {
    let merges = [r((0, 0), (1, 1))];
    let rows = Positions::new(10);
    let space = CellSpace::new(&rows, 10).merges(&merges);
    let mut sel = CellSelection::from_ranges(vec![r((2, 2), (3, 3)), r((6, 6), (6, 7))], c(6, 6)).unwrap();
    let ranges = sel.ranges().to_vec();
    let mut seen = vec![active(&sel)];
    for _ in 0..6 {
        assert!(sel.advance(CellAdvance::Tab, false, &space));
        seen.push(active(&sel));
    }
    assert_eq!(seen, [c(6, 6), c(6, 7), c(2, 2), c(2, 3), c(3, 2), c(3, 3), c(6, 6)]);
    // The order of `ranges()` stays; the current range follows the active cell.
    assert_eq!(sel.ranges(), ranges);
    assert_eq!(current(&sel), ranges[1]);
    assert!(sel.advance(CellAdvance::Tab, true, &space));
    assert_eq!(active(&sel), c(3, 3));
    assert_eq!(current(&sel), ranges[0]);
    assert!(sel.advance(CellAdvance::Enter, false, &space));
    assert_eq!(active(&sel), c(6, 6));

    let mut one = CellSelection::<u32>::default();
    one.select(c(1, 1), &space);
    assert!(one.advance(CellAdvance::Tab, false, &space));
    assert_eq!(one.ranges(), [CellRange::cell(c(0, 2))]);
    // A merge selected whole is one cell; `from_ranges` refuses empty input and a stray active cell.
    let mut merged = CellSelection::<u32>::default();
    merged.select(c(1, 1), &space);
    assert!(merged.is_single_cell(&space));
    assert!(CellSelection::<u32>::from_ranges(Vec::new(), c(0, 0)).is_none());
    assert!(CellSelection::from_ranges(vec![r((0, 0), (1, 1))], c(5, 5)).is_none());
}

// A hidden column inside a range is skipped by Tab and Enter inside a multi-cell selection.
#[test]
fn advance_inside_a_range_skips_hidden_lines() {
    let hidden_columns = |column: u32| column == 1;
    let rows = Positions::new(5);
    let space = CellSpace::new(&rows, 5).hidden_columns(&hidden_columns);
    let mut sel = CellSelection::range(r((0, 0), (1, 2)));
    let mut seen = Vec::new();
    for _ in 0..4 {
        sel.advance(CellAdvance::Tab, false, &space);
        seen.push(active(&sel));
    }
    assert_eq!(seen, [c(0, 2), c(1, 0), c(1, 2), c(0, 0)]);
}

// A cell the selectable rule refuses can't be selected or added; the selection stays as it was.
// `select_range` covers merges.
#[test]
fn locked_cells_refuse_selection_and_ranges_cover_merges() {
    let merges = [r((2, 2), (3, 3))];
    let selectable = |cell: CellAt<u32>| cell != c(5, 5);
    let rows = Positions::new(10);
    let space = CellSpace::new(&rows, 10).merges(&merges).selectable(&selectable);
    let mut sel = CellSelection::cell(c(0, 0));
    assert!(!sel.select(c(5, 5), &space));
    assert!(!sel.add(c(5, 5), &space));
    assert_eq!(sel, CellSelection::cell(c(0, 0)));
    assert!(sel.select_range(r((1, 1), (2, 2)), &space));
    assert_eq!(sel.ranges(), [r((1, 1), (3, 3))]);
}

// A stored selection with ranges but no active cell is refused, and so is an active cell with no ranges, where
// `advance` would otherwise index past the end; a stored selection keeps its order and current range through a
// round trip. The empty selection loads.
#[test]
fn a_stored_selection_is_checked_on_the_way_in() {
    assert_eq!(serde_json::from_str::<CellSelection<u32>>(r#"{"ranges":[]}"#).unwrap(), CellSelection::default());
    let no_ranges = r#"{"ranges":[],"active":{"row":0,"column":0},"anchor":{"row":0,"column":0}}"#;
    assert!(serde_json::from_str::<CellSelection<u32>>(no_ranges).is_err());
    let no_active = r#"{"ranges":[{"start":{"row":0,"column":0},"end":{"row":1,"column":1}}]}"#;
    assert!(serde_json::from_str::<CellSelection<u32>>(no_active).is_err());
    let outside = r#"{"ranges":[{"start":{"row":0,"column":0},"end":{"row":1,"column":1}}],"active":{"row":0,"column":4}}"#;
    assert!(serde_json::from_str::<CellSelection<u32>>(outside).is_err());
    let rows = Positions::new(10);
    let space = CellSpace::new(&rows, 10);
    let mut sel = CellSelection::from_ranges(vec![r((2, 2), (3, 3)), r((6, 6), (6, 7))], c(6, 6)).unwrap();
    sel.advance(CellAdvance::Tab, true, &space);
    let back: CellSelection<u32> = serde_json::from_str(&serde_json::to_string(&sel).unwrap()).unwrap();
    assert_eq!(back, sel);
}

// A selection stored by the legacy sheet loads, the last range current when `current` is absent, and saves the same
// bytes.
#[test]
fn a_legacy_stored_selection_loads_and_saves_the_same_bytes() {
    let stored = r#"{"ranges":[{"start":{"row":2,"column":2},"end":{"row":3,"column":3}},{"start":{"row":6,"column":6},"end":{"row":6,"column":7}}],"current":1,"active":{"row":6,"column":7},"anchor":{"row":6,"column":6},"tab_run":null}"#;
    let sel: CellSelection<u32> = serde_json::from_str(stored).unwrap();
    assert_eq!((current(&sel), active(&sel)), (r((6, 6), (6, 7)), c(6, 7)));
    assert_eq!(serde_json::to_string(&sel).unwrap(), stored);
    let without_current = stored.replace(r#""current":1,"#, "");
    let loaded: CellSelection<u32> = serde_json::from_str(&without_current).unwrap();
    assert_eq!(loaded, sel);
    assert_eq!(serde_json::to_string(&CellSelection::<u32>::default()).unwrap(), r#"{"ranges":[],"current":0,"tab_run":null}"#);
}

// Tab, Tab, Enter goes to the next row in the column the Tabs started in; an arrow ends the run.
#[test]
fn enter_after_tabs_returns_to_the_starting_column() {
    let rows = Positions::new(10);
    let space = CellSpace::new(&rows, 10);
    let mut sel = CellSelection::cell(c(2, 1));
    sel.advance(CellAdvance::Tab, false, &space);
    sel.advance(CellAdvance::Tab, false, &space);
    assert_eq!(active(&sel), c(2, 3));
    assert!(sel.advance(CellAdvance::Enter, false, &space));
    assert_eq!(active(&sel), c(3, 1));
    sel.advance(CellAdvance::Enter, false, &space);
    assert_eq!(active(&sel), c(4, 1));

    sel.advance(CellAdvance::Tab, false, &space);
    sel.step(CellDirection::Right, &space);
    sel.advance(CellAdvance::Enter, false, &space);
    assert_eq!(active(&sel), c(5, 3));
}

// Enter after a Tab run goes to the start column only when a walk may stop there: with it hidden
// or locked it takes the plain Down neighbour, and the run ends either way.
#[test]
fn enter_after_tabs_ignores_a_start_column_that_cannot_be_visited() {
    let hidden_columns = |column: u32| column == 1;
    let rows = Positions::new(10);
    let hidden = CellSpace::new(&rows, 10).hidden_columns(&hidden_columns);
    let mut sel = CellSelection::cell(c(2, 1));
    sel.advance(CellAdvance::Tab, false, &hidden);
    sel.advance(CellAdvance::Tab, false, &hidden);
    assert_eq!(active(&sel), c(2, 3));
    assert!(sel.advance(CellAdvance::Enter, false, &hidden));
    assert_eq!(active(&sel), c(3, 3));
    sel.advance(CellAdvance::Enter, false, &hidden);
    assert_eq!(active(&sel), c(4, 3));

    let selectable = |cell: CellAt<u32>| !(cell.row == 3 && cell.column == 1);
    let space = CellSpace::new(&rows, 10).selectable(&selectable);
    let mut sel = CellSelection::cell(c(2, 1));
    sel.advance(CellAdvance::Tab, false, &space);
    assert!(sel.advance(CellAdvance::Enter, false, &space));
    assert_eq!(active(&sel), c(3, 2));
    sel.advance(CellAdvance::Enter, false, &space);
    assert_eq!(active(&sel), c(4, 2));
}

// A document table's Tab runs on to the next row, visits a merge once and is None past the last
// cell and before the first.
#[test]
fn reading_order_tab_wraps_rows_and_ends_past_the_last_cell() {
    let merges = [r((0, 1), (1, 2))];
    let rows = Positions::new(3);
    let space = CellSpace::new(&rows, 3).merges(&merges);
    assert_eq!(space.next_in_reading_order(c(0, 0), false), Some(c(0, 1)));
    assert_eq!(space.next_in_reading_order(c(0, 1), false), Some(c(1, 0)));
    assert_eq!(space.next_in_reading_order(c(1, 0), false), Some(c(2, 0)));
    assert_eq!(space.next_in_reading_order(c(2, 2), false), None);
    assert_eq!(space.next_in_reading_order(c(1, 0), true), Some(c(0, 1)));
    assert_eq!(space.next_in_reading_order(c(0, 1), true), Some(c(0, 0)));
    assert_eq!(space.next_in_reading_order(c(0, 0), true), None);
}

// A whole column of a full-size sheet is one range, answered without a scan of its rows.
#[test]
fn a_whole_column_of_a_full_sheet_is_one_range() {
    const ROWS: u32 = 1_048_576;
    let rows = Positions::new(ROWS);
    let space = CellSpace::new(&rows, 16_384);
    let mut sel = CellSelection::<u32>::default();
    assert!(sel.select_range(space.whole_columns(2, 2), &space));
    assert_eq!(sel.ranges().len(), 1);
    let last = ROWS - 1;
    let at = |column| sel.contains_cell(RowAt::new(&last, last as usize, &rows), column);
    assert_eq!((at(2), at(3), at(1)), (true, false, false));
}

struct Names(Vec<&'static str>);

impl DisplayOrder<&'static str> for Names {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> &'static str {
        self.0[at]
    }
}

// A range holds its rows by key, so it follows a reorder of the source.
#[test]
fn a_range_over_keyed_rows_follows_a_reorder_of_the_source() {
    let before = Names(vec!["a", "b", "c", "d", "e"]);
    let mut sel = CellSelection::<&'static str>::default();
    let space = CellSpace::new(&before, 3);
    assert!(sel.select_range(CellRange::new("b", "d", 0..2, 1), &space));
    let holds = |order: &Names, key: &'static str| {
        let at = order.index_of(&key).unwrap();
        sel.contains_cell(RowAt::new(&key, at, order), 1)
    };
    assert_eq!(["a", "b", "c", "d", "e"].map(|key| holds(&before, key)), [false, true, true, true, false]);
    let after = Names(vec!["d", "a", "b", "e", "c"]);
    assert_eq!(["a", "b", "c", "d", "e"].map(|key| holds(&after, key)), [true, true, false, true, false]);
    let gone = Names(vec!["a", "b", "c", "e"]);
    assert_eq!(["a", "b", "c", "e"].map(|key| holds(&gone, key)), [false; 4]);
}

// The change a rule reports applies to a copy of the selection and gives the same selection.
#[test]
fn a_reported_change_rebuilds_the_selection_on_a_copy() {
    let merges = [r((4, 4), (5, 5))];
    let rows = Positions::new(10);
    let space = CellSpace::new(&rows, 10).merges(&merges);
    let mut sel = CellSelection::<u32>::default();
    sel.select(c(2, 3), &space);
    assert!(matches!(sel.change(), SelectionChange::Cells(_)));
    let mut copy = CellSelection::<u32>::default();
    copy.apply(sel.change());
    assert_eq!(copy, sel);

    sel.extend_to(c(3, 5), &space);
    sel.add(c(8, 8), &space);
    sel.advance(CellAdvance::Tab, false, &space);
    assert!(matches!(sel.change(), SelectionChange::CellSet(_)));
    copy.apply(sel.change());
    assert_eq!(copy, sel);

    let held = |selection: &CellSelection<u32>, row: u32, column| selection.contains_cell(RowAt::new(&row, row as usize, &rows), column);
    copy.apply(SelectionChange::ToggleCell(c(8, 8)));
    assert_eq!((held(&sel, 8, 8), held(&copy, 8, 8), copy.lead()), (true, false, Some(&8)));
    copy.apply(SelectionChange::ToggleCell(c(8, 8)));
    assert!(held(&copy, 8, 8));
}
