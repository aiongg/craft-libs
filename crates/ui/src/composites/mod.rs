//! The design system's composites: controls put together with their labels and text into the
//! rows that dialogs, preferences and panels are built from.
//!
//! - [`Field`]: a control with its label (beside it in a label column, or above it), and help or
//!   error text under it; the control is labelled and described by them in AccessKit.
//! - [`FieldGroup`]: a form's fields with one label column, as wide as the widest label.
//! - [`PropertyRow`]: an inspector row of a short label and a control that fills the width;
//!   [`PropertyRow::pair`] puts two in a row, and [`PropertyGroup`] gives a section's glyph labels
//!   one centre.
//!
//! - [`SectionHeader`]: the header row of a panel section, collapsible (chevron, persisted open
//!   state, Enter and Space) or always open, with an optional controls slot.
//! - [`PanelHeader`]: the title row of a flyout or floating panel, with actions, a panel menu
//!   button and optional float and close buttons.
//! - [`ColorArea`] and [`ColorStrip`]: a colour picker's 2D field and component strip, in one of
//!   the [`AreaMode`]s.
//! - [`ColorPicker`]: the colour picker's body, the area and strip with channel rows, the hex
//!   field and the host's hooks, reporting [`PickedColor`]s.
//!
//! Composites take semantic properties only and draw with the component tokens of what they hold
//! (`field.*` for [`Field`], `property.*` for [`PropertyRow`], `section.*` for [`SectionHeader`],
//! `panel_header.*` for [`PanelHeader`], `color_picker.*` for [`ColorArea`] and [`ColorStrip`]); each has a pure `style` function of
//! [`UiTokens`](crate::theme::UiTokens), dumped under `crates/ui/tests/dumps/components/`.
//!
//! ```
//! use craft_ui::behavior::FieldTab;
//! use craft_ui::composites::{Field, FieldGroup};
//! use craft_ui::controls::TextField;
//! # egui::__run_test_ui(|ui| { let name = String::from("Layer 1");
//! FieldTab::new("new-layer").show(ui, |ui| {
//!     FieldGroup::new("new-layer-fields").show(ui, |ui| {
//!         Field::new("Name:")
//!             .help("Shown in the Layers panel.")
//!             .show(ui, |ui, invalid| TextField::new("name", "Name", &name).invalid(invalid).show(ui));
//!     });
//! });
//! # });
//! ```

pub(crate) mod color;
pub(crate) mod field;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod field_tests;
mod measure;
pub(crate) mod panel_header;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_header_tests;
pub(crate) mod property_row;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod property_row_tests;
pub(crate) mod section_header;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod section_header_tests;

pub use color::{
    AreaMode, ColorArea, ColorPartsStyle, ColorPicker, ColorPickerLayout, ColorPickerResponse, ColorPopover, ColorPopoverResponse, ColorStrip,
    PickedColor, SwatchPalette,
};
pub use field::{Field, FieldControl, FieldGroup, FieldStyle};
pub use panel_header::{PanelHeader, PanelHeaderResponse, PanelHeaderStyle};
pub use property_row::{LabelKind, PropertyGroup, PropertyLabel, PropertyPair, PropertyRow, PropertyStyle, RowLabel};
pub use section_header::{SectionHeader, SectionStyle};
