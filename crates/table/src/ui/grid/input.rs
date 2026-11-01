//! Wheel, pointer and key handling of the cell grid.

use craft_ui::behavior::{ClaimedKey, KeyClaim};
use craft_ui::edit::{EditPolicy, Gesture};
use craft_ui::focus::key_owner::{OwnerGate, owner};
use egui::{CursorIcon, Event, EventFilter, ImeEvent, Key, Modifiers, PointerButton, Pos2, Response, Ui, Vec2};

use super::{
    CellAdvance, CellDirection, CellRange, CellRef, CellSelection, CellSpace, EDGE_GRAB_SHARE, GridEdit, GridMenuTarget, GridModel, GridState,
    MAX_ZOOM, MIN_ZOOM, Pass, SheetGeometry,
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
    let grab = pass.style.column_header_height * EDGE_GRAB_SHARE;
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

/// Scrolls and zooms with the wheel and pinch over the grid.
pub(super) fn wheel<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, state: &mut GridState) {
    if !ui.rect_contains_pointer(pass.rect) || !ui.is_enabled() {
        return;
    }
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

/// The pointer's work this pass: presses, drags, releases, double and secondary clicks.
pub(super) fn pointer<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, response: &Response, state: &mut GridState, edits: &mut Vec<GridEdit>) {
    let enabled = ui.is_enabled();
    let mods = ui.input(|i| i.modifiers);
    let (pressed, down, released) =
        ui.input(|i| (i.pointer.button_pressed(PointerButton::Primary), i.pointer.primary_down(), i.pointer.button_released(PointerButton::Primary)));
    let at = ui.input(|i| i.pointer.interact_pos());
    let key = pass.id.with("drag");
    let mut drag = ui.data(|d| d.get_temp::<Option<Drag>>(key)).flatten();
    let mut geo = pass.geometry(state);

    if enabled
        && pressed
        && response.is_pointer_button_down_on()
        && let Some(p) = at
    {
        response.request_focus();
        drag = begin(pass, &geo, state, geo.mirror_pos(p), mods);
    }
    let resizing = match drag {
        Some(Drag::Resize { axis, line, .. }) => Some((axis, line)),
        _ => None,
    };
    let width = |axis| resizing.filter(|(a, _)| *a == axis).map_or(0.0, |(_, line)| line_size(&geo, axis, line));
    let mut gestures =
        [Axis::Column, Axis::Row].map(|axis| Gesture::load(ui, pass.id.with(("resize", axis)), &width(axis), EditPolicy::default()).enabled(enabled));
    if let (Some(Drag::Resize { axis, origin, press, .. }), Some(p)) = (drag, at) {
        let gesture = &mut gestures[axis as usize];
        if pressed {
            gesture.begin();
        } else if !gesture.is_active() {
            drag = None;
        } else {
            let size = (origin + (axis.of(geo.mirror_pos(p)) - press) / geo.layout.zoom).max(0.0);
            if down || released {
                gesture.update(size, mods);
            }
            if released {
                gesture.commit();
            }
        }
    }
    if down && let (Some(drag), Some(p)) = (drag, at) {
        let logical = geo.mirror_pos(p);
        advance(pass, ui, state, &mut geo, drag, logical);
    }
    for (gesture, axis) in gestures.into_iter().zip([Axis::Column, Axis::Row]) {
        let (Some(event), Some((_, line))) = (gesture.finish(), resizing.filter(|(a, _)| *a == axis)) else {
            continue;
        };
        edits.push(match axis {
            Axis::Column => GridEdit::ResizeColumn { column: line, width: event },
            Axis::Row => GridEdit::ResizeRow { row: line, height: event },
        });
    }
    if released || !down {
        drag = None;
    }
    ui.data_mut(|d| d.insert_temp(key, drag));
    clicks(pass, response, state, edits);
    hover_cursor(pass, ui, &pass.geometry(state), drag);
}

/// The press: selects, or starts a resize.
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
            let cell = geo.cell_at(p);
            let selection = &mut state.selection;
            if mods.command {
                selection.add(cell, space);
            } else if mods.shift {
                selection.extend_to(cell, space);
            } else {
                selection.select(cell, space);
            }
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
        Drag::Resize { .. } => return,
    }
    let overshoot = p - clamped;
    if overshoot != Vec2::ZERO {
        state.scroll = (state.scroll + overshoot / state.zoom).max(Vec2::ZERO);
        *geo = pass.geometry(state);
        ui.ctx().request_repaint();
    }
}

/// Double clicks (AutoFit at a header edge, edit in a cell) and secondary clicks (the menu).
fn clicks<M: GridModel>(pass: &Pass<'_, M>, response: &Response, state: &mut GridState, edits: &mut Vec<GridEdit>) {
    let geo = pass.geometry(state);
    let Some(p) = response.interact_pointer_pos().map(|p| geo.mirror_pos(p)) else {
        return;
    };
    if response.double_clicked() {
        match target(&geo, p) {
            Target::ColumnHeader => edits.extend(edge(pass, &geo, Axis::Column, p).map(|column| GridEdit::AutoFitColumn { column })),
            Target::RowHeader => edits.extend(edge(pass, &geo, Axis::Row, p).map(|row| GridEdit::AutoFitRow { row })),
            Target::Cells if !state.editing => edits.push(GridEdit::BeginEdit { typed: None }),
            _ => {}
        }
    }
    if response.secondary_clicked() {
        context_menu(pass, state, &geo, p, edits);
    }
}

