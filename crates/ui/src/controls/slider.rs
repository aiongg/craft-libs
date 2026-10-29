//! [`Slider`]: a value on a horizontal track; and the scale, track, painting and keyboard parts it
//! shares with [`RangeSlider`](super::RangeSlider).

use std::collections::VecDeque;
use std::fmt::Debug;
use std::ops::RangeInclusive;

use craft_theme::{ControlSize, Srgba, States, TypeRole};
use egui::accesskit::{Action, ActionData, Role};
use egui::emath::GuiRounding as _;
use egui::{
    AsIdSalt, Color32, Event, EventFilter, Id, IdSalt, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};

use super::{RangeSlider, SIZE_NAMES, Swatch, field_wheel, gradient_track, size_arg};
use crate::composites::Form;
use crate::edit::ops::{self, Bound, ScalarOp};
use crate::edit::order::{self, Turn};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, EditResponse, Gesture, GestureHost, GestureId, UndoMarks};
use crate::primitives::{Content, Gap, Interactive, Stack, StateLayer, Surface, SurfaceEdge, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Args, Level, Story, variant_list};
use crate::theme::{StrokeExt as _, ToEgui, Tokens, TokensOf, UiTokens, corner_radius, scale};

/// A slider's sizes and colours, from [`Slider::style`]; [`RangeSlider`](super::RangeSlider)
/// draws with the same.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliderStyle {
    /// Height of the control: `slider.height`.
    pub height: f32,
    /// Default length: `slider.width`.
    pub width: f32,
    /// Height of a thumb, its diameter while round: `slider.thumb`.
    pub thumb: f32,
    /// Width of a thumb: `slider.thumb_width`.
    pub thumb_width: f32,
    /// Thickness of the track: `slider.track_height`.
    pub track_height: f32,
    /// Thickness of a gradient track: `slider.gradient_height`.
    pub gradient_height: f32,
    /// The track: flat, with no outline, filled with `slider.track`, with `slider.track_radius`
    /// corners clamped to half its height.
    pub track: SurfaceStyle,
    /// The filled part, from the start to the value: `slider.fill`.
    pub fill: States<Color32>,
    /// A thumb's shape: a knob [`Surface`] (its outline, or bevel, and the knob shadow) with
    /// `slider.radius` corners clamped to half its shorter side. Its fill and border colour change with
    /// the state; a disabled thumb has no shadow.
    pub thumb_surface: SurfaceStyle,
    /// `slider.thumb.fill`.
    pub thumb_fill: States<Color32>,
    /// `slider.thumb.border`; a bevel keeps its own colours.
    pub thumb_border: States<Color32>,
    /// The ring round a thumb, clear of its edge: `slider.ring`.
    pub ring: States<Color32>,
    /// `slider.ring_width`.
    pub ring_width: f32,
    /// From a thumb's edge to the ring's centre line: `slider.ring_offset`.
    pub ring_offset: f32,
    /// Height of the marks and ticks under the track: `slider.mark`.
    pub mark: f32,
    /// How near a mark a drag snaps the thumb to it, in points: `slider.snap`.
    pub snap: f32,
    /// The colour of marks and ticks: `line.mark`.
    pub mark_color: Color32,
}

/// How a slider's track maps positions to values.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SliderScale {
    /// Value and position are proportional.
    #[default]
    Linear,
    /// `value = start + (end - start) * position^k`; `k > 1` spends more track on small values.
    /// `k` must be a positive number.
    Power(f64),
    /// `value = start * (end / start)^position`; the range must be positive.
    Log,
}

/// The shape of a slider's thumb.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SliderThumb {
    /// A knob on the track.
    #[default]
    Round,
    /// A pointer under the track, as the Color panel draws it.
    Triangle,
}

/// A value in a range, set by dragging along a track:
///
/// ```
/// use craft_ui::controls::Slider;
/// use craft_ui::edit::EditEvent;
/// # egui::__run_test_ui(|ui| { let mut opacity = 80.0;
/// let edit = Slider::new("opacity", "Opacity", opacity, 0.0..=100.0).step(1.0).show(ui);
/// match edit.event {
///     Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. }) => opacity = value,
///     // Begin, Commit and Cancel go to the document's undo history (`edit::GestureHost`).
///     _ => {}
/// }
/// # });
/// ```
///
/// The host owns the value and reports every change back through the edit lifecycle
/// ([`crate::edit`]): a drag is one gesture (Begin on press, Updates while moving, Commit on
/// release, Cancel on Esc), and a key, a wheel step or a screen reader's request is one
/// [`Set`](crate::edit::EditEvent::Set). The slider draws the gesture's value, never a copy of
/// its own, and shows no value text: put a number field beside it.
///
/// - Pointer (primary button): a press on the track moves the thumb there; a press on the thumb
///   moves it by the pointer's movement, so the value doesn't jump. A press, or a tap (press and
///   release in one frame, a [`Set`](crate::edit::EditEvent::Set)), gives the slider keyboard
///   focus.
/// - Keys, while focused: Right and Up add the step (Shift: ten steps), Left and Down take it
///   away, Page Up and Page Down move a tenth of the range, Home and End go to its ends. The
///   step is [`step`](Slider::step), or a hundredth of the range.
/// - The wheel steps the value while the pointer is over the focused slider, and the panel under
///   it doesn't scroll; over a slider without focus the wheel scrolls the panel, unless
///   [`wheel_unfocused`](Slider::wheel_unfocused) is on.
/// - With a [`default_value`](Slider::default_value), a double-click on the track or the thumb
///   sets it, and so does Ctrl/Cmd+Backspace while focused. A press on the thumb never moves it.
/// - AccessKit: a `Slider` named `label` with its value, bounds and step, which takes increment,
///   decrement and set-value requests.
/// - While a drag is under way, keys, the wheel and screen-reader requests change nothing.
/// - A slider keeps its geometry right to left ([`Unmirrored::SliderValue`](crate::direction::Unmirrored::SliderValue)):
///   the value grows to the right, and the arrow keys act as they are.
///
/// With [`track_colors`](Slider::track_colors) the track shows the colours the value runs
/// through (a colour channel's gradient) and no fill.
///
/// With a step, values snap to its multiples from the range's start, and the range's end is a
/// stop too when the step doesn't divide the range. A value outside the range is drawn at the
/// nearer end; a value that isn't a finite number is drawn at the start and reported to AccessKit
/// without a value (mixed), and a drag begins no
/// gesture until the host gives a finite value.
#[derive(Clone, Debug)]
#[must_use = "show the slider with `show`"]
pub struct Slider {
    id_salt: IdSalt,
    label: String,
    value: f64,
    range: RangeInclusive<f64>,
    step: Option<f64>,
    policy: EditPolicy,
    size: ControlSize,
    enabled: bool,
    default: Option<f64>,
    wheel_unfocused: bool,
    track_colors: Option<Vec<Srgba>>,
    part: bool,
    width: Option<f32>,
    scale: SliderScale,
    marks: Vec<f64>,
    thumb: SliderThumb,
    ticks: usize,
}

