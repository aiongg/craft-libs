//! `BodyLayout` and the pure rules a body's layout plans with: the levels of a row's levelled units
//! (`plan_levels`), the widths of a row's shrinking units (`shrink_widths`), and the levelled unit and cell
//! helpers on the `Ui` cursor. The body itself is laid out by `contract/rt/container.rs` and the cursor's flow.

use crate::contract::Ui;
use craft_behavior::ColumnWidth;
use craft_ui_rt::{Rect, Vec2};
use std::borrow::Cow;

/// How a body lays out its children, in reading order: mirrored under a right-to-left direction.
#[derive(Clone, Debug, PartialEq, crate::contract::MemoProps)]
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
        /// What the grid does when its columns do not fit beside each other.
        fit: GridFit,
        /// Where a cell shorter than its row stands in it.
        cells: CellAlign,
    },
}

/// Where a `Grid` sets a cell that is shorter than its row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum CellAlign {
    /// In the row's middle: a form's label beside a taller control.
    #[default]
    Center,
    /// At the row's top: panels of different heights side by side whose first lines stand level (the Color Picker's
    /// field groups, its buttons beside its compare block).
    Top,
}

/// What a `Grid` does when the least widths of its columns do not fit beside each other in its room.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum GridFit {
    /// The cells go one under another across the room, so no column is cut: a form that reflows in a narrow panel.
    #[default]
    Stack,
    /// The columns stay beside each other at their least widths and the grid overflows its room: a component whose
    /// arrangement is fixed (the Color Picker's channel groups), which a narrow room must not rearrange.
    Fixed,
}

/// Where a `Row` places its block of children along the row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::contract::MemoProps)]
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

    /// Children row-major over `columns`: a `'static` slice, or columns built in the build. Each child shown in the
    /// body takes the next cell; `empty_cell` leaves one empty.
    pub fn grid(columns: impl Into<Cow<'static, [ColumnWidth]>>) -> Self {
        BodyLayout::Grid { columns: columns.into(), fit: GridFit::Stack, cells: CellAlign::Center }
    }

    /// A `Grid` with `fit`; any other layout is unchanged.
    pub fn fit(self, fit: GridFit) -> Self {
        match self {
            BodyLayout::Grid { columns, cells, .. } => BodyLayout::Grid { columns, fit, cells },
            other => other,
        }
    }

    /// A `Grid` with `cells`; any other layout is unchanged.
    pub fn cells(self, cells: CellAlign) -> Self {
        match self {
            BodyLayout::Grid { columns, fit, .. } => BodyLayout::Grid { columns, fit, cells },
            other => other,
        }
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

/// What makes a child of a content-sized `Stack` column a held child: it measures at its own width in the column's
/// first measuring sweep, while the column settles its width from the others, and shows at the column's width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeldKind {
    /// A picture whose height follows from its width that took its least for want of room
    /// (`MeasureCx::picture_size`).
    Picture,
}

/// The widths of a unit of a row that shrinks to fit: the width it asks for and the least it narrows to.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShrinkUnit {
    /// The width the unit takes where the row has room.
    pub ideal: f32,
    /// The least width the unit narrows to.
    pub least: f32,
}

impl ShrinkUnit {
    /// A unit that asks for `ideal` and narrows to `least`.
    pub const fn new(ideal: f32, least: f32) -> Self {
        ShrinkUnit { ideal, least }
    }
}

/// Which units of a row too long for its room give way first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShrinkOrder {
    /// Every unit gives the same share of what it can give, so all reach their least together.
    #[default]
    Even,
    /// The last unit in reading order goes to its least before the one before it gives anything: a bar gives way from
    /// its end, where the members that matter least sit.
    FromEnd,
}

