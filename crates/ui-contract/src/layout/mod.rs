//! The layout services: `BodyLayout` and its planning rules, `LabelColumn`, `ColumnGroup`, `measure_slot` and
//! `arrange`.

pub(crate) mod arrange;
pub(crate) mod body;
pub(crate) mod column_group;
mod label;
mod slot;

pub use arrange::{Placed, arrange};
pub use body::{BodyLayout, CellAlign, GridFit, RowAlign, ShrinkOrder, ShrinkUnit, empty_cell, levelled_unit, plan_levels, shrink_widths};
pub use column_group::ColumnGroup;
#[doc(hidden)]
pub use column_group::ambient_width;
#[doc(hidden)]
pub use column_group::report_beyond;
pub(crate) use column_group::{Columns, GLYPHS, LABELS, Reached, ambient_room, note_beside};
pub use label::{control_baseline, place_label, record_control_baseline};
pub use slot::measure_slot;

#[doc(hidden)]
pub use body::place_in_flow;
