//! `BodyLayout` and the layout of a body: the physical egui layout, the measured placement of `Row` End and
//! Center, and the cursors of `Grid` and of a `Row` that spreads its children.

use crate::runtime::egui::memory::Service;
use crate::runtime::egui::{alloc, child};
use crate::runtime::env::Env;
use crate::runtime::layout_memory;
use crate::runtime::measure;
use crate::runtime::memory::sub;
use craft_behavior::{Axis, ColumnWidth, Flow, GridCursor, GridMeasure};
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
    /// Spread over the row: the first child at the start edge, the last at the end edge, and the room left shared
    /// equally between the children (a footer's leading slot and its buttons). One child sits at the start edge, and so
    /// do the children of a row that wraps.
    SpaceBetween,
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

/// The cursors of the `Grid` and spread `Row` bodies running this pass, by body id.
#[derive(Default)]
struct Cursors(HashMap<Id, Cursor>);

/// The number of children that fill the main axis of each `Stack` or `Row` body measured this pass, by body id.
#[derive(Default)]
struct FillCounts(HashMap<Id, u32>);

/// How a child that fills the main axis of the body it sits in sizes along it.
pub(crate) enum MainFill {
    /// The body is being measured: the child takes no room along the axis and is counted.
    Measuring,
    /// The room it takes along the axis.
    Share(f32),
    /// Not a child that shares: it takes the room as before.
    Room,
}

/// What a child of `ui`'s body that fills along `axis` takes there. Called once per such child and pass.
pub(crate) fn main_fill(ui: &Ui, axis: Axis) -> MainFill {
    let env = Env::of(ui);
    let Some((body, _)) = env.fill_body.filter(|(_, along)| *along == axis) else { return MainFill::Room };
    if ui.is_sizing_pass() {
        Service::<FillCounts>::of(ui.ctx()).with(|counts| *counts.0.entry(body).or_insert(0) += 1);
        return MainFill::Measuring;
    }
    env.fill_share.map_or(MainFill::Room, MainFill::Share)
}

/// Runs `body` as the content of the `Stack` (along y) or unwrapped `Row` (along x) body `id` in `ui`, the block
/// it is placed in. A measuring run places the children that fill the main axis at no extent and records what the
/// others and the gaps took, with the number that fill; the real run gives each of them an equal share of the rest.
fn run_sharing<R>(ui: &mut Ui, id: Id, axis: Axis, layout: egui::Layout, body: impl FnOnce(&mut Ui) -> R) -> R {
    let ctx = ui.ctx().clone();
    let env = Env::of(ui);
    let given = if axis == Axis::Y { !env.content.1 } else { !env.content.0 };
    if !given {
        return child::block(ui, None, layout, body).0;
    }
    let key = sub(id, "share");
    let measuring = ui.is_sizing_pass();
    let (result, used) = child::block(ui, None, layout, |ui| {
        let room = ui.available_rect_before_wrap();
        let room = if axis == Axis::Y { room.height() } else { room.width() };
        let share = if measuring {
            Service::<FillCounts>::run(&ctx, |counts| counts.0.insert(id, 0));
            None
        } else {
            layout_memory::measured(&ctx, key)
                .filter(|found| found.took.y >= 1.0 && room.is_finite())
                .map(|found| ((room - found.took.x) / found.took.y).max(0.0))
        };
        let installed = Env::install_for(
            ui,
            Env::of(ui).derive_local(|scoped| {
                scoped.fill_body = Some((id, axis));
                scoped.fill_share = share;
            }),
        );
        let inner = body(ui);
        Env::restore(&ctx, installed);
        inner
    });
    if measuring {
        let fills = Service::<FillCounts>::run(&ctx, |counts| counts.0.remove(&id)).unwrap_or(0);
        let fixed = if axis == Axis::Y { used.height() } else { used.width() };
        layout_memory::record_measured(&ctx, key, layout_memory::Measured { room: Vec2::ZERO, took: Vec2::new(fixed, fills as f32) });
    }
    result
}

/// What places the children in flow of a body that places them itself.
enum Cursor {
    Grid(GridCursor),
    Spread(Spread),
}

