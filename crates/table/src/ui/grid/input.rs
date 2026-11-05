//! Wheel, pointer and key handling of the cell grid.

use craft_ui::behavior::{ClaimedKey, KeyClaim};
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::focus::key_owner::{OwnerGate, owner};
use craft_ui::ime::{Composer, Edit};
use egui::{CursorIcon, Event, EventFilter, Key, Modifiers, PointerButton, Pos2, Rect, Response, Ui, Vec2};

use super::{
    CellAdvance, CellDirection, CellRange, CellRef, CellSelection, CellSpace, GridEdit, GridMenuTarget, GridModel, GridState, MAX_ZOOM, MIN_ZOOM,
    Pass, SheetGeometry,
};

/// A grid axis: column headers resize widths, row headers heights.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Axis {
    Column,
    Row,
}

impl Axis {
    fn of(self, p: Pos2) -> f32 {
        match self {
            Axis::Column => p.x,
            Axis::Row => p.y,
        }
    }
}

/// What a pointer press started, kept until the button is released.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    /// Cells selected by dragging from the press.
    Cells,
    /// Whole lines selected by dragging over headers; `add` keeps the earlier ranges.
    Lines { axis: Axis, anchor: u32, add: bool },
    /// A header edge dragged: the line's size at the press and the pointer position then.
    Resize { axis: Axis, line: u32, origin: f32, press: f32 },
    /// The fill handle dragged: the selected range the fill grows from. With the secondary button
    /// (`menu`) the release opens the app's fill menu instead of filling (GridCraft `Drag::Fill`).
    Fill { source: CellRange, menu: bool },
    /// The selection dragged by its border: the range, the cell grabbed and the modifiers of the
    /// press (a press that never moves is a click with them).
    Move { source: CellRange, grab: CellRef, mods: Modifiers },
}

/// Where a point in logical coordinates is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Corner,
    ColumnHeader,
    RowHeader,
    Cells,
    Outside,
}

fn target(geo: &SheetGeometry, p: Pos2) -> Target {
    let (cells, header) = (geo.cells(), geo.header_size());
    let in_columns = p.x >= cells.left();
    let in_rows = p.y >= cells.top();
    if !geo.layout.rect.contains(p) {
        Target::Outside
    } else if in_columns && in_rows {
        Target::Cells
    } else if in_columns && header.y > 0.0 && p.y >= cells.top() - header.y {
        Target::ColumnHeader
    } else if in_rows && header.x > 0.0 && p.x >= cells.left() - header.x {
        Target::RowHeader
    } else if !in_columns && !in_rows && header.x > 0.0 && header.y > 0.0 && p.x >= cells.left() - header.x && p.y >= cells.top() - header.y {
        Target::Corner
    } else {
        Target::Outside
    }
}

fn line_at(geo: &SheetGeometry, axis: Axis, p: Pos2) -> u32 {
    match axis {
        Axis::Column => geo.column_at(p.x),
        Axis::Row => geo.row_at(p.y),
    }
}

/// Where `line` of `axis` ends on screen (logical x or y).
fn line_end(geo: &SheetGeometry, axis: Axis, line: u32) -> f32 {
    let zoom = geo.layout.zoom;
    match axis {
        Axis::Column => geo.x(line) + geo.layout.columns.size(line) as f32 * zoom,
        Axis::Row => geo.y(line) + geo.layout.rows.size(line) as f32 * zoom,
    }
}

/// The line of `axis` whose far edge is within the grab distance of `p`.
fn edge<M: GridModel>(pass: &Pass<'_, M>, geo: &SheetGeometry, axis: Axis, p: Pos2) -> Option<u32> {
    let grab = pass.style.edge_grab;
    let at = line_at(geo, axis, p);
    [at, at.saturating_sub(1)].into_iter().find(|&line| (axis.of(p) - line_end(geo, axis, line)).abs() <= grab)
}

fn line_size(geo: &SheetGeometry, axis: Axis, line: u32) -> f32 {
    match axis {
        Axis::Column => geo.layout.columns.size(line) as f32,
        Axis::Row => geo.layout.rows.size(line) as f32,
    }
}

/// The whole lines of `axis` from `first` to `last`.
fn whole_lines(space: &CellSpace<'_>, axis: Axis, first: u32, last: u32) -> CellRange {
    match axis {
        Axis::Column => space.whole_columns(first, last),
        Axis::Row => space.whole_rows(first, last),
    }
}

fn anchor_line(selection: &CellSelection, axis: Axis) -> u32 {
    match axis {
        Axis::Column => selection.anchor().column,
        Axis::Row => selection.anchor().row,
    }
}

/// `range` as the selection's last range, or after the others with `push`; the active cell and
/// the anchor at its top-left.
fn put_range(selection: &mut CellSelection, range: CellRange, push: bool, space: &CellSpace<'_>) {
    let range = space.cover(range);
    let mut ranges = selection.ranges().to_vec();
    match ranges.last_mut() {
        Some(last) if !push => *last = range,
        _ => ranges.push(range),
    }
    if let Some(next) = CellSelection::from_ranges(ranges, range.start) {
        *selection = next;
    }
}

