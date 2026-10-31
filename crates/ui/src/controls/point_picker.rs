//! [`PointPicker`]: a thumbnail with a draggable cross for a centre point, plus X and Y fields.

use std::ops::RangeInclusive;

use craft_theme::ControlSize;
use egui::accesskit::Role;
use egui::load::SizedTexture;
use egui::{
    AsIdSalt, Color32, CornerRadius, CursorIcon, Id, IdSalt, Key, Modifiers, PointerButton, Pos2, Rect, Sense, Shape, Stroke, Ui, UiBuilder, Vec2,
    pos2, vec2,
};

use super::NumberField;
use super::number_field::{self, FieldRule};
use super::slider::{KEY_FILTER, PressTurn, press_turn, tap};
use super::swatch::{Swatch, checkerboard};
use super::text_field::{BoxState, TextField, field_box};
use crate::edit::ops::{self, ScalarOp};
use crate::edit::{self, EditPolicy, EditResponse, Gesture, parts};
use crate::localized::{LocaleNumbers, Unit};
use crate::primitives::{Content, FocusRing, Gap, Interactive, Row, Stack, Text};
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// The style of one picker, from the tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct PointPickerStyle {
    /// The thumbnail's size, from `dialog.viewport_min`.
    pub thumbnail: Vec2,
    /// Cross arm length in points.
    pub cross: f32,
}

/// A 2D point picked on a thumbnail of the image (Radial Blur's Blur Center, Zoom Blur, Lens
/// Flare), with X and Y [`NumberField`](super::NumberField)s beside it.
///
/// The user sees the image fitted in a sunken well (a checkerboard when there is no image), a
/// cross at the point, and two fields. A press on the thumbnail moves the point there; a drag
/// follows; the fields edit the same value in the picker's unit (percent of the image by
/// default). Keyboard: the thumbnail is one Tab stop; arrows move the point by one unit, Shift
/// ten; Esc during a drag restores the value (edit lifecycle). AccessKit: a group named `label`
/// with the fields as its children.
///
/// Reports one edit per gesture through [`EditResponse`], the value in the picker's unit.
#[must_use = "show the picker with `show`"]
pub struct PointPicker<'a> {
    id_salt: IdSalt,
    label: String,
    value: [f64; 2],
    image: Option<SizedTexture>,
    range: RangeInclusive<f64>,
    unit: Unit,
    decimals: usize,
    policy: EditPolicy,
    enabled: bool,
    _numbers: std::marker::PhantomData<&'a ()>,
}

