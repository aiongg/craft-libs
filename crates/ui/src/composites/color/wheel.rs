//! [`ColorWheel`]: a hue ring round a saturation and brightness triangle that turns with the hue,
//! as PhotoCraft's Color Wheel.

use std::f32::consts::TAU;

use craft_theme::{ColorModel, HsbModel, ModelValues};
use egui::accesskit::{Action, Role};
use egui::epaint::PathShape;
use egui::{AsIdSalt, Id, IdSalt, InteractOptions, Mesh, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, UiBuilder, Vec2, vec2};

use super::area::{Axes, keys};
use super::{AreaMode, faded_stroke, parts_style, scale_of, spoken};
use crate::controls::slider::{KEY_FILTER, PressTurn, Sensing, press_turn, stepped, tap};
use crate::edit::ops::{Bound, ScalarOp};
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{FocusRing, Interactive};
use crate::theme::{Tokens, TokensOf, UiTokens};

/// Steps round the hue ring's mesh (PhotoCraft's 96).
const RING_STEPS: u32 = 96;

/// Cells along each side of the triangle's mesh (PhotoCraft's 16).
const TRIANGLE_CELLS: usize = 16;

/// The hue ring and the triangle: where a press began decides which one a drag edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelPart {
    /// The ring: a press or drag sets the hue to the pointer's angle.
    Ring,
    /// The triangle: a press or drag sets saturation and brightness.
    Triangle,
}

/// Where a wheel lies in its area: pure numbers, shared with the colour disc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelGeometry {
    pub center: Pos2,
    /// Radius of the ring's inner edge.
    pub inner: f32,
    /// Radius of the ring's outer edge.
    pub outer: f32,
    /// Radius of the circle round the triangle's corners.
    pub triangle: f32,
}

impl WheelGeometry {
    /// The wheel in `area`: the outer radius is half the shorter side less `outer_inset`; the
    /// ring is `ratio` of it wide, between `ring_min` and `ring_max` and at most half the
    /// radius; the triangle's circle lies `triangle_inset` inside the ring.
    pub fn new(area: Rect, outer_inset: f32, ring: (f32, f32), ratio: f32, triangle_inset: f32) -> Self {
        let outer = (area.width().min(area.height()) / 2.0 - outer_inset).max(1.0);
        let width = (outer * ratio).max(ring.0).min(ring.1).min(outer * 0.5);
        let inner = outer - width;
        Self { center: area.center(), inner, outer, triangle: (inner - triangle_inset).max(0.0) }
    }

    /// The part a press at `pos` takes: nothing beyond the ring's outer edge plus `grab_outer`,
    /// the ring from `grab_inner` inside its inner edge, the triangle within.
    pub fn part_at(&self, pos: Pos2, grab_outer: f32, grab_inner: f32) -> Option<WheelPart> {
        let d = pos.distance(self.center);
        if !d.is_finite() || d > self.outer + grab_outer {
            None
        } else if d >= self.inner - grab_inner {
            Some(WheelPart::Ring)
        } else {
            Some(WheelPart::Triangle)
        }
    }

    /// The middle of the ring at `hue` degrees: where its knob sits.
    pub fn ring_knob(&self, hue: f32) -> Pos2 {
        self.center + hue_direction(hue) * ((self.inner + self.outer) / 2.0)
    }

    /// The triangle's corners at `hue`, as [`triangle_corners`], grown by `by` points.
    pub fn corners(&self, hue: f32, by: f32) -> [Pos2; 3] {
        triangle_corners(self.center, self.triangle + by, hue)
    }
}

/// The unit vector on screen at `hue` degrees: red (0) to the right, growing counter-clockwise.
pub fn hue_direction(hue: f32) -> Vec2 {
    let a = hue.to_radians();
    vec2(a.cos(), -a.sin())
}