impl Slider {
    /// A slider named `label` (its accessible name) showing the host's `value` in `range`;
    /// `id_salt` keeps its gesture, focus and animations when the layout around it changes.
    /// `value` and `range` may be `f32` or `f64`. A reversed range (`100.0..=0.0`) is read in ascending order. Its bounds must be finite: a
    /// bound that isn't is a debug assertion, and in release builds the range shrinks to the
    /// finite bound (or 0).
    pub fn new<V: Into<f64> + Copy>(id_salt: impl AsIdSalt, label: impl Into<String>, value: V, range: RangeInclusive<V>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value: value.into(),
            range: (*range.start()).into()..=(*range.end()).into(),
            step: None,
            policy: EditPolicy::default(),
            size: ControlSize::Md,
            enabled: true,
            default: None,
            wheel_unfocused: false,
            track_colors: None,
            part: false,
            width: None,
            scale: SliderScale::Linear,
            marks: Vec::new(),
            thumb: SliderThumb::Round,
            ticks: 0,
        }
    }

    /// The step values snap to, and one arrow key's change. Without one the value is continuous
    /// and a key moves a hundredth of the range.
    pub fn step(mut self, step: impl Into<f64>) -> Self {
        self.step = Some(step.into());
        self
    }

    /// How the gesture reports ([`EditPolicy`]); the default reports every change at once.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The value a double-click on the track or the thumb, or Ctrl/Cmd+Backspace on the focused
    /// slider, sets (snapped to the step and kept in the range; a value that isn't finite is
    /// ignored). The first click of the double-click has already moved the thumb, so the reset
    /// ends the same gesture or is its own [`Set`](crate::edit::EditEvent::Set). A screen reader
    /// resets through its set-value request as ever.
    pub fn default_value(mut self, value: impl Into<f64>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Whether a wheel gesture that starts over the slider steps it without the slider having
    /// the focus, as PhotoCraft's dialog sliders do (off by default: in a panel the wheel
    /// scrolls). The rules are the unfocused number field's: a gesture that started elsewhere
    /// keeps scrolling over the slider, Alt, Ctrl/Cmd and sideways scrolls stay with the view,
    /// Shift steps ten. A [`RangeSlider`](crate::controls::RangeSlider) has no such option: its
    /// two thumbs give a wheel over the track no one value to step.
    pub fn wheel_unfocused(mut self, on: bool) -> Self {
        self.wheel_unfocused = on;
        self
    }

    /// Whether the slider is a part of a composite reporting through one edit, whose press takes
    /// its turn from the edit order ([`press_turn`]).
    pub(crate) fn part(mut self, part: bool) -> Self {
        self.part = part;
        self
    }

    /// The slider's length in points instead of `slider.width`, for a slider that fills a row.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Draws the track as a gradient through `stops`, spread evenly from the range's start to its
    /// end, over the checkerboard where a stop is translucent, and without the filled part: the
    /// track of a colour channel (hue, a channel with the others fixed, opacity). The track keeps
    /// its shape and edge.
    pub fn track_colors(mut self, stops: impl IntoIterator<Item = impl Into<Srgba>>) -> Self {
        self.track_colors = Some(stops.into_iter().map(Into::into).collect());
        self
    }

    /// How the track maps positions to values (default [`SliderScale::Linear`]). Only the track
    /// maps through the curve: keys, the wheel and steps stay linear in the value, as in
    /// PhotoCraft. A log slider without a [`step`](Slider::step) moves an arrow key by a tenth of
    /// a decade of the value and Page by a tenth of the track. A scale that doesn't fit the range
    /// (a log scale over a range that isn't positive, a power that isn't a positive number) is a
    /// debug assertion and in release builds the track stays linear.
    pub fn scale(mut self, scale: SliderScale) -> Self {
        self.scale = scale;
        self
    }

    /// Values the thumb snaps to when a drag or a click brings it within `slider.snap` points,
    /// drawn as marks under the track. A mark outside the range is ignored. Keys, the wheel and
    /// screen readers use the step only, so with a step the marks should lie on it.
    pub fn marks(mut self, marks: &[f64]) -> Self {
        self.marks = marks.iter().copied().filter(|m| m.is_finite()).collect();
        self
    }

    /// The thumb's shape; [`SliderThumb::Triangle`] has the round thumb's fill, border and hit
    /// area, is `slider.thumb` wide and points up at the track from below.
    pub fn thumb(mut self, thumb: SliderThumb) -> Self {
        self.thumb = thumb;
        self
    }

    /// `count + 1` evenly spaced tick marks along the track, without snapping (none for 0).
    pub fn ticks(mut self, count: usize) -> Self {
        self.ticks = count;
        self
    }

    /// Shows the slider and returns this frame's edit event. The response covers the control.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        let tokens = Tokens::of(ui);
        let style = Slider::style(&tokens, self.size);
        let scale = Scale::new(&self.range, self.step).with_kind(self.scale);
        let sensed = Interactive::salted(self.id_salt, Role::Slider, self.label.as_str())
            .sense(Sense::drag())
            .enabled(self.enabled)
            .state_layer(false)
            .interact(ui, vec2(self.width.unwrap_or(style.width), style.height));
        let response = sensed.response().clone();
        let track = Track::new(response.rect, &style, scale);
        let track = if self.track_colors.is_some() { track.with_height(style.gradient_height) } else { track };
        let sensing = Sensing::of(&response);
        let dragged_to = |start: f64, origin: f32, x: f32| track.snap_to_marks(&self.marks, style.snap, x, track.dragged(start, origin, x));

        let mut gesture = Gesture::load(ui, response.id, &self.value, self.policy).event_filter(KEY_FILTER).enabled(sensing.enabled);
        let tapped = tap(ui, &response);
        let pressed = response.drag_started_by(PointerButton::Primary).then(|| pointer(ui, &response).map(|(_, origin)| origin)).flatten();
        let turn = press_turn(ui, response.id, self.part, gesture.id(), pressed.or(tapped));
        if let PressTurn::Held(at) = turn
            && self.value.is_finite()
        {
            gesture.begin();
            if let Some(anchor) = gesture.anchor().copied() {
                gesture.update(dragged_to(scale.place(anchor), at.x, at.x), ui.input(|i| i.modifiers));
            }
            if !response.dragged_by(PointerButton::Primary) {
                gesture.commit();
            }
        }
        let waits = matches!(turn, PressTurn::Waits);
        if response.drag_started_by(PointerButton::Primary) {
            response.request_focus();
            if self.value.is_finite() && !waits {
                gesture.begin();
            }
        }
        if response.dragged_by(PointerButton::Primary)
            && let Some(anchor) = gesture.anchor().copied()
            && let Some((pos, origin)) = pointer(ui, &response)
        {
            gesture.update(dragged_to(scale.place(anchor), origin.x, pos.x), ui.input(|i| i.modifiers));
        }
        // The one-shot inputs, each an operation applied to the value the ones before it left.
        let mut ops = Vec::new();
        let mut set = |gesture: &mut Gesture<f64>, op: ScalarOp| {
            gesture.set(apply(&op, scale.place(*gesture.shown()), (scale.min, scale.max), &scale));
            ops.push(op);
        };
        let reset = self.default.filter(|d| d.is_finite());
        // The double-click's first click has moved the thumb to the pointer; the second resets.
        let double = reset.filter(|_| sensing.enabled && sensing.hovered && ui.input(|i| i.pointer.button_double_clicked(PointerButton::Primary)));
        if let Some(value) = double {
            response.request_focus();
            set(&mut gesture, ScalarOp::Set(value));
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        if let Some(at) = tapped {
            response.request_focus();
            if double.is_none() && !waits {
                let shown = scale.place(*gesture.shown());
                set(&mut gesture, ScalarOp::Set(dragged_to(shown, at.x, at.x)));
            }
        }
        // Taken during a drag too, which it doesn't change, as the arrows are.
        if let Some(value) = reset
            && sensing.enabled
            && ui.memory(|m| m.has_focus(response.id))
            && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Backspace))
            && !sensing.dragging
        {
            set(&mut gesture, ScalarOp::Set(value));
        }
        let wheel_over = self.wheel_unfocused.then_some(response.rect);
        if let Some(op) = stepped(ui, response.id, sensing, wheel_over, &scale) {
            set(&mut gesture, op);
        }
        if self.part && !ops.is_empty() {
            ops::report(ui.ctx(), response.id, ops);
        }

        let shown = scale.place(*gesture.shown());
        // A value that isn't finite is drawn at the start but reported as mixed, with no value.
        let response = if gesture.shown().is_finite() {
            sensed.numeric(shown, scale.min..=scale.max, scale.reported_step()).finish(ui)
        } else {
            sensed.numeric_mixed(scale.min..=scale.max, scale.reported_step()).finish(ui)
        };
        if ui.is_rect_visible(response.rect) {
            let color = |states: &States<Color32>| StateLayer::color(ui, states, &response);
            let x = track.x_of(shown);
            match &self.track_colors {
                Some(stops) => gradient_track::paint(ui, &track, stops, Swatch::style(&tokens, self.size).checker),
                None => {
                    paint_track(ui, &style, &track);
                    paint_fill(ui, &style, &track, track.ends().0, x, color(&style.fill));
                }
            }
            let paint =
                ThumbPaint { fill: color(&style.thumb_fill), border: color(&style.thumb_border), ring: color(&style.ring), enabled: sensing.enabled };
            paint_marks(ui, &style, &track, &self.marks, self.ticks);
            match self.thumb {
                SliderThumb::Round => _ = paint_thumb(ui, &style, pos2(x, track.y), paint),
                SliderThumb::Triangle => paint_pointer(ui, &style, &track, x, paint),
            }
        }
        EditResponse::new(response, gesture.finish())
    }

    pub fn style(tokens: &UiTokens, size: ControlSize) -> SliderStyle {
        let s = &tokens.component.slider;
        let (thumb, thumb_width) = (s.thumb.get(size), s.thumb_width.get(size));
        let track = SurfaceStyle {
            fill: s.track.to_egui(),
            radius: corner_radius(s.track_radius.min(s.track_height / 2.0)),
            edge: SurfaceEdge::None,
            shadow: None,
        };
        let mut thumb_surface = Surface::style(tokens, SurfaceKind::Knob);
        thumb_surface.radius = corner_radius(s.radius.min(thumb.min(thumb_width) / 2.0));
        let thumb_surface = knob(thumb_surface, s.thumb_colors.fill.rest.to_egui(), s.thumb_colors.border.rest.to_egui(), true);
        SliderStyle {
            height: s.height.get(size),
            width: s.width.get(size),
            thumb,
            thumb_width,
            track_height: s.track_height,
            gradient_height: s.gradient_height,
            track,
            fill: s.fill.to_egui(),
            thumb_surface,
            thumb_fill: s.thumb_colors.fill.to_egui(),
            thumb_border: s.thumb_colors.border.to_egui(),
            ring: s.ring.to_egui(),
            ring_width: s.ring_width,
            ring_offset: s.ring_offset,
            mark: s.mark.get(size),
            snap: s.snap,
            mark_color: tokens.color.line.mark.to_egui(),
        }
    }
}

