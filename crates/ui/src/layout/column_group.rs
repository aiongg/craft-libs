//! `ColumnGroup`: rows that are not one `Grid` share the widest label per column key.

use crate::runtime::layout_memory;
use crate::runtime::memory::sub;
use craft_behavior::ColumnKey;
use egui::{Id, Ui, Vec2};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// The widest content per column key of one scope, measured through the same `LayoutMemory` rule as a `Grid`
/// `Content` column.
pub struct ColumnGroup {
    id: Id,
    widest: HashMap<ColumnKey, f32>,
}

impl ColumnGroup {
    /// Runs `rows` with a group named `key` under `ui`. The widths each row reported are recorded when it ends.
    pub fn scope<R>(ui: &mut Ui, key: impl Hash + Debug, rows: impl FnOnce(&mut Ui, &mut ColumnGroup) -> R) -> R {
        let mut group = ColumnGroup { id: sub(ui.scope_id(), key), widest: HashMap::new() };
        let result = rows(ui, &mut group);
        let ctx = ui.ctx().clone();
        for (column, width) in &group.widest {
            layout_memory::record(&ctx, sub(group.id, column), Vec2::new(*width, 0.0));
        }
        result
    }

    /// The width every row gives column `column`: the widest content of last pass. `content` is this row's own
    /// width, kept for the next pass.
    pub fn width(&mut self, ui: &Ui, column: &ColumnKey, content: f32) -> f32 {
        let widest = self.widest.entry(column.clone()).or_insert(0.0);
        *widest = widest.max(content);
        layout_memory::read(ui.ctx(), sub(self.id, column)).map_or(0.0, |size| size.x)
    }
}