/// A secondary click at `p`: the selection moves as Excel moves it, then the app opens its menu.
fn context_menu<M: GridModel>(pass: &Pass<'_, M>, state: &mut GridState, geo: &SheetGeometry, p: Pos2, edits: &mut Vec<GridEdit>) {
    let space = &pass.space;
    let selection = &mut state.selection;
    let menu = match target(geo, p) {
        Target::Cells => {
            let cell = geo.cell_at(p);
            if !selection.contains(cell) {
                selection.select(cell, space);
            }
            GridMenuTarget::Cells
        }
        Target::ColumnHeader => {
            let column = geo.column_at(p.x);
            let whole = selection.ranges().iter().any(|r| r.start.row == 0 && r.end.row + 1 == space.row_count() && r.columns().contains(&column));
            if !whole {
                selection.select_range(space.whole_columns(column, column), space);
            }
            GridMenuTarget::ColumnHeader(column)
        }
        Target::RowHeader => {
            let row = geo.row_at(p.y);
            let whole = selection.ranges().iter().any(|r| r.start.column == 0 && r.end.column + 1 == space.column_count() && r.rows().contains(&row));
            if !whole {
                selection.select_range(space.whole_rows(row, row), space);
            }
            GridMenuTarget::RowHeader(row)
        }
        Target::Corner | Target::Outside => return,
    };
    edits.push(GridEdit::ContextMenu { at: geo.mirror_pos(p), target: menu });
}

/// Resize cursors over header edges and during a resize.
fn hover_cursor<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, geo: &SheetGeometry, drag: Option<Drag>) {
    let axis = match drag {
        Some(Drag::Resize { axis, .. }) => Some(axis),
        Some(_) => None,
        None => {
            ui.input(|i| i.pointer.hover_pos()).filter(|p| pass.rect.contains(*p)).map(|p| geo.mirror_pos(p)).and_then(|p| match target(geo, p) {
                Target::ColumnHeader => edge(pass, geo, Axis::Column, p).map(|_| Axis::Column),
                Target::RowHeader => edge(pass, geo, Axis::Row, p).map(|_| Axis::Row),
                _ => None,
            })
        }
    };
    match axis {
        Some(Axis::Column) => ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn),
        Some(Axis::Row) => ui.ctx().set_cursor_icon(CursorIcon::ResizeRow),
        None => {}
    }
}

/// Keys the grid claims from the app's shortcuts while it has the focus.
const CLAIMED: [ClaimedKey; 2] = [ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)];

/// The keyboard while the grid has the focus and the app's editor is closed.
pub(super) fn keys<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, response: &Response, state: &mut GridState, edits: &mut Vec<GridEdit>) {
    if !response.has_focus() || state.editing || !ui.is_enabled() || owner(ui.ctx(), &OwnerGate::default()).is_exclusive() {
        return;
    }
    ui.memory_mut(|m| m.set_focus_lock_filter(response.id, EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true }));
    KeyClaim::claim(ui.ctx(), response.id, &CLAIMED);
    let events = ui.input(|i| i.events.clone());
    // A shortcut key that also types a character (Shift+Space sends Text(" ")) must not start an edit.
    let shortcut_pressed = events.iter().any(
        |e| matches!(e, Event::Key { pressed: true, modifiers: m, key, .. } if !m.alt && (m.ctrl || m.command || (m.shift && *key == Key::Space))),
    );
    let geo = pass.geometry(state);
    let mut keys = Keys { pass, state, geo, edits };
    let mut shortcut_ran = false;
    for event in events {
        match event {
            Event::Copy => keys.edits.push(GridEdit::Copy),
            Event::Cut => keys.edits.push(GridEdit::Cut),
            Event::Paste(text) => keys.edits.push(GridEdit::Paste(text)),
            Event::Text(_) if shortcut_pressed || shortcut_ran => {}
            Event::Text(text) if !text.is_empty() && text.chars().all(|c| !c.is_control()) => {
                keys.edits.push(GridEdit::BeginEdit { typed: Some(text) });
                return;
            }
            Event::Ime(ImeEvent::Commit(text)) if !text.is_empty() => {
                keys.edits.push(GridEdit::BeginEdit { typed: Some(text) });
                return;
            }
            // The composition goes to the editor: it opens empty and takes the rest.
            Event::Ime(ImeEvent::Preedit { text, .. }) if !text.is_empty() => {
                keys.edits.push(GridEdit::BeginEdit { typed: Some(String::new()) });
                return;
            }
            // Alt shortcuts type their character too on Windows and Linux (Alt+= sends Text("=")).
            Event::Key { key, pressed: true, modifiers, .. } => shortcut_ran |= !keys.key(key, modifiers) && modifiers.alt && !modifiers.ctrl,
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

    /// The Menu key: the menu at the active cell's bottom left.
    fn menu_key(&mut self) {
        let at = self.geo.mirror_pos(self.geo.cell_rect(self.state.selection.active()).left_bottom());
        self.edits.push(GridEdit::ContextMenu { at, target: GridMenuTarget::Cells });
    }
}
