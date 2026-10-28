//! The design system's composites: controls put together with their labels and text into the
//! rows that dialogs, preferences and panels are built from.
//!
//! - [`Field`]: a control with its label (beside it in a label column, or above it), and help or
//!   error text under it; the control is labelled and described by them in AccessKit.
//! - [`FieldGroup`]: a form's fields with one label column, as wide as the widest label.
//! - [`PropertyRow`]: an inspector row of a short label and a control that fills the width;
//!   [`PropertyRow::pair`] puts two in a row, and [`PropertyGroup`] gives a section's glyph labels
//!   one centre.
//! - [`CurveEditor`]: a tone curve's graph and points, edited by pointer and keys, with the
//!   Input/Output row of the selected point; the value is a [`Curve`].
//! - [`GradientStops`], [`GradientRamp`] and [`GradientSwatch`]: the value of a gradient stop bar
//!   with its stop rules ([`gradient::stops`]), the paint of a gradient over the checkerboard, and
//!   a well that shows one.
//! - [`SectionHeader`]: the header row of a panel section, collapsible (chevron, persisted open
//!   state, Enter and Space) or always open, with an optional controls slot.
//! - [`Histogram`]: a plot of bins, one tone channel or RGB ribbons; not interactive.
//! - [`PanelHeader`]: the title row of a flyout or floating panel, with actions, a panel menu
//!   button and optional float and close buttons.
//! - [`ColorArea`] and [`ColorStrip`]: a colour picker's 2D field and component strip, in one of
//!   the [`AreaMode`]s.
//! - [`ColorPicker`]: the colour picker's body, the area and strip with channel rows, the hex
//!   field and the host's hooks, reporting [`PickedColor`]s.
//! - [`ToastStack`] and [`Toast`]: cards in the bottom corner of a host rect, with timers that
//!   pause on hover and keyboard focus.
//! - [`Notice`]: an inline status message in the tone's icon and colour.
//! - [`EmptyState`]: the message of an empty panel or list, or a centred no-document column.
//! - [`live`]: live regions, which announce a message once when it appears. [`Tone`] is the
//!   neutral, info, good, warn and bad tone of toasts and notices.
//!
//! Composites take semantic properties only and draw with the component tokens of what they hold
//! (`color_picker.*` for [`ColorArea`] and [`ColorStrip`], `curve.*` for [`CurveEditor`], `field.*` for [`Field`], `property.*` for [`PropertyRow`], `section.*` for [`SectionHeader`],
//! `panel_header.*` for [`PanelHeader`], `histogram.*` for [`Histogram`], `toast.*`, `notice.*` and `empty_state.*`); each has a pure `style` function of
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
pub(crate) mod curve;
pub(crate) mod curve_editor;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod curve_editor_tests;
pub(crate) mod empty_state;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod empty_state_tests;
pub(crate) mod field;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod field_tests;
pub mod gradient;
pub(crate) mod histogram;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod histogram_tests;
pub mod live;
mod measure;
pub(crate) mod notice;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod notice_tests;
pub(crate) mod panel_header;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_header_tests;
pub(crate) mod property_row;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod property_row_tests;
pub(crate) mod section_header;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod section_header_tests;
pub(crate) mod toast;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod toast_tests;

pub use color::{
    AreaMode, ColorArea, ColorPartsStyle, ColorPicker, ColorPickerLayout, ColorPickerResponse, ColorPopover, ColorPopoverResponse, ColorStrip,
    ColorWarning, PairWell, PickedColor, SwatchPair, SwatchPairResponse, SwatchPairStyle, SwatchPalette, Well,
};
pub use craft_theme::Tone;
pub use curve::Curve;
pub use curve_editor::{CurveChannel, CurveEditor, CurveLabels, CurveStyle, Sampler};
pub use empty_state::{EmptyState, EmptyStateResponse, EmptyStateStyle};
pub use field::{Field, FieldControl, FieldGroup, FieldStyle};
pub use gradient::{
    ColorStop, GradientRamp, GradientRampStyle, GradientStops, GradientSwatch, OpacityStop, Sampler as StopSampler, StopRow, StopRows, StopRules,
    default_sampler,
};
pub use histogram::{BinCount, ChannelColor, Histogram, HistogramData, HistogramLook, HistogramScale, HistogramStyle};
pub use notice::{Notice, NoticeStyle};
pub use panel_header::{PanelHeader, PanelHeaderResponse, PanelHeaderStyle};
pub use property_row::{LabelKind, PropertyGroup, PropertyLabel, PropertyPair, PropertyRow, PropertyStyle, RowLabel};
pub use section_header::{SectionHeader, SectionStyle};
pub use toast::{Toast, ToastEnd, ToastStack, ToastStyle};
