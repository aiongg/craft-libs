//! Where the picker's parts sit, in logical coordinates from the start edge and the top of its room. Measuring and
//! painting read the same plan.

use craft_theme::{ColorModel, HsbModel, OklchModel, RgbModel};
use egui::{Rect, Vec2, pos2, vec2};

/// The sizes the plan reads, from the picker's style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Sizes {
    /// The gap between items on one line.
    pub gap: f32,
    /// The gap between stacked rows.
    pub stack: f32,
    /// The gap between channel rows of the grid.
    pub tight: f32,
    /// The gap between groups.
    pub section: f32,
    /// A control's height.
    pub row: f32,
    /// The area: its least width and height.
    pub area: Vec2,
    /// The strip's width.
    pub strip: f32,
    /// The radio and label of a grid row.
    pub cell: f32,
    /// A number field's width.
    pub short: f32,
    /// The hex field's width.
    pub hex: f32,
    /// The compare swatch's width and one half's height.
    pub compare: Vec2,
    /// A caption's height.
    pub caption: f32,
    /// A warning chip's size.
    pub chip: Vec2,
}

/// Rows stacked from the top with the stack gap between them.
pub(super) struct Rows {
    y: f32,
    gap: f32,
    started: bool,
}

impl Rows {
    pub(super) fn new(gap: f32) -> Self {
        Rows { y: 0.0, gap, started: false }
    }

    /// The next row `height` high: its top.
    pub(super) fn take(&mut self, height: f32) -> f32 {
        if self.started {
            self.y += self.gap;
        }
        self.started = true;
        let top = self.y;
        self.y += height;
        top
    }

    pub(super) fn height(&self) -> f32 {
        self.y
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), vec2(w, h))
}

/// The Compact layout's rows.
#[derive(Clone, Debug)]
pub(super) struct CompactPlan {
    pub size: Vec2,
    pub area: Rect,
    pub strip: Rect,
    pub channels: Vec<Rect>,
    pub switch: Option<Rect>,
    pub hex: Rect,
    pub sample: Option<Rect>,
    pub alpha: Option<Rect>,
}

/// What the Compact layout shows.
#[derive(Clone, Copy, Debug)]
pub(super) struct CompactInput {
    /// The channels of the model whose rows show; the most any model has when measuring.
    pub channels: usize,
    pub switch: bool,
    pub sample: bool,
    pub alpha: bool,
}

pub(super) fn compact(sizes: &Sizes, width: f32, input: CompactInput) -> CompactPlan {
    let width = width.max(sizes.area.x + sizes.gap + sizes.strip);
    let area_width = width - sizes.gap - sizes.strip;
    let mut rows = Rows::new(sizes.stack);
    let top = rows.take(sizes.area.y);
    let area = rect(0.0, top, area_width, sizes.area.y);
    let strip = rect(area_width + sizes.gap, top, sizes.strip, sizes.area.y);
    let channels = (0..input.channels).map(|_| rect(0.0, rows.take(sizes.row), width, sizes.row)).collect();
    let switch = input.switch.then(|| rect(0.0, rows.take(sizes.row), width, sizes.row));
    let hex_top = rows.take(sizes.row);
    let hex_width = if input.sample { width - sizes.gap - sizes.row } else { width };
    let hex = rect(0.0, hex_top, hex_width, sizes.row);
    let sample = input.sample.then(|| rect(width - sizes.row, hex_top, sizes.row, sizes.row));
    let alpha = input.alpha.then(|| rect(0.0, rows.take(sizes.row), width, sizes.row));
    CompactPlan { size: vec2(width, rows.height()), area, strip, channels, switch, hex, sample, alpha }
}

/// A grid column of the Full layout: HSB and RGB, then Lab and the host's models, then OKLCH.
pub(super) fn column_of(model: &dyn ColorModel) -> usize {
    match model.id() {
        id if id == HsbModel.id() || id == RgbModel.id() => 0,
        id if id == OklchModel.id() => 2,
        _ => 1,
    }
}

/// A channel row of the grid.
#[derive(Clone, Copy, Debug)]
pub(super) struct Cell {
    /// The model's index among the picker's models.
    pub model: usize,
    pub channel: usize,
    pub label: Rect,
    pub field: Rect,
}

