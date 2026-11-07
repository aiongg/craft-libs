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

/// The groups open in this pass, innermost last.
#[derive(Default)]
struct Open(Vec<Frame>);

struct Frame {
    id: Id,
    widest: HashMap<ColumnKey, f32>,
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
        Service::<Open>::of(&ctx).with(|open| open.0.push(Frame { id, widest: HashMap::new() }));
        let mut group = ColumnGroup { id, ctx: ctx.clone() };
        let result = rows(ui, &mut group);
        let frame = Service::<Open>::of(&ctx).with(|open| open.0.pop());
        if let Some(frame) = frame {
            for (column, width) in &frame.widest {
                layout_memory::record(&ctx, sub(frame.id, column), Vec2::new(*width, 0.0));
            }
        }
        result
    }

    /// The width every row gives column `column`: the widest content of last pass. `content` is this row's own
    /// width, kept for the next pass.
    pub fn width(&mut self, ui: &Ui, column: &ColumnKey, content: f32) -> f32 {
        debug_assert!(ui.ctx() == &self.ctx, "a column group is used in the context that opened it");
        shared_width(&self.ctx, self.id, column, content)
    }
}

/// The width of `column` in the innermost open group, `None` when no group is open. The row reports `content`.
pub(crate) fn ambient_width(ctx: &Context, column: &ColumnKey, content: f32) -> Option<f32> {
    let id = Service::<Open>::of(ctx).with(|open| open.0.last().map(|frame| frame.id))?;
    Some(shared_width(ctx, id, column, content))
}

fn shared_width(ctx: &Context, group: Id, column: &ColumnKey, content: f32) -> f32 {
    Service::<Open>::of(ctx).with(|open| {
        if let Some(frame) = open.0.iter_mut().rev().find(|frame| frame.id == group) {
            let widest = frame.widest.entry(column.clone()).or_insert(0.0);
            *widest = widest.max(content);
        }
    });
    layout_memory::read(ctx, sub(group, column)).map_or(0.0, |size| size.x)
}