/// The keys a focused thumb keeps: the arrows step its value rather than move the focus.
pub(crate) const KEY_FILTER: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false };

/// A range with an optional step: where values may go and how keys move them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Scale {
    pub min: f64,
    pub max: f64,
    step: Option<f64>,
    kind: SliderScale,
}

impl Scale {
    /// The range in ascending order; a step that isn't a positive number is none. A bound that
    /// isn't finite debug-asserts, and the range shrinks to the finite bound, or to 0.
    pub fn new(range: &RangeInclusive<f64>, step: Option<f64>) -> Self {
        let (a, b) = (*range.start(), *range.end());
        debug_assert!(a.is_finite() && b.is_finite(), "a slider's range needs finite bounds, not {a}..={b}");
        let (min, max) = bounds(a, b);
        Self { min, max, step: step.filter(|s| s.is_finite() && *s > 0.0), kind: SliderScale::Linear }
    }

    /// The same range read through `kind`; one that doesn't fit the range debug-asserts and
    /// stays linear.
    pub fn with_kind(mut self, kind: SliderScale) -> Self {
        let fits = match kind {
            SliderScale::Linear => true,
            SliderScale::Power(k) => k.is_finite() && k > 0.0,
            SliderScale::Log => self.min > 0.0 && self.max > self.min,
        };
        debug_assert!(fits, "a slider's {kind:?} scale doesn't fit {}..={}", self.min, self.max);
        self.kind = if fits { kind } else { SliderScale::Linear };
        self
    }

    fn linear(&self) -> bool {
        self.kind == SliderScale::Linear
    }

    fn span(&self) -> f64 {
        self.max - self.min
    }

    /// Where `value` lies from the start (0) to the end (1).
    fn fraction(&self, value: f64) -> f32 {
        self.position(value) as f32
    }

    /// Where `value` lies on the track, from the start (0) to the end (1).
    pub fn position(&self, value: f64) -> f64 {
        if self.span() <= 0.0 {
            return 0.0;
        }
        let linear = ((value - self.min) / self.span()).clamp(0.0, 1.0);
        match self.kind {
            SliderScale::Linear => linear,
            SliderScale::Power(k) => linear.powf(1.0 / k),
            SliderScale::Log => ((value.max(self.min) / self.min).ln() / (self.max / self.min).ln()).clamp(0.0, 1.0),
        }
    }