impl Cursor {
    fn cell(&mut self, size: Vec2) -> Rect {
        match self {
            Cursor::Grid(grid) => grid.cell(size),
            Cursor::Spread(row) => row.cell(size),
        }
    }

    fn room(&self) -> Vec2 {
        match self {
            Cursor::Grid(grid) => grid.room(),
            Cursor::Spread(row) => row.room(),
        }
    }

    fn fill_room(&self) -> Vec2 {
        match self {
            Cursor::Grid(grid) => grid.fill_room(),
            Cursor::Spread(row) => row.room(),
        }
    }

    fn in_content_column(&self) -> bool {
        matches!(self, Cursor::Grid(grid) if grid.in_content_column())
    }

    fn widen(&mut self, size: Vec2) {
        match self {
            Cursor::Grid(grid) => grid.widen(size),
            Cursor::Spread(row) => row.widen(size),
        }
    }
}

/// The cursor of a `SpaceBetween` row: each child in flow takes the next place along the row, and the room the
/// children left last pass is shared between them. It places from last pass's measure, as `End` and `Center` do.
struct Spread {
    flow: Flow,
    gap: f32,
    /// The space between two children: the gap and the share of the room left.
    between: f32,
    /// How far from the start edge the last child placed ends.
    at: f32,
    /// The width of the last child placed.
    last: f32,
    /// What the children took this pass at the gap alone: their widths and the gaps between them.
    natural: f32,
    count: usize,
}

impl Spread {
    /// A cursor over `flow` with `gap` between children, spreading by `last`: the natural width and the number of
    /// the children last pass.
    fn new(flow: Flow, gap: f32, last: Option<Vec2>) -> Self {
        let width = flow.rect().width();
        let between = last.filter(|last| last.y >= 2.0 && last.x < width).map_or(gap, |last| gap + (width - last.x) / (last.y - 1.0));
        Spread { flow, gap, between, at: 0.0, last: 0.0, natural: 0.0, count: 0 }
    }

    fn start(&self) -> f32 {
        if self.count == 0 { 0.0 } else { self.at + self.between }
    }

    fn cell(&mut self, size: Vec2) -> Rect {
        let start = self.start();
        let span = self.flow.span(start, size.x);
        self.natural += size.x + if self.count == 0 { 0.0 } else { self.gap };
        (self.at, self.last, self.count) = (start + size.x, size.x, self.count + 1);
        Rect::from_min_size(span.min, Vec2::new(span.width(), size.y))
    }

    fn room(&self) -> Vec2 {
        let rect = self.flow.rect();
        Vec2::new((rect.width() - self.start()).max(0.0), rect.height())
    }

    fn widen(&mut self, size: Vec2) {
        let grow = (size.x - self.last).max(0.0);
        (self.at, self.natural, self.last) = (self.at + grow, self.natural + grow, self.last + grow);
    }

    /// What `new` spreads by next pass.
    fn measured(&self) -> Vec2 {
        Vec2::new(self.natural, self.count as f32)
    }
}

/// The rect a child of measured `size` takes in flow in `ui`: the next cell of the `Grid` or spread `Row` body that
/// encloses it directly, else the next allocation of `ui`'s layout.
pub(crate) fn place_in_flow(ui: &mut Ui, size: Vec2) -> Rect {
    let cell =
        Env::of(ui).grid.and_then(|body| Service::<Cursors>::of(ui.ctx()).with(|cursors| cursors.0.get_mut(&body).map(|cursor| cursor.cell(size))));
    match cell {
        Some(cell) => {
            alloc::advance_past(ui, cell);
            cell
        }
        None => alloc::space(ui, size),
    }
}

/// The room a child shown in flow in `ui` measures and sizes in: the room of the next cell of the `Grid` or spread
/// `Row` body that encloses `ui` directly, else the rest of `ui`'s layout.
pub(crate) fn flow_room(ui: &Ui) -> Vec2 {
    let cell = Env::of(ui).grid.and_then(|body| Service::<Cursors>::of(ui.ctx()).with(|cursors| cursors.0.get(&body).map(Cursor::room)));
    cell.unwrap_or_else(|| open_room(ui))
}

