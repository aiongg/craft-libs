//! [`AngleDial`]: a round dial whose needle points in a direction, with its degree field beside
//! it.

use std::ops::RangeInclusive;

use craft_theme::ControlSize;
use egui::accesskit::{Action, ActionData, Role};
use egui::{
    AsIdSalt, Color32, CornerRadius, Id, IdSalt, Key, Layout, Modifiers, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, UiBuilder,
    Vec2, vec2,
};

use super::number_field::{self, FieldRule};
use super::slider::{KEY_FILTER, PressTurn, Scale, press_turn, tap};
use super::{NumberField, SIZE_NAMES, size_arg};
use crate::behavior::FieldTab;
use crate::edit::ops::{self, Bound, ScalarOp};
use crate::edit::{self, EditPolicy, EditResponse, Gesture, parts};
use crate::localized::{LocaleNumbers, Unit};
use crate::primitives::{FocusRing, Gap, Interactive, Row};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// Degrees of a quarter turn's half: the step Page Up and Page Down take, and the grid Shift
/// snaps the pointer to.
const OCTANT: f64 = 45.0;

/// An operation of an angle dial's parts, on its value (`None`: mixed).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DialOp {
    /// The dial's ([`apply`]).
    Dial(ScalarOp),
    /// The degree field's ([`number_field::apply`]).
    Field(ScalarOp),
}

/// Where the dial's zero is and which way the value grows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AngleOrientation {
    /// 0 at 3 o'clock, counter-clockwise, as a mathematician's angle with y up (PhotoCraft,
    /// VectorCraft, DesignCraft).
    #[default]
    CounterClockwise,
    /// 0 at 12 o'clock, clockwise (EffectCraft).
    Clockwise,
}

impl AngleOrientation {
    /// The screen direction (y down) of the needle at `degrees`.
    fn direction(self, degrees: f64) -> Vec2 {
        let (sin, cos) = degrees.to_radians().sin_cos();
        match self {
            AngleOrientation::CounterClockwise => vec2(cos as f32, -sin as f32),
            AngleOrientation::Clockwise => vec2(sin as f32, -cos as f32),
        }
    }

    /// The angle in degrees, in `(-180, 180]`, of the screen offset `d` (y down) from the centre.
    fn angle_of(self, d: Vec2) -> f64 {
        let (x, up) = (f64::from(d.x), f64::from(-d.y));
        match self {
            AngleOrientation::CounterClockwise => up.atan2(x).to_degrees(),
            AngleOrientation::Clockwise => x.atan2(up).to_degrees(),
        }
    }
}

/// An angle dial's sizes and colours, from [`AngleDial::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AngleDialStyle {
    /// Side of the dial's box: `angle_dial.size`.
    pub size: f32,
    /// From the box to the face's edge: `angle_dial.margin`.
    pub margin: f32,
    /// The face: `angle_dial.face`.
    pub face: Color32,
    /// The face's edge, one point inside it: `angle_dial.edge`.
    pub edge: Stroke,
    /// The edge's colour while the pointer is over the dial or drags it: `text.normal`.
    pub edge_active: Color32,
    /// The needle: `angle_dial.needle_width` in `angle_dial.needle`.
    pub needle: Stroke,
    /// From the face's edge to the needle's tip: `angle_dial.needle_inset`.
    pub needle_inset: f32,
    /// Radius of the dot at the centre: `angle_dial.hub`, in `angle_dial.hub_fill`.
    pub hub: f32,
    pub hub_fill: Color32,
    /// Radius round the centre within which the pointer changes nothing: `angle_dial.dead_zone`.
    pub dead_zone: f32,
    /// Between the dial and its field: `gap.inline`.
    pub gap: f32,
}

