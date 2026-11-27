//! `ColumnGroup`: rows share the widest content per column key: the rows of a scope, or the cells of one `Grid`
//! column.
//!
//! The groups open in a build are a stack on the window's `Context`, innermost last; a row finds the group it reads in
//! it, as builds run nested in time. Each row reports its own width when it reads the column. A group keeps the widest
//! width per column its last complete run found (in session memory under its id), and every row reads the larger of
//! that and the widest reported so far, so a row before the widest one still gets the final column.
//!
//! A row that is kept runs nothing, so it reports nothing: every report is captured by the node whose build or run
//! made it ([`begin_capture`]/[`end_capture`], kept in `Kept::column_reports`), and a node that is kept, or answers a
//! measuring run from its size cache, hands its captured reports to the open groups again ([`replay`]). The group's
//! widths are thus those of every row it holds, run or kept.
//!
//! When a group closes with widths other than the ones it kept, it keeps the new ones; after a real run its node builds
//! again in this frame, so the rows that read the old widths take the new ones. A measuring run before the real run
//! (a container's plan) records them first, so the real run is right at once.

use crate::contract::{Context, Id, Ui};
use crate::runtime::memory::{sub, with_session};
use craft_behavior::ColumnKey;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// The column the label step puts side labels in.
pub(crate) const LABELS: ColumnKey = ColumnKey::new("labels");

/// The column the label step puts glyph labels (an icon or a short text such as "W") in.
pub(crate) const GLYPHS: ColumnKey = ColumnKey::new("glyphs");

/// A width per column key.
pub(crate) type Columns = HashMap<ColumnKey, f32>;

/// One row's report to a group: its own width in a column. Kept with the node that made it, so a kept node reports
/// again without running.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Report {
    group: Id,
    column: ColumnKey,
    width: f32,
}

/// The widths a group's last complete run found, in session memory under the group's id.
#[derive(Clone, Debug, Default, PartialEq)]
struct Recorded(Columns);

/// The window's open groups and the report captures of the nodes running, both innermost last.
#[derive(Default)]
struct Open {
    frames: Vec<Frame>,
    captures: Vec<Vec<Report>>,
}

enum Frame {
    /// A group: rows report their widths and read the widest. `measuring`: the group opened in a measuring run.
    Group { id: Id, widest: Columns, measuring: bool },
    /// A labelled container's body, or a component's label step for a child: the field's control. Its labels stay
    /// its own, so rows inside read no label column; every other column (a content-sized number field's value and
    /// unit) they share with the group around.
    Field,
    /// No group: rows inside read none, whatever group is open outside.
    Barrier,
    /// A component in a cell of a grid: its rows read and report the group of the cell's column.
    Cell { group: Id },
}

/// Whether `column` is one the label step puts labels in.
fn labels(column: &ColumnKey) -> bool {
    *column == LABELS || *column == GLYPHS
}

impl Open {
    /// The group a row in the innermost open frame reads `column` from; `None` when no group is open, a barrier is, or
    /// a field's body is and `column` is a label column.
    fn innermost(&self, column: &ColumnKey) -> Option<Id> {
        for frame in self.frames.iter().rev() {
            match frame {
                Frame::Group { id, .. } => return Some(*id),
                Frame::Cell { group } => return Some(*group),
                Frame::Field if labels(column) => return None,
                Frame::Field => {}
                Frame::Barrier => return None,
            }
        }
        None
    }

    /// Adds `report` to its group when that group is open, and to the innermost capture; returns the group's widest
    /// width in the column so far.
    fn add(&mut self, report: Report) -> Option<f32> {
        let widest = self.frames.iter_mut().rev().find_map(|frame| match frame {
            Frame::Group { id, widest, .. } if *id == report.group => {
                let width = widest.entry(report.column.clone()).or_insert(0.0);
                *width = width.max(report.width);
                Some(*width)
            }
            _ => None,
        });
        if let Some(capture) = self.captures.last_mut() {
            capture.push(report);
        }
        widest
    }
}

/// The column group of column `column` of the `Grid` body `grid`.
fn column_group(grid: Id, column: usize) -> Id {
    sub(grid, ("labels", column))
}

/// The widths group `group` kept from its last complete run.
fn recorded(ctx: &Context, group: Id) -> Columns {
    with_session::<Recorded, _>(ctx, sub(group, "widths"), |kept| kept.0.clone())
}

/// The width `column` of `group` gives a row whose own width is `content`: the widest reported so far or the group's
/// kept width, whichever is larger.
fn shared_width(ctx: &Context, group: Id, column: &ColumnKey, content: f32) -> f32 {
    let so_far = ctx.service::<Open, _>(|open| open.add(Report { group, column: column.clone(), width: content })).unwrap_or(content);
    let kept = with_session::<Recorded, _>(ctx, sub(group, "widths"), |kept| kept.0.get(column).copied());
    kept.map_or(so_far, |kept| kept.max(so_far))
}

/// Starts capturing the reports made in the build or run of the node about to run (module docs).
pub(crate) fn begin_capture(ctx: &Context) {
    ctx.service::<Open, _>(|open| open.captures.push(Vec::new()));
}

/// Ends the capture [`begin_capture`] started and returns the node's reports, for its `Kept`. They also join the
/// capture of the node around it, which reports them again when it is kept.
pub(crate) fn end_capture(ctx: &Context) -> Vec<Report> {
    ctx.service::<Open, _>(|open| {
        let reports = open.captures.pop().unwrap_or_default();
        if let Some(outer) = open.captures.last_mut() {
            outer.extend(reports.iter().cloned());
        }
        reports
    })
}