impl<'a> PointPicker<'a> {
    /// A picker named `label` showing `value` (x, y) in `0..=100` percent, over `image` when
    /// there is one.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: [f64; 2], image: Option<SizedTexture>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            image,
            range: 0.0..=100.0,
            unit: Unit::new("{n}%"),
            decimals: 0,
            policy: EditPolicy::default(),
            enabled: true,
            _numbers: std::marker::PhantomData,
        }
    }

    /// The fields' range on each axis and the unit they show (pixels of the image, percent).
    pub fn range(mut self, range: RangeInclusive<f64>, unit: Unit) -> Self {
        self.range = range;
        self.unit = unit;
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The style, dumped under `components/point_picker`.
    // new: sizes from `dialog.viewport_min` and the field tokens; no PhotoCraft token.
    pub fn style(tokens: &UiTokens) -> PointPickerStyle {
        PointPickerStyle { thumbnail: Vec2::from(tokens.component.dialog.viewport_min), cross: tokens.component.field.icon.get(ControlSize::Md) }
    }

    /// Draws the thumbnail and the two fields.
    // pc: crates/ui-egui/src/dialogs/filter_dialog/zoom_center.rs:21 `row` (:50 `interact` for
    // the press and drag, :74 `paint` for the cross) with an image in place of the streak diagram.
    pub fn show(self, ui: &mut Ui) -> EditResponse<[f64; 2]> {
        let id = ui.make_persistent_id(self.id_salt);
        let label = self.label.clone();
        let builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(label);
        ui.scope_builder(builder, |ui| self.show_inside(ui, id)).inner
    }

    fn show_inside(self, ui: &mut Ui, id: Id) -> EditResponse<[f64; 2]> {
        let tokens = Tokens::of(ui);
        let style = PointPicker::style(&tokens);
        let enabled = self.enabled && ui.is_enabled();
        let mut state = ui.data(|d| d.get_temp::<parts::State<PointOp>>(id)).unwrap_or_default();
        let outer = Gesture::load(ui, id, &self.value, self.policy.cancellable(false)).enabled(enabled);
        let policy = EditPolicy::default().cancellable(self.policy.cancellable).cost(self.policy.cost);
        let mut edits = parts::Edits::<[f64; 2], PointOp>::new(id, &state, &outer, edit::in_scope(ui));
        let rule = FieldRule::new(&self.range, 1.0, self.decimals);
        let apply_op = |op: &PointOp, now: &[f64; 2]| {
            Some(match op {
                PointOp::Thumb(ThumbOp::Set(to)) => [rule.settle(to[0]), rule.settle(to[1])],
                PointOp::Thumb(ThumbOp::By(by)) => [rule.settle(now[0] + by[0]), rule.settle(now[1] + by[1])],
                PointOp::Field(axis, op) => {
                    let mut next = *now;
                    next[*axis] = number_field::apply(op, now[*axis], &rule);
                    next
                }
            })
        };
        edits.replay(ui.ctx(), apply_op);

        let thumb = Row::new(Gap::Inline)
            .show(ui, |ui| {
                let (thumb, thumb_ops) = self.thumbnail(ui, &style, &tokens, *edits.value(), &rule, policy, enabled);
                edits.push(&thumb, thumb_ops, apply_op, |v, _| v);
                Stack::new(Gap::Stack).show(ui, |ui| {
                    for (axis, name) in ["X", "Y"].into_iter().enumerate() {
                        let field = self.field(ui, name, axis, edits.value()[axis], policy, enabled);
                        let field_ops = ops::take::<ScalarOp>(ui.ctx(), field.response.id);
                        edits.typing(&field, field_ops.into_iter().map(|op| PointOp::Field(axis, op)), apply_op, move |v, base| {
                            let mut next = *base;
                            next[axis] = v;
                            next
                        });
                    }
                });
                thumb
            })
            .inner;
        let outer = edits.feed(outer, &mut state);
        ui.data_mut(|d| d.insert_temp(id, state));
        EditResponse::new(thumb.response, outer.finish())
    }

    /// One axis's field, its name before it.
    fn field(&self, ui: &mut Ui, name: &str, axis: usize, shown: f64, policy: EditPolicy, enabled: bool) -> EditResponse<f64> {
        Row::new(Gap::Inline)
            .show(ui, |ui| {
                ui.add(Text::new(name).content(Content::Muted));
                let numbers = LocaleNumbers::of(ui);
                let width = NumberField::fit_width(ui, ControlSize::Md, &numbers, self.range.clone(), self.decimals, &self.unit);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    NumberField::new(("field", axis), format!("{} {name}", self.label), shown, &numbers)
                        .range(self.range.clone())
                        .step(1.0)
                        .decimals(0..=self.decimals)
                        .unit(self.unit.clone())
                        .policy(policy)
                        .part(true)
                        .enabled(enabled)
                        .show(ui)
                })
                .inner
            })
            .inner
    }

    /// The thumbnail showing `value`: its interaction, look and AccessKit node, and the
    /// operations its one-shot inputs made.
    #[allow(clippy::too_many_arguments)]
    fn thumbnail(
        &self,
        ui: &mut Ui,
        style: &PointPickerStyle,
        tokens: &UiTokens,
        value: [f64; 2],
        rule: &FieldRule,
        policy: EditPolicy,
        enabled: bool,
    ) -> (EditResponse<[f64; 2]>, Vec<PointOp>) {
        let sensed = Interactive::new("thumbnail", Role::Slider, self.label.as_str())
            .sense(Sense::drag())
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact(ui, style.thumbnail);
        let response = sensed.response().clone();
        let (well, dragging) = (response.rect, response.dragged());
        let plot = self.plot(well);
        let at = |pos: Pos2| {
            let t = |p: f32, lo: f32, size: f32| f64::from(((p - lo) / size.max(f32::EPSILON)).clamp(0.0, 1.0));
            let (lo, hi) = (*self.range.start(), *self.range.end());
            [lo + t(pos.x, plot.left(), plot.width()) * (hi - lo), lo + t(pos.y, plot.top(), plot.height()) * (hi - lo)].map(|v| rule.settle(v))
        };

        let mut gesture = Gesture::load(ui, response.id, &value, policy).event_filter(KEY_FILTER).enabled(enabled);
        let tapped = tap(ui, &response);
        let pressed = response.drag_started_by(PointerButton::Primary).then(|| response.interact_pointer_pos()).flatten();
        let turn = press_turn(ui, response.id, true, gesture.id(), pressed.or(tapped));
        if let PressTurn::Held(held) = turn {
            gesture.begin();
            gesture.update(at(held), ui.input(|i| i.modifiers));
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
            && gesture.is_active()
            && let Some(pos) = response.interact_pointer_pos()
        {
            gesture.update(at(pos), ui.input(|i| i.modifiers));
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        if tapped.is_some() {
            response.request_focus();
        }
        let mut ops = Vec::new();
        if let Some(pos) = tapped.filter(|_| !waits) {
            let to = at(pos);
            gesture.set(to);
            ops.push(PointOp::Thumb(ThumbOp::Set(to)));
        }
        if let Some(by) = nudge(ui, &response, dragging) {
            let now = *gesture.shown();
            let to = [rule.settle(now[0] + by[0]), rule.settle(now[1] + by[1])];
            gesture.set(to);
            ops.push(PointOp::Thumb(ThumbOp::By(by)));
        }

        let shown = *gesture.shown();
        let numbers = LocaleNumbers::of(ui);
        let axis = |v: f64| format!("{}{}{}", self.unit.prefix(), numbers.format(v, self.decimals..=self.decimals), self.unit.suffix());
        let text = format!("{}, {}", axis(shown[0]), axis(shown[1]));
        let response = sensed.value(text).finish(ui);
        if enabled {
            response.clone().on_hover_cursor(CursorIcon::Crosshair);
        }
        if ui.is_rect_visible(well) {
            self.paint(ui, style, tokens, well, plot, shown, &response, enabled);
        }
        (EditResponse::new(response, gesture.finish()), ops)
    }

    /// Where the range maps to: the image fitted in the well, else the well less room for the
    /// cross at its edges.
    fn plot(&self, well: Rect) -> Rect {
        match &self.image {
            Some(image) if image.size.x > 0.0 && image.size.y > 0.0 => {
                let scale = (well.width() / image.size.x).min(well.height() / image.size.y);
                Rect::from_center_size(well.center(), image.size * scale)
            }
            _ => well,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &self,
        ui: &Ui,
        style: &PointPickerStyle,
        tokens: &UiTokens,
        well: Rect,
        plot: Rect,
        value: [f64; 2],
        response: &egui::Response,
        enabled: bool,
    ) {
        let field = TextField::style(tokens, ControlSize::Md);
        let swatch = Swatch::style(tokens, ControlSize::Md);
        let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.5) };
        let painter = ui.painter_at(well);
        let radius: CornerRadius = field.surface.radius;
        let state = BoxState { enabled, focused: false, hovered: response.contains_pointer(), invalid: false };
        ui.painter().extend(field_box(ui, &field, well, state));
        match &self.image {
            Some(image) => {
                painter.add(Shape::rect_filled(well, radius, field.surface.fill));
                egui::Image::new(*image).tint(fade(Color32::WHITE)).paint_at(ui, plot);
            }
            None => {
                painter.extend(checkerboard(well, radius, swatch.checker.map(&fade), swatch.checker_cell));
            }
        }
        let (lo, hi) = (*self.range.start(), *self.range.end());
        let t = |v: f64| if hi > lo { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) as f32 } else { 0.0 };
        let center = pos2(plot.left() + t(value[0]) * plot.width(), plot.top() + t(value[1]) * plot.height());
        let arm = style.cross / 2.0;
        let ink = fade(tokens.color.text.normal.to_egui());
        let halo = fade(field.surface.fill);
        for (color, width) in [(halo, 3.0), (ink, 1.0)] {
            let stroke = Stroke::new(width, color);
            painter.line_segment([center - vec2(arm, 0.0), center + vec2(arm, 0.0)], stroke);
            painter.line_segment([center - vec2(0.0, arm), center + vec2(0.0, arm)], stroke);
        }
        FocusRing::paint_radius(ui, well, radius, response);
    }
}

