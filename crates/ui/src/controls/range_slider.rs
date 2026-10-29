//! [`RangeSlider`]: a lower and an upper value on one track.

use std::ops::RangeInclusive;

use craft_theme::{ControlSize, States};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, Id, IdSalt, InteractOptions, PointerButton, Sense, Ui, Vec2, pos2};

use super::slider::{
    KEY_FILTER, Scale, Sensing, ThumbPaint, Track, apply, labelled, paint_fill, paint_thumb, paint_track, pointer, stepped, store, tap, with_values,
};
use super::{SIZE_NAMES, Slider, size_arg};
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{FocusRing, Gap, Interactive, Stack, StateLayer};
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf};

/// A range within a range, set with two thumbs on one track:
///
/// ```
/// use craft_ui::controls::RangeSlider;
/// # egui::__run_test_ui(|ui| { let mut hue = (40.0, 160.0);
/// let edit = RangeSlider::new("hue", ("Hue from", "Hue to"), hue, 0.0..=360.0).step(1.0).show(ui);
/// if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
///     hue = *value;
/// }
/// # });
/// ```
///
/// It works as a [`Slider`] with two thumbs, the part between them filled, and reports both values
/// as one `(lower, upper)` through the edit lifecycle.
///
/// - Each thumb is a Tab stop and an AccessKit `Slider` named by its label, whose bounds are the
///   range's end and the other thumb: the thumbs can't cross.
/// - Pointer (primary button): a press on a thumb drags it; a press between the thumbs drags
///   both, keeping the distance between them; a press elsewhere moves the nearer thumb there. Where
///   the thumbs meet, the drag's direction picks the one that moves. A press focuses the thumb it
///   moves (the nearer one between the thumbs, and the picked one where they meet), which keeps
///   the focus after the release. A tap (press and release in one frame) is a
///   [`Set`](crate::edit::EditEvent::Set) to the value the same press and release would give.
/// - Keys, the wheel and screen-reader requests move the focused thumb as on a [`Slider`]; Home
///   and End go to its bounds.
#[derive(Clone, Debug)]
#[must_use = "show the range slider with `show`"]
pub struct RangeSlider {
    id_salt: IdSalt,
    labels: (String, String),
    value: (f64, f64),
    range: RangeInclusive<f64>,
    step: Option<f64>,
    policy: EditPolicy,
    size: ControlSize,
    enabled: bool,
}

/// What a press at a point moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Lower,
    Upper,
    /// Both thumbs, by the same amount: a press between them.
    Span,
    /// Thumbs that meet under the press: the drag's direction picks one.
    Either,
}

impl Part {
    /// Whether the part moves the lower thumb, the upper one and the span between them.
    fn lights(self) -> (bool, bool, bool) {
        match self {
            Part::Lower => (true, false, false),
            Part::Upper => (false, true, false),
            Part::Span => (true, true, true),
            Part::Either => (true, true, false),
        }
    }
}