    /// The value at `position` on the track (0 to 1), unsnapped: the inverse of
    /// [`position`](Scale::position).
    pub fn at_position(&self, position: f64) -> f64 {
        let t = position.clamp(0.0, 1.0);
        match self.kind {
            SliderScale::Linear => self.min + t * self.span(),
            SliderScale::Power(k) => self.min + t.powf(k) * self.span(),
            SliderScale::Log => self.min * (self.max / self.min).powf(t),
        }
    }

    /// Where `value` is drawn: in the range, and at the start when it isn't a finite number.
    pub fn place(&self, value: f64) -> f64 {
        if value.is_finite() { value.clamp(self.min, self.max) } else { self.min }
    }

    /// `value` in the range on the nearest stop, when there is a step: a multiple of the step
    /// from the start, or the end. A value on a stop stays as it is.
    pub fn snap(&self, value: f64) -> f64 {
        let value = self.place(value);
        let Some(step) = self.step else { return value };
        let at = (value - self.min) / step;
        if value == self.max || on_stop(at) {
            return value;
        }
        let below = (self.min + at.floor() * step).min(self.max);
        let above = (below + step).min(self.max);
        if value - below < above - value { below } else { above }
    }

    /// `value` moved by `delta` towards the next stops: whole steps from the stop at or beyond
    /// `value` in the direction of the move, so a value between stops (or the range's end off the
    /// step grid) goes to the next stop first. Unsnapped and unclamped.
    pub fn moved(&self, value: f64, delta: f64) -> f64 {
        let Some(step) = self.step else {
            return if self.kind == SliderScale::Log { self.at_position(self.position(value) + delta) } else { value + delta };
        };
        let at = (value - self.min) / step;
        let steps = (delta / step).round();
        let from = if on_stop(at) {
            at.round()
        } else if steps > 0.0 {
            at.floor()
        } else {
            at.ceil()
        };
        self.min + (from + steps) * step
    }

    /// One arrow key's change: the step, or a hundredth of the range. On a log scale without a
    /// step it is a change of position (a tenth of a decade of the value), and
    /// [`moved`](Scale::moved) takes it so.
    pub fn key_step(&self) -> f64 {
        match (self.step, self.kind) {
            (Some(step), _) => step,
            (None, SliderScale::Linear | SliderScale::Power(_)) => self.span() / 100.0,
            (None, SliderScale::Log) => 0.1 / (self.max / self.min).log10(),
        }
    }

    /// The step AccessKit reports: a change of value.
    pub fn reported_step(&self) -> f64 {
        self.step.unwrap_or(self.span() / 100.0)
    }

    /// One Page key's change: a tenth of the range, in whole steps when there is a step; on a
    /// log scale without a step, a tenth of the track.
    pub fn page_step(&self) -> f64 {
        let page = self.span() / 10.0;
        match (self.step, self.kind) {
            (Some(step), _) => (page / step).round().max(1.0) * step,
            (None, SliderScale::Linear | SliderScale::Power(_)) => page,
            (None, SliderScale::Log) => 0.1,
        }
    }
}

/// The bounds `a..=b` in ascending order, falling back to the finite bound (or 0) for one that
/// isn't finite.
pub(super) fn bounds(a: f64, b: f64) -> (f64, f64) {
    match (a.is_finite(), b.is_finite()) {
        (true, true) => (a.min(b), a.max(b)),
        (true, false) => (a, a),
        (false, true) => (b, b),
        (false, false) => (0.0, 0.0),
    }
}

/// Whether a position `at` steps from the start is on a stop, allowing for rounding.
fn on_stop(at: f64) -> bool {
    (at - at.round()).abs() < 1e-9
}

/// Where values sit along a control's rect: a thumb's centre runs from half a thumb inside the
/// start to half a thumb inside the end, so the thumb stays inside the control, and the track
/// runs between the same two points.
#[derive(Clone, Copy, Debug)]
pub(super) struct Track {
    /// The thumb centre's x at the minimum and the maximum.
    x0: f32,
    x1: f32,
    /// The centre line.
    pub y: f32,
    /// Half a thumb: how far from its centre a press grabs a thumb.
    pub reach: f32,
    scale: Scale,
    track_height: f32,
}

impl Track {
    pub fn new(rect: Rect, style: &SliderStyle, scale: Scale) -> Self {
        let reach = style.thumb_width / 2.0;
        Self { x0: rect.left() + reach, x1: rect.right() - reach, y: rect.center().y, reach, scale, track_height: style.track_height }
    }

    /// This track with a bar `height` thick.
    pub fn with_height(mut self, height: f32) -> Self {
        self.track_height = height;
        self
    }

    /// A track on `rect` for thumbs of another look: a thumb centre runs from `inset` inside the
    /// start to `inset` inside the end, and a press within `reach` of it grabs the thumb. It has
    /// no bar.
    pub fn inset(rect: Rect, inset: f32, reach: f32, scale: Scale) -> Self {
        Self { x0: rect.left() + inset, x1: rect.right() - inset, y: rect.center().y, reach, scale, track_height: 0.0 }
    }

    /// The thumb centre's x for `value`, as [placed](Scale::place).
    pub fn x_of(&self, value: f64) -> f32 {
        egui::lerp(self.x0..=self.x1, self.scale.fraction(self.scale.place(value)))
    }

    /// The value whose thumb centre is at `x`, unsnapped.
    pub fn value_at(&self, x: f32) -> f64 {
        let t = if self.x1 > self.x0 { ((x - self.x0) / (self.x1 - self.x0)).clamp(0.0, 1.0) } else { 0.0 };
        self.scale.at_position(f64::from(t))
    }

    /// The change of value a pointer movement of `dx` points makes.
    pub fn value_per(&self, dx: f32) -> f64 {
        if self.x1 > self.x0 { f64::from(dx / (self.x1 - self.x0)) * self.scale.span() } else { 0.0 }
    }

    /// The value of the thumb at `start` moved by the pointer's movement of `dx` points,
    /// unsnapped: along the track, so on a power or log scale the change depends on where the
    /// thumb is.
    fn moved_by(&self, start: f64, dx: f32) -> f64 {
        if self.scale.linear() {
            return start + self.value_per(dx);
        }
        let len = f64::from(self.x1 - self.x0);
        if len > 0.0 { self.scale.at_position(self.scale.position(start) + f64::from(dx) / len) } else { start }
    }

    /// `value`, or the mark whose thumb position is within `snap` points of the pointer at `x`.
    pub fn snap_to_marks(&self, marks: &[f64], snap: f32, x: f32, value: f64) -> f64 {
        marks
            .iter()
            .copied()
            .filter(|m| (self.scale.min..=self.scale.max).contains(m))
            .map(|m| (m, (self.x_of(m) - x).abs()))
            .filter(|(_, d)| *d <= snap)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(value, |(m, _)| m)
    }

    /// Whether a press at `x` is on the thumb at `value`.
    pub fn on_thumb(&self, value: f64, x: f32) -> bool {
        (x - self.x_of(value)).abs() <= self.reach
    }

