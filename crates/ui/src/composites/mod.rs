//! The design system's composites: controls put together with their labels and text into the
//! rows that dialogs, preferences and panels are built from.
//!
//! - [`Field`]: a control with its label (beside it in a label column, or above it), and help or
//!   error text under it; the control is labelled and described by them in AccessKit.
//! - [`FieldGroup`]: a form's fields with one label column, as wide as the widest label.
//!
//! Composites take semantic properties only and draw with the component tokens of what they hold
//! (`field.*` for [`Field`]); each has a pure `style` function of
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

pub(crate) mod field;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod field_tests;

pub use field::{Field, FieldControl, FieldGroup, FieldStyle};