impl RangeSlider {
    /// A range slider showing the host's `(lower, upper)` value in `range`. `labels` name the
    /// lower and the upper thumb (their accessible names), as whole phrases the app translates,
    /// e.g. `("Hue from", "Hue to")`; `id_salt` keeps its gesture, focus and animations when the
    /// layout around it changes. The value is read in order, lower end first. The range is read
    /// as [`Slider::new`] reads it (reversed is reordered, bounds must be finite), and a value
    /// end that isn't a finite number as on a [`Slider`].
    pub fn new(id_salt: impl AsIdSalt, labels: (impl Into<String>, impl Into<String>), value: (f64, f64), range: RangeInclusive<f64>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            labels: (labels.0.into(), labels.1.into()),
            value,
            range,
            step: None,
            policy: EditPolicy::default(),
            size: ControlSize::Md,
            enabled: true,
        }
    }

    /// The step values snap to, as [`Slider::step`].
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// How the gesture reports ([`EditPolicy`]).
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

    /// Shows the range slider and returns this frame's edit event, with the value as
    /// `(lower, upper)`. The response covers the track.
    pub fn show(self, ui: &mut Ui) -> EditResponse<(f64, f64)> {
        let tokens = Tokens::of(ui);
        let style = Slider::style(&tokens, self.size);
        let scale = Scale::new(&self.range, self.step);
        let (_, rect) = ui.allocate_space(Vec2::new(style.width, style.height));
        let id = ui.make_persistent_id(self.id_salt);
        let options = InteractOptions { enabled: self.enabled, ..Default::default() };
        let response = ui.interact_opt(rect, id, Sense::DRAG, options);
        let sensing = Sensing::of(&response);
        let track = Track::new(rect, &style, scale);
        let salts = (id.with("lower"), id.with("upper"));
        let ids = (Interactive::id_in(ui, salts.0), Interactive::id_in(ui, salts.1));
        let thumb_id = |lower: bool| if lower { ids.0 } else { ids.1 };
        let host = ordered(self.value);

        // The gesture lives on the track, which takes every press; the focus goes to the thumbs.
        let mut gesture = Gesture::load(ui, id, &host, self.policy).focus_owners(&[ids.0, ids.1]).event_filter(KEY_FILTER).enabled(sensing.enabled);
        let at = |values: (f64, f64), x: f32| part_at(&track, values, x);
        // The thumb to give the focus this frame, once the thumbs have sensed the pointer.
        let mut focus = None;
        if response.drag_started_by(PointerButton::Primary)
            && let Some((_, origin)) = pointer(ui, &response)
        {
            focus = Some(thumb_id(pressed_lower(&track, placed(&scale, *gesture.shown()), origin.x)));
            if host.0.is_finite() && host.1.is_finite() {
                gesture.begin();
            }
        }
        let mut active = None;
        if response.dragged_by(PointerButton::Primary)
            && let Some(anchor) = gesture.anchor().copied()
            && let Some((pos, origin)) = pointer(ui, &response)
        {
            let (anchor, start) = (placed(&scale, anchor), placed(&scale, *gesture.start().unwrap_or(&anchor)));
            let part = at(anchor, origin.x);
            active = Some(part);
            if part == Part::Either && pos.x != origin.x {
                focus = Some(thumb_id(pos.x < origin.x));
            }
            gesture.update(dragged(&track, &scale, part, anchor, start, origin.x, pos.x), ui.input(|i| i.modifiers));
        }
        if response.drag_stopped() {
            gesture.commit();
            // A release away from the focused thumb would take its focus (egui's click-away).
            focus = [ids.0, ids.1].into_iter().find(|t| ui.memory(|m| m.has_focus(*t)));
        }
        if let Some(at_x) = tap(ui, &response).map(|p| p.x) {
            let values = placed(&scale, *gesture.shown());
            focus = Some(thumb_id(pressed_lower(&track, values, at_x)));
            gesture.set(dragged(&track, &scale, at(values, at_x), values, values, at_x, at_x));
        }

        let (lo, hi) = placed(&scale, *gesture.shown());
        if let Some(op) = stepped(ui, ids.0, sensing, None, &scale) {
            gesture.set((apply(&op, lo, (scale.min, hi), &scale), hi));
        }
        let (lo, hi) = placed(&scale, *gesture.shown());
        if let Some(op) = stepped(ui, ids.1, sensing, None, &scale) {
            gesture.set((lo, apply(&op, hi, (lo, scale.max), &scale)));
        }

        let mixed = !(gesture.shown().0.is_finite() && gesture.shown().1.is_finite());
        let (lo, hi) = placed(&scale, *gesture.shown());
        let (x_lo, x_hi) = (track.x_of(lo), track.x_of(hi));
        let thumb_rect = |x: f32| egui::Rect::from_center_size(pos2(x, track.y), egui::vec2(style.thumb_width, style.thumb));
        let thumbs = [(salts.0, &self.labels.0, lo, scale.min..=hi, x_lo), (salts.1, &self.labels.1, hi, lo..=scale.max, x_hi)].map(
            |(salt, label, value, bounds, x)| {
                let sensed = Interactive::new(salt, Role::Slider, label.as_str())
                    .sense(Sense::FOCUSABLE)
                    .enabled(sensing.enabled)
                    .state_layer(false)
                    .focus_ring(false)
                    .interact_at(ui, thumb_rect(x));
                // A pair with a value that isn't finite is drawn at the start but reported mixed.
                match mixed {
                    true => sensed.numeric_mixed(bounds, scale.key_step()).finish(ui),
                    false => sensed.numeric(value, bounds, scale.key_step()).finish(ui),
                }
            },
        );
        if let Some(thumb) = focus
            && !ui.memory(|m| m.has_focus(thumb))
        {
            ui.memory_mut(|m| m.request_focus(thumb));
        }

        if ui.is_rect_visible(rect) {
            // The part a press would move lights up while hovered, and the dragged part while pressed.
            let lit = active.or_else(|| response.hover_pos().map(|p| at((lo, hi), p.x)));
            let (lit_lo, lit_hi, lit_span) = lit.map_or((false, false, false), Part::lights);
            let down = response.is_pointer_button_down_on();
            let color = |states: &States<Color32>, key: Id, lit: bool| StateLayer::color_of(ui, states, key, sensing.enabled, lit, lit && down);
            paint_track(ui, &style, &track);
            paint_fill(ui, &style, &track, x_lo, x_hi, color(&style.fill, id, lit_span));
            for ((response, x), lit) in thumbs.iter().zip([x_lo, x_hi]).zip([lit_lo, lit_hi]) {
                let paint = ThumbPaint {
                    fill: color(&style.thumb_fill, response.id, lit),
                    border: color(&style.thumb_border, response.id, lit),
                    ring: color(&style.ring, response.id, lit),
                    enabled: sensing.enabled,
                };
                let rect = paint_thumb(ui, &style, pos2(x, track.y), paint);
                FocusRing::paint_radius(ui, rect, style.thumb_surface.radius, response);
            }
        }
        EditResponse::new(response, gesture.finish())
    }
}