/// The sheet's rows and columns past which the scroll stops: the last 40 rows and 20 columns
/// still fit the window (GridCraft `grid/show.rs:129` `MAX_ROWS - 40`, `MAX_COLS - 20`).
const LAST_ROWS_SHOWN: u32 = 40;
const LAST_COLUMNS_SHOWN: u32 = 20;

/// Keeps the scroll inside the sheet: no further than the start of the last
/// [`LAST_ROWS_SHOWN`] rows and [`LAST_COLUMNS_SHOWN`] columns, measured from the first scrolling
/// line.
pub(super) fn clamp_scroll<M: GridModel>(pass: &Pass<'_, M>, state: &mut GridState) {
    let (frozen_rows, frozen_columns) = pass.model.frozen();
    let reach = |lines: &super::LineIndex, frozen: u32, shown: u32| {
        let frozen = frozen.min(lines.count());
        (lines.start(lines.count().saturating_sub(shown).max(frozen)) - lines.start(frozen)) as f32
    };
    let max = Vec2::new(reach(pass.model.columns(), frozen_columns, LAST_COLUMNS_SHOWN), reach(pass.model.rows(), frozen_rows, LAST_ROWS_SHOWN));
    state.scroll = state.scroll.max(Vec2::ZERO).min(max);
}

/// Scrolls and zooms with the wheel and pinch over the grid.
pub(super) fn wheel<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, state: &mut GridState) {
    if !ui.rect_contains_pointer(pass.rect) || !ui.is_enabled() {
        return;
    }
    // egui gives no way to consume the zoom delta, so a zoomable parent (Scene, a plot) would zoom too.
    let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
    if pinch != 1.0
        && pinch.is_finite()
        && let Some(pointer) = ui.input(|i| i.pointer.hover_pos())
    {
        zoom_about(pass, state, pointer, pinch);
    }
    if scroll != Vec2::ZERO {
        let mirrored = if pass.right_to_left { -1.0 } else { 1.0 };
        state.scroll.x = (state.scroll.x - mirrored * scroll.x / state.zoom).max(0.0);
        state.scroll.y = (state.scroll.y - scroll.y / state.zoom).max(0.0);
        ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
    }
    clamp_scroll(pass, state);
}

/// Zoom by `factor`, keeping the content under `pointer` (screen) where it is.
fn zoom_about<M: GridModel>(pass: &Pass<'_, M>, state: &mut GridState, pointer: Pos2, factor: f32) {
    let geo = pass.geometry(state);
    let from = state.zoom;
    let to = (from * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    let p = geo.mirror_pos(pointer) - geo.cells().min;
    let shift = 1.0 / from - 1.0 / to;
    state.zoom = to;
    state.scroll = Vec2::new(state.scroll.x + p.x * shift, state.scroll.y + p.y * shift).max(Vec2::ZERO);
}

/// The primary button and the pointer in this pass.
#[derive(Clone, Copy)]
struct Primary {
    enabled: bool,
    mods: Modifiers,
    pressed: bool,
    down: bool,
    released: bool,
    /// The pointer in screen coordinates.
    at: Option<Pos2>,
}

impl Primary {
    /// The button went up this pass, or is no longer down: the gesture ends.
    fn ends(&self) -> bool {
        self.released || !self.down
    }
}

/// What a drag in progress shows over the cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Preview {
    /// The range the fill handle's drag would fill.
    Fill(CellRange),
    /// Where the dragged selection would land.
    Move(CellRange),
}

