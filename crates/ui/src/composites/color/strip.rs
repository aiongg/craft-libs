//! [`ColorStrip`]: one component of a colour along a vertical strip, the others fixed.

use craft_theme::ModelValues;
use egui::accesskit::{Orientation, Role};
use egui::{AsIdSalt, CornerRadius, IdSalt, Mesh, PointerButton, Rect, Sense, Shape, StrokeKind, Ui, Vec2, pos2, vec2};

use super::{AreaMode, ColorArea, ColorPartsStyle, faded, faded_stroke, fraction, scale_of, value_at};
use crate::controls::slider::{KEY_FILTER, PressTurn, Scale, Sensing, press_turn, stepped, tap};
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{FocusRing, Interactive};
use crate::theme::{Tokens, TokensOf, UiTokens};

/// Steps of the strip's colour mesh (PhotoCraft's 24).
const MESH_STEPS: usize = 24;

/// The component strip beside a colour picker's [`ColorArea`]: the [`AreaMode`]'s strip
/// component from its lowest value at the bottom to its highest at the top, the others fixed
/// (PhotoCraft's component strip; hue runs from 360° at the top to 0° at the bottom). The HSB hue
/// strip shows pure hues; the others show the colour's own values.
///
/// As the area, it edits the mode's model values and reports through the edit lifecycle
/// ([`crate::edit`]), changing only its component:
///
/// - Pointer (primary button): the press sets the value under the pointer, drawn in the press
///   frame and reported in the next; dragging moves it (the pointer may leave the strip); the
///   release commits; a press and release in one frame is a [`Set`](crate::edit::EditEvent::Set).
///   Values snap to the channel's step.
/// - Keys, while focused (each a [`Set`](crate::edit::EditEvent::Set)): Up and Right step the
///   value up, Down and Left down; Shift steps ten. Page Up and Page Down move a tenth of the
///   range, Home and End go to its ends. During a drag keys change nothing. The wheel scrolls the
///   panel.
/// - Esc during a drag cancels it, as the area's.
/// - AccessKit: a vertical `Slider` named `label` ("Hue"), with its value, bounds and step, which
///   takes increment, decrement and set-value requests.
///
/// Its rect is the strip itself; it takes the room of a pointer triangle and the edge on each
/// side of it besides, so a host laid out with no gap round it still shows both pointers. Like
/// the area it is not a field ([`FieldTab`](crate::behavior::FieldTab) skips it) and keeps its
/// direction right to left.
#[derive(Clone, Debug)]
#[must_use = "show the strip with `show`"]
pub struct ColorStrip {
    id_salt: IdSalt,
    label: String,
    mode: AreaMode,
    values: ModelValues,
    size: Option<Vec2>,
    policy: EditPolicy,
    enabled: bool,
    part: bool,
}

