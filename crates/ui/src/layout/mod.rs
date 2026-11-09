//! The layout services: `LabelColumn`, `ColumnGroup`, the `Grid` cursor, `measure_slot` and `arrange`.

pub(crate) mod arrange;
mod body;
mod column_group;
mod label;
mod slot;

pub use arrange::{Placed, arrange};
pub use body::{BodyLayout, RowAlign};
pub use column_group::ColumnGroup;
pub(crate) use column_group::{GLYPHS, LABELS, ambient_room, ambient_width};
pub use label::{place_label, record_control_baseline};
pub use slot::measure_slot;

pub(crate) use body::{
    MainFill, flow_fill_room, flow_room, in_grid_content_column, main_fill, place_in_flow, run_body, run_body_measured, widen_cell,
};
