//! The list rules v1 reaches through craft-ui-contract, ported toolkit-neutrally (look sheet section 5): pointer
//! picks, keyboard moves on craft-behavior's `nav::step` and `keys::intent`, the flat drop gap, stacking-key moves,
//! edge scroll, page size and reveal. Rows are indices into the panel order; no toolkit type appears here, so v3 and
//! v4 share this crate.

use craft_behavior::Axis;
use craft_behavior::intent::{Intent, MoveMods};
use craft_behavior::keys::{KeyRole, ReorderKeys, intent};
use craft_behavior::nav::{NavOutline, NavShape, NavState, NavStep, step};
use craft_host::target::Os;
use craft_keymap::{Key, Mods};

/// A selection change in the terms the fixture applies (v1's glue maps craft's `SelectionChange` to these).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Replace(usize),
    Toggle(usize),
    Range(usize),
    Clear,
}

/// Which leading toggle cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Eye,
    Lock,
}

/// What the list asks the document to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListOp {
    Pick(Pick),
    Toggle {
        row: usize,
        column: Column,
    },
    /// The trailing lock indicator: unlock the row.
    Unlock(usize),
    /// Open the rename field on the row.
    Rename(usize),
    RenameCommit {
        row: usize,
        text: String,
    },
    RenameCancel,
    /// Move (or copy) `rows` next to the row `target`.
    Move {
        rows: Vec<usize>,
        target: usize,
        after: bool,
        copy: bool,
    },
    Delete,
}

/// The document operation a list request means (v1's glue does the same for craft's events). `None` for requests
/// that only change the list's own state (open or cancel a rename).
pub fn doc_op(doc: &crate::fixture::Doc, op: ListOp) -> Option<crate::fixture::Op> {
    use crate::fixture::{Op, SelectMode};
    let id = |row: usize| doc.layers.get(row).map(|l| l.id);
    match op {
        ListOp::Pick(pick) => match pick {
            Pick::Replace(row) => id(row).map(|l| Op::Select(l, SelectMode::Replace)),
            Pick::Toggle(row) => id(row).map(|l| Op::Select(l, SelectMode::Toggle)),
            Pick::Range(row) => id(row).map(|l| Op::Select(l, SelectMode::Range)),
            Pick::Clear => Some(Op::ClearSelection),
        },
        ListOp::Toggle { row, column: Column::Eye } => id(row).map(Op::ToggleVisible),
        ListOp::Toggle { row, column: Column::Lock } => id(row).map(Op::ToggleLock),
        ListOp::Unlock(row) => doc.layers.get(row).filter(|l| l.locked).map(|l| Op::ToggleLock(l.id)),
        ListOp::RenameCommit { row, text } => doc.layers.get(row).and_then(|l| rename_commit(&l.name, &text).map(|name| Op::Rename(l.id, name))),
        ListOp::Rename(_) | ListOp::RenameCancel => None,
        ListOp::Move { rows, target, after, copy } => {
            let ids = rows.iter().filter_map(|&r| id(r)).collect();
            id(target).map(|target| Op::Move { ids, target, after, copy })
        }
        ListOp::Delete => Some(Op::DeleteSelected),
    }
}

/// What a primary press on a row does now and what its release as a click does (`press_step`, `items/press.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PressPlan {
    pub now: Option<Pick>,
    pub on_click: Option<Pick>,
}

/// A primary press on row `at` with `mods` held, the `clicks`-th of a multi-click. `held(at)`: the row is selected.
pub fn press_pick(at: usize, held: bool, mods: Mods, clicks: u32) -> PressPlan {
    let plain = !mods.shift && !mods.cmd;
    let none = PressPlan { now: None, on_click: None };
    if plain && clicks > 1 && held {
        return none;
    }
    if plain {
        // A selected row keeps the selection whole so a drag carries it; the click narrows it. An unselected row is
        // picked by the click, so a drag carries only it and leaves the selection.
        return PressPlan { now: None, on_click: Some(Pick::Replace(at)) };
    }
    // Shift (with or without Cmd) sets or adds a range ending at the row; the fixture applies both as a range to it.
    if mods.shift {
        return PressPlan { now: Some(Pick::Range(at)), on_click: None };
    }
    PressPlan { now: Some(Pick::Toggle(at)), on_click: None }
}