    /// The value of the thumb at `start` dragged from a press at `origin` to `x`, snapped. A
    /// press on the thumb moves the value by the pointer's movement, so the thumb doesn't jump
    /// and a still pointer leaves the value as it is; a press elsewhere puts the thumb under the
    /// pointer.
    pub fn dragged(&self, start: f64, origin: f32, x: f32) -> f64 {
        if !self.on_thumb(start, origin) {
            self.scale.snap(self.value_at(x))
        } else if x == origin {
            start
        } else {
            self.scale.snap(self.moved_by(start, x - origin))
        }
    }

    /// The thumb centre's x at the minimum and at the maximum.
    pub fn ends(&self) -> (f32, f32) {
        (self.x0, self.x1)
    }

    /// The track's rect, on the pixel grid.
    pub(super) fn bar(&self, pixels_per_point: f32) -> Rect {
        let half = self.track_height / 2.0;
        Rect::from_min_max(pos2(self.x0, self.y - half), pos2(self.x1, self.y + half)).round_to_pixels(pixels_per_point)
    }
}

/// The pointer now and where its button went down, in the coordinates of `response`'s layer,
/// while `response` is pressed or was released this frame.
pub(super) fn pointer(ui: &Ui, response: &Response) -> Option<(Pos2, Pos2)> {
    let pos = response.interact_pointer_pos()?;
    let origin = ui.input(|i| i.pointer.press_origin())?;
    Some((pos, to_layer(ui, response, origin)))
}

/// Where a tap on `response` fell this frame, in its layer's coordinates: a press and release of
/// the primary button in one frame over the enabled control. egui reports no drag for it.
pub(crate) fn tap(ui: &Ui, response: &Response) -> Option<Pos2> {
    if !response.enabled() || !response.hovered() {
        return None;
    }
    let pos = ui.input(|i| {
        let p = &i.pointer;
        let tapped = p.button_pressed(PointerButton::Primary) && p.button_released(PointerButton::Primary) && !p.button_down(PointerButton::Primary);
        p.interact_pos().filter(|_| tapped)
    })?;
    Some(to_layer(ui, response, pos))
}

/// When a part of a composite runs a primary press ([`press_turn`]).
#[derive(Clone, Copy, Debug)]
pub(crate) enum PressTurn {
    /// The control handles this pass's input as usual.
    Now,
    /// A press waits for a later pass: the control takes no pointer input in this one.
    Waits,
    /// The press held from an earlier pass, at this position in the control's layer, runs now,
    /// before this pass's input; a button already up ends it at once.
    Held(Pos2),
}

/// When control `id` runs a primary press at `press` (this pass's, in its layer's coordinates).
/// A `part` of a composite that reports its parts through one edit takes its turn from the edit
/// order (`edit::order`): while another control's typing run has the focus, the press asks it to
/// end and waits, and runs in the first pass after the run's ending event has gone out, against
/// the host's value then. `own` is the control's own gesture. A control that isn't a part runs
/// its press at once.
pub(crate) fn press_turn(ui: &Ui, id: Id, part: bool, own: Option<GestureId>, press: Option<Pos2>) -> PressTurn {
    if !part {
        return PressTurn::Now;
    }
    let key = id.with("craft_ui::held_press");
    let pass = ui.ctx().cumulative_pass_nr();
    // Last pass held, where.
    let held = ui.data_mut(|d| d.remove_temp::<(u64, Pos2)>(key)).filter(|(last, _)| last + 1 == pass).map(|(_, at)| at);
    let Some(at) = held.or(press) else { return PressTurn::Now };
    match order::turn(ui.ctx(), id, own) {
        Turn::Waits => {
            ui.data_mut(|d| d.insert_temp(key, (pass, at)));
            PressTurn::Waits
        }
        Turn::Now if held.is_none() => PressTurn::Now,
        Turn::Now | Turn::Runs => PressTurn::Held(at),
    }
}

/// A screen position in the coordinates of `response`'s layer.
fn to_layer(ui: &Ui, response: &Response, pos: Pos2) -> Pos2 {
    ui.ctx().layer_transform_from_global(response.layer_id).map_or(pos, |t| t * pos)
}

/// What a control's response says about this frame's input, for [`stepped`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sensing {
    pub enabled: bool,
    /// The pointer is over the control.
    pub hovered: bool,
    /// A drag of the control is under way.
    pub dragging: bool,
    /// The wheel steps the control; off for one the wheel never steps, so the panel scrolls.
    pub wheel: bool,
    /// The arrow keys, Page keys and the wheel move the thumb the way they point while its value
    /// falls to the right (a levels gamma thumb): Right lowers the value, and Home goes to the
    /// upper bound. Screen-reader requests act on the value as ever.
    pub falling: bool,
}

impl Sensing {
    pub fn of(response: &Response) -> Self {
        Self { enabled: response.enabled(), hovered: response.hovered(), dragging: response.dragged(), wheel: true, falling: false }
    }
}

/// The operation this frame's keys, wheel and screen-reader requests make on the thumb `id`;
/// `None` when nothing asked for a change. [`apply`] gives its value. Keys and the wheel act
/// while the thumb has focus, the wheel only while the pointer is over the control. While a drag
/// is under way they change nothing; keys and the wheel are still taken. With a `wheel_over`
/// rect, a thumb without focus takes the wheel gestures that start over that rect too
/// ([`field_wheel::notches`]). A move to an end or a screen reader's value wins over the steps of
/// the same frame.
pub(crate) fn stepped(ui: &Ui, id: Id, sensing: Sensing, wheel_over: Option<Rect>, scale: &Scale) -> Option<ScalarOp> {
    let (step, page) = (scale.key_step(), scale.page_step());
    let focused = sensing.enabled && ui.memory(|m| m.has_focus(id));
    let mut delta = if sensing.wheel { wheel(ui, id, step, page, focused && sensing.hovered, sensing.dragging) } else { 0.0 };
    if let Some(rect) = wheel_over
        && !focused
    {
        delta += step * field_wheel::notches(ui, id.with("craft_ui::slider::unfocused"), false, rect, sensing.enabled && !sensing.dragging, 10.0);
    }
    let mut to = None;
    let ends = if sensing.falling { (Bound::High, Bound::Low) } else { (Bound::Low, Bound::High) };
    if focused {
        ui.input_mut(|i| {
            let mut count = |modifiers, key| i.count_and_consume_key(modifiers, key) as f64;
            for (key, sign) in [(Key::ArrowRight, 1.0), (Key::ArrowUp, 1.0), (Key::ArrowLeft, -1.0), (Key::ArrowDown, -1.0)] {
                // Shift first: `Modifiers::NONE` also matches a press with Shift held.
                delta += sign * step * (10.0 * count(Modifiers::SHIFT, key) + count(Modifiers::NONE, key));
            }
            delta += page * (count(Modifiers::NONE, Key::PageUp) - count(Modifiers::NONE, Key::PageDown));
            if count(Modifiers::NONE, Key::Home) > 0.0 {
                to = Some(ScalarOp::ToBound(ends.0));
            }
            if count(Modifiers::NONE, Key::End) > 0.0 {
                to = Some(ScalarOp::ToBound(ends.1));
            }
        });
    }
    if sensing.falling {
        delta = -delta;
    }
    if sensing.enabled {
        ui.input(|i| {
            let requests = |action| i.num_accesskit_action_requests(id, action) as f64;
            delta += step * (requests(Action::Increment) - requests(Action::Decrement));
            let set = i.accesskit_action_requests(id, Action::SetValue).filter_map(|r| match r.data {
                Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                _ => None,
            });
            if let Some(v) = set.last() {
                to = Some(ScalarOp::Set(v));
            }
        });
    }
    if sensing.dragging || (delta == 0.0 && to.is_none()) {
        return None;
    }
    Some(to.unwrap_or(ScalarOp::By(delta)))
}

