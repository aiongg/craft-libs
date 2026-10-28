//! [`Slider`]: a value on a horizontal track; and the scale, track, painting and keyboard parts it
//! shares with [`RangeSlider`](super::RangeSlider).

use std::collections::VecDeque;
use std::fmt::Debug;
use std::ops::RangeInclusive;

use craft_theme::{ControlSize, States, TypeRole};
use egui::accesskit::{Action, ActionData, Role};
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Color32, Event, EventFilter, Id, IdSalt, Key, Modifiers, MouseWheelUnit, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

use super::{RangeSlider, SIZE_NAMES, size_arg};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, EditResponse, Gesture, GestureHost, UndoMarks};
use crate::primitives::{Content, Gap, Interactive, Row, Stack, StateLayer, Surface, SurfaceEdge, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius};

/// A slider's sizes and colours, from [`Slider::style`]; [`RangeSlider`](super::RangeSlider)
/// draws with the same.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliderStyle {
    /// Height of the control: `slider.height`.
    pub height: f32,
    /// Default length: `slider.width`.
    pub width: f32,
    /// Diameter (or side) of a thumb: `slider.thumb`.
    pub thumb: f32,
    /// Thickness of the track: `slider.track_height`.
    pub track_height: f32,
    /// The track: a sunken [`Surface`] (its border, or bevel) filled with `slider.track`, with
    /// `slider.radius` corners clamped to half its height.
    pub track: SurfaceStyle,
    /// The filled part, from the start to the value: `slider.fill`.
    pub fill: States<Color32>,
    /// A thumb's shape: a raised [`Surface`] (its border, or bevel) with `slider.radius` corners
    /// clamped to half its side. Its fill and border colour change with the state.
    pub thumb_surface: SurfaceStyle,
    /// `slider.thumb.fill`.
    pub thumb_fill: States<Color32>,
    /// `slider.thumb.border`; a bevel keeps its own colours.
    pub thumb_border: States<Color32>,
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
/// - Pointer: a press on the track moves the thumb there; a press on the thumb keeps the offset,
///   so the value doesn't jump. A press also gives the slider keyboard focus.
/// - Keys, while focused: Right and Up add the step (Shift: ten steps), Left and Down take it
///   away, Page Up and Page Down move a tenth of the range, Home and End go to its ends. The
///   step is [`step`](Slider::step), or a hundredth of the range.
/// - The wheel steps the value while the pointer is over the focused slider, and the panel under
///   it doesn't scroll; over a slider without focus the wheel scrolls the panel.
/// - AccessKit: a `Slider` named `label` with its value, bounds and step, which takes increment,
///   decrement and set-value requests.
/// - A slider keeps its geometry right to left ([`Unmirrored::SliderValue`](crate::direction::Unmirrored::SliderValue)):
///   the value grows to the right, and the arrow keys act as they are.
///
/// Values snap to multiples of the step from the range's start, when there is a step. A value
/// outside the range is drawn at the nearer end.
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
}