/// The pointer's work this pass: presses, drags, releases, double and secondary clicks.
pub(super) fn pointer<M: GridModel>(
    pass: &Pass<'_, M>,
    ui: &Ui,
    response: &Response,
    state: &mut GridState,
    edits: &mut Vec<GridEdit>,
) -> Option<Preview> {
    let primary = ui.input(|i| Primary {
        enabled: ui.is_enabled(),
        mods: i.modifiers,
        pressed: i.pointer.button_pressed(PointerButton::Primary),
        down: i.pointer.primary_down(),
        released: i.pointer.button_released(PointerButton::Primary),
        at: i.pointer.interact_pos(),
    });
    let key = pass.id.with("drag");
    let mut drag = ui.data(|d| d.get_temp::<Option<Drag>>(key)).flatten();
    let mut geo = pass.geometry(state);

    // egui reports the button down on a widget until the release, so a press and release in one
    // pass (a tap) is told by the widget having been interacted with.
    let on_grid = response.is_pointer_button_down_on() || (primary.released && response.interact_pointer_pos().is_some());
    if primary.enabled
        && primary.pressed
        && on_grid
        && let Some(p) = primary.at
    {
        response.request_focus();
        drag = begin(pass, &geo, state, geo.mirror_pos(p), primary.mods);
    }
    let secondary_pressed = ui.input(|i| i.pointer.button_pressed(PointerButton::Secondary));
    if primary.enabled
        && secondary_pressed
        && on_grid
        && let Some(p) = primary.at
        && let Some(source) = fill_handle_at(pass, &geo, state, geo.mirror_pos(p))
    {
        drag = Some(Drag::Fill { source, menu: true });
    }
    let mut preview = None;
    let keeps = match drag {
        Some(Drag::Resize { axis, line, origin, press }) => resize(pass, ui, &geo, edits, primary, (axis, line, origin, press)),
        Some(Drag::Fill { source, menu: true }) => fill_menu(pass, ui, state, &mut geo, edits, source, &mut preview),
        Some(Drag::Fill { source, menu: false }) => fill(pass, ui, state, &mut geo, edits, (response, primary), source, &mut preview),
        Some(Drag::Move { source, grab, mods }) => {
            move_cells(pass, ui, state, &mut geo, edits, (response, primary), (source, grab, mods), &mut preview)
        }
        Some(selecting) => {
            if primary.down
                && let Some(p) = primary.at
            {
                let logical = geo.mirror_pos(p);
                advance(pass, ui, state, &mut geo, selecting, logical);
            }
            !primary.ends()
        }
        None => false,
    };
    if !keeps {
        drag = None;
    }
    ui.data_mut(|d| d.insert_temp(key, drag));
    clicks(pass, response, state, edits);
    hover_cursor(pass, ui, state, &pass.geometry(state), drag);
    preview
}

/// A header edge dragged: the width or height as a gesture, its events reported for the line the
/// press began on. Whether the drag goes on.
fn resize<M: GridModel>(
    pass: &Pass<'_, M>,
    ui: &Ui,
    geo: &SheetGeometry,
    edits: &mut Vec<GridEdit>,
    primary: Primary,
    (axis, line, origin, press): (Axis, u32, f32, f32),
) -> bool {
    let mut gesture = Gesture::load(ui, pass.id.with(("resize", axis)), &line_size(geo, axis, line), EditPolicy::default()).enabled(primary.enabled);
    if primary.pressed {
        gesture.begin();
    }
    if gesture.is_active()
        && let Some(p) = primary.at
    {
        gesture.update((origin + (axis.of(geo.mirror_pos(p)) - press) / geo.layout.zoom).max(0.0), primary.mods);
    }
    // A press and release in one pass (a tap) begins and commits here: one `Set`, or nothing when
    // the size didn't change.
    let ends = primary.ends();
    if ends {
        gesture.commit();
    }
    let keeps = gesture.is_active() && !ends;
    if let Some(event) = gesture.finish() {
        edits.push(match axis {
            Axis::Column => GridEdit::ResizeColumn { column: line, width: event },
            Axis::Row => GridEdit::ResizeRow { row: line, height: event },
        });
    }
    keeps
}

/// Where the pointer is for a drag from the cells: inside the cells, and the cell there. A pointer
/// outside scrolls the sheet toward it.
fn drag_cell<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, state: &mut GridState, geo: &mut SheetGeometry, p: Pos2) -> CellRef {
    let inside = geo.cells().shrink(pass.hairline);
    let clamped = Pos2::new(p.x.clamp(inside.left(), inside.right()), p.y.clamp(inside.top(), inside.bottom()));
    let cell = geo.cell_at(clamped);
    scroll_toward(pass, ui, state, geo, p - clamped);
    cell
}

/// Scrolls by `overshoot`, how far the pointer is outside the cells.
fn scroll_toward<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, state: &mut GridState, geo: &mut SheetGeometry, overshoot: Vec2) {
    if overshoot != Vec2::ZERO {
        state.scroll = (state.scroll + overshoot / state.zoom).max(Vec2::ZERO);
        clamp_scroll(pass, state);
        *geo = pass.geometry(state);
        ui.ctx().request_repaint();
    }
}

/// The range a fill handle dragged to `cell` grows `source` to: along the axis the pointer is
/// farther from the range on (GridCraft `grid/interact.rs:862` `fill_target`).
fn fill_target(source: CellRange, cell: CellRef) -> CellRange {
    let (below, above) = (i64::from(cell.row) - i64::from(source.end.row), i64::from(source.start.row) - i64::from(cell.row));
    let (right, left) = (i64::from(cell.column) - i64::from(source.end.column), i64::from(source.start.column) - i64::from(cell.column));
    let (vertical, horizontal) = (below.max(above), right.max(left));
    if vertical <= 0 && horizontal <= 0 {
        source
    } else if vertical >= horizontal {
        if below > 0 {
            CellRange::new(source.start, CellRef::new(cell.row, source.end.column))
        } else {
            CellRange::new(CellRef::new(cell.row, source.start.column), source.end)
        }
    } else if right > 0 {
        CellRange::new(source.start, CellRef::new(source.end.row, cell.column))
    } else {
        CellRange::new(CellRef::new(source.start.row, cell.column), source.end)
    }
}