/// An angle in degrees: a dial that points the needle where the pointer is, and a number field
/// with the degrees beside it. Both edit the host's `value` through one [`EditResponse`]; a
/// dial drag, a field scrub or a typing run is one gesture.
///
/// ```
/// use craft_ui::controls::AngleDial;
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| { let mut angle = 120.0;
/// let numbers = LocaleNumbers::from_tag("en");
/// let edit = AngleDial::new("light-angle", "Angle", angle, &numbers).range(-180.0..=180.0).show(ui);
/// if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
///     angle = *value;
/// }
/// # });
/// ```
///
/// - **Pointer**: a press points the needle at the pointer in the press frame and a drag follows
///   it, in whole steps; Shift snaps to 45 degrees. Within the dead zone at the centre the pointer
///   changes nothing. The value stays in the range: a press past an end of a narrow range goes to
///   that end, and a range wider than a turn takes the angle nearest the current value.
/// - **Keys** on the focused dial: Right and Up raise the value by a step, Left and Down lower it,
///   Shift takes ten steps, Page Up and Page Down 45 degrees. A range of exactly one turn (the
///   default, -180 to 180) wraps round; any other range stops at its ends. Keys change nothing
///   during a drag. There is no wheel and no Home or End: the ends of a turn point the same way.
///   Esc cancels a drag.
/// - **Tab**: the dial and the field are a Tab stop each. In a [`FieldTab`] scope Tab visits the
///   field only, and Ctrl+Tab reaches the dial.
/// - **AccessKit**: the dial is a `Slider` named `label` with its value, bounds, step and the
///   text "120°", and the actions increment, decrement and set value; the field is named
///   `label` too.
/// - The dial keeps its geometry right to left ([`Unmirrored::Graph`](crate::direction::Unmirrored)):
///   the angle is the document's. The row order follows the reading direction.
///
/// - **Mixed**: a `value` that isn't a finite number (a mixed selection) draws no needle and
///   leaves the field blank. A press on the dial focuses it and begins no drag; a click sets the
///   angle it points at (taking the nearest of several in a range wider than a turn from the range's start), and a number typed in the field commits.
///   The accessible node has no value, as a mixed number field's.
///
/// Drawing it with [`field(false)`](AngleDial::field) leaves the dial alone.
#[derive(Clone, Debug)]
#[must_use = "show the dial with `show`"]
pub struct AngleDial<'a> {
    id_salt: IdSalt,
    label: String,
    value: f64,
    numbers: &'a LocaleNumbers,
    range: RangeInclusive<f64>,
    step: f64,
    orientation: AngleOrientation,
    policy: EditPolicy,
    size: ControlSize,
    enabled: bool,
    field: bool,
}

