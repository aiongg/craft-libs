//! The layout services: `LabelColumn`, `ColumnGroup`, the `Grid` cursor, `measure_slot` and `arrange`.

pub(crate) mod arrange;
mod body;
mod column_group;
mod label;
mod slot;

pub use arrange::{Placed, arrange};
pub use body::{BodyLayout, RowAlign};
pub use column_group::ColumnGroup;
pub(crate) use column_group::{GLYPHS, LABELS, ambient_room, ambient_width, report_beyond};
pub use label::{control_baseline, place_label, record_control_baseline};
pub use slot::measure_slot;

pub(crate) use body::{
    InFlow, MainFill, Shrink, counting_fills, fill_share, flow_fill_room, flow_region, flow_room, in_grid_content_column, main_fill, place_in_cell,
    place_in_flow, report_shrink, run_body, run_body_measured, shrink_cap, take_in_flow,
};
