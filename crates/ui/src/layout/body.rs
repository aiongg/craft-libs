//! `BodyLayout` and the layout of a body: the physical egui layout, the measured placement of `Row` End and
//! Center, and the `Grid` cursor.
#![cfg_attr(not(feature = "testing"), expect(dead_code, reason = "used by the body step"))]

use crate::runtime::egui::{alloc, child};
use crate::runtime::env::Env;
use crate::runtime::layout_memory;
use crate::runtime::memory::sub;
use craft_behavior::{ColumnWidth, Flow, GridCursor, GridMeasure};
use egui::{Id, Rect, Ui, Vec2};
use std::borrow::Cow;

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

/// The cells of a body: step 6 of a child in flow asks for its rect here.
pub(crate) struct Cells {
    cursor: Option<GridCursor>,
}

impl Cells {
    /// The rect a child of measured `size` takes in flow: the next cell of a `Grid` body, else the next
    /// allocation of the body's layout.
    pub(crate) fn place(&mut self, ui: &mut Ui, size: Vec2) -> Rect {
        match &mut self.cursor {
            Some(cursor) => {
                let cell = cursor.cell(size);
                alloc::advance_past(ui, cell);
                cell
            }
            None => alloc::space(ui, size),
        }
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
pub(crate) fn run_body<R>(ui: &mut Ui, id: Id, layout: &BodyLayout, gap: Vec2, body: impl FnOnce(&mut Ui, &mut Cells) -> R) -> R {
    let rtl = Env::of(ui).direction.is_rtl();
    let ctx = ui.ctx().clone();
    match layout {
        BodyLayout::Stack => child::block(ui, None, child::column_layout(rtl), |ui| body(ui, &mut Cells { cursor: None })).0,
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
            let (inner, took) = child::block(ui, rect, child::row_layout(rtl, *wrap), |ui| body(ui, &mut Cells { cursor: None }));
            if *align != RowAlign::Start {
                layout_memory::record(&ctx, id, took.size());
            }
            inner
        }
        BodyLayout::Grid { columns } => {
            let (inner, cursor) = child::block(ui, None, child::column_layout(rtl), |ui| {
                let last = last_measure(&ctx, id, columns.len());
                let cursor = GridCursor::new(Flow::new(ui.available_rect_before_wrap(), rtl), columns, gap, &last);
                let mut cells = Cells { cursor: Some(cursor) };
                let inner = body(ui, &mut cells);
                (inner, cells.cursor)
            })
            .0;
            if let Some(cursor) = cursor {
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