impl<'a> AngleDial<'a> {
    /// A dial named `label` (its accessible name, and the field's) showing the host's `value` in
    /// degrees; `numbers` writes the field and the accessible value. `id_salt` keeps its
    /// gesture and focus when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: f64, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            numbers,
            range: -180.0..=180.0,
            step: 1.0,
            orientation: AngleOrientation::default(),
            policy: EditPolicy::default(),
            size: ControlSize::Md,
            enabled: true,
            field: true,
        }
    }

    /// The values the angle may take; the default is one turn, -180 to 180. Its bounds must be
    /// finite.
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.range = range;
        self
    }

    /// The step values snap to, and one arrow key's change (default 1 degree).
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    pub fn orientation(mut self, orientation: AngleOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// How the gesture reports ([`EditPolicy`]); the default reports every change at once.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The size of the field; the dial's diameter is `angle_dial.size` at every [`ControlSize`].
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether the degree field is drawn beside the dial (default); `false` draws the dial alone.
    pub fn field(mut self, field: bool) -> Self {
        self.field = field;
        self
    }

    pub fn style(tokens: &UiTokens) -> AngleDialStyle {
        let d = &tokens.component.angle_dial;
        AngleDialStyle {
            size: d.size,
            margin: d.margin,
            face: d.face.to_egui(),
            edge: Stroke::new(craft_theme::Stroke::W1.width(1.0), d.edge.to_egui()),
            edge_active: tokens.color.text.normal.to_egui(),
            needle: Stroke::new(d.needle_width, d.needle.to_egui()),
            needle_inset: d.needle_inset,
            hub: d.hub,
            hub_fill: d.hub_fill.to_egui(),
            dead_zone: d.dead_zone,
            gap: Gap::Inline.points(tokens),
        }
    }

    /// Shows the dial and its field and returns this frame's edit. The response is the dial's.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        let id = ui.make_persistent_id(self.id_salt);
        let inner = ui.scope_builder(UiBuilder::new().scope_id(id), |ui| self.show_inside(ui, id));
        inner.inner
    }

    fn show_inside(self, ui: &mut Ui, id: Id) -> EditResponse<f64> {
        let tokens = Tokens::of(ui);
        let style = AngleDial::style(&tokens);
        let enabled = self.enabled && ui.is_enabled();
        let mut state = ui.data(|d| d.get_temp::<parts::State<DialOp>>(id)).unwrap_or_default();
        // Esc is the parts': each cancels its own gesture, which cancels this one.
        // `None` is a mixed value (the host's is not a finite number): a gesture needs a host
        // value equal to itself, which a NaN is not.
        let host = self.value.is_finite().then_some(self.value);
        let outer = Gesture::load(ui, id, &host, self.policy.cancellable(false)).enabled(enabled);
        let policy = EditPolicy::default().cancellable(self.policy.cancellable).cost(self.policy.cost);
        let mut edits = parts::Edits::<Option<f64>, DialOp>::new(id, &state, &outer, edit::in_scope(ui));
        let (scale, rule) = (self.scale(), self.rule());
        let apply_op = |op: &DialOp, now: &Option<f64>| {
            Some(Some(match op {
                DialOp::Dial(op) => apply(op, *now, &scale),
                DialOp::Field(op) => number_field::apply(op, now.unwrap_or_else(|| rule.blank()), &rule),
            }))
        };
        edits.replay(ui.ctx(), apply_op);

        let dial = Row::new(Gap::Inline)
            .show(ui, |ui| {
                let (dial, dial_ops) = self.dial(ui, &style, *edits.value(), policy, enabled);
                edits.push(&dial, dial_ops.into_iter().map(DialOp::Dial), apply_op, |v, _| Some(v));
                if self.field {
                    let field = self.degrees(ui, edits.value().unwrap_or(f64::NAN), policy, enabled);
                    let field_ops = ops::take::<ScalarOp>(ui.ctx(), field.response.id);
                    edits.typing(&field, field_ops.into_iter().map(DialOp::Field), apply_op, |v, _| Some(v));
                }
                dial
            })
            .inner;
        let outer = edits.feed(outer, &mut state);
        ui.data_mut(|d| d.insert_temp(id, state));
        // The host's own value stands for a mixed one in the events that carry none.
        let event = outer.finish().map(|event| event.map(|value| value.unwrap_or(self.value)));
        EditResponse::new(dial.response, event)
    }

    fn scale(&self) -> Scale {
        Scale::new(&self.range, Some(self.step))
    }

    /// How the degree field takes its operations.
    fn rule(&self) -> FieldRule {
        FieldRule::new(&self.range, self.step, decimals_of(self.step))
    }

    /// The degree field, `°` inside, as wide as the range's largest value needs.
    fn degrees(&self, ui: &mut Ui, shown: f64, policy: EditPolicy, enabled: bool) -> EditResponse<f64> {
        let unit = Unit::new("{n}°");
        let places = decimals_of(self.step);
        let width = NumberField::fit_width(ui, self.size, self.numbers, self.range.clone(), places, &unit);
        ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down_justified(egui::Align::Min), |ui| {
            NumberField::new("field", &self.label, shown, self.numbers)
                .range(self.range.clone())
                .step(self.step)
                .decimals(0..=places)
                .unit(unit)
                .policy(policy)
                .part(true)
                .size(self.size)
                .enabled(enabled)
                .show(ui)
        })
        .inner
    }

    /// The dial, showing `value` (`None`: mixed): its interaction, look and AccessKit node, and
    /// the operations its one-shot inputs made.
    fn dial(&self, ui: &mut Ui, style: &AngleDialStyle, value: Option<f64>, policy: EditPolicy, enabled: bool) -> (EditResponse<f64>, Vec<ScalarOp>) {
        let (shown, mixed) = (value.unwrap_or(f64::NAN), value.is_none());
        let scale = self.scale();
        let sensed = Interactive::new("dial", Role::Slider, self.label.as_str())
            .sense(Sense::drag())
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact(ui, Vec2::splat(style.size));
        let response = sensed.response().clone();
        let (rect, dragging) = (response.rect, response.dragged());
        let pointing = Pointing { scale, orientation: self.orientation, dead_zone: style.dead_zone, center: rect.center() };

        let mut gesture = Gesture::load(ui, response.id, &shown, policy).event_filter(KEY_FILTER).enabled(enabled);
        let shift = || ui.input(|i| i.modifiers.shift);
        let tapped = tap(ui, &response);
        let pressed = response.drag_started_by(PointerButton::Primary).then(|| response.interact_pointer_pos()).flatten();
        let turn = press_turn(ui, response.id, true, gesture.id(), pressed.or(tapped));
        // As a slider's, a press begins no gesture while the value is mixed; a click still sets one.
        let begins = !mixed;
        if let PressTurn::Held(held) = turn
            && begins
        {
            gesture.begin();
            if let Some(start) = gesture.start().copied()
                && let Some(value) = pointing.at(held, start, shift())
            {
                gesture.update(value, ui.input(|i| i.modifiers));
            }
            if !response.dragged_by(PointerButton::Primary) {
                gesture.commit();
            }
        }
        let waits = matches!(turn, PressTurn::Waits);
        if response.drag_started_by(PointerButton::Primary) {
            response.request_focus();
            if begins && !waits {
                gesture.begin();
            }
        }
        if response.dragged_by(PointerButton::Primary)
            && gesture.is_active()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(value) = pointing.at(pos, *gesture.shown(), shift())
        {
            gesture.update(value, ui.input(|i| i.modifiers));
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        if tapped.is_some() {
            response.request_focus();
        }
        // The one-shot inputs, each an operation applied to the value the ones before it left.
        let mut ops = Vec::new();
        let mut set = |gesture: &mut Gesture<f64>, op: ScalarOp| {
            let now = *gesture.shown();
            gesture.set(apply(&op, now.is_finite().then_some(now), &scale));
            ops.push(op);
        };
        if let Some(pos) = tapped.filter(|_| !waits)
            && let Some(value) = pointing.at(pos, scale.place(*gesture.shown()), shift())
        {
            set(&mut gesture, ScalarOp::Set(value));
        }
        if let Some(op) = keys(ui, &response, dragging, &scale) {
            set(&mut gesture, op);
        }

        let value = scale.place(*gesture.shown());
        let response = if mixed {
            sensed.numeric_mixed(scale.min..=scale.max, self.step).finish(ui)
        } else {
            let text = format!("{}°", self.numbers.format(value, 0..=decimals_of(self.step)));
            sensed.numeric(value, scale.min..=scale.max, self.step).value(text).finish(ui)
        };
        if ui.is_rect_visible(rect) {
            let active = enabled && (response.hovered() || dragging);
            self.paint(ui, style, rect, (!mixed).then_some(value), active, enabled);
        }
        let face = rect.shrink(style.margin);
        FocusRing::paint_radius(ui, face, CornerRadius::same((face.width() / 2.0).round() as u8), &response);
        (EditResponse::new(response, gesture.finish()), ops)
    }

    fn paint(&self, ui: &Ui, style: &AngleDialStyle, rect: Rect, value: Option<f64>, active: bool, enabled: bool) {
        let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.5) };
        let painter = ui.painter();
        let (center, radius) = (rect.center(), rect.width() / 2.0 - style.margin);
        painter.add(Shape::circle_filled(center, radius, style.face));
        let edge = Stroke::new(style.edge.width, if active { style.edge_active } else { style.edge.color });
        painter.add(Shape::circle_stroke(center, radius - edge.width / 2.0, Stroke::new(edge.width, fade(edge.color))));
        if let Some(value) = value {
            let tip = center + self.orientation.direction(value) * (radius - style.needle_inset).max(0.0);
            painter.add(Shape::line_segment([center, tip], Stroke::new(style.needle.width, fade(style.needle.color))));
        }
        painter.add(Shape::circle_filled(center, style.hub, fade(style.hub_fill)));
    }
}