impl Slider {
    /// A slider named `label` (its accessible name) showing the host's `value` in `range`;
    /// `id_salt` keeps its gesture, focus and animations when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: f64, range: RangeInclusive<f64>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            range,
            step: None,
            policy: EditPolicy::default(),
            size: ControlSize::Md,
            enabled: true,
        }
    }

    /// The step values snap to, and one arrow key's change. Without one the value is continuous
    /// and a key moves a hundredth of the range.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
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

    /// Shows the slider and returns this frame's edit event. The response covers the control.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        let tokens = Tokens::of(ui);
        let style = Slider::style(&tokens, self.size);
        let scale = Scale::new(&self.range, self.step);
        let sensed = Interactive::new(self.id_salt, Role::Slider, self.label.as_str())
            .sense(Sense::drag())
            .enabled(self.enabled)
            .state_layer(false)
            .interact(ui, vec2(style.width, style.height));
        let response = sensed.response().clone();
        let track = Track::new(response.rect, &style, scale);

        let mut gesture = Gesture::load(ui, response.id, &self.value, self.policy).event_filter(KEY_FILTER);
        if response.drag_started() {
            response.request_focus();
            gesture.begin();
        }
        if response.dragged()
            && let Some(start) = gesture.start().copied()
            && let Some((pos, origin)) = pointer(ui)
        {
            let grab = track.grab(start, origin.x);
            gesture.update(scale.snap(track.value_at(pos.x + grab)), ui.input(|i| i.modifiers));
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        if let Some(value) = stepped(ui, response.id, response.enabled(), response.hovered(), *gesture.shown(), (scale.min, scale.max), &scale) {
            gesture.set(value);
        }

        let shown = scale.clamp(*gesture.shown());
        let response = sensed.numeric(shown, scale.min..=scale.max, scale.key_step()).finish(ui);
        if ui.is_rect_visible(response.rect) {
            let color = |states: &States<Color32>| StateLayer::color(ui, states, &response);
            paint_track(ui, &style, &track);
            let x = track.x_of(shown);
            paint_fill(ui, &style, &track, track.rect.left(), x, color(&style.fill));
            paint_thumb(ui, &style, pos2(x, track.y), color(&style.thumb_fill), color(&style.thumb_border));
        }
        EditResponse::new(response, gesture.finish())
    }

    pub fn style(tokens: &UiTokens, size: ControlSize) -> SliderStyle {
        let s = &tokens.component.slider;
        let thumb = s.thumb.get(size);
        let mut track = Surface::style(tokens, SurfaceKind::Sunken);
        track.fill = s.track.to_egui();
        track.radius = corner_radius(s.radius.min(s.track_height / 2.0));
        let mut thumb_surface = Surface::style(tokens, SurfaceKind::Raised);
        thumb_surface.fill = s.thumb_colors.fill.rest.to_egui();
        thumb_surface.radius = corner_radius(s.radius.min(thumb / 2.0));
        if let SurfaceEdge::Border(stroke) = thumb_surface.edge {
            thumb_surface.edge = SurfaceEdge::Border(Stroke::new(stroke.width, s.thumb_colors.border.rest.to_egui()));
        }
        SliderStyle {
            height: s.height.get(size),
            width: s.width.get(size),
            thumb,
            track_height: s.track_height,
            track,
            fill: s.fill.to_egui(),
            thumb_surface,
            thumb_fill: s.thumb_colors.fill.to_egui(),
            thumb_border: s.thumb_colors.border.to_egui(),
        }
    }
}

/// The keys a focused thumb keeps: the arrows step its value rather than move the focus.
pub(super) const KEY_FILTER: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false };

/// A range with an optional step: where values may go and how keys move them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Scale {
    pub min: f64,
    pub max: f64,
    step: Option<f64>,
}

impl Scale {
    /// The range in ascending order; a step that isn't a positive number is none.
    pub fn new(range: &RangeInclusive<f64>, step: Option<f64>) -> Self {
        let (a, b) = (*range.start(), *range.end());
        let (min, max) = if a <= b { (a, b) } else { (b, a) };
        Self { min, max, step: step.filter(|s| s.is_finite() && *s > 0.0) }
    }

    fn span(&self) -> f64 {
        self.max - self.min
    }

    /// Where `value` lies from the start (0) to the end (1).
    fn fraction(&self, value: f64) -> f32 {
        if self.span() > 0.0 { ((value - self.min) / self.span()).clamp(0.0, 1.0) as f32 } else { 0.0 }
    }

    pub fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }

    /// `value` in the range, on a multiple of the step from the start when there is one.
    pub fn snap(&self, value: f64) -> f64 {
        let value = self.clamp(value);
        match self.step {
            Some(step) => self.clamp(self.min + ((value - self.min) / step).round() * step),
            None => value,
        }
    }

    /// One arrow key's change: the step, or a hundredth of the range.
    pub fn key_step(&self) -> f64 {
        self.step.unwrap_or(self.span() / 100.0)
    }

    /// One Page key's change: a tenth of the range, in whole steps when there is a step.
    fn page_step(&self) -> f64 {
        let page = self.span() / 10.0;
        match self.step {
            Some(step) => (page / step).round().max(1.0) * step,
            None => page,
        }
    }
}

/// Where values sit along a control's rect: a thumb's centre runs from half a thumb inside the
/// start to half a thumb inside the end, so the thumb stays inside the control.
#[derive(Clone, Copy, Debug)]
pub(super) struct Track {
    pub rect: Rect,
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
        let reach = style.thumb / 2.0;
        Self { rect, x0: rect.left() + reach, x1: rect.right() - reach, y: rect.center().y, reach, scale, track_height: style.track_height }
    }

    /// The thumb centre's x for `value`.
    pub fn x_of(&self, value: f64) -> f32 {
        egui::lerp(self.x0..=self.x1, self.scale.fraction(value))
    }

    /// The value whose thumb centre is at `x`, unsnapped.
    pub fn value_at(&self, x: f32) -> f64 {
        let t = if self.x1 > self.x0 { ((x - self.x0) / (self.x1 - self.x0)).clamp(0.0, 1.0) } else { 0.0 };
        self.scale.min + f64::from(t) * self.scale.span()
    }

    /// The change of value a pointer movement of `dx` points makes.
    pub fn value_per(&self, dx: f32) -> f64 {
        if self.x1 > self.x0 { f64::from(dx / (self.x1 - self.x0)) * self.scale.span() } else { 0.0 }
    }

    /// The offset from a press at `x` to the centre of the thumb at `value`, when the press is on
    /// that thumb, so dragging it doesn't jump; 0 otherwise, so the thumb jumps to the pointer.
    pub fn grab(&self, value: f64, x: f32) -> f32 {
        let centre = self.x_of(value);
        if (x - centre).abs() <= self.reach { centre - x } else { 0.0 }
    }

    /// The track's rect, on the pixel grid.
    fn bar(&self, pixels_per_point: f32) -> Rect {
        let half = self.track_height / 2.0;
        Rect::from_min_max(pos2(self.rect.left(), self.y - half), pos2(self.rect.right(), self.y + half)).round_to_pixels(pixels_per_point)
    }
}