/// The hue in whole degrees, 0 to 359, of the direction from `center` to `p`; `None` for a point
/// that isn't finite.
pub fn hue_at(center: Pos2, p: Pos2) -> Option<f32> {
    let a = (center.y - p.y).atan2(p.x - center.x);
    a.is_finite().then(|| (a.rem_euclid(TAU).to_degrees().round()).rem_euclid(360.0))
}

/// The triangle's corners for `hue` degrees inside radius `r`: the pure hue at the hue's angle,
/// white 120 degrees on and black 240 degrees on.
pub fn triangle_corners(center: Pos2, r: f32, hue: f32) -> [Pos2; 3] {
    [0.0, 120.0, 240.0].map(|turn| center + hue_direction(hue + turn) * r)
}

/// The point of saturation `s` and brightness `b` (0 to 1) in the triangle: black + b (white -
/// black) + b s (hue - white).
pub fn triangle_point([hue, white, black]: [Pos2; 3], s: f32, b: f32) -> Pos2 {
    black + (white - black) * b + (hue - white) * (b * s)
}

/// Saturation and brightness (0 to 1) at `p`, kept inside the triangle; `None` for a degenerate
/// triangle or a point that isn't finite.
pub fn triangle_sv([hue, white, black]: [Pos2; 3], p: Pos2) -> Option<(f32, f32)> {
    let (a, c, d) = (white - black, hue - white, p - black);
    let det = a.x * c.y - a.y * c.x;
    if det.abs() < 1e-6 {
        return None;
    }
    let (b, bs) = ((d.x * c.y - d.y * c.x) / det, (a.x * d.y - a.y * d.x) / det);
    if !b.is_finite() || !bs.is_finite() {
        return None;
    }
    let b = b.clamp(0.0, 1.0);
    let bs = bs.clamp(0.0, b);
    Some((if b > 0.0 { bs / b } else { 0.0 }, b))
}

/// The wheel's sizes, edge and markers, from [`ColorWheel::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColorWheelStyle {
    /// Default height of the area: `color_wheel.side`.
    pub side: f32,
    /// Least height of the area: `color_wheel.side_min`.
    pub side_min: f32,
    pub ring_min: f32,
    pub ring_max: f32,
    pub ring_ratio: f32,
    pub outer_inset: f32,
    pub triangle_inset: f32,
    /// A press this far outside the ring still takes it, and this far inside its inner edge.
    pub grab_outer: f32,
    pub grab_inner: f32,
    /// One point of `line.control_edge` round the ring and the triangle.
    pub edge: Stroke,
    /// The knob's inner circle and outer ring, as [`ColorArea`]'s marker.
    pub marker_inner_radius: f32,
    pub marker_inner: Stroke,
    pub marker_outer_radius: f32,
    pub marker_outer: Stroke,
}