/// The fill handle dragged: the range the selection grows to, as a gesture that begins when egui
/// starts the drag (a click on the handle reports nothing). Whether the drag goes on.
#[allow(clippy::too_many_arguments)]
fn fill<M: GridModel>(
    pass: &Pass<'_, M>,
    ui: &Ui,
    state: &mut GridState,
    geo: &mut SheetGeometry,
    edits: &mut Vec<GridEdit>,
    (response, primary): (&Response, Primary),
    source: CellRange,
    preview: &mut Option<Preview>,
) -> bool {
    let mut gesture = Gesture::load(ui, pass.id.with("fill"), &source, EditPolicy::default()).enabled(primary.enabled);
    if response.drag_started() {
        gesture.begin();
    }
    if gesture.is_active()
        && let Some(p) = primary.at
    {
        let logical = geo.mirror_pos(p);
        let cell = drag_cell(pass, ui, state, geo, logical);
        gesture.update(pass.space.cover(fill_target(source, cell)), primary.mods);
    }
    let ends = primary.ends();
    if ends {
        gesture.commit();
    }
    let keeps = !ends;
    if keeps && gesture.is_active() {
        *preview = Some(Preview::Fill(*gesture.shown()));
    }
    if let Some(event) = gesture.finish() {
        if let Some(range) = committed(&event) {
            state.selection.select_range(range, &pass.space);
        }
        edits.push(GridEdit::Fill { range: event });
    }
    keeps
}

/// The fill handle dragged with the secondary button: the range it would grow to is previewed, and
/// the release reports [`GridEdit::FillMenu`] at the pointer when the range grew and Esc didn't
/// cancel (GridCraft `grid/interact.rs:708-716`). Nothing is filled or selected. Whether the drag
/// goes on.
fn fill_menu<M: GridModel>(
    pass: &Pass<'_, M>,
    ui: &Ui,
    state: &mut GridState,
    geo: &mut SheetGeometry,
    edits: &mut Vec<GridEdit>,
    source: CellRange,
    preview: &mut Option<Preview>,
) -> bool {
    let (down, released, at) =
        ui.input(|i| (i.pointer.secondary_down(), i.pointer.button_released(PointerButton::Secondary), i.pointer.interact_pos()));
    let mut target = source;
    if let Some(p) = at {
        let cell = drag_cell(pass, ui, state, geo, geo.mirror_pos(p));
        target = pass.space.cover(fill_target(source, cell));
    }
    let cancelled = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
    let ends = released || !down || cancelled;
    if ends {
        if !cancelled
            && target != source
            && let Some(anchor) = at
        {
            edits.push(GridEdit::FillMenu { range: target, anchor });
        }
        return false;
    }
    *preview = Some(Preview::Fill(target));
    true
}

/// The value a gesture's event ended with when it changed something.
fn committed<T: Copy>(event: &EditEvent<T>) -> Option<T> {
    match event {
        EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. } => Some(*value),
        _ => None,
    }
}

/// `source` with its top-left corner at `to`.
fn moved(source: CellRange, to: CellRef) -> CellRange {
    let (rows, columns) = (source.end.row - source.start.row, source.end.column - source.start.column);
    CellRange::new(to, CellRef::new(to.row + rows, to.column + columns))
}

/// Where `source`'s top-left corner lands when the pointer moves from `grab` to `cell`, kept
/// inside the sheet (GridCraft `grid/interact.rs:665`, `offset_clamped`).
fn move_target(source: CellRange, grab: CellRef, cell: CellRef, space: &CellSpace<'_>) -> CellRef {
    let place = |start: u32, grab: u32, cell: u32, extent: u32, count: u32| {
        let room = i64::from(count) - 1 - i64::from(extent);
        (i64::from(start) + i64::from(cell) - i64::from(grab)).clamp(0, room.max(0)) as u32
    };
    CellRef::new(
        place(source.start.row, grab.row, cell.row, source.end.row - source.start.row, space.row_count()),
        place(source.start.column, grab.column, cell.column, source.end.column - source.start.column, space.column_count()),
    )
}

