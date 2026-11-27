//! The layout services: `BodyLayout` and its planning rules, `LabelColumn`, `ColumnGroup`, `measure_slot` and
//! `arrange`.

pub(crate) mod arrange;
pub(crate) mod body;
pub(crate) mod column_group;
mod slot;

pub use arrange::{Placed, arrange};
pub use body::{BodyLayout, CellAlign, GridFit, RowAlign, ShrinkOrder, ShrinkUnit, empty_cell, levelled_unit, plan_levels, shrink_widths};
pub use column_group::ColumnGroup;
#[doc(hidden)]
pub use column_group::ambient_width;
pub(crate) use column_group::{GLYPHS, LABELS, Report, begin_capture, end_capture, held_sweep, replay, report_held};
pub use slot::measure_slot;

#[doc(hidden)]
pub use body::place_in_flow;
pub(crate) use body::{HeldKind, flow_room, grid_cell};