/// A hue ring round a saturation and brightness triangle, editing a colour's HSB values:
///
/// ```
/// use craft_theme::{ColorModel, HsbModel, Srgba};
/// use craft_ui::composites::ColorWheel;
/// # egui::__run_test_ui(|ui| { let mut color = Srgba::rgb(0.2, 0.5, 0.8);
/// let values = HsbModel.from_srgb(color);
/// let wheel = ColorWheel::new("wheel", "Color wheel", values).show(ui);
/// if let Some(v) = wheel.edit.event.as_ref().and_then(|e| e.value()) {
///     color = HsbModel.to_srgb(&v.values);
/// }
/// # });
/// ```
///
/// The host gives the colour as HSB [`ModelValues`] and gets them back through the edit lifecycle
/// ([`crate::edit`]) with only the picked components changed: the ring changes the hue, the
/// triangle saturation and brightness. A host that keeps the values it was sent
/// ([`kept_or_converted`](craft_theme::kept_or_converted)) keeps the hue of a grey and the hue
/// and saturation of black.
///
/// - Pointer (primary button): the part under the press is kept for the whole drag, so a drag
///   that leaves the ring keeps turning the hue; a press further outside the ring than
///   `color_wheel.grab_outer` starts nothing. The press sets the value under the pointer (whole
///   degrees, whole percents) and is the gesture's first value; the release commits. A press
///   and release in one frame is a [`Set`](crate::edit::EditEvent::Set). A press focuses the
///   part.
/// - Keys on the focused ring: Left and Down lower the hue by 1 degree, Right and Up raise it,
///   wrapping at 360; Shift steps ten; Page Up and Page Down 36 degrees; Home 0, End 359. On the
///   triangle (as [`ColorArea`]): Left and Right saturation, Up and Down brightness; Shift ten;
///   Page keys a tenth of brightness; Home and End saturation 0 and 100. The directions are not
///   screen directions: the triangle turns with the hue. Each key is a
///   [`Set`](crate::edit::EditEvent::Set); during a drag keys change nothing.
/// - Esc during a drag cancels it.
/// - Two Tab stops, the ring and then the triangle; a [`FieldTab`](crate::behavior::FieldTab)
///   scope skips both and Ctrl+Tab reaches them.
/// - AccessKit: a `Group` named `label` holding a `Slider` for the hue (0 to 360, step 1,
///   increment, decrement and set value) and a `Slider` for saturation and brightness (value
///   text, no numeric value).
/// - Right to left nothing mirrors: red stays at 3 o'clock.
#[derive(Clone, Debug)]
#[must_use = "show the wheel with `show`"]
pub struct ColorWheel {
    id_salt: IdSalt,
    label: String,
    values: ModelValues,
    side: Option<f32>,
    policy: EditPolicy,
    enabled: bool,
    part: bool,
    names: [String; 4],
}

/// What [`ColorWheel::show`] reports.
#[derive(Clone, Debug)]
pub struct ColorWheelResponse {
    /// This frame's edit event; the response is the group's, covering the area.
    pub edit: EditResponse<ModelValues>,
    /// The area the wheel was given, for a host's button over its end corner.
    pub area: Rect,
    pub geometry: WheelGeometry,
    /// The ids of the two focus stops.
    pub ring: Id,
    pub triangle: Id,
}