/// The widths of the units of a row whose units, with `fixed` (its other children and gaps), ask for more than `room`:
/// the shortfall comes off the units in `order`, never past the least each reports (its ideal less what it can give).
/// A row that fits gives each unit its ideal, and so does a row that does not fit even with every unit at its least:
/// it overflows (or its body scrolls) at its natural widths, instead of cutting its units to their least and
/// overflowing all the same.
pub fn shrink_widths(units: &[ShrinkUnit], fixed: f32, room: f32, order: ShrinkOrder) -> Vec<f32> {
    let short = fixed + units.iter().map(|unit| unit.ideal).sum::<f32>() - room;
    let slack: f32 = units.iter().map(|unit| unit.ideal - unit.least).sum();
    // Half a point: less than a pixel shows at any scale the box runs.
    let fits_at_least = short - slack < 0.5;
    if short <= 0.0 || slack <= 0.0 || !fits_at_least {
        return units.iter().map(|unit| unit.ideal).collect();
    }
    match order {
        ShrinkOrder::Even => {
            let share = (short / slack).min(1.0);
            units.iter().map(|unit| unit.ideal - (unit.ideal - unit.least) * share).collect()
        }
        ShrinkOrder::FromEnd => {
            let mut left = short;
            let mut widths: Vec<f32> = units
                .iter()
                .rev()
                .map(|unit| {
                    let given = (unit.ideal - unit.least).min(left);
                    left -= given;
                    unit.ideal - given
                })
                .collect();
            widths.reverse();
            widths
        }
    }
}

/// The widths of the units of a row too long for its room, in turns. First every unit gives the room it holds beyond
/// its content, down to its `whole` width, in `order`: a field gives its blank length before a readout beside it
/// wraps. Only when that does not cover the shortfall does content give way, from the whole widths: the units whose
/// content is made to be cut (`first`) give what they can, and the others only the rest. A row that does not fit
/// even with every unit at its least keeps the natural widths, as `shrink_widths` does.
fn shrink_in_turns(units: &[ShrinkUnit], whole: &[f32], first: &[bool], fixed: f32, room: f32, order: ShrinkOrder) -> Vec<f32> {
    let at_whole: Vec<ShrinkUnit> =
        units.iter().zip(whole).map(|(unit, whole)| ShrinkUnit { ideal: unit.ideal, least: whole.max(unit.least).min(unit.ideal) }).collect();
    let slack = |units: &[ShrinkUnit]| units.iter().map(|unit| unit.ideal - unit.least).sum::<f32>();
    let short = fixed + units.iter().map(|unit| unit.ideal).sum::<f32>() - room;
    // Half a point: less than a pixel shows at any scale the box runs.
    if short - slack(&at_whole) < 0.5 {
        return shrink_widths(&at_whole, fixed, room, order);
    }
    let cut: Vec<ShrinkUnit> = at_whole.iter().zip(units).map(|(whole, unit)| ShrinkUnit { ideal: whole.least, least: unit.least }).collect();
    if short - slack(&at_whole) - slack(&cut) >= 0.5 {
        return units.iter().map(|unit| unit.ideal).collect();
    }
    shrink_widths_in_turn(&cut, first, fixed, room, order)
}

/// What the shrink step of a row gave one of its units (`shrink_or_drop`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShrinkCap {
    /// The width the unit shows at most.
    pub width: f32,
    /// The unit dropped out of the row: it shows nothing and takes no place.
    pub dropped: bool,
    /// The row gave the unit less than it asks for in no room, or dropped it: what the unit shows depends on the row's
    /// room. A unit the row did not narrow shows as in no room.
    pub narrowed: bool,
}

