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
    /// A group: rows report their widths and read the widest of last pass.
    Group { id: Id, widest: HashMap<ColumnKey, f32> },
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
    /// Runs `rows` with a group named `key` under `ui`. The widths each row reported are recorded when it ends.
    pub fn scope<R>(ui: &mut Ui, key: impl Hash + Debug, rows: impl FnOnce(&mut Ui, &mut ColumnGroup) -> R) -> R {
        let ctx = ui.ctx().clone();
        let id = sub(ui.scope_id(), key);
        Service::<Open>::run(&ctx, |open| open.0.push(Frame::Group { id, widest: HashMap::new() }));
        let mut group = ColumnGroup { id, ctx: ctx.clone() };
        let result = rows(ui, &mut group);
        if let Some(Frame::Group { id, widest }) = Service::<Open>::run(&ctx, |open| open.0.pop()) {
            for (column, width) in &widest {
                layout_memory::record(&ctx, sub(id, column), Vec2::new(*width, 0.0));
            }
        }
        result
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

    /// The width every row gives column `column`: the widest content of last pass. `content` is this row's own
    /// width, kept for the next pass.
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

fn shared_width(ctx: &Context, group: Id, column: &ColumnKey, content: f32) -> f32 {
    Service::<Open>::run(ctx, |open| {
        let found = open.0.iter_mut().rev().find_map(|frame| match frame {
            Frame::Group { id, widest } if *id == group => Some(widest),
            _ => None,
        });
        if let Some(widest) = found {
            let widest = widest.entry(column.clone()).or_insert(0.0);
            *widest = widest.max(content);
        }
    });
    layout_memory::read(ctx, sub(group, column)).map_or(0.0, |size| size.x)
}