/// The selection dragged by its border: where it moves to, as a gesture that begins when egui
/// starts the drag. A press and release without a drag is a click on the border and selects the
/// cell as a click does. Whether the drag goes on.
#[allow(clippy::too_many_arguments)]
fn move_cells<M: GridModel>(
    pass: &Pass<'_, M>,
    ui: &Ui,
    state: &mut GridState,
    geo: &mut SheetGeometry,
    edits: &mut Vec<GridEdit>,
    (response, primary): (&Response, Primary),
    (source, grab, pressed_with): (CellRange, CellRef, Modifiers),
    preview: &mut Option<Preview>,
) -> bool {
    let mut gesture = Gesture::load(ui, pass.id.with("move"), &source.start, EditPolicy::default()).enabled(primary.enabled);
    if response.drag_started() {
        gesture.begin();
    }
    if gesture.is_active()
        && let Some(p) = primary.at
    {
        let logical = geo.mirror_pos(p);
        let cell = drag_cell(pass, ui, state, geo, logical);
        gesture.update(move_target(source, grab, cell, &pass.space), primary.mods);
    }
    let ends = primary.ends();
    if ends {
        gesture.commit();
    }
    let keeps = !ends;
    if keeps && gesture.is_active() && *gesture.shown() != source.start {
        *preview = Some(Preview::Move(moved(source, *gesture.shown())));
    }
    let event = gesture.finish();
    match &event {
        Some(event) => {
            if let Some(to) = committed(event) {
                state.selection.select_range(moved(source, to), &pass.space);
            }
            edits.push(GridEdit::MoveCells { to: event.clone() });
        }
        None if response.clicked() => select_cell(&pass.space, &mut state.selection, grab, pressed_with),
        None => {}
    }
    keeps
}

/// A click on `cell`: Cmd/Ctrl adds it, Shift extends from the anchor, else it is selected.
fn select_cell(space: &CellSpace<'_>, selection: &mut CellSelection, cell: CellRef, mods: Modifiers) {
    if mods.command {
        selection.add(cell, space);
    } else if mods.shift {
        selection.extend_to(cell, space);
    } else {
        selection.select(cell, space);
    }
}

/// How far from the selection's border a press still grabs it (the header edges' distance).
fn on_border(rect: Rect, p: Pos2, grab: f32) -> bool {
    rect.expand(grab).contains(p) && !rect.shrink(grab).contains(p)
}

/// The fill handle's rect: a square on the current range's bottom-right corner.
fn handle_rect(rect: Rect, side: f32) -> Rect {
    Rect::from_center_size(rect.right_bottom(), Vec2::splat(side))
}

/// The selection the fill handle at `p` (logical) grows, when the press is on the handle.
fn fill_handle_at<M: GridModel>(pass: &Pass<'_, M>, geo: &SheetGeometry, state: &GridState, p: Pos2) -> Option<CellRange> {
    if state.editing || !matches!(target(geo, p), Target::Cells) {
        return None;
    }
    let source = state.selection.current();
    let rect = geo.range_rect(source);
    let shown = geo.range_panes(source).iter().any(|pane| pane.contains(rect.right_bottom()));
    (handle_rect(rect, pass.style.fill_handle).contains(p) && shown).then_some(source)
}

/// The press: selects, or starts a resize, a fill or a move.
fn begin<M: GridModel>(pass: &Pass<'_, M>, geo: &SheetGeometry, state: &mut GridState, p: Pos2, mods: Modifiers) -> Option<Drag> {
    let space = &pass.space;
    match target(geo, p) {
        Target::Corner => {
            state.selection.select_range(space.all(), space);
            None
        }
        Target::ColumnHeader => header_press(pass, geo, state, Axis::Column, p, mods),
        Target::RowHeader => header_press(pass, geo, state, Axis::Row, p, mods),
        Target::Cells => {
            if !state.editing {
                let source = state.selection.current();
                let rect = geo.range_rect(source);
                let panes = geo.range_panes(source);
                let shown = |q: Pos2| panes.iter().any(|pane| pane.contains(q));
                if let Some(source) = fill_handle_at(pass, geo, state, p) {
                    return Some(Drag::Fill { source, menu: false });
                }
                if on_border(rect, p, pass.style.border_grab) && shown(p) {
                    return Some(Drag::Move { source, grab: geo.cell_at(p), mods });
                }
            }
            select_cell(space, &mut state.selection, geo.cell_at(p), mods);
            Some(Drag::Cells)
        }
        Target::Outside => None,
    }
}

fn header_press<M: GridModel>(pass: &Pass<'_, M>, geo: &SheetGeometry, state: &mut GridState, axis: Axis, p: Pos2, mods: Modifiers) -> Option<Drag> {
    if let Some(line) = edge(pass, geo, axis, p) {
        return Some(Drag::Resize { axis, line, origin: line_size(geo, axis, line), press: axis.of(p) });
    }
    let line = line_at(geo, axis, p);
    let anchor = if mods.shift { anchor_line(&state.selection, axis) } else { line };
    let range = whole_lines(&pass.space, axis, anchor.min(line), anchor.max(line));
    put_range(&mut state.selection, range, mods.command, &pass.space);
    Some(Drag::Lines { axis, anchor, add: mods.command })
}

