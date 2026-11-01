//! The Brush Settings panel.
//!
//! - [`BrushSettingsPanel`]: the frame: the section list, the selected section's pane (the app's
//!   content), the stroke strip and the New, Reset and Update buttons.
//! - [`BrushSectionList`] with [`BrushSection`] rows: Photoshop's list of sections with enable
//!   boxes, locks and switch rows, reporting [`BrushSectionEvent`]s.
//! - [`PreviewStroke`] and [`BrushStrokeStrip`]: the S-curve test stroke as data, and the strip that
//!   caches the app's rendering of it.
//! - The rows the section bodies share: [`BrushSizeField`], [`TipEllipse`] and
//!   [`DynamicControlRow`].
//!
//! The brush settings and the section bodies stay in the app (its engine's types); the library
//! draws the frame and the shared rows, and every control reports edit events the app sends to its
//! tool options host.
//!
//! ```no_run
//! use craft_draw::ui::brush_settings::{BrushSection, BrushSectionEvent, BrushSettingsPanel};
//! # egui::__run_test_ui(|ui| {
//! let sections = [BrushSection::pane(0, "Brush Tip Shape"), BrushSection::pane(1, "Shape Dynamics").enabled(true).locked(false)];
//! let out = BrushSettingsPanel::new("brush-settings").sections(&sections).selected(Some(&0)).show(ui, |ui, section| {
//!     ui.label(format!("section {section}"));
//! });
//! for event in out.sections {
//!     if let BrushSectionEvent::Select(key) = event { /* show that pane next */ }
//! }
//! # });
//! ```

mod ellipse;
#[cfg(test)]
mod ellipse_tests;
mod labels;
mod panel;
#[cfg(test)]
mod panel_tests;
mod preview;
#[cfg(test)]
mod preview_tests;
mod rows;
#[cfg(test)]
mod rows_tests;
mod sections;
#[cfg(test)]
mod sections_tests;
mod size_field;
#[cfg(test)]
mod size_field_tests;
pub(crate) mod stories;
mod style;

pub use ellipse::{TipAngle, TipEllipse, TipEllipseGrab, ellipse_drag, ellipse_grab, ellipse_handles};
pub use labels::BrushSettingsLabels;
pub use panel::{BrushPanelAction, BrushSettingsOutput, BrushSettingsPanel};
pub use preview::{BrushStrokeStrip, PreviewSample, PreviewStroke};
pub use rows::{DynamicControl, DynamicControlRow, DynamicControlRowResponse};
pub use sections::{BrushSection, BrushSectionEvent, BrushSectionKind, BrushSectionList, BrushSectionListResponse};
pub use size_field::BrushSizeField;
pub use style::{BrushSettingsStyle, TipEllipseStyle};