/// The decimals a step needs: none for whole degrees, up to three.
fn decimals_of(step: f64) -> usize {
    (0..3).find(|d| (step * 10f64.powi(*d)).fract().abs() < 1e-9).map_or(3, |d| d as usize)
}

/// Where the pointer points the dial.
struct Pointing {
    scale: Scale,
    orientation: AngleOrientation,
    dead_zone: f32,
    center: Pos2,
}

impl Pointing {
    /// The value for the pointer at `pos`, on the step grid and in the range, or `None` within the
    /// dead zone. `current` picks among the angles a range wider than a turn holds for the
    /// direction; `snap` rounds the direction to 45 degrees first.
    fn at(&self, pos: Pos2, current: f64, snap: bool) -> Option<f64> {
        let d = pos - self.center;
        if d.length() <= self.dead_zone {
            return None;
        }
        let mut angle = self.orientation.angle_of(d);
        if snap {
            angle = (angle / OCTANT).round() * OCTANT;
        }
        let (min, max) = (self.scale.min, self.scale.max);
        let nearest = (-2..=2)
            .map(|turns| angle + 360.0 * f64::from(turns))
            .filter(|v| (min..=max).contains(v))
            .min_by(|a, b| (a - current).abs().total_cmp(&(b - current).abs()));
        Some(self.scale.snap(nearest.unwrap_or_else(|| angle.clamp(min, max))))
    }
}

