//! The Navigator's zoom row: percent field, Zoom Out, slider, Zoom In.

use craft_theme::ControlSize;
use egui::{Id, Rect, Ui};
use kurbo::Size;

use super::labels::NavigatorLabels;
use super::style::NavigatorStyle;
use crate::canvas::{CanvasView, ZoomLadder, zoom};
use crate::controls::{IconButton, NumberField, Slider};
use crate::direction;
use crate::edit::{CancelReason, EditEvent};
use crate::icons::Icon;
use crate::localized::{self, LocaleNumbers, Unit};
use crate::theme::{Tokens, TokensOf as _};

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
///   call L6, not PhotoCraft's 64). A run's Esc restores the zoom at its start; any other end of a run leaves the zoom as it is.
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
#[expect(clippy::too_many_arguments, reason = "the row's inputs are the navigator's own and are passed once per pass")]
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
    let ppp = ui.ctx().pixels_per_point();
    let side = IconButton::style(&Tokens::of(ui), control).side;
    let mut change = RowChange { rect: Rect::NOTHING, changed: false, animate: false };
    let row = direction::horizontal(ui, |ui| {
        ui.spacing_mut().item_spacing.x = style.row_gap;
        let step = Step { size, ppp, control, animated_zoom };
        field(ui, view, &step, numbers, labels, &mut change);
        // The slider's value grows to the right, so right to left the row is field, Zoom In,
        // slider, Zoom Out from the right edge.
        let (first, last) = if direction::is_rtl(ui) {
            ((Icon::Plus, &labels.zoom_in, 1), (Icon::Minus, &labels.zoom_out, -1))
        } else {
            ((Icon::Minus, &labels.zoom_out, -1), (Icon::Plus, &labels.zoom_in, 1))
        };
        button(ui, view, &step, first, &mut change);
        let width = (ui.available_width() - side - style.row_gap).max(0.0);
        slider(ui, view, &step, numbers, labels, width, &mut change);
        button(ui, view, &step, last, &mut change);
    });
    change.rect = row.response.rect;
    change
}

/// What every control of the row needs besides the view.
struct Step {
    size: Size,
    ppp: f32,
    control: ControlSize,
    animated_zoom: bool,
}

/// Sets the view's zoom to `device` (clamped to the ladder's range); whether it changed.
fn set_device(view: &mut CanvasView, device: f64, step: &Step) -> bool {
    let zoom = CanvasView::points_per_unit(ZoomLadder::clamp(device, step.size), step.ppp);
    let changed = zoom != view.zoom;
    view.zoom = zoom;
    changed
}

/// Applies one control event: a run's start is remembered under `key`; Esc puts it back, and
/// any other end of the run drops it.
/// `device` maps the control's value to a device zoom.
fn apply(ui: &Ui, key: Id, event: &EditEvent<f64>, view: &mut CanvasView, step: &Step, device: impl Fn(f64) -> f64) -> bool {
    match event {
        EditEvent::Begin { value, .. } => {
            ui.data_mut(|d| d.insert_temp(key, view.zoom));
            value.is_some_and(|v| set_device(view, device(v), step))
        }
        EditEvent::Update { value, .. } | EditEvent::Set { value, .. } => set_device(view, device(*value), step),
        EditEvent::Commit { value, .. } => {
            ui.data_mut(|d| d.remove_temp::<f64>(key));
            set_device(view, device(*value), step)
        }
        // Only Esc puts the start back: any other cancel means the zoom was changed elsewhere.
        EditEvent::Cancel { reason, .. } => {
            let start = ui.data_mut(|d| {
                let start = d.get_temp::<f64>(key);
                d.remove_temp::<f64>(key);
                start
            });
            match start {
                Some(start) if *reason == CancelReason::Escape => {
                    let changed = start != view.zoom;
                    view.zoom = start;
                    changed
                }
                _ => false,
            }
        }
    }
}

/// The percent field: Up and Down step the ladder, a typed value applies at once.
fn field(ui: &mut Ui, view: &mut CanvasView, step: &Step, numbers: &LocaleNumbers, labels: &NavigatorLabels, change: &mut RowChange) {
    let device = zoom::device_zoom(view, step.ppp);
    let range = ZoomLadder::percent_range(device, step.size);
    let ladder = (range.start() / 100.0)..=(range.end() / 100.0);
    let edit = NumberField::new("navigator-zoom-field", labels.zoom.as_str(), device * 100.0, numbers)
        .range(range)
        .decimals(0..=2)
        .unit(Unit::new("{n}%"))
        .steps_with(move |percent, dir| ZoomLadder::step_in_range(percent / 100.0, dir, ladder.clone()) * 100.0)
        .size(step.control)
        .show(ui);
    if let Some(event) = &edit.event {
        change.changed |= apply(ui, edit.response.id.with("start"), event, view, step, |percent| percent / 100.0);
    }
}

/// Zoom Out (`dir` -1) or Zoom In (1): one ladder rung.
fn button(ui: &mut Ui, view: &mut CanvasView, step: &Step, (icon, label, dir): (Icon, &String, i32), change: &mut RowChange) {
    if ui.add(IconButton::new(icon, label.as_str()).size(step.control)).clicked() && zoom::step_view(view, dir, step.size, step.ppp) {
        change.changed = true;
        change.animate |= step.animated_zoom;
    }
}

/// The log2 slider over the whole zoom range, always holding the current zoom.
fn slider(ui: &mut Ui, view: &mut CanvasView, step: &Step, numbers: &LocaleNumbers, labels: &NavigatorLabels, width: f32, change: &mut RowChange) {
    let device = zoom::device_zoom(view, step.ppp);
    let low = ZoomLadder::min(step.size).min(device).log2();
    let high = ZoomLadder::MAX.max(device).log2();
    let percent = numbers.format(device * 100.0, 0..=2);
    let text = localized::fmt(&labels.zoom_value, &[("n", &percent)]);
    let edit = Slider::new("navigator-zoom-slider", labels.zoom.as_str(), device.log2(), low..=high)
        .size(step.control)
        .width(width)
        .value_text(text)
        .show(ui);
    if let Some(event) = &edit.event {
        change.changed |= apply(ui, edit.response.id.with("start"), event, view, step, f64::exp2);
    }
}