/// The value `op` gives a thumb at `value` with `bounds` (the range, or what its neighbours leave
/// it): on the scale's stops and within `bounds`. A step goes on from `value` kept within
/// `bounds`; [`Presses`](ScalarOp::Presses) count as arrow keys (Shift: ten steps).
pub(crate) fn apply(op: &ScalarOp, value: f64, bounds: (f64, f64), scale: &Scale) -> f64 {
    let by = |delta| scale.moved(value.clamp(bounds.0, bounds.1), delta);
    let target = match op {
        ScalarOp::By(delta) => by(*delta),
        ScalarOp::Presses(presses) => by(scale.key_step() * presses.iter().map(|(m, n)| if m.shift { 10.0 * n } else { *n }).sum::<f64>()),
        ScalarOp::Set(v) => *v,
        ScalarOp::ToBound(Bound::Low) => bounds.0,
        ScalarOp::ToBound(Bound::High) => bounds.1,
    };
    scale.snap(target).clamp(bounds.0, bounds.1)
}

/// A slider's (or a thumb's) wheel input between frames.
#[derive(Clone, Copy, Debug, Default)]
struct Wheel {
    /// Lines turned and not yet a whole step.
    lines: f32,
    /// The slider acted on a notch that egui is still spreading over frames
    /// (`smooth_scroll_delta`): the rest of it is the slider's too.
    taking: bool,
}

/// The change this frame's wheel makes while it `acts` (the pointer is over the focused slider):
/// a step per line (Shift: ten), a page step per page. Point deltas (trackpads) count a line per
/// egui's `line_scroll_speed` points; the fraction of a line left over is kept while the wheel
/// acts and dropped once it doesn't. The panel under the slider doesn't scroll, for the whole of
/// each notch the slider acts on, also once the pointer leaves or the focus moves. While `held`
/// (a drag) the wheel is taken and changes nothing.
pub(super) fn wheel(ui: &Ui, id: Id, step: f64, page: f64, acts: bool, held: bool) -> f64 {
    let key = id.with("craft_ui::slider::wheel");
    let line = ui.ctx().options(|o| o.input_options.line_scroll_speed).max(1.0);
    let mut state = ui.data(|d| d.get_temp::<Wheel>(key)).unwrap_or_default();
    let (mut lines, mut pages, mut turned) = (0.0, 0.0, false);
    ui.input(|i| {
        for event in &i.events {
            if let Event::MouseWheel { unit, delta, modifiers, .. } = event
                && !modifiers.command
                && !modifiers.ctrl
            {
                turned = true;
                let d = delta.x + delta.y;
                let times = if modifiers.shift { 10.0 } else { 1.0 };
                match unit {
                    MouseWheelUnit::Line => lines += times * d,
                    MouseWheelUnit::Point => lines += times * d / line,
                    MouseWheelUnit::Page => pages += d,
                }
            }
        }
    });
    let scrolling = ui.input(|i| i.is_scrolling());
    let mut change = 0.0;
    if acts {
        state.taking |= turned;
        if held {
            state.lines = 0.0;
        } else {
            let all = state.lines + lines;
            state.lines = all.fract();
            change = step * f64::from(all.trunc()) + page * f64::from(pages);
        }
    } else {
        state.lines = 0.0;
        // A notch the slider doesn't act on is the panel's, with whatever is left of an earlier one.
        state.taking &= !turned;
    }
    state.taking &= scrolling || turned;
    if acts || state.taking {
        ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
    }
    ui.data_mut(|d| d.insert_temp(key, state));
    change
}

/// Paints the track.
pub(super) fn paint_track(ui: &Ui, style: &SliderStyle, track: &Track) {
    let ppp = ui.pixels_per_point();
    ui.painter().extend(style.track.shapes(track.bar(ppp), ppp));
}

/// Paints the filled part of the track between `from` and `to` (x).
pub(super) fn paint_fill(ui: &Ui, style: &SliderStyle, track: &Track, from: f32, to: f32, color: Color32) {
    let bar = track.bar(ui.pixels_per_point());
    let fill = Rect::from_x_y_ranges(from.min(to)..=from.max(to), bar.y_range()).round_to_pixels(ui.pixels_per_point());
    if fill.width() > 0.0 {
        ui.painter().rect_filled(fill, style.track.radius, color);
    }
}

/// A thumb's colours now, from its state.
#[derive(Clone, Copy, Debug)]
pub(super) struct ThumbPaint {
    pub fill: Color32,
    pub border: Color32,
    pub ring: Color32,
    pub enabled: bool,
}

/// Paints a thumb centred on `center` and returns its rect.
pub(super) fn paint_thumb(ui: &Ui, style: &SliderStyle, center: Pos2, paint: ThumbPaint) -> Rect {
    let ppp = ui.pixels_per_point();
    let rect = Rect::from_center_size(center, vec2(style.thumb_width, style.thumb)).round_to_pixels(ppp);
    let mut surface = knob(style.thumb_surface, paint.fill, paint.border, paint.enabled);
    // An outline is centred on the face's edge, so the face keeps the thumb size and the outline
    // reaches half its width beyond it; a bevel stays inside.
    let outline = match surface.edge {
        SurfaceEdge::Border(stroke) => Some(stroke),
        _ => None,
    };
    if outline.is_some() {
        surface.edge = SurfaceEdge::None;
    }
    ui.painter().extend(surface.shapes(rect, ppp));
    if let Some(stroke) = outline {
        ui.painter().rect_stroke(rect, surface.radius, stroke, StrokeKind::Middle);
    }
    if paint.ring != Color32::TRANSPARENT {
        let stroke = Stroke::new(style.ring_width, paint.ring);
        let radius = f32::from(surface.radius.nw);
        if rect.width() == rect.height() && radius * 2.0 >= rect.width() {
            ui.painter().circle_stroke(rect.center(), rect.width() / 2.0 + style.ring_offset, stroke);
        } else {
            ui.painter().rect_stroke(rect.expand(style.ring_offset), corner_radius(radius + style.ring_offset), stroke, StrokeKind::Middle);
        }
    }
    rect
}