impl ColorWheel {
    /// The English words the wheel speaks, the keys a host translates with
    /// [`text`](ColorWheel::text): the ring's name, the triangle's, and the two components in
    /// the triangle's value text.
    pub const TEXTS: [&'static str; 4] = ["Hue", "Saturation and brightness", "Saturation", "Brightness"];

    /// A wheel named `label` ("Color wheel", translated by the host) showing `values`, HSB values;
    /// `id_salt` keeps its gesture and focus when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, values: ModelValues) -> Self {
        debug_assert_eq!(values.model, HsbModel.id(), "the wheel's values must be HSB");
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            values,
            side: None,
            policy: EditPolicy::default(),
            enabled: true,
            part: false,
            names: Self::TEXTS.map(str::to_owned),
        }
    }

    /// The area's height; `color_wheel.side` without it. Never below `color_wheel.side_min`.
    pub fn side(mut self, side: f32) -> Self {
        self.side = Some(side);
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

    /// Translates the wheel's words, given as the English [`TEXTS`](ColorWheel::TEXTS).
    pub fn text(mut self, translate: impl Fn(&str) -> String) -> Self {
        self.names = Self::TEXTS.map(translate);
        self
    }

    /// Whether a press waits for another control's typing run to commit and then edits the values
    /// that run left: on inside a panel with number fields, off standalone.
    pub fn press_waits(mut self, waits: bool) -> Self {
        self.part = waits;
        self
    }

    pub fn style(tokens: &UiTokens) -> ColorWheelStyle {
        let t = &tokens.component.color_wheel;
        let parts = parts_style(tokens);
        ColorWheelStyle {
            side: t.side,
            side_min: t.side_min,
            ring_min: t.ring_min,
            ring_max: t.ring_max,
            ring_ratio: t.ring_ratio,
            outer_inset: t.outer_inset,
            triangle_inset: t.triangle_inset,
            grab_outer: t.grab_outer,
            grab_inner: t.grab_inner,
            edge: parts.edge,
            marker_inner_radius: parts.marker_inner_radius,
            marker_inner: parts.marker_inner,
            marker_outer_radius: parts.marker_outer_radius,
            marker_outer: parts.marker_outer,
        }
    }

    /// Where the wheel lies in `area`.
    pub fn geometry(style: &ColorWheelStyle, area: Rect) -> WheelGeometry {
        WheelGeometry::new(area, style.outer_inset, (style.ring_min, style.ring_max), style.ring_ratio, style.triangle_inset)
    }

    /// Shows the wheel and returns this frame's edit event.
    pub fn show(self, ui: &mut Ui) -> ColorWheelResponse {
        let style = ColorWheel::style(&Tokens::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(self.label.clone());
        let inner = ui.scope_builder(builder, |ui| self.show_inside(ui, &style, id));
        let (event, area, geometry, ring, triangle) = inner.inner;
        ColorWheelResponse { edit: EditResponse::new(inner.response, event), area, geometry, ring, triangle }
    }

    fn show_inside(self, ui: &mut Ui, style: &ColorWheelStyle, id: Id) -> (Option<crate::edit::EditEvent<ModelValues>>, Rect, WheelGeometry, Id, Id) {
        let height = self.side.filter(|s| s.is_finite()).unwrap_or(style.side).max(style.side_min);
        let width = if ui.available_width().is_finite() { ui.available_width() } else { height };
        let (_, area) = ui.allocate_space(vec2(width, height));
        let geometry = ColorWheel::geometry(style, area);
        let ring_id = Interactive::id_in(ui, "ring");
        let triangle_id = Interactive::id_in(ui, "triangle");

        // One widget takes every press and drag; the focus goes to the two part widgets. It covers
        // the wheel's square and the grab slack only, so presses beside it reach what lies there.
        let pointer_id = id.with("pointer");
        let options = InteractOptions { enabled: self.enabled, ..Default::default() };
        let reach = Rect::from_center_size(geometry.center, Vec2::splat(2.0 * (geometry.outer + style.grab_outer))).intersect(area);
        let response = ui.interact_opt(reach, pointer_id, Sense::DRAG, options);
        let enabled = response.enabled();
        let grab = |p: Pos2| geometry.part_at(p, style.grab_outer, style.grab_inner);

        let hue_scale = scale_of(&HsbModel, 0);
        let axes = Axes::of(AreaMode::Hue, None);
        let ring_rect = Rect::from_center_size(geometry.center, Vec2::splat(2.0 * geometry.outer));
        let corners = geometry.corners(self.values.values[0], 0.0);
        let triangle_rect = Rect::from_points(&corners).intersect(ring_rect).expand(1.0);
        let ring = Interactive::new("ring", Role::Slider, self.names[0].as_str())
            .sense(Sense::FOCUSABLE)
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, ring_rect);
        let triangle = Interactive::new("triangle", Role::Slider, self.names[1].as_str())
            .sense(Sense::FOCUSABLE)
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, triangle_rect);
        let triangle_response = triangle.response().clone();

        let mut gesture =
            Gesture::load(ui, pointer_id, &self.values, self.policy).focus_owners(&[ring_id, triangle_id]).event_filter(KEY_FILTER).enabled(enabled);
        let modifiers = || ui.input(|i| i.modifiers);
        let tapped = tap(ui, &response);
        let origin = response.drag_started_by(egui::PointerButton::Primary).then(|| response.interact_pointer_pos()).flatten();
        let press = origin.or(tapped).filter(|p| grab(*p).is_some());
        let turn = press_turn(ui, pointer_id, self.part, gesture.id(), press);
        // The part the press took, for the whole drag: the press is in layer coordinates, and so
        // is every drag position.
        let part_key = pointer_id.with("craft_ui::wheel_part");
        let mut pressed_part = ui.data(|d| d.get_temp::<Option<WheelPart>>(part_key)).flatten();
        // The part to give the focus this frame.
        let mut focus = None;
        if let PressTurn::Held(held) = turn
            && let Some(part) = grab(held)
        {
            focus = Some(part);
            pressed_part = Some(part);
            gesture.begin();
            if let Some(start) = gesture.start().copied() {
                gesture.update(value_at(&geometry, &start, part, held), modifiers());
            }
            if !response.dragged_by(egui::PointerButton::Primary) {
                gesture.commit();
            }
        }
        let waits = matches!(turn, PressTurn::Waits);
        if let Some(part) = origin.and_then(grab) {
            focus = Some(part);
            if !waits {
                pressed_part = Some(part);
                gesture.begin();
            }
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(start) = gesture.start().copied()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(part) = pressed_part
        {
            gesture.update(value_at(&geometry, &start, part, pos), modifiers());
        }
        if response.drag_stopped() {
            gesture.commit();
            pressed_part = None;
        }
        ui.data_mut(|d| d.insert_temp(part_key, pressed_part));
        if let Some(pos) = tapped.filter(|_| !waits)
            && let Some(part) = grab(pos)
        {
            focus = Some(part);
            gesture.set(value_at(&geometry, gesture.shown(), part, pos));
        }

        let sensing = Sensing { wheel: false, ..Sensing::of(&response) };
        if let Some(op) = stepped(ui, ring_id, sensing, None, None, &hue_scale) {
            gesture.set(hue_applied(&op, gesture.shown()));
        }
        if let Some(op) = keys(ui, &triangle_response, response.dragged(), &axes) {
            gesture.set(super::area::apply(&op, gesture.shown(), AreaMode::Hue, None));
        }
        if enabled {
            ui.input(|i| {
                for (id, part) in [(ring_id, WheelPart::Ring), (triangle_id, WheelPart::Triangle)] {
                    if i.num_accesskit_action_requests(id, Action::Click) > 0 {
                        focus = Some(part);
                    }
                }
            });
        }

        let shown = *gesture.shown();
        let [hue, saturation, brightness] = [0, 1, 2].map(|c| finite(shown.values[c]));
        let model = &HsbModel as &dyn ColorModel;
        let text = format!("{}, {}", spoken(&self.names[2], model, 1, saturation), spoken(&self.names[3], model, 2, brightness));
        let hue_value = hue_scale.place(f64::from(hue)).rem_euclid(360.0);
        let ring_response = ring.numeric(hue_value, 0.0..=360.0, 1.0).finish(ui);
        let triangle_response = triangle.value(text).finish(ui);
        if enabled {
            for id in [ring_id, triangle_id] {
                ui.ctx().accesskit_node_builder(id, |node| node.add_action(Action::Click));
            }
        }
        if let Some(part) = focus {
            let id = if part == WheelPart::Ring { ring_id } else { triangle_id };
            if !ui.memory(|m| m.has_focus(id)) {
                ui.memory_mut(|m| m.request_focus(id));
            }
        }
        if ui.is_rect_visible(area) {
            paint(ui, style, &geometry, [hue, saturation, brightness], enabled, [&ring_response, &triangle_response]);
        }
        (gesture.finish(), area, geometry, ring_id, triangle_id)
    }
}