/// The operation this frame's keys and screen-reader requests make on the focused dial, or
/// `None`. The keys are taken during a drag too, which they don't change. A screen reader's value
/// wins over the steps of the same frame.
fn keys(ui: &Ui, response: &Response, dragging: bool, scale: &Scale) -> Option<ScalarOp> {
    let enabled = response.enabled();
    let focused = enabled && ui.memory(|m| m.has_focus(response.id));
    let step = scale.key_step();
    let mut delta = 0.0;
    let mut to = None;
    if focused {
        ui.input_mut(|i| {
            let mut count = |modifiers, key| i.count_and_consume_key(modifiers, key) as f64;
            for (key, sign) in [(Key::ArrowRight, 1.0), (Key::ArrowUp, 1.0), (Key::ArrowLeft, -1.0), (Key::ArrowDown, -1.0)] {
                // Shift first: `Modifiers::NONE` also matches a press with Shift held.
                delta += sign * step * (10.0 * count(Modifiers::SHIFT, key) + count(Modifiers::NONE, key));
            }
            delta += OCTANT * (count(Modifiers::NONE, Key::PageUp) - count(Modifiers::NONE, Key::PageDown));
        });
    }
    if enabled {
        ui.input(|i| {
            let requests = |action| i.num_accesskit_action_requests(response.id, action) as f64;
            delta += step * (requests(Action::Increment) - requests(Action::Decrement));
            let set = i.accesskit_action_requests(response.id, Action::SetValue).filter_map(|r| match r.data {
                Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                _ => None,
            });
            if let Some(v) = set.last() {
                to = Some(v);
            }
        });
    }
    if dragging || (delta == 0.0 && to.is_none()) {
        return None;
    }
    Some(to.map_or(ScalarOp::By(delta), ScalarOp::Set))
}

/// The value `op` gives a dial at `value` (`None`: mixed, which steps from the range's start), on
/// the step grid and in the range. A step wraps round a range of exactly one turn ([`wrapped`]);
/// [`Presses`](ScalarOp::Presses) count as arrow keys (Shift: ten steps).
pub(crate) fn apply(op: &ScalarOp, value: Option<f64>, scale: &Scale) -> f64 {
    let by = |delta| wrapped(scale, scale.moved(scale.place(value.unwrap_or(f64::NAN)), delta));
    let target = match op {
        ScalarOp::By(delta) => by(*delta),
        ScalarOp::Presses(presses) => by(scale.key_step() * presses.iter().map(|(m, n)| if m.shift { 10.0 * n } else { *n }).sum::<f64>()),
        ScalarOp::Set(v) => *v,
        ScalarOp::ToBound(Bound::Low) => scale.min,
        ScalarOp::ToBound(Bound::High) => scale.max,
    };
    scale.snap(target)
}