/// The rest of `ui`'s layout (the room offered to the container, less the earlier siblings and gaps); unbounded along the axes of the scrolling body that encloses it, whose region is only
/// as large as egui can make it.
fn open_room(ui: &Ui) -> Vec2 {
    let (x, y) = Env::of(ui).unbounded;
    let mut room = ui.available_size_before_wrap();
    // A body sized from its content offers its children the width it was offered, and a row body that placed its
    // block behind a lead the width of its band; either less what they took.
    let env = Env::of(ui);
    if let Some(offered) = env.offered.or(env.row_room) {
        room.x += offered - ui.max_rect().width();
    }
    Vec2::new(if x { f32::INFINITY } else { room.x }, if y { f32::INFINITY } else { room.y })
}

/// The room a child that fills its width fills in `ui`: `flow_room`, except that in a `Content` column of a `Grid` it
/// is the column's width, so the child never measures as the room and pushes the next column out of it.
pub(crate) fn flow_fill_room(ui: &Ui) -> Vec2 {
    let cell = Env::of(ui).grid.and_then(|body| Service::<Cursors>::of(ui.ctx()).with(|cursors| cursors.0.get(&body).map(Cursor::fill_room)));
    cell.unwrap_or_else(|| open_room(ui))
}

/// Whether the next child shown in flow in `ui` goes in a `Content` column of the `Grid` body that encloses `ui`
/// directly: like a content-sized column, a child that fills its width is placed at its measured width there.
pub(crate) fn in_grid_content_column(ui: &Ui) -> bool {
    Env::of(ui).grid.is_some_and(|body| Service::<Cursors>::of(ui.ctx()).with(|cursors| cursors.0.get(&body).is_some_and(Cursor::in_content_column)))
}

