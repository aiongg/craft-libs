//! The Navigator's zoom row: percent field, Zoom Out, slider, Zoom In.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::ControlSize;
use egui::{Rect, Ui};
use kurbo::Size;

use super::labels::NavigatorLabels;
use super::style::NavigatorStyle;
use crate::canvas::CanvasView;
use crate::localized::LocaleNumbers;

/// What the row did this pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RowChange {
    /// The row's rect, laid out before the preview takes the space above it.
    pub(super) rect: Rect,
    /// The view's zoom changed.
    pub(super) changed: bool,
    /// The change came from Zoom Out or Zoom In with Animated Zoom on.
    pub(super) animate: bool,
}

/// Lays out one row at the bottom of `ui`'s space, `style.row_gap` above it and between its
/// controls, all at `size`, and applies what the user did to `view.zoom` (about the view
/// centre):
///
/// - **Field** ([`NumberField`](crate::controls::NumberField), named [`NavigatorLabels::zoom`]):
///   the zoom in percent with the `%` unit, range
///   [`ZoomLadder::percent_range`](crate::canvas::ZoomLadder::percent_range) (the document's
///   range widened to include the zoom, so the field never pulls it back); Up and Down step the
///   ladder (`NumberField::steps_with` and [`ZoomLadder::step`](crate::canvas::ZoomLadder::step));
///   a typed value applies without animation. The field takes the library's default width (lead
///   call L6, not PhotoCraft's 64). A run's Esc restores the zoom at its start.
/// - **Zoom Out**, **Zoom In** ([`IconButton`](crate::controls::IconButton) with `Icon::Minus`
///   and `Icon::Plus`, names [`NavigatorLabels::zoom_out`] and [`NavigatorLabels::zoom_in`]):
///   one ladder rung; `animate` is `animated_zoom`.
/// - **Slider** ([`Slider`](crate::controls::Slider), named [`NavigatorLabels::zoom`], value text
///   [`NavigatorLabels::zoom_value`]): log2 of the zoom over `log2(min(ZoomLadder::min, zoom))`
///   ..= `log2(max(ZoomLadder::MAX, zoom))`, so it always holds the current zoom; it fills the
///   space between the buttons; no animation.
///
/// Order in reading direction: field, Zoom Out, slider, Zoom In. The slider's value grows to the
/// right in every direction (`Unmirrored::SliderValue`), so in right-to-left the row reads field,
/// Zoom In, slider, Zoom Out from the right edge. Zooms go through the ladder's device zoom
/// ([`device_zoom`](crate::canvas::zoom::device_zoom) with the context's pixels per point).
#[expect(clippy::too_many_arguments, reason = "skeleton: the fill may fold these into a context struct")]
pub(super) fn zoom_row(
    ui: &mut Ui,
    view: &mut CanvasView,
    size: Size,
    numbers: &LocaleNumbers,
    labels: &NavigatorLabels,
    style: &NavigatorStyle,
    control: ControlSize,
    animated_zoom: bool,
) -> RowChange {
    // pc: crates/ui-egui/src/panels/navigator.rs:106-149 (row, `set` closure, field, buttons, log2 slider);
    // pc: crates/ui-egui/src/panels/navigator.rs:153 `NAV_ROW_H` (22, replaced by the control size).
    todo!()
}