/// A selecting drag at logical position `p`; outside the cells it scrolls toward the pointer.
fn advance<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, state: &mut GridState, geo: &mut SheetGeometry, drag: Drag, p: Pos2) {
    let inside = geo.cells().shrink(pass.hairline);
    let clamped = Pos2::new(p.x.clamp(inside.left(), inside.right()), p.y.clamp(inside.top(), inside.bottom()));
    match drag {
        Drag::Cells => {
            state.selection.extend_to(geo.cell_at(clamped), &pass.space);
        }
        Drag::Lines { axis, anchor, add } => {
            let line = line_at(geo, axis, clamped);
            let range = whole_lines(&pass.space, axis, anchor.min(line), anchor.max(line));
            if add {
                put_range(&mut state.selection, range, false, &pass.space);
            } else {
                state.selection.select_range(range, &pass.space);
            }
        }
        Drag::Resize { .. } | Drag::Fill { .. } | Drag::Move { .. } => return,
    }
    scroll_toward(pass, ui, state, geo, p - clamped);
}

/// The column header's rect on screen.
fn column_header_rect(geo: &SheetGeometry, column: u32) -> Rect {
    let (cells, header) = (geo.cells(), geo.header_size());
    let width = geo.cell_rect(CellRef::new(0, column)).width();
    geo.mirror_rect(Rect::from_min_size(Pos2::new(geo.x(column), cells.top() - header.y), Vec2::new(width, header.y)))
}

/// The row header's rect on screen.
fn row_header_rect(geo: &SheetGeometry, row: u32) -> Rect {
    let (cells, header) = (geo.cells(), geo.header_size());
    let height = geo.cell_rect(CellRef::new(row, 0)).height();
    geo.mirror_rect(Rect::from_min_size(Pos2::new(cells.left() - header.x, geo.y(row)), Vec2::new(header.x, height)))
}

/// Double clicks (AutoFit or Unhide at a header edge, edit in a cell) and secondary clicks (the
/// menu).
fn clicks<M: GridModel>(pass: &Pass<'_, M>, response: &Response, state: &mut GridState, edits: &mut Vec<GridEdit>) {
    let geo = pass.geometry(state);
    let Some(screen) = response.interact_pointer_pos() else {
        return;
    };
    let p = geo.mirror_pos(screen);
    if response.double_clicked() {
        match target(&geo, p) {
            Target::ColumnHeader => edits.extend(edge(pass, &geo, Axis::Column, p).map(|column| {
                if geo.layout.columns.size(column) <= 0.0 { GridEdit::UnhideColumn { column } } else { GridEdit::AutoFitColumn { column } }
            })),
            Target::RowHeader => edits.extend(
                edge(pass, &geo, Axis::Row, p)
                    .map(|row| if geo.layout.rows.size(row) <= 0.0 { GridEdit::UnhideRow { row } } else { GridEdit::AutoFitRow { row } }),
            ),
            Target::Cells if !state.editing => edits.push(GridEdit::BeginEdit { typed: None }),
            _ => {}
        }
    }
    if response.secondary_clicked() {
        context_menu(pass, state, &geo, p, screen, edits);
    }
}

/// A secondary click at `p` (`screen` on screen): the selection moves as Excel moves it, then the
/// app opens its menu.
fn context_menu<M: GridModel>(pass: &Pass<'_, M>, state: &mut GridState, geo: &SheetGeometry, p: Pos2, screen: Pos2, edits: &mut Vec<GridEdit>) {
    let space = &pass.space;
    let selection = &mut state.selection;
    let (anchor, menu) = match target(geo, p) {
        Target::Cells => {
            let cell = geo.cell_at(p);
            if !selection.contains(cell) {
                selection.select(cell, space);
            }
            (geo.mirror_rect(geo.cell_rect(cell)), GridMenuTarget::Cells)
        }
        Target::ColumnHeader => {
            let column = geo.column_at(p.x);
            let whole = selection.ranges().iter().any(|r| r.start.row == 0 && r.end.row + 1 == space.row_count() && r.columns().contains(&column));
            if !whole {
                selection.select_range(space.whole_columns(column, column), space);
            }
            (column_header_rect(geo, column), GridMenuTarget::ColumnHeader(column))
        }
        Target::RowHeader => {
            let row = geo.row_at(p.y);
            let whole = selection.ranges().iter().any(|r| r.start.column == 0 && r.end.column + 1 == space.column_count() && r.rows().contains(&row));
            if !whole {
                selection.select_range(space.whole_rows(row, row), space);
            }
            (row_header_rect(geo, row), GridMenuTarget::RowHeader(row))
        }
        Target::Corner | Target::Outside => return,
    };
    edits.push(GridEdit::ContextMenu { anchor, pointer: Some(screen), target: menu });
}