/// A primary press below the last row: clears a non-empty selection (`allow_empty`).
pub fn press_empty(any_selected: bool) -> Option<Pick> {
    any_selected.then_some(Pick::Clear)
}

/// Whether a double click at `x` (list-relative) on a row starts rename: the name band, off the thumbnail and the lock
/// indicator (`names`, `items/press.rs:29-36`).
pub fn names(clicks: u32, in_name_band: bool) -> bool {
    clicks == 2 && in_name_band
}

/// The click slop, `size.hit.slop`: a press that moves further starts a drag.
pub fn past_slop(dx: f32, dy: f32, slop: f32) -> bool {
    dx.hypot(dy) > slop
}

/// The insert gap a drag over the rows lands in (`drop_spot` with `tree_gap_at`, flat rows): the row under the pointer
/// (or the last row, below the rows) splits at its middle. `None` above the first row or with no rows.
pub fn drop_insert(len: usize, row_height: f32, y_in_content: f32) -> Option<usize> {
    if len == 0 || y_in_content < 0.0 {
        return None;
    }
    let over = ((y_in_content / row_height).floor() as usize).min(len - 1);
    let from_top = (y_in_content - over as f32 * row_height) / row_height;
    Some(if from_top < 0.5 { over } else { over + 1 })
}

/// Where `rows` (sorted) land for an insert gap of the full order, as the fixture's move takes it: the remaining row
/// the moved rows go before, else the last remaining row they go after (`host_place`). `None` when nothing moves.
pub fn place(len: usize, rows: &[usize], insert: usize, copy: bool) -> Option<(usize, bool)> {
    let to = insert - rows.iter().filter(|&&r| r < insert).count();
    let remaining: Vec<usize> = (0..len).filter(|r| !rows.contains(r)).collect();
    let together = rows.windows(2).all(|w| w[1] == w[0] + 1);
    if !copy && together && rows.first() == Some(&to) {
        return None;
    }
    match remaining.get(to) {
        Some(&next) => Some((next, false)),
        None => remaining.last().map(|&prev| (prev, true)),
    }
}

/// The insert gap of a stacking-key move of the sorted `rows` (`reorder::key_move`, flat rows).
pub fn key_move(len: usize, rows: &[usize], toward_end: bool, all_the_way: bool) -> Option<usize> {
    let (&first, &last) = (rows.first()?, rows.last()?);
    let packed = rows.windows(2).all(|w| w[1] == w[0] + 1);
    match (toward_end, all_the_way) {
        (false, false) if first > 0 => Some(first - 1),
        (false, true) if !(first == 0 && packed) => Some(0),
        (true, false) if last + 1 < len => Some(last + 2),
        (true, true) if !(last + 1 == len && packed) => Some(len),
        _ => None,
    }
}

/// Rows per second an edge scroll runs at, at the band's full depth (`EDGE_ROWS_PER_SECOND`).
const EDGE_ROWS_PER_SECOND: f32 = 4.0;

/// The scroll step of a drag near the rows' top or bottom edge (`reorder::edge_step`); `at` is view-relative.
pub fn edge_step(view: f32, row: f32, at: f32, dt: f32) -> f32 {
    let band = row.min(view / 2.0);
    if band <= 0.0 {
        return 0.0;
    }
    let (depth, direction) = if at < band { ((band - at) / band, -1.0) } else { ((at - (view - band)) / band, 1.0) };
    if depth <= 0.0 {
        return 0.0;
    }
    direction * depth.min(1.0) * EDGE_ROWS_PER_SECOND * row * dt
}

/// Rows a page move covers (`geometry.rs:40-43`).
pub fn page_rows(view: f32, row: f32) -> usize {
    ((view / row).floor() as usize).saturating_sub(1).max(1)
}

/// The offset that shows the row wholly, moving by the smaller distance and no margin (`aim_reveal`).
pub fn reveal(offset: f32, view: f32, top: f32, height: f32) -> f32 {
    if top < offset {
        top
    } else if top + height > offset + view {
        top + height - view
    } else {
        offset
    }
}