/// The pointer now and where its button went down, while it is down.
pub(super) fn pointer(ui: &Ui) -> Option<(Pos2, Pos2)> {
    ui.input(|i| i.pointer.interact_pos().zip(i.pointer.press_origin()))
}

/// The value this frame's keys, wheel and screen-reader requests give the thumb `id` at `value`,
/// kept within `bounds`; `None` when nothing asked for a change. Keys and the wheel act while the
/// thumb has focus, the wheel only while the pointer is over the control (`hovered`).
pub(super) fn stepped(ui: &Ui, id: Id, enabled: bool, hovered: bool, value: f64, bounds: (f64, f64), scale: &Scale) -> Option<f64> {
    if !enabled {
        return None;
    }
    let (step, page) = (scale.key_step(), scale.page_step());
    let mut delta = 0.0;
    let mut to = None;
    if ui.memory(|m| m.has_focus(id)) {
        ui.memory_mut(|m| m.set_focus_lock_filter(id, KEY_FILTER));
        ui.input_mut(|i| {
            let mut count = |modifiers, key| i.count_and_consume_key(modifiers, key) as f64;
            for (key, sign) in [(Key::ArrowRight, 1.0), (Key::ArrowUp, 1.0), (Key::ArrowLeft, -1.0), (Key::ArrowDown, -1.0)] {
                // Shift first: `Modifiers::NONE` also matches a press with Shift held.
                delta += sign * step * (10.0 * count(Modifiers::SHIFT, key) + count(Modifiers::NONE, key));
            }
            delta += page * (count(Modifiers::NONE, Key::PageUp) - count(Modifiers::NONE, Key::PageDown));
            if count(Modifiers::NONE, Key::Home) > 0.0 {
                to = Some(bounds.0);
            }
            if count(Modifiers::NONE, Key::End) > 0.0 {
                to = Some(bounds.1);
            }
        });
        if hovered {
            delta += wheel(ui, id, step, page);
        }
    }
    ui.input(|i| {
        let requests = |action| i.num_accesskit_action_requests(id, action) as f64;
        delta += step * (requests(Action::Increment) - requests(Action::Decrement));
        let set = i.accesskit_action_requests(id, Action::SetValue).filter_map(|r| match r.data {
            Some(ActionData::NumericValue(v)) => Some(v),
            _ => None,
        });
        if let Some(v) = set.last() {
            to = Some(v);
        }
    });
    if delta == 0.0 && to.is_none() {
        return None;
    }
    let target = to.unwrap_or(value.clamp(bounds.0, bounds.1) + delta);
    Some(scale.snap(target).clamp(bounds.0, bounds.1))
}