/// Resize cursors over header edges and during a resize; crosshair on the fill handle, move on the
/// selection's border.
fn hover_cursor<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, state: &GridState, geo: &SheetGeometry, drag: Option<Drag>) {
    let hovered = || ui.input(|i| i.pointer.hover_pos()).filter(|p| pass.rect.contains(*p)).map(|p| geo.mirror_pos(p));
    let icon = match drag {
        Some(Drag::Resize { axis: Axis::Column, .. }) => Some(CursorIcon::ResizeColumn),
        Some(Drag::Resize { axis: Axis::Row, .. }) => Some(CursorIcon::ResizeRow),
        Some(Drag::Fill { .. }) => Some(CursorIcon::Crosshair),
        Some(Drag::Move { .. }) => Some(CursorIcon::Move),
        Some(_) => None,
        None => hovered().and_then(|p| match target(geo, p) {
            Target::ColumnHeader => edge(pass, geo, Axis::Column, p).map(|_| CursorIcon::ResizeColumn),
            Target::RowHeader => edge(pass, geo, Axis::Row, p).map(|_| CursorIcon::ResizeRow),
            Target::Cells if !state.editing => {
                let rect = geo.range_rect(state.selection.current());
                if handle_rect(rect, pass.style.fill_handle).contains(p) {
                    Some(CursorIcon::Crosshair)
                } else if on_border(rect, p, pass.style.border_grab) {
                    Some(CursorIcon::Move)
                } else {
                    None
                }
            }
            _ => None,
        }),
    };
    if let Some(icon) = icon {
        ui.ctx().set_cursor_icon(icon);
    }
}

/// Keys the grid claims from the app's shortcuts while it has the focus.
const CLAIMED: [ClaimedKey; 2] = [ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)];

/// The keyboard while the grid has the focus and the app's editor is closed. Typing goes through
/// an [`ime::Composer`](craft_ui::ime::Composer) with its caret at the active cell, so a platform
/// input method starts composing there; the first composition or typed text reports
/// [`GridEdit::BeginEdit`], and the app's editor takes the rest.
pub(super) fn keys<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, response: &Response, state: &mut GridState, edits: &mut Vec<GridEdit>) {
    let ctx = ui.ctx();
    let id = pass.id.with("composer");
    let mut composer = ui.data(|d| d.get_temp::<Composer>(id)).unwrap_or_else(|| Composer::new().with_owner(pass.id));
    let active = response.has_focus() && !state.editing && ui.is_enabled() && !owner(ctx, &OwnerGate::default()).is_exclusive();
    // A shortcut key that also types a character (Shift+Space sends Text(" ")) must not start an edit.
    let shortcut_pressed = ui.input(|i| {
        i.events.iter().any(
            |e| matches!(e, Event::Key { pressed: true, modifiers: m, key, .. } if !m.alt && (m.ctrl || m.command || (m.shift && *key == Key::Space))),
        )
    });
    let typed = composer.update(ctx, active);
    if active {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(response.id, EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true })
        });
        KeyClaim::claim(ctx, response.id, &CLAIMED);
        run(pass, state, edits, typed, shortcut_pressed);
        let caret = pass.geometry(state);
        composer.set_caret(ctx, caret.mirror_rect(caret.cell_rect(state.selection.active())));
    }
    ui.data_mut(|d| d.insert_temp(id, composer));
}

/// One pass of the composer's edits on the grid, in the order the platform sent them.
fn run<M: GridModel>(pass: &Pass<'_, M>, state: &mut GridState, edits: &mut Vec<GridEdit>, typed: Vec<Edit>, shortcut_pressed: bool) {
    let geo = pass.geometry(state);
    let mut keys = Keys { pass, state, geo, edits };
    let mut shortcut_ran = false;
    for edit in typed {
        match edit {
            Edit::Copy => keys.edits.push(GridEdit::Copy),
            Edit::Cut => keys.edits.push(GridEdit::Cut),
            Edit::Paste(text) => keys.edits.push(GridEdit::Paste(text)),
            Edit::Insert(_) if shortcut_pressed || shortcut_ran => {}
            Edit::Insert(text) if !text.is_empty() && text.chars().all(|c| !c.is_control()) => {
                keys.edits.push(GridEdit::BeginEdit { typed: Some(text) });
                return;
            }
            // The composition goes to the editor: it opens empty and takes the rest.
            Edit::Preedit { text, .. } if !text.is_empty() => {
                keys.edits.push(GridEdit::BeginEdit { typed: Some(String::new()) });
                return;
            }
            Edit::LineBreak { modifiers } => {
                keys.key(Key::Enter, modifiers);
            }
            // Alt shortcuts type their character too on Windows and Linux (Alt+= sends Text("=")).
            Edit::Key { key, modifiers, .. } => shortcut_ran |= !keys.key(key, modifiers) && modifiers.alt && !modifiers.ctrl,
            _ => {}
        }
    }
}

/// One pass of key events on the grid's state.
struct Keys<'a, 'b, M: GridModel> {
    pass: &'a Pass<'b, M>,
    state: &'a mut GridState,
    geo: SheetGeometry,
    edits: &'a mut Vec<GridEdit>,
}