/// The marks (at the values) and `ticks` (`count + 1` evenly spaced along the track) under it.
fn paint_marks(ui: &Ui, style: &SliderStyle, track: &Track, marks: &[f64], ticks: usize) {
    let (x0, x1) = track.ends();
    let ticks = (ticks > 0).then(|| (0..=ticks).map(move |i| egui::lerp(x0..=x1, i as f32 / ticks as f32)));
    let marks = marks.iter().filter(|m| (track.scale.min..=track.scale.max).contains(m)).map(|m| track.x_of(*m));
    // PhotoCraft leaves a gap as long as the mark (3 points there) under the track.
    let top = track.bar(ui.pixels_per_point()).bottom() + style.mark;
    let stroke = scale::Stroke::Hairline.egui(ui.ctx(), style.mark_color);
    for x in ticks.into_iter().flatten().chain(marks) {
        ui.painter().line_segment([pos2(x, top), pos2(x, top + style.mark)], stroke);
    }
}

/// The triangle thumb: the round thumb's fill and border, a pointer under the track.
fn paint_pointer(ui: &Ui, style: &SliderStyle, track: &Track, x: f32, paint: ThumbPaint) {
    let (half, drop, top) = (style.thumb_width / 2.0, style.thumb / 2.0, track.bar(ui.pixels_per_point()).bottom());
    let points = vec![pos2(x, top), pos2(x + half, top + drop), pos2(x - half, top + drop)];
    let width = match style.thumb_surface.edge {
        SurfaceEdge::Border(stroke) => stroke.width,
        _ => scale::Stroke::Hairline.egui(ui.ctx(), paint.border).width,
    };
    if paint.ring != Color32::TRANSPARENT {
        ui.painter().add(egui::Shape::closed_line(points.clone(), Stroke::new(2.0 * style.ring_width + width, paint.ring)));
    }
    ui.painter().add(egui::Shape::convex_polygon(points, paint.fill, Stroke::new(width, paint.border)));
}

/// A knob surface (a slider or switch thumb) with these colours; a bevel keeps its own. A
/// disabled knob lies flat, so it never stands out more than an enabled one.
pub(super) fn knob(mut surface: SurfaceStyle, fill: Color32, border: Color32, enabled: bool) -> SurfaceStyle {
    surface.fill = fill;
    if let SurfaceEdge::Border(stroke) = surface.edge {
        surface.edge = SurfaceEdge::Border(Stroke::new(stroke.width, border));
    }
    if !enabled {
        surface.shadow = None;
    }
    surface
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Slider"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("disabled", false);
    let story = |id, name, show| Story::new(id, Level::Controls, "Slider", name, args, show);
    vec![
        Story::new(
            "controls/slider/values",
            Level::Controls,
            "Slider",
            "Values",
            || {
                Args::new()
                    .with_choice("size", SIZE_NAMES, 1)
                    .with_bool("disabled", false)
                    .with_bool("default value (50)", false)
                    .with_bool("wheel unfocused", false)
            },
            show_values,
        ),
        story("controls/slider/step", "Step", show_step),
        story("controls/slider/disabled", "Disabled", show_disabled),
        story("controls/slider/right-to-left", "Right to left", show_right_to_left),
        Story::new(
            "controls/slider/event-log",
            Level::Controls,
            "Slider",
            "Event log",
            || Args::new().with_choice("update rate", edit::RATES, 0).with_bool("cancellable", true).with_bool("on release only", false),
            show_event_log,
        ),
    ]
}

/// Values kept between frames, as a host keeps them.
pub(crate) fn with_values<const N: usize, T: Clone + Send + Sync + 'static>(
    ui: &mut Ui,
    key: &str,
    initial: [T; N],
    show: impl FnOnce(&mut Ui, &mut [T; N]),
) {
    let id = ui.scope_id().with(key);
    let mut values = ui.data(|d| d.get_temp::<[T; N]>(id)).unwrap_or(initial);
    show(ui, &mut values);
    ui.data_mut(|d| d.insert_temp(id, values));
}

/// A one-shot or gesture value into the host, as a host without undo applies it.
pub(crate) fn store<T: Clone>(edit: &EditResponse<T>, value: &mut T) -> Response {
    if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
        *value = v.clone();
    }
    edit.response.clone()
}

/// A property row: the name in the label column, then the control and its value. The control and
/// its value keep their geometry right to left.
pub(super) fn labelled(form: &mut Form<'_>, label: &str, value: &str, control: impl FnOnce(&mut Ui) -> Response) {
    crate::story::variant_row(form, label, |ui| {
        crate::direction::unmirrored_row(ui, crate::direction::Unmirrored::SliderValue, |ui| {
            let response = control(ui);
            ui.add(Text::new(value).content(Content::Muted));
            response
        })
        .inner
    });
}

fn slider_row(form: &mut Form<'_>, args: &Args, salt: &str, label: &str, value: &mut f64, step: Option<f64>) {
    slider_row_with(form, args, salt, label, value, step, |slider| slider);
}

fn slider_row_with(
    form: &mut Form<'_>,
    args: &Args,
    salt: &str,
    label: &str,
    value: &mut f64,
    step: Option<f64>,
    more: impl FnOnce(Slider) -> Slider,
) {
    let (size, enabled) = (size_arg(args.choice("size")), !args.bool("disabled"));
    let text = format!("{value:.1}");
    labelled(form, label, &text, |ui| {
        let mut slider = Slider::new(salt, label, *value, 0.0..=100.0).size(size).enabled(enabled);
        if let Some(step) = step {
            slider = slider.step(step);
        }
        store(&more(slider).show(ui), value)
    });
}

/// Sliders at the minimum, the middle and the maximum.
fn show_values(ui: &mut Ui, args: &mut Args) {
    with_values(ui, "slider-values", [0.0, 50.0, 100.0, 25.0, 100.0, 50.0, 30.0], |ui, values| {
        variant_list(ui, "slider-values", |form| {
            let (minimum_to_maximum, shaped) = values.split_at_mut(3);
            for (i, (label, value)) in ["Minimum", "Middle", "Maximum"].into_iter().zip(minimum_to_maximum.iter_mut()).enumerate() {
                let (default, wheel) = (args.bool("default value (50)"), args.bool("wheel unfocused"));
                slider_row_with(form, args, &format!("slider-values-{i}"), label, value, None, |slider| {
                    let slider = slider.wheel_unfocused(wheel);
                    if default { slider.default_value(50.0) } else { slider }
                });
            }
            let (size, enabled) = (size_arg(args.choice("size")), !args.bool("disabled"));
            let [power, log, marked, triangle] = shaped else { return };
            labelled(form, "Power 2", &format!("{power:.1}"), |ui| {
                let slider = Slider::new("slider-power", "Power 2", *power, 0.0..=100.0).scale(SliderScale::Power(2.0));
                store(&slider.size(size).enabled(enabled).show(ui), power)
            });
            labelled(form, "Log", &format!("{log:.1}"), |ui| {
                let slider = Slider::new("slider-log", "Log", *log, 1.0..=1000.0).scale(SliderScale::Log);
                store(&slider.size(size).enabled(enabled).show(ui), log)
            });
            labelled(form, "Marks", &format!("{marked:.1}"), |ui| {
                let slider = Slider::new("slider-marks", "Marks", *marked, 0.0..=100.0).marks(&[0.0, 25.0, 50.0, 100.0]);
                store(&slider.size(size).enabled(enabled).show(ui), marked)
            });
            labelled(form, "Triangle, ticks", &format!("{triangle:.1}"), |ui| {
                let slider = Slider::new("slider-triangle", "Triangle", *triangle, 0.0..=100.0).thumb(SliderThumb::Triangle).ticks(10);
                store(&slider.size(size).enabled(enabled).show(ui), triangle)
            });
        });
    });
}