pub(super) fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// `base` with what a press at `pos` in `part` picks: the hue for the ring, saturation and
/// brightness for the triangle (turned to `base`'s hue); `base` when `pos` picks nothing.
fn value_at(geometry: &WheelGeometry, base: &ModelValues, part: WheelPart, pos: Pos2) -> ModelValues {
    let mut out = *base;
    match part {
        WheelPart::Ring => {
            if let Some(hue) = hue_at(geometry.center, pos) {
                out.values[0] = hue;
            }
        }
        WheelPart::Triangle => {
            if let Some((s, b)) = triangle_sv(geometry.corners(finite(base.values[0]), 0.0), pos) {
                out.values[1] = (s * 100.0).round();
                out.values[2] = (b * 100.0).round();
            }
        }
    }
    out
}

/// `values` with `op` applied to the hue, which wraps at 360 and has no 360 of its own: End is 359.
fn hue_applied(op: &ScalarOp, values: &ModelValues) -> ModelValues {
    let hue = f64::from(finite(values.values[0]));
    let hue = match op {
        ScalarOp::By(delta) => hue + delta,
        ScalarOp::Set(value) if value.is_finite() => *value,
        ScalarOp::ToBound(Bound::Low) => 0.0,
        ScalarOp::ToBound(Bound::High) => 359.0,
        _ => hue,
    };
    let mut out = *values;
    out.values[0] = hue.round().rem_euclid(360.0) as f32;
    out
}