impl<M: GridModel> Keys<'_, '_, M> {
    /// Handles `key`; false when the grid has nothing for it.
    fn key(&mut self, key: Key, m: Modifiers) -> bool {
        let mirrored = self.pass.right_to_left;
        let direction = match (key, mirrored) {
            (Key::ArrowUp, _) => Some(CellDirection::Up),
            (Key::ArrowDown, _) => Some(CellDirection::Down),
            (Key::ArrowLeft, false) | (Key::ArrowRight, true) => Some(CellDirection::Left),
            (Key::ArrowRight, false) | (Key::ArrowLeft, true) => Some(CellDirection::Right),
            _ => None,
        };
        if let Some(direction) = direction {
            self.arrow(direction, m);
            return true;
        }
        match key {
            Key::PageUp | Key::PageDown if m.command => {
                self.edits.push(if key == Key::PageDown { GridEdit::NextSheet } else { GridEdit::PreviousSheet });
            }
            Key::PageUp => self.page(CellDirection::Up, m.shift),
            Key::PageDown => self.page(CellDirection::Down, m.shift),
            Key::Home | Key::End => self.home_end(key == Key::End, m),
            // Excel: Enter while a copied range is marked pastes it and ends copy mode.
            Key::Enter if !m.any() && !self.pass.copy_mark.is_empty() => self.edits.push(GridEdit::PasteAtActive),
            Key::Escape if !self.pass.copy_mark.is_empty() => self.edits.push(GridEdit::ClearCopyMark),
            Key::Enter if m.alt || m.ctrl => self.edits.push(GridEdit::BeginEdit { typed: None }),
            Key::Enter => self.advance(CellAdvance::Enter, m.shift),
            Key::Tab => self.advance(CellAdvance::Tab, m.shift),
            Key::A if m.command => {
                self.state.selection.select_range(self.pass.space.all(), &self.pass.space);
            }
            Key::F2 => self.edits.push(GridEdit::BeginEdit { typed: None }),
            Key::Delete => self.edits.push(GridEdit::Clear),
            Key::Backspace => self.edits.push(GridEdit::ClearAndEdit),
            Key::Menu => self.menu_key(),
            Key::F10 if m.shift => self.menu_key(),
            _ => return false,
        }
        true
    }

    /// Scrolls so `cell` (its whole merged cell) shows.
    fn reveal(&mut self, cell: CellRef) {
        let range = self.pass.space.merge_at(cell).unwrap_or(CellRange::cell(cell));
        self.state.scroll = self.geo.scroll_to_reveal(range);
        self.geo = self.pass.geometry(self.state);
    }

    fn reveal_moved(&mut self, extending: bool) {
        let selection = &self.state.selection;
        let cell = if extending { selection.head() } else { selection.active() };
        self.reveal(cell);
    }

    fn arrow(&mut self, direction: CellDirection, m: Modifiers) {
        let space = &self.pass.space;
        let selection = &mut self.state.selection;
        if m.command {
            let from = if m.shift { selection.head() } else { selection.active() };
            let to = self.pass.model.data_edge(from, direction);
            if m.shift {
                selection.extend_to(to, space);
            } else {
                selection.select(to, space);
            }
        } else if m.shift {
            selection.extend(direction, space);
        } else {
            selection.step(direction, space);
        }
        self.reveal_moved(m.shift);
    }

    /// Page Up / Down: a screen of rows.
    fn page(&mut self, direction: CellDirection, extend: bool) {
        let ((first, last), _) = self.geo.visible();
        let space = &self.pass.space;
        for _ in 0..(last - first).max(1) {
            let selection = &mut self.state.selection;
            let moved = if extend { selection.extend(direction, space) } else { selection.step(direction, space) };
            if !moved {
                break;
            }
        }
        self.reveal_moved(extend);
    }

    /// Home, End and their Cmd/Ctrl forms.
    fn home_end(&mut self, end: bool, m: Modifiers) {
        let space = &self.pass.space;
        let (frozen_rows, frozen_columns) = self.pass.model.frozen();
        let head = if m.shift { self.state.selection.head() } else { self.state.selection.active() };
        let to = match (end, m.command) {
            (false, false) => CellRef::new(head.row, 0),
            (false, true) => CellRef::new(frozen_rows, frozen_columns),
            (true, true) => self.pass.model.last_used(),
            (true, false) => return,
        };
        if m.shift {
            self.state.selection.extend_to(to, space);
        } else {
            self.state.selection.select(to, space);
        }
        self.reveal_moved(m.shift);
    }

    fn advance(&mut self, along: CellAdvance, back: bool) {
        self.state.selection.advance(along, back, &self.pass.space);
        self.reveal_moved(false);
    }

    /// The Menu key: the menu at the active cell.
    fn menu_key(&mut self) {
        let anchor = self.geo.mirror_rect(self.geo.cell_rect(self.state.selection.active()));
        self.edits.push(GridEdit::ContextMenu { anchor, pointer: None, target: GridMenuTarget::Cells });
    }
}