/// The app's rename rule (`map.rs:142-151`): a commit renames only to a trimmed, non-empty, different name.
pub fn rename_commit(old: &str, text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty() && trimmed != old).then(|| trimmed.to_owned())
}

/// The list's key role (look sheet 5.3): reorderable with the stacking keys, rename, deletes, multi-select, space.
pub fn key_role() -> KeyRole {
    KeyRole::new()
        .reorderable(Some(ReorderKeys::Stacking))
        .reorder_axis(Axis::Y)
        .rename(true)
        .space(true)
        .multi_select(true)
        .moves(true)
        .deletes(true)
        .stacked(true)
}

struct Flat(usize);

impl NavOutline for Flat {
    fn len(&self) -> usize {
        self.0
    }
    fn depth(&self, _at: usize) -> usize {
        0
    }
    fn has_children(&self, _at: usize) -> bool {
        false
    }
    fn open(&self, _at: usize) -> bool {
        false
    }
    fn is_section(&self, _at: usize) -> bool {
        false
    }
    fn enabled(&self, _at: usize) -> bool {
        true
    }
}

/// Where the list's keyboard stands: its lead row and range anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lead {
    pub lead: Option<usize>,
    pub anchor: Option<usize>,
}

/// What a key press did to the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyResult {
    /// The key was the list's.
    pub handled: bool,
    pub ops: Vec<ListOp>,
    /// The lead moved by key and its row should show (`reveal`).
    pub reveal: Option<usize>,
}

/// The list's answer to one key press (`items/keys.rs`, look sheet 5.3). `selected` lists the selected rows, sorted.
pub fn key_press(os: Os, key: Key, mods: Mods, len: usize, page: usize, lead: &mut Lead, selected: &[usize]) -> KeyResult {
    let mut out = KeyResult::default();
    let Some(intent) = intent(os, key, mods, key_role()) else { return out };
    let held = |row: usize| selected.binary_search(&row).is_ok();
    match intent {
        Intent::Move(mv, move_mods) => {
            out.handled = true;
            let mut state = NavState::new().page(page);
            state.lead = lead.lead;
            state.anchor = lead.anchor;
            let moved = step(NavShape::Flat, &Flat(len), &mut state, mv, move_mods);
            if let NavStep::Moved(to) = moved {
                lead.lead = Some(to);
                pick_after_move(&mut out, lead, to, move_mods);
                out.reveal = Some(to);
            }
        }
        Intent::TogglePick => {
            out.handled = true;
            if let Some(at) = lead.lead {
                out.ops.push(ListOp::Pick(Pick::Toggle(at)));
                lead.anchor = Some(at);
            }
        }
        Intent::Space => {
            out.handled = true;
            if let Some(at) = lead.lead {
                out.ops.push(ListOp::Pick(Pick::Replace(at)));
                lead.anchor = Some(at);
            }
        }
        Intent::Activate | Intent::Submit => {
            out.handled = true;
            if let Some(at) = lead.lead.filter(|&at| !held(at)) {
                out.ops.push(ListOp::Pick(Pick::Replace(at)));
                lead.anchor = Some(at);
            }
        }
        Intent::Rename => {
            out.handled = true;
            if let Some(at) = lead.lead {
                out.ops.push(ListOp::Rename(at));
            }
        }
        Intent::Delete { .. } => {
            out.handled = true;
            if lead.lead.is_some() {
                out.ops.push(ListOp::Delete);
            }
        }
        Intent::MoveItem { front, all_the_way, .. } => {
            out.handled = true;
            let Some(at) = lead.lead else { return out };
            let rows = if held(at) { selected.to_vec() } else { vec![at] };
            if let Some(insert) = key_move(len, &rows, !front, all_the_way)
                && let Some((target, after)) = place(len, &rows, insert, false)
            {
                out.ops.push(ListOp::Move { rows, target, after, copy: false });
            }
        }
        _ => {}
    }
    out
}

fn pick_after_move(out: &mut KeyResult, lead: &mut Lead, to: usize, mods: MoveMods) {
    if mods.lead {
        return;
    }
    if mods.extend {
        out.ops.push(ListOp::Pick(Pick::Range(to)));
    } else {
        out.ops.push(ListOp::Pick(Pick::Replace(to)));
        lead.anchor = Some(to);
    }
}