/// The widths of the units of a row too long for its room, as `shrink_in_turns` gives them, in a row that `drops`
/// (one whose body does not scroll): when the units do not fit even at their least, the units whose content is made
/// to be cut (`first`) drop out from the end, each with one `gap`, until the rest fits at their least. A unit that
/// drops gets no width and takes no place, so nothing overflows into a neighbour; controls never drop. A row that
/// scrolls keeps every unit and scrolls instead.
///
/// Before those drops, the units that carry a rank (`ranks`) leave whole, scrolling row or not: the lowest rank first,
/// the last of a rank first, until the kept units fit at their ideal widths in `rank_room` (at most `room`). A unit
/// with no rank never leaves by rank.
fn shrink_or_drop(
    units: &[ShrinkUnit],
    whole: &[f32],
    (first, ranks): (&[bool], &[Option<u8>]),
    (fixed, gap): (f32, f32),
    (room, rank_room): (f32, f32),
    order: ShrinkOrder,
    drops: bool,
) -> Vec<ShrinkCap> {
    let mut kept = vec![true; units.len()];
    let mut fixed = fixed;
    // Half a point: less than a pixel shows at any scale the box runs.
    let fits_at = |kept: &[bool], fixed: f32, width: fn(&ShrinkUnit) -> f32, room: f32| {
        fixed + units.iter().zip(kept).filter(|(_, kept)| **kept).map(|(unit, _)| width(unit)).sum::<f32>() - room < 0.5
    };
    let fits = |kept: &[bool], fixed: f32| fits_at(kept, fixed, |unit| unit.least, room);
    while !fits_at(&kept, fixed, |unit| unit.ideal, rank_room.min(room)) {
        let Some(rank) = (0..units.len()).filter(|at| kept[*at]).filter_map(|at| ranks[at]).min() else { break };
        let Some(at) = (0..units.len()).rev().find(|at| kept[*at] && ranks[*at] == Some(rank)) else { break };
        kept[at] = false;
        fixed -= gap;
    }
    while drops && !fits(&kept, fixed) {
        let Some(at) = (0..units.len()).rev().find(|at| kept[*at] && first[*at]) else { break };
        kept[at] = false;
        fixed -= gap;
    }
    let pick = |all: &[f32]| -> Vec<f32> { all.iter().zip(&kept).filter(|(_, kept)| **kept).map(|(value, _)| *value).collect() };
    let shown: Vec<ShrinkUnit> = units.iter().zip(&kept).filter(|(_, kept)| **kept).map(|(unit, _)| *unit).collect();
    let shown_first: Vec<bool> = first.iter().zip(&kept).filter(|(_, kept)| **kept).map(|(first, _)| *first).collect();
    let mut widths = shrink_in_turns(&shown, &pick(whole), &shown_first, fixed, room, order).into_iter();
    kept.iter()
        .zip(units)
        .map(|(kept, unit)| match kept {
            true => {
                let width = widths.next().unwrap_or_default();
                ShrinkCap { width, dropped: false, narrowed: unit.ideal - width >= 0.5 }
            }
            false => ShrinkCap { width: 0.0, dropped: true, narrowed: true },
        })
        .collect()
}

/// The shrink step of an unwrapped `Row` too long for its `room`: the width each child shows at most, from its width
/// in no room (`ideal`) and its least (`least`), both from the row's measuring run, with `gap` between children. A child
/// that fills the row (`ideal[i].1`) is no unit: it takes what the units leave, and gets `None`. The units give way
/// evenly down to their least; a row too long even then keeps their ideal widths. Nothing drops: a row whose body does
/// not scroll keeps every unit.
pub(crate) fn row_shrink(ideal: &[(Vec2, bool)], least: &[(Vec2, bool)], gap: f32, room: f32) -> Vec<Option<f32>> {
    let units: Vec<ShrinkUnit> = ideal
        .iter()
        .zip(least)
        .filter(|((_, fills), _)| !*fills)
        .map(|((ideal, _), (least, _))| ShrinkUnit { ideal: ideal.x, least: least.x.min(ideal.x) })
        .collect();
    let whole: Vec<f32> = units.iter().map(|unit| unit.ideal).collect();
    let none = vec![false; units.len()];
    let ranks = vec![None; units.len()];
    let fixed = gap * ideal.len().saturating_sub(1) as f32;
    let mut caps = shrink_or_drop(&units, &whole, (&none, &ranks), (fixed, gap), (room, room), ShrinkOrder::Even, false).into_iter();
    ideal.iter().map(|(_, fills)| if *fills { None } else { caps.next().map(|cap| cap.width) }).collect()
}

