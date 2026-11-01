//! Document tables (N3.07, N3.12, N3.24, N3.25): the table parts of Word, InDesign and
//! PowerPoint that sit in a page's text: the border edge picker, the table properties panel, the
//! Insert Table size grid and the selected cells' overlay. The spreadsheet grid is craft-table's.

pub mod edges;
#[cfg(test)]
mod edges_tests;
pub mod properties;
#[cfg(test)]
mod properties_tests;
pub mod selection;
#[cfg(test)]
mod selection_tests;
pub mod size_picker;
#[cfg(test)]
mod size_picker_tests;

pub use edges::{BorderEdges, BorderPreset, EdgeKey, EdgePicker, EdgePickerLabels, EdgePickerResponse};
pub use properties::{CellAlign, RowHeight, TableChange, TableProperties, TablePropertiesLabels, TablePropertiesOutput, TableValues, TextDirection};
pub use selection::{CellRange, CellSelection, CellSelectionOverlay, TableGeometry};
pub use size_picker::{TableSizePicker, TableSizePickerLabels, TableSizeResponse};