/// Hands the reports a kept node captured in its last run to the groups open now (module docs).
pub(crate) fn replay(ctx: &Context, reports: &[Report]) {
    if reports.is_empty() {
        return;
    }
    ctx.service::<Open, _>(|open| {
        for report in reports {
            open.add(report.clone());
        }
    });
}

/// The widest content per column key of one scope.
pub struct ColumnGroup {
    id: Id,
    ctx: Context,
}

impl ColumnGroup {
    /// Runs `rows` with a group named `key` under `ui`. Each row reads the widest width per column of the group's rows.
    pub fn scope<R>(ui: &mut Ui, key: impl Hash + Debug, rows: impl FnOnce(&mut Ui, &mut ColumnGroup) -> R) -> R {
        let id = sub(ui.scope_id(), key);
        let mut group = ColumnGroup { id, ctx: ui.ctx().clone() };
        Self::open(ui, &[id]);
        let result = rows(ui, &mut group);
        Self::close(ui, 1);
        result
    }

    /// Runs `cells`, the cells of the `Grid` body `grid` over `count` columns, with a group per column: the rows of a
    /// cell read the group of the cell's column ([`ColumnGroup::cell`]), so the labels of one column share a width and
    /// its controls start at one place. Code of the body outside a cell reads none.
    pub(crate) fn grid<R>(ui: &mut Ui, grid: Id, count: usize, cells: impl FnOnce(&mut Ui) -> R) -> R {
        let groups: Vec<Id> = (0..count).map(|column| column_group(grid, column)).collect();
        Self::open(ui, &groups);
        ui.ctx().service::<Open, _>(|open| open.frames.push(Frame::Barrier));
        let result = cells(ui);
        ui.ctx().service::<Open, _>(|open| open.frames.pop());
        Self::close(ui, count);
        result
    }

    /// Runs `show`, a component shown in a cell of column `column` of the `Grid` body `grid`: its rows read the group
    /// of that column.
    pub(crate) fn cell<R>(ctx: &Context, grid: Id, column: usize, show: impl FnOnce() -> R) -> R {
        ctx.service::<Open, _>(|open| open.frames.push(Frame::Cell { group: column_group(grid, column) }));
        let result = show();
        ctx.service::<Open, _>(|open| open.frames.pop());
        result
    }

    /// Runs `rows` with no group open: a labelled row inside reads no column, whatever group is open outside.
    pub fn barrier<R>(ui: &mut Ui, rows: impl FnOnce(&mut Ui) -> R) -> R {
        Self::framed(ui, Frame::Barrier, rows)
    }

    /// Runs the `rows` of a labelled container's body, or a component's own label step for one of its children: they
    /// read no label column, and share every other column with the group around, so the values of content-sized
    /// fields line up across the group's fields.
    pub(crate) fn field<R>(ui: &mut Ui, rows: impl FnOnce(&mut Ui) -> R) -> R {
        Self::framed(ui, Frame::Field, rows)
    }

    fn framed<R>(ui: &mut Ui, frame: Frame, rows: impl FnOnce(&mut Ui) -> R) -> R {
        ui.ctx().service::<Open, _>(|open| open.frames.push(frame));
        let result = rows(ui);
        ui.ctx().service::<Open, _>(|open| open.frames.pop());
        result
    }

    /// Opens a group frame per id in `groups`, in order.
    fn open(ui: &Ui, groups: &[Id]) {
        let measuring = ui.is_measuring();
        ui.ctx().service::<Open, _>(|open| {
            open.frames.extend(groups.iter().map(|&id| Frame::Group { id, widest: Columns::new(), measuring }));
        });
    }

    /// Closes the `count` innermost group frames. A group whose rows found other widths than it kept keeps the new
    /// ones; after a real run the node builds again in this frame (module docs).
    fn close(ui: &mut Ui, count: usize) {
        let ctx = ui.ctx().clone();
        let frames = ctx.service::<Open, _>(|open| {
            let at = open.frames.len().saturating_sub(count);
            open.frames.split_off(at)
        });
        let mut moved = false;
        for frame in frames {
            let Frame::Group { id, widest, measuring } = frame else { continue };
            if recorded(&ctx, id) == widest {
                continue;
            }
            with_session::<Recorded, _>(&ctx, sub(id, "widths"), |kept| kept.0 = widest);
            moved |= !measuring;
        }
        if moved {
            ui.build_cx().rebuild();
        }
    }

    /// The width every row gives column `column`: the widest content of the group's rows. `content` is this row's own
    /// width.
    pub fn width(&mut self, ui: &Ui, column: &ColumnKey, content: f32) -> f32 {
        debug_assert!(ui.ctx().same(&self.ctx), "a column group is used in the window that opened it");
        shared_width(&self.ctx, self.id, column, content)
    }
}

/// The width of `column` in the group the innermost open frame reads, `None` when no group is open or a barrier is.
/// The row reports `content`.
#[doc(hidden)]
pub fn ambient_width(ctx: &Context, column: &ColumnKey, content: f32) -> Option<f32> {
    let group = ctx.service::<Open, _>(|open| open.innermost(column))?;
    Some(shared_width(ctx, group, column, content))
}