/// A leaf's measured `size` in a row whose plan narrows its units (`Shrink::Apply`): at most the `room` the plan gave
/// it, which is never below its least, so a leaf whose measure does not follow its room still fits.
pub(crate) fn shrink_cap(env: &crate::runtime::env::Env, room: Vec2, size: Vec2) -> Vec2 {
    match env.shrink {
        Some(crate::runtime::env::Shrink::Apply(_)) => Vec2::new(size.x.min(room.x), size.y),
        _ => size,
    }
}

/// Whether the children of an unwrapped `Row`, at `sizes` with `gap` between them, overflow `room`.
pub(crate) fn row_overflows(sizes: &[(Vec2, bool)], gap: f32, room: f32) -> bool {
    let used = sizes.iter().filter(|(_, fills)| !*fills).map(|(size, _)| size.x).sum::<f32>() + gap * sizes.len().saturating_sub(1) as f32;
    // Half a point: less than a pixel shows at any scale the box runs.
    room.is_finite() && used - room >= 0.5
}

/// `shrink_widths` in two turns: the units whose content is made to be cut (`first`) give what they can of the
/// shortfall, and the others give only the rest. A row that does not fit even with every unit at its least keeps the
/// natural widths, as `shrink_widths` does.
fn shrink_widths_in_turn(units: &[ShrinkUnit], first: &[bool], fixed: f32, room: f32, order: ShrinkOrder) -> Vec<f32> {
    let pick = |turn: bool| -> Vec<ShrinkUnit> { units.iter().zip(first).filter(|(_, first)| **first == turn).map(|(unit, _)| *unit).collect() };
    let (cut, rest) = (pick(true), pick(false));
    let slack = |units: &[ShrinkUnit]| units.iter().map(|unit| unit.ideal - unit.least).sum::<f32>();
    let short = fixed + units.iter().map(|unit| unit.ideal).sum::<f32>() - room;
    // Half a point: less than a pixel shows at any scale the box runs.
    if short <= 0.0 || slack(&cut) <= 0.0 || short - slack(&cut) - slack(&rest) >= 0.5 {
        return shrink_widths(units, fixed, room, order);
    }
    let ideal = |units: &[ShrinkUnit]| units.iter().map(|unit| unit.ideal).sum::<f32>();
    // The cut units take the shortfall against the others at their ideal, down to their least.
    let cut_widths =
        if short >= slack(&cut) { cut.iter().map(|unit| unit.least).collect() } else { shrink_widths(&cut, fixed + ideal(&rest), room, order) };
    let rest_widths = shrink_widths(&rest, fixed + cut_widths.iter().sum::<f32>(), room, order);
    let (mut cut_widths, mut rest_widths) = (cut_widths.into_iter(), rest_widths.into_iter());
    first.iter().map(|first| if *first { cut_widths.next() } else { rest_widths.next() }.unwrap_or_default()).collect()
}

