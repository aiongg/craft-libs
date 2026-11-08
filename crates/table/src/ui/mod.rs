//! The table widgets, for egui. The look follows Excel (GridCraft), the condition builder
//! LightCraft's smart-collection rules.
//!
//! - [`grid`] has the cell-selection model, [`LineIndex`], [`SheetGeometry`] and the [`CellGrid`].
//! - [`conditions`] has the [`ConditionBuilder`].
//! - [`outline`] has the [`OutlineBand`].
//! - [`fields`] has the [`FieldListPane`].
//! - [`chart`] has [`ChartPrimitive`] and [`paint_chart`].
//! - [`slicer`] has the [`Slicer`].
//! - [`refpick`] has the [`ReferencePicker`].
//! - [`stats`] has [`SelectionStats`] and the [`SelectionStatsReadout`].
//! - [`error_tag`] has the [`ErrorTag`].
//! - [`filter`] has the [`FilterButton`].
//!
//! The cell-selection model ([`CellAt`], [`CellRange`], [`CellSelection`], [`CellSpace`]) is craft-behavior's,
//! re-exported here.

pub mod chart;
pub mod conditions;
pub mod error_tag;
pub mod fields;
pub mod filter;
pub mod grid;
pub mod outline;
pub mod refpick;
pub mod slicer;
pub mod stats;

#[cfg(feature = "screen-testing")]
pub mod testing {
    craft_ui::testing_registered!();
}

pub use chart::{ChartPrimitive, HAlign, VAlign, paint_chart};
pub use conditions::{
    Condition, ConditionBuilder, ConditionFields, ConditionGroup, ConditionItem, ConditionKind, ConditionValue, ConditionWords, Match,
};
pub use error_tag::{ErrorTag, ErrorTagOutput, ErrorTagWords};
pub use fields::{
    FieldArea, FieldGroup, FieldListEdit, FieldListPane, FieldListPaneOutput, FieldListWords, FieldSort, PivotField, ReportLayout, ShowAs, Summary,
    ValueSetting, ValuesRow,
};
pub use filter::{FilterButton, FilterButtonEvent, FilterButtonOutput, FilterButtonWords};
pub use grid::{
    CellAdvance, CellAt, CellDirection, CellGrid, CellProblem, CellRange, CellSelection, CellSpace, ColumnFilter, GridEdit, GridLayout,
    GridMenuTarget, GridModel, GridState, HEADER_MIN_ZOOM, LineIndex, SHEET_COLUMNS, SHEET_ROWS, SheetGeometry, SheetMetrics,
    absolute_range_reference, absolute_reference, column_letters,
};
pub use outline::{OutlineAction, OutlineAxis, OutlineBand, OutlineBandOutput, OutlineBandWords, OutlineGroup};
pub use refpick::ReferencePicker;
pub use slicer::{Slicer, SlicerAction, SlicerLayout, SlicerOutput, SlicerWords, slicer_layout};
pub use stats::{SelectionStats, SelectionStatsOutput, SelectionStatsReadout, StatPart, StatsShown, StatsWords, stats_parts};
