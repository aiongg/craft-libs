//! [`ColorArea`]: two components of a colour across and down a field, the others fixed.

use craft_theme::ModelValues;
use egui::accesskit::{Action, Role};
use egui::{AsIdSalt, CornerRadius, IdSalt, Key, Mesh, Modifiers, PointerButton, Pos2, Rect, Response, Sense, StrokeKind, Ui, Vec2, pos2};

use super::{AreaMode, ColorPartsStyle, faded_stroke, fraction, parts_style, scale_of, spoken, value_at};
use crate::controls::slider::{KEY_FILTER, Scale, tap};
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{FocusRing, Interactive};
use crate::theme::{Tokens, TokensOf, UiTokens};

/// Vertices along each side of the area's colour mesh (PhotoCraft's 32).
const MESH: usize = 32;

/// The 2D field of a colour picker: two components of the colour across and down, the others
/// fixed, as an [`AreaMode`] lays them out (PhotoCraft's Color Picker field):
///
/// ```
/// use craft_theme::{HsbModel, Srgba, ColorModel};
/// use craft_ui::composites::{AreaMode, ColorArea};
/// # egui::__run_test_ui(|ui| { let mut color = Srgba::rgb(0.2, 0.5, 0.8);
/// let values = HsbModel.from_srgb(color);
/// let edit = ColorArea::new("field", "Saturation and brightness", AreaMode::Hue, values).show(ui);
/// if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
///     color = HsbModel.to_srgb(&v.values);
/// }
/// # });
/// ```
///
/// The host gives the colour as the mode's model values and gets them back through the edit
/// lifecycle ([`crate::edit`]), with only the area's two components changed: a host that keeps
/// the values it was sent ([`kept_or_converted`](craft_theme::kept_or_converted)) keeps the hue
/// of a grey and the hue and saturation of black.
///
/// - Pointer (primary button): the press sets the value under the pointer; it is the gesture's
///   first value, drawn in the press frame and reported in the next (Begin goes out first).
///   Dragging moves it, also outside the area, where it stays at the edge; the release commits.
///   A press and release in one frame is a [`Set`](crate::edit::EditEvent::Set). A press focuses
///   the area. Values snap to the channels' steps.
/// - Keys, while focused (each a [`Set`](crate::edit::EditEvent::Set)): Left and Right step the
///   across component, Up and Down the down component (Up raises it); Shift steps ten. Page Up
///   and Page Down move the down component a tenth of its range; Home and End set the across
///   component to its ends. A step is the channel's (1°, 1 %, 1 of 255, 0.001 of chroma). During
///   a drag keys change nothing. The wheel scrolls the panel.
/// - Esc during a drag cancels it; the area shows the start value until the host echoes it.
/// - AccessKit: a `Slider` named `label`, with both components as its value text
///   ("Saturation 40 %, Brightness 70 %", from [`channel_names`](ColorArea::channel_names)) and
///   no numeric value; it takes focus and click (which focuses it). The exact values are in the
///   picker's channel fields.
/// - Right to left the area keeps its axes: across still grows to the right.
///
/// The area is not a field: a dialog's [`FieldTab`](crate::behavior::FieldTab) skips it, and
/// Ctrl+Tab reaches it. The edge (one point of `line.control_edge`) and the focus ring lie
/// outside its rect; the marker is two rings, `overlay.stroke` inside `overlay.halo`, so one
/// shows on any colour.
#[derive(Clone, Debug)]
#[must_use = "show the area with `show`"]
pub struct ColorArea {
    id_salt: IdSalt,
    label: String,
    mode: AreaMode,
    values: ModelValues,
    names: Option<[String; 2]>,
    size: Option<Vec2>,
    policy: EditPolicy,
    enabled: bool,
}