/// The value with its lower end first.
fn ordered((a, b): (f64, f64)) -> (f64, f64) {
    if a <= b { (a, b) } else { (b, a) }
}

/// The value in order and in the range, as drawn ([`Scale::place`]).
fn placed(scale: &Scale, value: (f64, f64)) -> (f64, f64) {
    ordered((scale.place(value.0), scale.place(value.1)))
}

/// Whether a press at `x` focuses the lower thumb: the thumb it moves, else the nearer one.
fn pressed_lower(track: &Track, (lo, hi): (f64, f64), x: f32) -> bool {
    match part_at(track, (lo, hi), x) {
        Part::Lower => true,
        Part::Upper => false,
        Part::Span | Part::Either => (x - track.x_of(lo)).abs() <= (x - track.x_of(hi)).abs(),
    }
}

/// What a press at `x` moves, with the thumbs at `values`.
fn part_at(track: &Track, (lo, hi): (f64, f64), x: f32) -> Part {
    let (x_lo, x_hi) = (track.x_of(lo), track.x_of(hi));
    let (d_lo, d_hi) = ((x - x_lo).abs(), (x - x_hi).abs());
    if d_lo.min(d_hi) <= track.reach {
        return match d_lo.partial_cmp(&d_hi) {
            Some(std::cmp::Ordering::Less) => Part::Lower,
            Some(std::cmp::Ordering::Greater) => Part::Upper,
            _ => Part::Either,
        };
    }
    if x > x_lo && x < x_hi {
        Part::Span
    } else if x <= x_lo {
        Part::Lower
    } else {
        Part::Upper
    }
}

/// The value while dragging `part` from a press at `origin` to `x` ([`Track::dragged`] for one
/// thumb): a dragged thumb from `anchor`, the value the drag began from, and a thumb the drag
/// doesn't move at `start`, the host's value when the gesture began (a gesture the host ended
/// carries on from it, so a value set meanwhile stays). The thumbs don't cross, and the span
/// keeps its width inside the range; a still pointer leaves the value as it is.
fn dragged(track: &Track, scale: &Scale, part: Part, anchor: (f64, f64), start: (f64, f64), origin: f32, x: f32) -> (f64, f64) {
    let (lo, hi) = anchor;
    let lower = || (track.dragged(lo, origin, x).min(start.1), start.1);
    let upper = || (start.0, track.dragged(hi, origin, x).max(start.0));
    match part {
        Part::Lower => lower(),
        Part::Upper => upper(),
        Part::Either if x < origin => lower(),
        Part::Either if x > origin => upper(),
        Part::Either => start,
        Part::Span if x == origin => (lo, hi),
        Part::Span => {
            let width = hi - lo;
            let lo = scale.snap(lo + track.value_per(x - origin)).min(scale.max - width).max(scale.min);
            (lo, (lo + width).min(scale.max))
        }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Range slider"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "controls/range-slider/range",
        level: Level::Controls,
        component: "Range slider",
        name: "Range",
        args: || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("disabled", false),
        show,
    }]
}

/// A continuous range and one in steps of 5.
fn show(ui: &mut Ui, args: &mut Args) {
    let (size, enabled) = (size_arg(args.choice("size")), !args.bool("disabled"));
    with_values(ui, "range-slider", [(20.0, 60.0), (25.0, 75.0)], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            for (i, (name, value)) in ["Levels", "Steps of 5"].into_iter().zip(values.iter_mut()).enumerate() {
                let text = format!("{:.1} to {:.1}", value.0, value.1);
                labelled(ui, name, &text, |ui| {
                    let mut slider = RangeSlider::new(("range-slider", i), (format!("{name} from"), format!("{name} to")), *value, 0.0..=100.0)
                        .size(size)
                        .enabled(enabled);
                    if i == 1 {
                        slider = slider.step(5.0);
                    }
                    store(&slider.show(ui), value);
                });
            }
        });
    });
}
