//! [`ColorArea`]: two components of a colour across and down a field, the others fixed.

use craft_theme::{ColorModel, ModelValues};
use egui::accesskit::{Action, Role};
use egui::{AsIdSalt, CornerRadius, IdSalt, Key, Mesh, Modifiers, PointerButton, Pos2, Rect, Response, Sense, StrokeKind, Ui, Vec2, pos2};

use super::{AreaMode, ColorPartsStyle, QUANTIZE_CELLS, faded_stroke, fraction, parts_style, quantized, scale_of, snapped, spoken, value_at};
use crate::controls::slider::{self, KEY_FILTER, PressTurn, Scale, press_turn, tap};
use crate::edit::ops::{Bound, ScalarOp};
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
/// outside its rect; the marker is two rings, `color_picker.marker` (white) inside
/// `color_picker.marker_halo` (dark), so one shows on any colour and neither tints it.
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
    part: bool,
    quantize: Option<usize>,
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
            part: false,
            quantize: None,
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

    /// Puts the colour the area reports on `levels` levels (at least two) per red, green and
    /// blue channel, whatever the mode: a web-safe field has 6. The pointer takes the nearest
    /// such colour and a key steps to the next different one, in the mode's model values again.
    /// The area shows a grid of flat cells of the snapped colours; the marker stays at the
    /// values it was given.
    pub fn quantize(mut self, levels: usize) -> Self {
        self.quantize = Some(levels.max(2));
        self
    }

    /// Whether the area is a part of a colour picker, whose press takes its turn from the edit
    /// order ([`press_turn`]): it waits for a field's typing run to end.
    pub(crate) fn part(mut self, part: bool) -> Self {
        self.part = part;
        self
    }

    pub fn style(tokens: &UiTokens) -> ColorPartsStyle {
        parts_style(tokens)
    }

    /// Shows the area and returns this frame's edit event. The response covers the area.
    pub fn show(self, ui: &mut Ui) -> EditResponse<ModelValues> {
        self.show_with_ops(ui).0
    }

    /// [`show`](ColorArea::show), with the operations this frame's one-shot inputs made.
    pub(crate) fn show_with_ops(self, ui: &mut Ui) -> (EditResponse<ModelValues>, Vec<AreaOp>) {
        let style = ColorArea::style(&Tokens::of(ui));
        let size = self.size.unwrap_or(Vec2::splat(style.area));
        let model = self.mode.model();
        let [across, down] = self.mode.area();
        let axes = Axes::of(self.mode, self.quantize);
        let sensed = Interactive::salted(self.id_salt, Role::Slider, self.label.as_str())
            .sense(Sense::drag())
            .enabled(self.enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact(ui, size);
        let response = sensed.response().clone();
        let (rect, enabled, dragging) = (response.rect, response.enabled(), response.dragged());

        let mut gesture = Gesture::load(ui, response.id, &self.values, self.policy).event_filter(KEY_FILTER).enabled(enabled);
        let tapped = tap(ui, &response);
        let pressed = response.drag_started_by(PointerButton::Primary).then(|| response.interact_pointer_pos()).flatten();
        let turn = press_turn(ui, response.id, self.part, gesture.id(), pressed.or(tapped));
        if let PressTurn::Held(held) = turn {
            gesture.begin();
            if let Some(start) = gesture.start().copied() {
                gesture.update(axes.at(rect, &start, held), ui.input(|i| i.modifiers));
            }
            if !response.dragged_by(PointerButton::Primary) {
                gesture.commit();
            }
        }
        let waits = matches!(turn, PressTurn::Waits);
        if response.drag_started_by(PointerButton::Primary) {
            response.request_focus();
            if !waits {
                gesture.begin();
            }
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
        if tapped.is_some() {
            response.request_focus();
        }
        // The one-shot inputs, each an operation applied to the values the ones before it left.
        let mut ops = Vec::new();
        let mut set = |gesture: &mut Gesture<ModelValues>, op: AreaOp| {
            gesture.set(axes.apply(&op, gesture.shown()));
            ops.push(op);
        };
        if let Some(pos) = tapped.filter(|_| !waits) {
            set(&mut gesture, axes.op_at(rect, pos));
        }
        if let Some(op) = keys(ui, &response, dragging, &axes) {
            set(&mut gesture, op);
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
        (EditResponse::new(response, gesture.finish()), ops)
    }
}

/// What a one-shot input did to the area's values: an operation on its across channel and one on
/// its down channel.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AreaOp {
    pub(crate) x: ScalarOp,
    pub(crate) y: ScalarOp,
}

/// `values` with `op` applied to the two channels `mode`'s area shows, as the area applies it.
pub(super) fn apply(op: &AreaOp, values: &ModelValues, mode: AreaMode, quantize: Option<usize>) -> ModelValues {
    Axes::of(mode, quantize).apply(op, values)
}

/// The area's two channels and their scales.
struct Axes {
    across: usize,
    down: usize,
    x: Scale,
    y: Scale,
    /// The model, and the levels per red, green and blue channel the reported colour is on.
    quantize: Option<(&'static dyn ColorModel, usize)>,
}

impl Axes {
    fn of(mode: AreaMode, quantize: Option<usize>) -> Axes {
        let model = mode.model();
        let [across, down] = mode.area();
        Axes { across, down, x: scale_of(model, across), y: scale_of(model, down), quantize: quantize.map(|levels| (model, levels)) }
    }

    /// `values` on the quantised colour, if the area quantises.
    fn settled(&self, values: ModelValues) -> ModelValues {
        self.quantize.map_or(values, |(model, levels)| quantized(model, &values, levels))
    }

    /// `values` with `op` applied. A quantising area steps a relative `op` again until the
    /// colour is a different one, or the components can go no further.
    fn apply(&self, op: &AreaOp, values: &ModelValues) -> ModelValues {
        let mut out = self.apply_once(op, values);
        if self.quantize.is_some() && matches!((&op.x, &op.y), (ScalarOp::By(_), ScalarOp::By(_))) {
            let from = self.settled(*values);
            loop {
                let next = self.apply_once(op, &out);
                if self.settled(out) != from || next == out {
                    break;
                }
                out = next;
            }
        }
        self.settled(out)
    }

    /// `values` with `op` applied, each channel on its scale. A channel `op` doesn't step stays
    /// where it is, unsnapped.
    fn apply_once(&self, op: &AreaOp, values: &ModelValues) -> ModelValues {
        let channel = |op: &ScalarOp, scale: &Scale, value: f32| {
            let value = scale.place(f64::from(value));
            match op {
                ScalarOp::By(delta) if *delta == 0.0 => value,
                op => slider::apply(op, value, (scale.min, scale.max), scale),
            }
        };
        let mut out = *values;
        out.values[self.across] = channel(&op.x, &self.x, values.values[self.across]) as f32;
        out.values[self.down] = channel(&op.y, &self.y, values.values[self.down]) as f32;
        out
    }

    /// The across and down components at `pos` in `rect`, kept to the area's edges.
    fn components_at(&self, rect: Rect, pos: Pos2) -> [f32; 2] {
        let along = |p: f32, from: f32, length: f32| if length > 0.0 { (p - from) / length } else { 0.0 };
        [value_at(&self.x, along(pos.x, rect.left(), rect.width())), value_at(&self.y, 1.0 - along(pos.y, rect.top(), rect.height()))]
    }

    /// A tap at `pos` in `rect`, as an operation.
    fn op_at(&self, rect: Rect, pos: Pos2) -> AreaOp {
        let [x, y] = self.components_at(rect, pos).map(|v| ScalarOp::Set(f64::from(v)));
        AreaOp { x, y }
    }

    /// `base` with the components at `pos` in `rect`.
    fn at(&self, rect: Rect, base: &ModelValues, pos: Pos2) -> ModelValues {
        let mut values = *base;
        [values.values[self.across], values.values[self.down]] = self.components_at(rect, pos);
        self.settled(values)
    }

    /// Where `values` sit in `rect`.
    fn pos(&self, rect: Rect, values: &ModelValues) -> Pos2 {
        let (fx, fy) = (fraction(&self.x, values.values[self.across]), fraction(&self.y, values.values[self.down]));
        pos2(egui::lerp(rect.x_range(), fx), egui::lerp(rect.y_range(), 1.0 - fy))
    }
}

/// The operation this frame's keys make on the focused area; `None` when no key asked for a
/// change, and during a drag, which keys don't change (they are still taken).
fn keys(ui: &Ui, response: &Response, dragging: bool, axes: &Axes) -> Option<AreaOp> {
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
            to_x = Some(Bound::Low);
        }
        if count(Modifiers::NONE, Key::End) > 0.0 {
            to_x = Some(Bound::High);
        }
    });
    if dragging || (dx == 0.0 && dy == 0.0 && to_x.is_none()) {
        return None;
    }
    Some(AreaOp { x: to_x.map_or(ScalarOp::By(dx), ScalarOp::ToBound), y: ScalarOp::By(dy) })
}