/// The colour of HSB `[h, s, b]` (degrees and percents) as a vertex colour.
pub(super) fn vertex(h: f32, s: f32, b: f32) -> egui::Color32 {
    HsbModel.to_srgb(&ModelValues::new("hsb", &[h, s, b]).values).into()
}

fn ring_mesh(geometry: &WheelGeometry) -> Mesh {
    let mut mesh = Mesh::default();
    for k in 0..=RING_STEPS {
        let hue = k as f32 / RING_STEPS as f32 * 360.0;
        let color = vertex(hue, 100.0, 100.0);
        let dir = hue_direction(hue);
        mesh.colored_vertex(geometry.center + dir * geometry.inner, color);
        mesh.colored_vertex(geometry.center + dir * geometry.outer, color);
    }
    for k in 0..RING_STEPS {
        let a = k * 2;
        mesh.add_triangle(a, a + 1, a + 3);
        mesh.add_triangle(a, a + 3, a + 2);
    }
    mesh
}

fn triangle_mesh(corners: [Pos2; 3], hue: f32) -> Mesh {
    let n = TRIANGLE_CELLS;
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (s, b) = (i as f32 / n as f32, j as f32 / n as f32);
            mesh.colored_vertex(triangle_point(corners, s, b), vertex(hue, s * 100.0, b * 100.0));
        }
    }
    let row = (n + 1) as u32;
    for j in 0..n as u32 {
        for i in 0..n as u32 {
            let a = j * row + i;
            mesh.add_triangle(a, a + 1, a + row + 1);
            mesh.add_triangle(a, a + row + 1, a + row);
        }
    }
    mesh
}

/// Paints the ring, the triangle, their edges, the knobs and the focus ring of the focused part.
fn paint(ui: &Ui, style: &ColorWheelStyle, geometry: &WheelGeometry, [hue, saturation, brightness]: [f32; 3], enabled: bool, parts: [&Response; 2]) {
    let painter = ui.painter();
    let corners = geometry.corners(hue, 0.0);
    painter.add(ring_mesh(geometry));
    painter.add(triangle_mesh(corners, hue));
    let edge = faded_stroke(style.edge, enabled);
    painter.circle_stroke(geometry.center, geometry.outer + edge.width / 2.0, edge);
    let outline = geometry.corners(hue, edge.width).to_vec();
    painter.add(Shape::Path(PathShape::closed_line(outline.clone(), edge)));
    let knobs = [geometry.ring_knob(hue), triangle_point(corners, saturation / 100.0, brightness / 100.0)];
    for at in knobs {
        painter.circle_stroke(at, style.marker_inner_radius, faded_stroke(style.marker_inner, enabled));
        painter.circle_stroke(at, style.marker_outer_radius, faded_stroke(style.marker_outer, enabled));
    }
    FocusRing::paint_circle(ui, geometry.center, geometry.outer + edge.width, Some(geometry.inner), parts[0].has_focus());
    FocusRing::paint_polygon(ui, &outline, parts[1].has_focus());
}