impl ColorArea {
    /// An area named `label` (its accessible name, translated by the host: "Saturation and
    /// brightness") showing `values`, values of `mode`'s model; `id_salt` keeps its gesture and
    /// focus when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, mode: AreaMode, values: ModelValues) -> Self {
        debug_assert_eq!(values.model, mode.model().id(), "the area's values must be of its mode's model");
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            mode,
            values,
            names: None,
            size: None,
            policy: EditPolicy::default(),
            enabled: true,
        }
    }

    /// The across and down components' names in the value text, translated by the host; the
    /// model's English channel names without them.
    pub fn channel_names(mut self, across: impl Into<String>, down: impl Into<String>) -> Self {
        self.names = Some([across.into(), down.into()]);
        self
    }

    /// The area's size; `color_picker.area` square without it (the Full picker's).
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    /// How the gesture reports ([`EditPolicy`]); the default reports every change at once.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn style(tokens: &UiTokens) -> ColorPartsStyle {
        parts_style(tokens)
    }

    /// Shows the area and returns this frame's edit event. The response covers the area.
    pub fn show(self, ui: &mut Ui) -> EditResponse<ModelValues> {
        let style = ColorArea::style(&Tokens::of(ui));
        let size = self.size.unwrap_or(Vec2::splat(style.area));
        let model = self.mode.model();
        let [across, down] = self.mode.area();
        let axes = Axes { across, down, x: scale_of(model, across), y: scale_of(model, down) };
        let sensed = Interactive::new(self.id_salt, Role::Slider, self.label.as_str())
            .sense(Sense::drag())
            .enabled(self.enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact(ui, size);
        let response = sensed.response().clone();
        let (rect, enabled, dragging) = (response.rect, response.enabled(), response.dragged());

        let mut gesture = Gesture::load(ui, response.id, &self.values, self.policy).event_filter(KEY_FILTER).enabled(enabled);
        if response.drag_started_by(PointerButton::Primary) {
            response.request_focus();
            gesture.begin();
        }
        if response.dragged_by(PointerButton::Primary)
            && let Some(start) = gesture.start().copied()
            && let Some(pos) = response.interact_pointer_pos()
        {
            gesture.update(axes.at(rect, &start, pos), ui.input(|i| i.modifiers));
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        if let Some(pos) = tap(ui, &response) {
            response.request_focus();
            let shown = *gesture.shown();
            gesture.set(axes.at(rect, &shown, pos));
        }
        if let Some(values) = keys(ui, &response, dragging, &axes, gesture.shown()) {
            gesture.set(values);
        }
        if enabled && ui.input(|i| i.num_accesskit_action_requests(response.id, Action::Click)) > 0 {
            response.request_focus();
        }

        let shown = *gesture.shown();
        let names = self.names.clone().unwrap_or_else(|| [model.channels()[across].name, model.channels()[down].name].map(str::to_owned));
        let text = format!("{}, {}", spoken(&names[0], model, across, shown.values[across]), spoken(&names[1], model, down, shown.values[down]));
        let response = sensed.value(text).finish(ui);
        if enabled {
            ui.ctx().accesskit_node_builder(response.id, |node| node.add_action(Action::Click));
        }
        if ui.is_rect_visible(rect.expand(style.marker_outer_radius)) {
            paint(ui, &style, &self, &axes, rect, &shown, enabled);
        }
        FocusRing::paint_radius(ui, rect.expand(style.edge.width), CornerRadius::ZERO, &response);
        EditResponse::new(response, gesture.finish())
    }
}

/// The area's two channels and their scales.
struct Axes {
    across: usize,
    down: usize,
    x: Scale,
    y: Scale,
}

impl Axes {
    /// `base` with the components at `pos` in `rect`, kept to the area's edges.
    fn at(&self, rect: Rect, base: &ModelValues, pos: Pos2) -> ModelValues {
        let along = |p: f32, from: f32, length: f32| if length > 0.0 { (p - from) / length } else { 0.0 };
        let mut values = *base;
        values.values[self.across] = value_at(&self.x, along(pos.x, rect.left(), rect.width()));
        values.values[self.down] = value_at(&self.y, 1.0 - along(pos.y, rect.top(), rect.height()));
        values
    }

    /// Where `values` sit in `rect`.
    fn pos(&self, rect: Rect, values: &ModelValues) -> Pos2 {
        let (fx, fy) = (fraction(&self.x, values.values[self.across]), fraction(&self.y, values.values[self.down]));
        pos2(egui::lerp(rect.x_range(), fx), egui::lerp(rect.y_range(), 1.0 - fy))
    }
}

/// The values this frame's keys give the focused area at `values`; `None` when no key asked for
/// a change, and during a drag, which keys don't change (they are still taken).
fn keys(ui: &Ui, response: &Response, dragging: bool, axes: &Axes, values: &ModelValues) -> Option<ModelValues> {
    if !response.enabled() || !response.has_focus() {
        return None;
    }
    let (mut dx, mut dy, mut to_x) = (0.0, 0.0, None);
    let (step_x, step_y, page_y) = (axes.x.key_step(), axes.y.key_step(), axes.y.page_step());
    ui.input_mut(|i| {
        let mut count = |modifiers, key| i.count_and_consume_key(modifiers, key) as f64;
        // Shift first: `Modifiers::NONE` also matches a press with Shift held.
        let mut steps = |plus, minus| {
            10.0 * (count(Modifiers::SHIFT, plus) - count(Modifiers::SHIFT, minus)) + count(Modifiers::NONE, plus) - count(Modifiers::NONE, minus)
        };
        dx += step_x * steps(Key::ArrowRight, Key::ArrowLeft);
        dy += step_y * steps(Key::ArrowUp, Key::ArrowDown);
        dy += page_y * (count(Modifiers::NONE, Key::PageUp) - count(Modifiers::NONE, Key::PageDown));
        if count(Modifiers::NONE, Key::Home) > 0.0 {
            to_x = Some(axes.x.min);
        }
        if count(Modifiers::NONE, Key::End) > 0.0 {
            to_x = Some(axes.x.max);
        }
    });
    if dragging || (dx == 0.0 && dy == 0.0 && to_x.is_none()) {
        return None;
    }
    let moved = |scale: &Scale, value: f32, delta: f64| {
        let value = scale.place(f64::from(value));
        if delta == 0.0 { value } else { scale.snap(scale.moved(value, delta)) }
    };
    let mut out = *values;
    out.values[axes.across] = to_x.unwrap_or_else(|| moved(&axes.x, values.values[axes.across], dx)) as f32;
    out.values[axes.down] = moved(&axes.y, values.values[axes.down], dy) as f32;
    Some(out)
}

/// Paints the colours, the edge and the marker.
fn paint(ui: &Ui, style: &ColorPartsStyle, area: &ColorArea, axes: &Axes, rect: Rect, shown: &ModelValues, enabled: bool) {
    let model = area.mode.model();
    let painter = ui.painter();
    let mut mesh = Mesh::default();
    let last = (MESH - 1) as f32;
    for row in 0..MESH {
        for col in 0..MESH {
            let (fx, fy) = (col as f32 / last, row as f32 / last);
            let mut v = *shown;
            v.values[axes.across] = egui::lerp(axes.x.min as f32..=axes.x.max as f32, fx);
            v.values[axes.down] = egui::lerp(axes.y.max as f32..=axes.y.min as f32, fy);
            mesh.colored_vertex(pos2(egui::lerp(rect.x_range(), fx), egui::lerp(rect.y_range(), fy)), model.to_srgb(&v.values).into());
        }
    }
    for row in 0..MESH - 1 {
        for col in 0..MESH - 1 {
            let a = (row * MESH + col) as u32;
            let b = a + MESH as u32;
            mesh.add_triangle(a, a + 1, b + 1);
            mesh.add_triangle(a, b + 1, b);
        }
    }
    painter.add(mesh);
    painter.rect_stroke(rect, CornerRadius::ZERO, faded_stroke(style.edge, enabled), StrokeKind::Outside);
    let at = axes.pos(rect, shown);
    painter.circle_stroke(at, style.marker_outer_radius, faded_stroke(style.marker_outer, enabled));
    painter.circle_stroke(at, style.marker_inner_radius, faded_stroke(style.marker_inner, enabled));
}