/// The cell `place_in_flow` placed last in the `Grid` or spread `Row` body that encloses `ui` directly measured `size`
/// after all: a block placed without a size entry learns its real size once its regions have run.
pub(crate) fn widen_cell(ui: &Ui, size: Vec2) {
    if let Some(body) = Env::of(ui).grid {
        Service::<Cursors>::of(ui.ctx()).with(|cursors| {
            if let Some(cursor) = cursors.0.get_mut(&body) {
                cursor.widen(size);
            }
        });
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
/// `Row` with `End` or `Center` places its block from the size it took last pass, `SpaceBetween` shares the room
/// its children left last pass between them, and `Grid` places its cells from last pass's widest child per column
/// and tallest per row. A change of those sizes asks for one discard pass under the tolerance rule.
pub(crate) fn run_body<R>(ui: &mut Ui, id: Id, layout: &BodyLayout, gap: Vec2, body: impl FnOnce(&mut Ui) -> R) -> R {
    run_body_measured(ui, id, layout, gap, body).0
}

/// `run_body`, with the width the children take at the gap alone for a `Row` that places its block itself (`End`,
/// `Center`, `SpaceBetween`): the width the body asks for, where the room it is placed in spans wider.
pub(crate) fn run_body_measured<R>(ui: &mut Ui, id: Id, layout: &BodyLayout, gap: Vec2, body: impl FnOnce(&mut Ui) -> R) -> (R, Option<f32>) {
    let rtl = Env::of(ui).direction.is_rtl();
    let ctx = ui.ctx().clone();
    match layout {
        BodyLayout::Stack => (run_sharing(ui, id, Axis::Y, child::column_layout(rtl), body), None),
        BodyLayout::Row { wrap: false, align: RowAlign::SpaceBetween } => {
            let (inner, _) = child::block(ui, None, child::row_layout(rtl, false), |ui| {
                let flow = Flow::new(ui.available_rect_before_wrap(), rtl);
                let spread = Spread::new(flow, gap.x, layout_memory::read(&ctx, id));
                Service::<Cursors>::run(&ctx, |cursors| cursors.0.insert(id, Cursor::Spread(spread)));
                let installed = Env::install_for(ui, Env::of(ui).derive_local(|scoped| scoped.grid = Some(id)));
                let inner = body(ui);
                Env::restore(&ctx, installed);
                inner
            });
            let measured = match Service::<Cursors>::run(&ctx, |cursors| cursors.0.remove(&id)) {
                Some(Cursor::Spread(spread)) => Some(spread.measured()),
                _ => None,
            };
            if let Some(measured) = measured {
                layout_memory::record(&ctx, id, measured);
            }
            (inner, measured.map(|measured| measured.x))
        }
        BodyLayout::Row { wrap: false, align: RowAlign::Start } => {
            let room = ui.available_rect_before_wrap();
            let inner = run_sharing(ui, id, Axis::X, child::row_layout(rtl, false), |ui| {
                let installed = Env::install_for(ui, Env::of(ui).derive_local(|scoped| scoped.row_room = Some(room.width())));
                let inner = body(ui);
                Env::restore(&ctx, installed);
                inner
            });
            (inner, None)
        }
        BodyLayout::Row { wrap, align } => {
            let room = ui.available_rect_before_wrap();
            let rect = match *align {
                RowAlign::Start | RowAlign::SpaceBetween => None,
                RowAlign::Center | RowAlign::End => {
                    let last = layout_memory::read(&ctx, id);
                    // Without a measured width from last pass the block is placed as `Start`.
                    last.filter(|last| last.x > 0.0 && last.x < room.width()).map(|last| {
                        let spare = room.width() - last.x;
                        let lead = if *align == RowAlign::Center { spare / 2.0 } else { spare };
                        Flow::new(room, rtl).take_start(lead).1.rect()
                    })
                }
            };
            let (inner, took) = child::block(ui, rect, child::row_layout(rtl, *wrap), |ui| {
                let installed = Env::install_for(ui, Env::of(ui).derive_local(|scoped| scoped.row_room = Some(room.width())));
                let inner = body(ui);
                Env::restore(&ctx, installed);
                inner
            });
            if *align != RowAlign::Start {
                layout_memory::record(&ctx, id, took.size());
            }
            (inner, (*align != RowAlign::Start).then(|| took.width()))
        }
        BodyLayout::Grid { columns } => {
            let inner = child::block(ui, None, child::column_layout(rtl), |ui| {
                let last = last_measure(&ctx, id, columns.len());
                let sizing = measure::active(&ctx) && Env::of(ui).content.0;
                // A measuring run under a body sized from its content gives each fill column the width its cells
                // take, as a content column: the room it would share is not the body's to claim.
                let cursor = if sizing {
                    let kinds: Vec<ColumnWidth> =
                        columns.iter().map(|kind| if matches!(kind, ColumnWidth::Fill { .. }) { ColumnWidth::Content } else { *kind }).collect();
                    let unknown = GridMeasure::new(Vec::new(), last.rows.clone());
                    GridCursor::new(Flow::new(ui.available_rect_before_wrap(), rtl), &kinds, gap, &unknown)
                } else {
                    GridCursor::new(Flow::new(ui.available_rect_before_wrap(), rtl), columns, gap, &last)
                };
                Service::<Cursors>::run(&ctx, |cursors| cursors.0.insert(id, Cursor::Grid(cursor)));
                let installed = Env::install_for(ui, Env::of(ui).derive_local(|scoped| scoped.grid = Some(id)));
                let inner = body(ui);
                Env::restore(&ctx, installed);
                inner
            })
            .0;
            if let Some(Cursor::Grid(cursor)) = Service::<Cursors>::run(&ctx, |cursors| cursors.0.remove(&id)) {
                let measured = cursor.measured();
                for (i, width) in measured.columns.iter().enumerate() {
                    layout_memory::record(&ctx, column_id(id, i), Vec2::new(*width, 0.0));
                }
                for (i, height) in measured.rows.iter().enumerate() {
                    layout_memory::record(&ctx, row_id(id, i), Vec2::new(0.0, *height));
                }
                layout_memory::record(&ctx, sub(id, "rows"), Vec2::new(measured.rows.len() as f32, 0.0));
            }
            (inner, None)
        }
    }
}