/// Steps of 10 and of 25.
fn show_step(ui: &mut Ui, args: &mut Args) {
    with_values(ui, "slider-step", [40.0, 50.0], |ui, values| {
        variant_list(ui, "slider-step", |form| {
            for (i, (step, value)) in [10.0, 25.0].into_iter().zip(values.iter_mut()).enumerate() {
                slider_row(form, args, &format!("slider-step-{i}"), &format!("Step {step}"), value, Some(step));
            }
        });
    });
}

/// Disabled sliders at the minimum, a value and the maximum.
fn show_disabled(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    variant_list(ui, "slider-disabled", |form| {
        for (i, value) in [0.0, 30.0, 100.0].into_iter().enumerate() {
            labelled(form, &format!("Disabled {}", i + 1), &format!("{value:.1}"), |ui| {
                Slider::new(("slider-disabled", i), format!("Disabled {}", i + 1), value, 0.0..=100.0).size(size).enabled(false).show(ui).response
            });
        }
    });
}

/// A labelled slider row: with the gallery's direction set to right to left, the row mirrors
/// around the slider and its value.
fn show_right_to_left(ui: &mut Ui, args: &mut Args) {
    with_values(ui, "slider-rtl", [25.0], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let note = "With the direction set to right to left the label moves to the right. The slider keeps its geometry: the value grows to the right and the arrow keys act as they are.";
            ui.add(Text::new(note).content(Content::Muted));
            variant_list(ui, "slider-rtl", |form| slider_row(form, args, "slider-rtl", "Opacity", &mut values[0], Some(1.0)));
        });
    });
}

/// Events kept in the event log.
const LOG_LEN: usize = 20;

/// The event log's document: both controls' values and an undo history of whole snapshots.
#[derive(Clone, Debug)]
struct LogDoc {
    level: f64,
    range: (f64, f64),
    undo: Vec<(f64, (f64, f64))>,
}

impl LogDoc {
    /// Applies `edit` as one undo step.
    fn edit(&mut self, edit: impl FnOnce(&mut Self)) {
        self.undo.push((self.level, self.range));
        edit(self);
    }
}

impl UndoMarks for LogDoc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&(level, range)) = self.undo.get(mark) {
            (self.level, self.range) = (level, range);
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

#[derive(Clone, Debug)]
struct LogState {
    doc: LogDoc,
    host: GestureHost<LogDoc>,
    /// Newest first.
    log: VecDeque<String>,
}

impl Default for LogState {
    fn default() -> Self {
        Self { doc: LogDoc { level: 40.0, range: (20.0, 70.0), undo: Vec::new() }, host: GestureHost::new(), log: VecDeque::new() }
    }
}

impl LogState {
    fn log(&mut self, line: String) {
        self.log.push_front(line);
        self.log.truncate(LOG_LEN);
    }

    fn handle<T: Clone + PartialEq + Send + Sync + Debug + 'static>(
        &mut self,
        ui: &Ui,
        control: &str,
        event: Option<EditEvent<T>>,
        apply: impl Fn(&mut LogDoc, T) + Send + Sync + 'static,
    ) {
        let Some(event) = event else { return };
        self.log(format!("{control}: {}", edit::describe(&event)));
        self.host.handle(ui.ctx(), &mut self.doc, event, apply);
    }

    /// Undo: during a gesture it cancels the gesture instead of undoing a step.
    fn undo(&mut self, ui: &Ui) {
        match self.host.active() {
            Some(id) => {
                self.host.cancel(&mut self.doc, id);
                edit::cancel_all(ui.ctx(), CancelReason::UndoDuringGesture);
                self.log(format!("undo: cancel_all({:?})", CancelReason::UndoDuringGesture));
            }
            None => {
                if let Some((level, range)) = self.doc.undo.pop() {
                    (self.doc.level, self.doc.range) = (level, range);
                    self.log("undo: one step".to_owned());
                }
            }
        }
    }
}

/// A slider and a range slider in steps of 1 under a [`GestureHost`], their events newest first.
/// Cmd/Ctrl+Z undoes a step, or cancels the gesture under way (it works during a drag).
fn show_event_log(ui: &mut Ui, args: &mut Args) {
    let policy = edit::policy(args);
    let state_id = ui.scope_id().with("slider-event-log");
    let mut state = ui.data(|d| d.get_temp::<LogState>(state_id)).unwrap_or_default();
    if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
        state.undo(ui);
    }
    Stack::new(Gap::Stack).show(ui, |ui| {
        variant_list(ui, "slider-event-log", |form| {
            let level = state.doc.level;
            labelled(form, "Level", &format!("{level:.0}"), |ui| {
                let edit = Slider::new("slider-log-level", "Level", level, 0.0..=100.0).step(1.0).policy(policy).show(ui);
                state.handle(ui, "slider", edit.event, |doc, value| doc.edit(|d| d.level = value));
                edit.response
            });
            let range = state.doc.range;
            labelled(form, "Range", &format!("{:.0} to {:.0}", range.0, range.1), |ui| {
                let edit = RangeSlider::new("slider-log-range", ("Range from", "Range to"), range, 0.0..=100.0).step(1.0).policy(policy).show(ui);
                state.handle(ui, "range slider", edit.event, |doc, value| doc.edit(|d| d.range = value));
                edit.response
            });
        });
        if let Some(event) = state.host.end_frame(ui.ctx(), &mut state.doc) {
            let line = format!("host: {}", edit::describe(&event));
            state.log(line);
        }
        ui.add(Text::new(format!("{} undo steps", state.doc.undo.len())).content(Content::Muted));
        Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
            ui.set_min_width(LOG_WIDTH);
            if state.log.is_empty() {
                ui.add(Text::new("No events yet").content(Content::Faint));
            }
            for line in &state.log {
                ui.add(Text::new(line.as_str()).role(TypeRole::Mono));
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, state));
}

const LOG_WIDTH: f32 = 420.0;
