//! The layout services: `LabelColumn`, `ColumnGroup`, the `Grid` cursor, `measure_slot` and `arrange`.

mod arrange;
mod body;
mod column_group;
mod label;
mod slot;

pub use arrange::{Placed, Show, arrange};
pub use body::{BodyLayout, RowAlign};
pub use column_group::ColumnGroup;
pub use label::{place_label, record_control_baseline};
pub use slot::measure_slot;

#[cfg_attr(not(feature = "testing"), expect(unused_imports, reason = "used by the body step"))]
pub(crate) use body::run_body;
