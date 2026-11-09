//! `ColumnGroup`: rows that are not one `Grid` share the widest label per column key.

use crate::runtime::egui::memory::Service;
use crate::runtime::layout_memory;
use crate::runtime::memory::sub;
use craft_behavior::ColumnKey;
use egui::{Context, Id, Ui, Vec2};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// The column the label step puts side labels in.
pub(crate) const LABELS: ColumnKey = ColumnKey::new("labels");

/// The column the label step puts glyph labels (an icon or a short text such as "W") in.
pub(crate) const GLYPHS: ColumnKey = ColumnKey::new("glyphs");

/// The groups open in this pass, innermost last.
#[derive(Default)]
struct Open(Vec<Frame>);

enum Frame {
    /// A group: rows report their widths and read the widest this pass found. `beyond` is, per column, the widest a
    /// row that sits beside its label in a content-sized column reaches past the column: the gap and its control.
    Group { id: Id, widest: HashMap<ColumnKey, f32>, beyond: HashMap<ColumnKey, f32>, room: Option<f32> },
    /// No group: rows inside read none, whatever group is open outside.
    Barrier,
}

/// The widest content per column key of one scope, measured through the same `LayoutMemory` rule as a `Grid`
/// `Content` column.
pub struct ColumnGroup {
    id: Id,
    ctx: Context,
}

impl ColumnGroup {
    /// Runs `rows` with a group named `key` under `ui`. A measuring run records the widest width per column the rows
    /// reported, and a real run's rows read it.
    pub fn scope<R>(ui: &mut Ui, key: impl Hash + Debug, rows: impl FnOnce(&mut Ui, &mut ColumnGroup) -> R) -> R {
        Self::scope_measured(ui, key, rows).0
    }

    /// `scope`, with the width the group's rows take in a measuring run once every row has reported: the widest
    /// column and what the rows beside it reach past it. Only the end of the scope knows the widest column, so a
    /// row that ran before the widest one measured against a narrower column than the real run gives it.
    pub(crate) fn scope_measured<R>(ui: &mut Ui, key: impl Hash + Debug, rows: impl FnOnce(&mut Ui, &mut ColumnGroup) -> R) -> (R, Option<f32>) {
        let ctx = ui.ctx().clone();
        let id = sub(ui.scope_id(), key);
        Service::<Open>::run(&ctx, |open| open.0.push(Frame::Group { id, widest: HashMap::new(), beyond: HashMap::new(), room: None }));
        let mut group = ColumnGroup { id, ctx: ctx.clone() };
        let result = rows(ui, &mut group);
        let Some(Frame::Group { id, widest, beyond, .. }) = Service::<Open>::run(&ctx, |open| open.0.pop()) else { return (result, None) };
        if !ui.is_sizing_pass() {
            return (result, None);
        }
        let mut extent = None::<f32>;
        for (column, width) in &widest {
            // A row whose record a later sweep reused reports nothing there, so the widest of the pass stands.
            let width = column_width(&ctx, id, column).map_or(*width, |found| found.max(*width));
            layout_memory::record_measured(&ctx, sub(id, column), layout_memory::Measured { room: Vec2::ZERO, took: Vec2::new(width, 0.0) });
            if let Some(past) = beyond.get(column) {
                extent = Some(extent.unwrap_or(0.0).max(width + past));
            }
        }
        (result, extent)
    }

    /// Runs `rows` with no group open: a labelled row inside reads no column, so only the rows that sit directly in a
    /// group's body share its widths. A component's own label steps and a nested container's body run in one.
    pub fn barrier<R>(ui: &mut Ui, rows: impl FnOnce(&mut Ui) -> R) -> R {
        let ctx = ui.ctx().clone();
        Service::<Open>::run(&ctx, |open| open.0.push(Frame::Barrier));
        let result = rows(ui);
        Service::<Open>::run(&ctx, |open| open.0.pop());
        result
    }

    /// Records the width the group's container was offered. A row that sits directly in the group decides whether
    /// its label stacks against that width, not against the width the container shrank to last pass, so a row that
    /// needs more than last pass's content widens it instead of stacking.
    pub(crate) fn offer(&mut self, width: f32) {
        let id = self.id;
        Service::<Open>::run(&self.ctx, |open| {
            if let Some(Frame::Group { room, .. }) =
                open.0.iter_mut().rev().find(|frame| matches!(frame, Frame::Group { id: group, .. } if *group == id))
            {
                *room = Some(width);
            }
        });
    }

    /// The width every row gives column `column`: the widest content this pass's measuring run found, or in that run
    /// the widest so far. `content` is this row's own width.
    pub fn width(&mut self, ui: &Ui, column: &ColumnKey, content: f32) -> f32 {
        debug_assert!(ui.ctx() == &self.ctx, "a column group is used in the context that opened it");
        shared_width(&self.ctx, self.id, column, content)
    }
}

/// The width of `column` in the innermost open frame, `None` when no group is open or a barrier is. The row reports
/// `content`.
pub(crate) fn ambient_width(ctx: &Context, column: &ColumnKey, content: f32) -> Option<f32> {
    let id = Service::<Open>::run(ctx, |open| match open.0.last() {
        Some(Frame::Group { id, .. }) => Some(*id),
        _ => None,
    })?;
    Some(shared_width(ctx, id, column, content))
}

/// The width the innermost open frame's container was offered, `None` when no group is open or a barrier is, or the
/// container recorded none.
pub(crate) fn ambient_room(ctx: &Context) -> Option<f32> {
    Service::<Open>::run(ctx, |open| match open.0.last() {
        Some(Frame::Group { room, .. }) => *room,
        _ => None,
    })
}

/// Reports that a row beside its label in column `column` of the innermost open group, in a content-sized column,
/// reaches `past` beyond the column: the gap and its control. Nothing happens where no group is open or a barrier is.
pub(crate) fn report_beyond(ctx: &Context, column: &ColumnKey, past: f32) {
    Service::<Open>::run(ctx, |open| {
        if let Some(Frame::Group { beyond, .. }) = open.0.last_mut() {
            let widest = beyond.entry(column.clone()).or_insert(0.0);
            *widest = widest.max(past);
        }
    });
}

fn shared_width(ctx: &Context, group: Id, column: &ColumnKey, content: f32) -> f32 {
    let so_far = Service::<Open>::run(ctx, |open| {
        let widest = open.0.iter_mut().rev().find_map(|frame| match frame {
            Frame::Group { id, widest, .. } if *id == group => Some(widest),
            _ => None,
        })?;
        let widest = widest.entry(column.clone()).or_insert(0.0);
        *widest = widest.max(content);
        Some(*widest)
    })
    .unwrap_or(content);
    column_width(ctx, group, column).map_or(so_far, |found| found.max(so_far))
}

/// The widest width of `column` a measuring run of this pass recorded for `group`.
fn column_width(ctx: &Context, group: Id, column: &ColumnKey) -> Option<f32> {
    layout_memory::measured(ctx, sub(group, column)).map(|found| found.took.x)
}