/// The level of each unit of a row whose units show in discrete levels, for `room`. `units` holds each unit's width
/// at each of its levels, widest (level 0) first. The units give way from the end: first each steps down to its level
/// 1, then each to its level 2, and so on, stopping as soon as the row fits. Then, from the start, each unit takes back
/// a level while the row still fits: stepping a wide unit down can free room an earlier unit no longer needed to give
/// up. A unit with fewer levels stays at its last. A width that is not a number never fits, so it gives nothing back.
///
/// Worked through for units `[300, 250, 60]`, `[500, 500, 60]`, `[60]`, `[60]` in 800: the total 920 does not fit;
/// stepping to level 1 from the end leaves 870, still too wide; stepping to level 2 from the end reaches the second
/// unit (60), and the row fits at 430. Giving back from the start: the first unit takes level 0 (480); the second
/// cannot (920 > 800). The levels are `[0, 2, 0, 0]`.
pub fn plan_levels(units: &[Vec<f32>], room: f32) -> Vec<usize> {
    let width = |unit: &[f32], level: usize| unit.get(level).or(unit.last()).copied().unwrap_or_default();
    let deepest = units.iter().map(|unit| unit.len().saturating_sub(1)).max().unwrap_or_default();
    let mut levels = vec![0; units.len()];
    let mut total: f32 = units.iter().map(|unit| width(unit, 0)).sum();
    for target in 1..=deepest {
        for (unit, level) in units.iter().zip(levels.iter_mut()).rev() {
            if total <= room {
                break;
            }
            if target < unit.len() {
                total += width(unit, target) - width(unit, *level);
                *level = target;
            }
        }
    }
    for (unit, level) in units.iter().zip(levels.iter_mut()) {
        while *level > 0 {
            let next = total + width(unit, *level - 1) - width(unit, *level);
            if !next.is_finite() || next > room {
                break;
            }
            total = next;
            *level -= 1;
        }
    }
    levels
}

/// Shows `show`, one unit of the unwrapped `Row` body around it that fits in discrete levels (a ribbon group: full,
/// compact, collapsed), at the level the row's plan gave it. `levels` is how many levels the unit has; `show` shows it
/// at a level from 0 (the widest) to `levels - 1`, and must be free of side effects in a measuring run, as every body
/// is: the row's measuring run runs it once per level to measure each. `key` names the unit within its row.
///
/// The row's plan gives its levelled units their levels (`plan_levels`, in the room its other children leave), and
/// the real run shows each unit once, at its level. Outside a row's body the unit shows at level 0. The unit's
/// children are placed in the unit's own flow, in the width its level took.
pub fn levelled_unit(ui: &mut Ui, key: crate::contract::Id, levels: usize, mut show: impl FnMut(&mut Ui, usize)) {
    let levels = levels.max(1);
    if ui.flow.measuring.is_some() {
        let mut widths = Vec::with_capacity(levels);
        let mut widest = Vec2::ZERO;
        for level in 0..levels {
            let took = ui.unit(key, false, Vec2::ZERO, &mut |ui| show(ui, level));
            if level == 0 {
                widest = took;
            }
            widths.push(took.x);
        }
        ui.flow.body.units.push(widths);
        ui.flow.allocate(None, widest, false);
        return;
    }
    let level = ui.flow.body.next_level().min(levels - 1);
    ui.unit(key, true, Vec2::ZERO, &mut |ui| show(ui, level));
}

/// Places a child of `size` in the flow of `ui` and returns its rect: a cell of a `Grid`, the next place in a `Row`
/// or `Stack`.
#[doc(hidden)]
pub fn place_in_flow(ui: &mut Ui, size: Vec2) -> Rect {
    ui.flow.allocate(None, size, false)
}

/// The room a child shown in flow in `ui` gets at the next place: its cell in a `Grid`, what the line or column
/// leaves in a `Row` or `Stack`, the room the body's plan gave it.
pub(crate) fn flow_room(ui: &Ui) -> Vec2 {
    ui.flow.room()
}

/// The `Grid` body and column the next child shown in `ui` takes a cell of, for its label step's
/// `ColumnGroup::cell`; `None` outside a grid's body.
pub(crate) fn grid_cell(ui: &Ui) -> Option<(crate::contract::Id, usize)> {
    let column = ui.flow.grid_column()?;
    Some((crate::runtime::memory::sub(ui.flow.id, "grid"), column))
}

/// Leaves the next cell of a `Grid` body empty; in any other body it takes no room.
pub fn empty_cell(ui: &mut Ui) {
    if ui.flow.is_grid() {
        ui.flow.allocate(None, Vec2::ZERO, false);
    }
}