/// `value` brought back into a range of exactly one turn from the end it passed; any other range
/// leaves it for the scale to clamp.
fn wrapped(scale: &Scale, value: f64) -> f64 {
    let span = scale.max - scale.min;
    if (span - 360.0).abs() > 1e-9 {
        return value;
    }
    scale.min + (value - scale.min).rem_euclid(span)
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Angle dial"];

const RANGES: &[&str] = &["-180 to 180", "-360 to 360", "0 to 90"];
const ORIENTATIONS: &[&str] = &["counter-clockwise", "clockwise"];

fn range_of(name: &str) -> RangeInclusive<f64> {
    match name {
        "-360 to 360" => -360.0..=360.0,
        "0 to 90" => 0.0..=90.0,
        _ => -180.0..=180.0,
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "controls/angle-dial/dial",
            level: Level::Controls,
            component: "Angle dial",
            name: "Angle dial",
            args: || {
                Args::new()
                    .with_choice("range", RANGES, 0)
                    .with_choice("orientation", ORIENTATIONS, 0)
                    .with_bool("field", true)
                    .with_choice("size", SIZE_NAMES, 1)
                    .with_bool("disabled", false)
            },
            show: show_dial,
        },
        Story {
            id: "controls/angle-dial/dialog",
            level: Level::Controls,
            component: "Angle dial",
            name: "Dialog (field scope)",
            args: Args::new,
            show: show_dialog,
        },
    ]
}

fn show_dial(ui: &mut Ui, args: &mut Args) {
    let numbers = LocaleNumbers::from_tag("en");
    super::slider::with_values(ui, "angle-dial", [120.0], |ui, state| {
        let clockwise = args.choice("orientation") == ORIENTATIONS[1];
        let dial = AngleDial::new("angle-dial", "Angle", state[0], &numbers)
            .range(range_of(args.choice("range")))
            .orientation(if clockwise { AngleOrientation::Clockwise } else { AngleOrientation::CounterClockwise })
            .field(args.bool("field"))
            .size(size_arg(args.choice("size")))
            .enabled(!args.bool("disabled"))
            .show(ui);
        super::slider::store(&dial, &mut state[0]);
        let last = dial.event.as_ref().map(edit::describe);
        let key = ui.scope_id().with("log");
        let log = ui.data_mut(|d| {
            let slot = d.get_temp_mut_or_insert_with(key, String::new);
            if let Some(last) = last {
                *slot = last;
            }
            slot.clone()
        });
        ui.add(crate::primitives::Text::new(format!("Value: {}°, last event: {log}", state[0])).content(crate::primitives::Content::Muted));
    });
}

/// A dial between two text fields in a dialog's [`FieldTab`] scope: Tab goes from field to field
/// and passes the dial; Ctrl+Tab reaches it.
fn show_dialog(ui: &mut Ui, _: &mut Args) {
    use crate::controls::TextField;
    use crate::primitives::Stack;
    let numbers = LocaleNumbers::from_tag("en");
    super::slider::with_values(ui, "angle-dial-dialog", [(String::from("Drop shadow"), 120.0, String::from("Soft"))], |ui, state| {
        let (name, angle, note) = &mut state[0];
        FieldTab::new("angle-dial-dialog").show(ui, |ui| {
            Stack::new(Gap::Stack).show(ui, |ui| {
                if let Some(text) = TextField::new("angle-dial-dialog-name", "Name", name).show(ui).event.and_then(|e| e.value().cloned()) {
                    *name = text;
                }
                let dial = AngleDial::new("angle-dial-dialog-angle", "Angle", *angle, &numbers).show(ui);
                super::slider::store(&dial, angle);
                if let Some(text) = TextField::new("angle-dial-dialog-note", "Note", note).show(ui).event.and_then(|e| e.value().cloned()) {
                    *note = text;
                }
            });
        });
    });
}