/// The Full layout's parts.
#[derive(Clone, Debug)]
pub(super) struct FullPlan {
    pub size: Vec2,
    pub area: Rect,
    pub strip: Rect,
    pub web: Option<Rect>,
    pub alpha: Option<Rect>,
    pub sample: Option<Rect>,
    /// The compare swatch's captions, then its halves: new over current.
    pub captions: Option<[Rect; 2]>,
    pub halves: Option<[Rect; 2]>,
    pub warnings: Vec<Rect>,
    pub cells: Vec<Cell>,
    /// The "#" label and the hex field.
    pub hex: (Rect, Rect),
}

/// What the Full layout shows.
#[derive(Clone, Copy, Debug)]
pub(super) struct FullInput<'a> {
    pub models: &'a [&'a dyn ColorModel],
    pub web: bool,
    pub alpha: bool,
    pub sample: bool,
    pub compare: bool,
    pub warnings: usize,
}

pub(super) fn full(sizes: &Sizes, input: FullInput<'_>) -> FullPlan {
    let side = sizes.area.x.max(sizes.area.y);
    let left_width = side + sizes.gap + sizes.strip;
    let area = rect(0.0, 0.0, side, side);
    let strip = rect(side + sizes.gap, 0.0, sizes.strip, side);
    let mut left = Rows::new(sizes.stack);
    left.take(side);
    let web = input.web.then(|| rect(0.0, left.take(sizes.row), left_width, sizes.row));
    let alpha = input.alpha.then(|| rect(0.0, left.take(sizes.row), left_width, sizes.row));
    let sample = input.sample.then(|| rect(0.0, left.take(sizes.row), sizes.row, sizes.row));
    let x0 = left_width + sizes.section;

    let mut top = 0.0;
    let (mut captions, mut halves) = (None, None);
    if input.compare {
        let (w, half) = (sizes.compare.x, sizes.compare.y);
        captions = Some([rect(x0, 0.0, w, sizes.caption), rect(x0, sizes.caption + 2.0 * half, w, sizes.caption)]);
        halves = Some([rect(x0, sizes.caption, w, half), rect(x0, sizes.caption + half, w, half)]);
        top = 2.0 * sizes.caption + 2.0 * half;
    }
    let chips_x = x0 + if input.compare { sizes.compare.x + sizes.gap } else { 0.0 };
    let chips_y = if input.compare { sizes.caption } else { 0.0 };
    let mut chip_rows = Rows::new(sizes.stack);
    let warnings: Vec<Rect> = (0..input.warnings).map(|_| rect(chips_x, chips_y + chip_rows.take(sizes.chip.y), sizes.chip.x, sizes.chip.y)).collect();
    if input.warnings > 0 {
        top = top.max(chips_y + chip_rows.height());
    }
    let grid_top = if top > 0.0 { top + sizes.stack } else { 0.0 };

    let mut columns: [Vec<usize>; 3] = Default::default();
    for (index, model) in input.models.iter().enumerate() {
        columns[column_of(*model)].push(index);
    }
    let mut cells = Vec::new();
    let mut hex = (Rect::ZERO, Rect::ZERO);
    let mut x = x0;
    let mut grid_bottom = grid_top;
    let mut shown = 0;
    for (c, column) in columns.iter().enumerate() {
        if c != 0 && column.is_empty() {
            continue;
        }
        let field_width = if c == 0 { sizes.short.max(sizes.hex) } else { sizes.short };
        let mut y = grid_top;
        for (g, index) in column.iter().enumerate() {
            if g > 0 {
                y += sizes.stack - sizes.tight;
            }
            for channel in 0..input.models[*index].channels().len() {
                let label = rect(x, y, sizes.cell, sizes.row);
                let field = rect(x + sizes.cell + sizes.gap, y, sizes.short, sizes.row);
                cells.push(Cell { model: *index, channel, label, field });
                y += sizes.row + sizes.tight;
            }
        }
        if c == 0 {
            y += sizes.stack - sizes.tight;
            hex = (rect(x, y, sizes.cell, sizes.row), rect(x + sizes.cell + sizes.gap, y, field_width, sizes.row));
            y += sizes.row + sizes.tight;
        }
        grid_bottom = grid_bottom.max(y - sizes.tight);
        x += sizes.cell + sizes.gap + field_width + sizes.section;
        shown += 1;
    }
    let right = if shown > 0 { x - sizes.section } else { x0 };
    let height = left.height().max(grid_bottom);
    FullPlan { size: vec2(right, height), area, strip, web, alpha, sample, captions, halves, warnings, cells, hex }
}