/// An operation on the point.
#[derive(Clone, Debug, PartialEq)]
enum PointOp {
    Thumb(ThumbOp),
    /// An axis's field (0: x, 1: y).
    Field(usize, ScalarOp),
}

/// What the thumbnail's one-shot inputs did.
#[derive(Clone, Debug, PartialEq)]
enum ThumbOp {
    Set([f64; 2]),
    By([f64; 2]),
}

/// The arrow keys of the focused thumbnail as a change of the point, one unit a press and ten with
/// Shift; none while dragging. Right and down grow the value.
fn nudge(ui: &Ui, response: &egui::Response, dragging: bool) -> Option<[f64; 2]> {
    if dragging || !response.enabled() || !ui.memory(|m| m.has_focus(response.id)) {
        return None;
    }
    let mut by = [0.0; 2];
    ui.input_mut(|i| {
        let mut count = |modifiers, key| i.count_and_consume_key(modifiers, key) as f64;
        for (key, axis, sign) in [(Key::ArrowRight, 0, 1.0), (Key::ArrowLeft, 0, -1.0), (Key::ArrowDown, 1, 1.0), (Key::ArrowUp, 1, -1.0)] {
            // Shift first: `Modifiers::NONE` also matches a press with Shift held.
            by[axis] += sign * (10.0 * count(Modifiers::SHIFT, key) + count(Modifiers::NONE, key));
        }
    });
    (by != [0.0; 2]).then_some(by)
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Point picker"];

#[derive(StoryArgs, Debug, PartialEq)]
struct PickerArgs {
    #[arg(name = "pixels")]
    pixels: bool,
    disabled: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "controls/point-picker/picker",
        Level::Controls,
        "Point picker",
        "Point picker",
        PickerArgs { pixels: false, disabled: false },
        show_picker
    )]
}

/// A Blur Center over the checkerboard, in percent or in the pixels of a 640 by 480 image.
fn show_picker(ui: &mut Ui, args: &mut PickerArgs) {
    super::slider::with_values(ui, "point-picker", [[50.0, 50.0]], |ui, state| {
        let picker = PointPicker::new("point-picker", "Blur Center", state[0], None).enabled(!args.disabled);
        let picker = if args.pixels { picker.range(0.0..=640.0, Unit::new("{n} px")) } else { picker };
        let edit = picker.show(ui);
        if let Some(value) = edit.event.as_ref().and_then(edit::EditEvent::value) {
            state[0] = *value;
        }
    });
}