/// The change this frame's wheel makes: a step per line (Shift: ten), a page step per page; the
/// panel under the slider doesn't scroll. Point deltas (trackpads) count a line per egui's
/// `line_scroll_speed` points, keeping the remainder for the next frame.
fn wheel(ui: &Ui, id: Id, step: f64, page: f64) -> f64 {
    let key = id.with("craft_ui::slider::wheel");
    let line = ui.ctx().options(|o| o.input_options.line_scroll_speed).max(1.0);
    let mut lines = ui.data(|d| d.get_temp::<f32>(key)).unwrap_or(0.0);
    let mut pages = 0.0;
    ui.input(|i| {
        for event in &i.events {
            if let Event::MouseWheel { unit, delta, modifiers, .. } = event
                && !modifiers.command
                && !modifiers.ctrl
            {
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
    let whole = lines.trunc();
    ui.data_mut(|d| d.insert_temp(key, lines - whole));
    ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
    step * f64::from(whole) + page * f64::from(pages)
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

/// Paints a thumb centred on `center` and returns its rect.
pub(super) fn paint_thumb(ui: &Ui, style: &SliderStyle, center: Pos2, fill: Color32, border: Color32) -> Rect {
    let ppp = ui.pixels_per_point();
    let rect = Rect::from_center_size(center, Vec2::splat(style.thumb)).round_to_pixels(ppp);
    let mut surface = style.thumb_surface;
    surface.fill = fill;
    if let SurfaceEdge::Border(stroke) = surface.edge {
        surface.edge = SurfaceEdge::Border(Stroke::new(stroke.width, border));
    }
    ui.painter().extend(surface.shapes(rect, ppp));
    rect
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Slider"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("disabled", false);
    let story = |id, name, show| Story { id, level: Level::Controls, component: "Slider", name, args, show };
    vec![
        story("controls/slider/values", "Values", show_values),
        story("controls/slider/step", "Step", show_step),
        story("controls/slider/disabled", "Disabled", show_disabled),
        story("controls/slider/right-to-left", "Right to left", show_right_to_left),
        Story {
            id: "controls/slider/event-log",
            level: Level::Controls,
            component: "Slider",
            name: "Event log",
            args: || Args::new().with_choice("update rate", edit::RATES, 0).with_bool("cancellable", true).with_bool("on release only", false),
            show: show_event_log,
        },
    ]
}

/// Values kept between frames, as a host keeps them.
pub(super) fn with_values<const N: usize, T: Clone + Send + Sync + 'static>(
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
pub(super) fn apply<T: Clone>(edit: &EditResponse<T>, value: &mut T) {
    if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
        *value = v.clone();
    }
}

/// A labelled row: the name at the reading start, then the control and its value. The control and
/// its value keep their geometry right to left.
pub(super) fn labelled(ui: &mut Ui, label: &str, value: &str, control: impl FnOnce(&mut Ui)) {
    Row::new(Gap::Inline).show(ui, |ui| {
        ui.add(Text::new(label));
        crate::direction::unmirrored_row(ui, crate::direction::Unmirrored::SliderValue, |ui| {
            control(ui);
            ui.add(Text::new(value).content(Content::Muted));
        });
    });
}

fn slider_row(ui: &mut Ui, args: &Args, salt: &str, label: &str, value: &mut f64, step: Option<f64>) {
    let (size, enabled) = (size_arg(args.choice("size")), !args.bool("disabled"));
    let text = format!("{value:.1}");
    labelled(ui, label, &text, |ui| {
        let mut slider = Slider::new(salt, label, *value, 0.0..=100.0).size(size).enabled(enabled);
        if let Some(step) = step {
            slider = slider.step(step);
        }
        apply(&slider.show(ui), value);
    });
}

/// Sliders at the minimum, the middle and the maximum.
fn show_values(ui: &mut Ui, args: &mut Args) {
    with_values(ui, "slider-values", [0.0, 50.0, 100.0], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            for (i, (label, value)) in ["Minimum", "Middle", "Maximum"].into_iter().zip(values.iter_mut()).enumerate() {
                slider_row(ui, args, &format!("slider-values-{i}"), label, value, None);
            }
        });
    });
}

/// Steps of 10 and of 25.
fn show_step(ui: &mut Ui, args: &mut Args) {
    with_values(ui, "slider-step", [40.0, 50.0], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            for (i, (step, value)) in [10.0, 25.0].into_iter().zip(values.iter_mut()).enumerate() {
                slider_row(ui, args, &format!("slider-step-{i}"), &format!("Step {step}"), value, Some(step));
            }
        });
    });
}

/// Disabled sliders at the minimum, a value and the maximum.
fn show_disabled(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (i, value) in [0.0, 30.0, 100.0].into_iter().enumerate() {
            labelled(ui, &format!("Disabled {}", i + 1), &format!("{value:.1}"), |ui| {
                let _ = Slider::new(("slider-disabled", i), format!("Disabled {}", i + 1), value, 0.0..=100.0).size(size).enabled(false).show(ui);
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
            slider_row(ui, args, "slider-rtl", "Opacity", &mut values[0], Some(1.0));
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
        apply: impl FnMut(&mut LogDoc, T),
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
        let level = state.doc.level;
        labelled(ui, "Level", &format!("{level:.0}"), |ui| {
            let edit = Slider::new("slider-log-level", "Level", level, 0.0..=100.0).step(1.0).policy(policy).show(ui);
            state.handle(ui, "slider", edit.event, |doc, value| doc.edit(|d| d.level = value));
        });
        let range = state.doc.range;
        labelled(ui, "Range", &format!("{:.0} to {:.0}", range.0, range.1), |ui| {
            let edit = RangeSlider::new("slider-log-range", ("Range from", "Range to"), range, 0.0..=100.0).step(1.0).policy(policy).show(ui);
            state.handle(ui, "range slider", edit.event, |doc, value| doc.edit(|d| d.range = value));
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