impl ColorStrip {
    /// A strip named `label` (its accessible name, translated by the host: "Hue") showing
    /// `values`, values of `mode`'s model; `id_salt` keeps its gesture and focus when the layout
    /// around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, mode: AreaMode, values: ModelValues) -> Self {
        debug_assert_eq!(values.model, mode.model().id(), "the strip's values must be of its mode's model");
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            mode,
            values,
            size: None,
            policy: EditPolicy::default(),
            enabled: true,
            part: false,
        }
    }

    /// The strip's size, without its pointers' room; `color_picker.strip_full` by
    /// `color_picker.area` without it (the Full picker's).
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

    /// Whether the strip is a part of a colour picker, whose press takes its turn from the edit
    /// order ([`press_turn`]): it waits for a field's typing run to end.
    pub(crate) fn part(mut self, part: bool) -> Self {
        self.part = part;
        self
    }

    /// The same as [`ColorArea::style`].
    pub fn style(tokens: &UiTokens) -> ColorPartsStyle {
        ColorArea::style(tokens)
    }

    /// The width a strip of `width` takes in the layout: its pointers' room and edges besides.
    pub fn outer_width(style: &ColorPartsStyle, width: f32) -> f32 {
        width + 2.0 * (style.edge.width + style.pointer.x)
    }

    /// Shows the strip and returns this frame's edit event. The response covers the strip
    /// without its pointers' room.
    pub fn show(self, ui: &mut Ui) -> EditResponse<ModelValues> {
        let style = ColorStrip::style(&Tokens::of(ui));
        let size = self.size.unwrap_or(vec2(style.strip_full, style.area));
        let (_, outer) = ui.allocate_space(vec2(ColorStrip::outer_width(&style, size.x), size.y));
        let rect = Rect::from_center_size(outer.center(), size);
        let model = self.mode.model();
        let channel = self.mode.strip();
        let scale = scale_of(model, channel);
        let sensed = Interactive::salted(self.id_salt, Role::Slider, self.label.as_str())
            .sense(Sense::drag())
            .enabled(self.enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, rect);
        let response = sensed.response().clone();
        let at = |base: &ModelValues, y: f32| {
            let mut values = *base;
            let t = if rect.height() > 0.0 { (rect.bottom() - y) / rect.height() } else { 0.0 };
            values.values[channel] = value_at(&scale, t);
            values
        };

        let mut gesture = Gesture::load(ui, response.id, &self.values, self.policy).event_filter(KEY_FILTER).enabled(response.enabled());
        let tapped = tap(ui, &response);
        let pressed = response.drag_started_by(PointerButton::Primary).then(|| response.interact_pointer_pos()).flatten();
        let turn = press_turn(ui, response.id, self.part, gesture.id(), pressed.or(tapped));
        if let PressTurn::Held(held) = turn {
            gesture.begin();
            if let Some(start) = gesture.start().copied() {
                gesture.update(at(&start, held.y), ui.input(|i| i.modifiers));
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
            gesture.update(at(&start, pos.y), ui.input(|i| i.modifiers));
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        if tapped.is_some() {
            response.request_focus();
        }
        if let Some(pos) = tapped.filter(|_| !waits) {
            let shown = *gesture.shown();
            gesture.set(at(&shown, pos.y));
        }
        let sensing = Sensing { wheel: false, ..Sensing::of(&response) };
        let shown = *gesture.shown();
        let value = scale.place(f64::from(shown.values[channel]));
        if let Some(to) = stepped(ui, response.id, sensing, None, value, (scale.min, scale.max), &scale) {
            let mut values = shown;
            values.values[channel] = to as f32;
            gesture.set(values);
        }

        let shown = *gesture.shown();
        let value = scale.place(f64::from(shown.values[channel]));
        let response = sensed.numeric(value, scale.min..=scale.max, scale.key_step()).finish(ui);
        ui.ctx().accesskit_node_builder(response.id, |node| node.set_orientation(Orientation::Vertical));
        if ui.is_rect_visible(outer) {
            paint(ui, &style, &self, &scale, rect, &shown, response.enabled());
        }
        FocusRing::paint_radius(ui, rect.expand(style.edge.width), CornerRadius::ZERO, &response);
        EditResponse::new(response, gesture.finish())
    }
}

/// Paints the colours, the edge and the pointers.
fn paint(ui: &Ui, style: &ColorPartsStyle, strip: &ColorStrip, scale: &Scale, rect: Rect, shown: &ModelValues, enabled: bool) {
    let (model, channel) = (strip.mode.model(), strip.mode.strip());
    let base = strip.mode.strip_base(shown);
    let painter = ui.painter();
    let mut mesh = Mesh::default();
    for k in 0..=MESH_STEPS {
        let t = k as f32 / MESH_STEPS as f32;
        let mut v = base;
        v.values[channel] = egui::lerp(scale.max as f32..=scale.min as f32, t);
        let color = model.to_srgb(&v.values).into();
        let y = egui::lerp(rect.y_range(), t);
        mesh.colored_vertex(pos2(rect.left(), y), color);
        mesh.colored_vertex(pos2(rect.right(), y), color);
    }
    for k in 0..MESH_STEPS as u32 {
        let a = 2 * k;
        mesh.add_triangle(a, a + 1, a + 3);
        mesh.add_triangle(a, a + 3, a + 2);
    }
    painter.add(mesh);
    painter.rect_stroke(rect, CornerRadius::ZERO, faded_stroke(style.edge, enabled), StrokeKind::Outside);
    let y = egui::lerp(rect.bottom()..=rect.top(), fraction(scale, shown.values[channel]));
    let (depth, half) = (style.pointer.x, style.pointer.y / 2.0);
    let color = faded(style.pointer_color, enabled);
    let edge = style.edge.width;
    for (tip, dir) in [(rect.left() - edge, -1.0), (rect.right() + edge, 1.0)] {
        let base = tip + dir * depth;
        let points = vec![pos2(tip, y), pos2(base, y - half), pos2(base, y + half)];
        painter.add(Shape::convex_polygon(points, color, egui::Stroke::NONE));
    }
}
