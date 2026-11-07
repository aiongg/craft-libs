//! `BodyLayout` and the layout of a body: the physical egui layout, the measured placement of `Row` End and
//! Center, and the `Grid` cursor.

use crate::runtime::egui::memory::Service;
use crate::runtime::egui::{alloc, child};
use crate::runtime::env::Env;
use crate::runtime::layout_memory;
use crate::runtime::memory::sub;
use craft_behavior::{ColumnWidth, Flow, GridCursor, GridMeasure};
use egui::{Id, Rect, Ui, Vec2};
use std::borrow::Cow;
use std::collections::HashMap;

/// How a body lays out its children, in reading order: mirrored under a right-to-left direction.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum BodyLayout {
    /// Children one under another.
    Stack,
    /// Children side by side. `wrap` continues on a new line; `align` places the block along the row.
    #[non_exhaustive]
    Row {
        /// Whether children continue on a new line.
        wrap: bool,
        /// Where the block of children sits along the row.
        align: RowAlign,
    },
    /// Children row-major over `columns`: a child in flow takes the next cell.
    #[non_exhaustive]
    Grid {
        /// The width rule of each column.
        columns: Cow<'static, [ColumnWidth]>,
    },
}

/// Where a `Row` places its block of children along the row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RowAlign {
    /// At the start edge.
    #[default]
    Start,
    /// Centred.
    Center,
    /// At the end edge.
    End,
}

impl BodyLayout {
    /// Children one under another.
    pub const fn stack() -> Self {
        BodyLayout::Stack
    }

    /// Children side by side from the start edge, with no wrap.
    pub const fn row() -> Self {
        BodyLayout::Row { wrap: false, align: RowAlign::Start }
    }

    /// Children row-major over `columns`.
    pub const fn grid(columns: &'static [ColumnWidth]) -> Self {
        BodyLayout::Grid { columns: Cow::Borrowed(columns) }
    }

    /// A `Row` that wraps; any other layout is unchanged.
    pub fn wrap(self, on: bool) -> Self {
        match self {
            BodyLayout::Row { align, .. } => BodyLayout::Row { wrap: on, align },
            other => other,
        }
    }

    /// A `Row` with `align`; any other layout is unchanged.
    pub fn align(self, align: RowAlign) -> Self {
        match self {
            BodyLayout::Row { wrap, .. } => BodyLayout::Row { wrap, align },
            other => other,
        }
    }
}

/// The cursors of the `Grid` bodies running this pass, by body id.
#[derive(Default)]
struct Cursors(HashMap<Id, GridCursor>);

/// The rect a child of measured `size` takes in flow in `ui`: the next cell of the `Grid` body that encloses it
/// directly, else the next allocation of `ui`'s layout.
pub(crate) fn place_in_flow(ui: &mut Ui, size: Vec2) -> Rect {
    let cell =
        Env::of(ui).grid.and_then(|grid| Service::<Cursors>::of(ui.ctx()).with(|cursors| cursors.0.get_mut(&grid).map(|cursor| cursor.cell(size))));
    match cell {
        Some(cell) => {
            alloc::advance_past(ui, cell);
            cell
        }
        None => alloc::space(ui, size),
    }
}

fn column_id(grid: Id, index: usize) -> Id {
    sub(grid, ("column", index))
}

fn row_id(grid: Id, index: usize) -> Id {
    sub(grid, ("row", index))
}

/// What the grid measured last pass; each entry is read as a placement that used it.
fn last_measure(ctx: &egui::Context, grid: Id, columns: usize) -> GridMeasure {
    let rows = layout_memory::read(ctx, sub(grid, "rows")).map_or(0, |count| count.x as usize);
    GridMeasure::new(
        (0..columns).map(|i| layout_memory::read(ctx, column_id(grid, i)).map_or(0.0, |size| size.x)).collect(),
        (0..rows).map(|i| layout_memory::read(ctx, row_id(grid, i)).map_or(0.0, |size| size.y)).collect(),
    )
}

/// Runs `body` in a child `Ui` laid out by `layout` under `id`, in the direction of `ui`'s env. `gap` is the
/// space between the cells of a `Grid`.
///
/// `Row` with `End` or `Center` places its block from the size it took last pass, and `Grid` places its
/// cells from last pass's widest child per column and tallest per row. A change of those sizes asks for one
/// discard pass under the tolerance rule.
pub(crate) fn run_body<R>(ui: &mut Ui, id: Id, layout: &BodyLayout, gap: Vec2, body: impl FnOnce(&mut Ui) -> R) -> R {
    let rtl = Env::of(ui).direction.is_rtl();
    let ctx = ui.ctx().clone();
    match layout {
        BodyLayout::Stack => child::block(ui, None, child::column_layout(rtl), |ui| body(ui)).0,
        BodyLayout::Row { wrap, align } => {
            let room = ui.available_rect_before_wrap();
            let rect = match *align {
                RowAlign::Start => None,
                RowAlign::Center | RowAlign::End => {
                    let last = layout_memory::read(&ctx, id);
                    last.filter(|last| last.x < room.width()).map(|last| {
                        let spare = room.width() - last.x;
                        let lead = if *align == RowAlign::Center { spare / 2.0 } else { spare };
                        Flow::new(room, rtl).take_start(lead).1.rect()
                    })
                }
            };
            let (inner, took) = child::block(ui, rect, child::row_layout(rtl, *wrap), |ui| body(ui));
            if *align != RowAlign::Start {
                layout_memory::record(&ctx, id, took.size());
            }
            inner
        }
        BodyLayout::Grid { columns } => {
            let inner = child::block(ui, None, child::column_layout(rtl), |ui| {
                let last = last_measure(&ctx, id, columns.len());
                let cursor = GridCursor::new(Flow::new(ui.available_rect_before_wrap(), rtl), columns, gap, &last);
                Service::<Cursors>::of(&ctx).with(|cursors| cursors.0.insert(id, cursor));
                let installed = Env::install_for(ui, Env::of(ui).derive(|scoped| scoped.grid = Some(id)));
                let inner = body(ui);
                Env::restore(&ctx, installed);
                inner
            })
            .0;
            if let Some(cursor) = Service::<Cursors>::of(&ctx).with(|cursors| cursors.0.remove(&id)) {
                let measured = cursor.measured();
                for (i, width) in measured.columns.iter().enumerate() {
                    layout_memory::record(&ctx, column_id(id, i), Vec2::new(*width, 0.0));
                }
                for (i, height) in measured.rows.iter().enumerate() {
                    layout_memory::record(&ctx, row_id(id, i), Vec2::new(0.0, *height));
                }
                layout_memory::record(&ctx, sub(id, "rows"), Vec2::new(measured.rows.len() as f32, 0.0));
            }
            inner
        }
    }
}