/// Paints the colours, the edge and the marker.
fn paint(ui: &Ui, style: &ColorPartsStyle, area: &ColorArea, axes: &Axes, rect: Rect, shown: &ModelValues, enabled: bool) {
    let model = area.mode.model();
    let painter = ui.painter();
    let mut mesh = Mesh::default();
    match area.quantize {
        Some(levels) => flat_cells(&mut mesh, model, axes, rect, shown, levels),
        None => smooth_mesh(&mut mesh, model, axes, rect, shown),
    }
    painter.add(mesh);
    painter.rect_stroke(rect, CornerRadius::ZERO, faded_stroke(style.edge, enabled), StrokeKind::Outside);
    let at = axes.pos(rect, shown);
    painter.circle_stroke(at, style.marker_outer_radius, faded_stroke(style.marker_outer, enabled));
    painter.circle_stroke(at, style.marker_inner_radius, faded_stroke(style.marker_inner, enabled));
}

/// The colours as a mesh of vertices, blending between them.
fn smooth_mesh(mesh: &mut Mesh, model: &dyn ColorModel, axes: &Axes, rect: Rect, shown: &ModelValues) {
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
}

/// The colours as `QUANTIZE_CELLS` by `QUANTIZE_CELLS` flat cells, each the snapped colour at its
/// centre.
fn flat_cells(mesh: &mut Mesh, model: &dyn ColorModel, axes: &Axes, rect: Rect, shown: &ModelValues, levels: usize) {
    let n = QUANTIZE_CELLS;
    let (w, h) = (rect.width() / n as f32, rect.height() / n as f32);
    for row in 0..n {
        for col in 0..n {
            let (fx, fy) = ((col as f32 + 0.5) / n as f32, (row as f32 + 0.5) / n as f32);
            let mut v = *shown;
            v.values[axes.across] = egui::lerp(axes.x.min as f32..=axes.x.max as f32, fx);
            v.values[axes.down] = egui::lerp(axes.y.max as f32..=axes.y.min as f32, fy);
            let min = pos2(rect.left() + col as f32 * w, rect.top() + row as f32 * h);
            mesh.add_colored_rect(Rect::from_min_size(min, Vec2::new(w, h)), snapped(model.to_srgb(&v.values), levels).into());
        }
    }
}
