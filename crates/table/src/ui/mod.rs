//! The table widgets, for egui. The look follows Excel (GridCraft), the condition builder
//! LightCraft's smart-collection rules.
//!
//! - [`grid`] has the cell-selection model, [`LineIndex`], [`GridGeometry`] and the [`CellGrid`].
//! - [`conditions`] has the [`ConditionBuilder`].
//! - [`outline`] has the [`OutlineBand`].
//! - [`fields`] has the [`FieldListPane`].
//! - [`chart`] has [`ChartPrimitive`] and [`paint_chart`].
//! - [`slicer`] has the [`Slicer`].
//! - [`refpick`] has the [`ReferencePicker`].
//! - [`stats`] has [`SelectionStats`] and the [`SelectionStatsReadout`].
//! - [`error_tag`] has the [`ErrorTag`].
//! - [`filter`] has the [`FilterButton`].

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

mod stories;

#[cfg(test)]
mod chart_tests;
#[cfg(test)]
mod conditions_tests;
#[cfg(test)]
mod fields_tests;
#[cfg(test)]
mod grid_tests;
#[cfg(test)]
mod outline_tests;
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
pub use error_tag::ErrorTag;
pub use fields::{FieldArea, FieldListEdit, FieldListLabels, FieldListPane, PivotField};
pub use filter::{FilterButton, filter_button_rect};
pub use grid::{
    CellGrid, CellGridResponse, CellRange, CellRef, CellSelection, GridEdit, GridGeometry, GridLayout, GridModel, GridState, LineIndex,
    column_letters,
};
pub use outline::{OutlineAction, OutlineAxis, OutlineBand, OutlineGroup};
pub use refpick::ReferencePicker;
pub use slicer::{Slicer, SlicerAction, SlicerLayout, slicer_layout};
pub use stats::{SelectionStats, SelectionStatsReadout, StatsLabels, StatsShown, stats_parts};

use craft_ui::localized::{Labels, Word};

/// The stories of the table widgets (not registered yet; see the spec).
pub fn stories() -> Vec<craft_ui::story::Story> {
    stories::stories()
}

/// The labels tables of the table widgets, for the words registry.
pub const WORDS: &[&[Word]] = &[<ConditionLabels as Labels>::WORDS, <FieldListLabels as Labels>::WORDS, <StatsLabels as Labels>::WORDS];
