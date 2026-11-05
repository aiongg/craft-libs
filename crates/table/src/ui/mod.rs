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
//! - [`style`] has [`SheetStyle`], every size the widgets read (`sheet.*` tokens).
//!
//! The cell-selection model ([`CellRef`], [`CellRange`], [`CellSelection`], [`CellSpace`]) is
//! craft-ui's `behavior::cells`, re-exported here.

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
pub mod style;

mod stories;

#[cfg(test)]
mod chart_tests;
#[cfg(test)]
mod conditions_tests;
#[cfg(test)]
mod fields_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod grid_tests;
#[cfg(test)]
mod outline_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod overlay_tests;
#[cfg(test)]
mod refpick_tests;
#[cfg(test)]
mod slicer_tests;
#[cfg(test)]
mod stats_tests;
#[cfg(test)]
mod tag_tests;

pub use chart::{ChartPrimitive, HAlign, VAlign, paint_chart};
pub use conditions::{
    Condition, ConditionBuilder, ConditionFields, ConditionGroup, ConditionItem, ConditionKind, ConditionLabels, ConditionValue, Match,
};
pub use error_tag::{ErrorTag, ErrorTagLabels, ErrorTagResponse};
pub use fields::{
    FieldArea, FieldGroup, FieldListEdit, FieldListLabels, FieldListPane, FieldListPaneOutput, FieldSort, PivotField, ReportLayout, ShowAs, Summary,
    ValueSetting, ValuesRow,
};
pub use filter::{FilterButton, FilterButtonEvent, FilterButtonLabels, FilterButtonOutput, filter_button_rect};
pub use grid::{
    CellAdvance, CellDirection, CellGrid, CellGridResponse, CellRange, CellRef, CellSelection, CellSpace, GridEdit, GridLayout, GridMenuTarget,
    GridModel, GridState, HEADER_MIN_ZOOM, LineIndex, SHEET_COLUMNS, SHEET_ROWS, SheetGeometry, absolute_range_reference, absolute_reference,
    column_letters,
};
pub use outline::{OutlineAction, OutlineAxis, OutlineBand, OutlineBandLabels, OutlineBandOutput, OutlineGroup};
pub use refpick::ReferencePicker;
pub use slicer::{Slicer, SlicerAction, SlicerLabels, SlicerLayout, SlicerOutput, slicer_layout};
pub use stats::{SelectionStats, SelectionStatsReadout, StatPart, StatsLabels, StatsShown, stats_parts};
pub use style::SheetStyle;

use craft_ui::localized::{Labels, Word};

/// The stories of the table widgets.
pub fn stories() -> Vec<craft_ui::story::Story> {
    stories::stories()
}

/// The labels tables of the table widgets, for the words registry.
pub const WORDS: &[&[Word]] = &[
    <ConditionLabels as Labels>::WORDS,
    <FieldListLabels as Labels>::WORDS,
    <StatsLabels as Labels>::WORDS,
    <OutlineBandLabels as Labels>::WORDS,
    <SlicerLabels as Labels>::WORDS,
    <ErrorTagLabels as Labels>::WORDS,
    <FilterButtonLabels as Labels>::WORDS,
    <stories::TableStoryLabels as Labels>::WORDS,
];
