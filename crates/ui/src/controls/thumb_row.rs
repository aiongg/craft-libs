//! [`ThumbRow`]: triangle thumbs on one row, each setting a value of its own: a levels editor's
//! input row (black, gamma, white) and output row (black, white).

use std::ops::RangeInclusive;

use craft_theme::States;
use egui::accesskit::Role;
use egui::{
    AsIdSalt, Color32, CornerRadius, Id, IdSalt, InteractOptions, Key, Modifiers, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, Ui,
    Vec2, pos2, vec2,
};

use super::slider::{KEY_FILTER, PressTurn, Scale, Sensing, Track, pointer, press_turn, stepped, tap};
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{FocusRing, Interactive, StateLayer};

/// Width of a thumb's edge, in points.
const EDGE: f32 = 1.0;

/// A thumb row's sizes and colours, built from the levels editor's style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ThumbRowStyle {
    /// Height of the row: `levels.row`.
    pub height: f32,
    /// A triangle's width and height: `levels.thumb_width`, `levels.thumb_height`.
    pub thumb: Vec2,
    /// Distance from a thumb's centre within which a press grabs it: `levels.hit`.
    pub hit: f32,
    /// A thumb's edge: `levels.thumb_edge`.
    pub edge: Color32,
    /// A disabled thumb's edge: `levels.disabled_edge`.
    pub disabled_edge: Color32,
    /// The ring round a thumb, outside its edge: `levels.ring`.
    pub ring: States<Color32>,
    /// `slider.ring_width`.
    pub ring_width: f32,
}

impl ThumbRowStyle {
    /// How far a thumb's centre stays inside each end of the row: half a thumb, so the triangles
    /// stay inside it.
    pub fn inset(&self) -> f32 {
        self.thumb.x / 2.0
    }
}

/// How a thumb's value is kept among the row's other values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Order {
    /// Anywhere in its range: it passes the other thumbs.
    Free,
    /// At least `gap` above the value of thumb `below` and below that of thumb `above`. A drag
    /// stops there; it doesn't push the other thumb.
    Between { below: Option<usize>, above: Option<usize>, gap: f64 },
}

/// Where a thumb sits that a function of the row's values places (a levels gamma thumb, between
/// the black and the white one). The functions get the row's values as drawn.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Follow {
    /// The row value (a level on the row's range) the thumb sits at, for its value.
    pub at: fn(&[f64], f64) -> f64,
    /// The thumb's value that puts it at row value `x`, unsnapped and finite.
    pub value_at: fn(&[f64], f64) -> f64,
}

/// One thumb of a [`ThumbRow`].
#[derive(Clone, Debug)]
pub(crate) struct Thumb {
    label: String,
    face: Color32,
    /// Its own range and step; the row's when `None`.
    scale: Option<Scale>,
    follow: Option<Follow>,
    order: Order,
    default: Option<f64>,
    unset: Option<f64>,
    falling: bool,
}

impl Thumb {
    /// A thumb named `label` (its accessible name, a whole phrase the app translates) with a
    /// triangle of `face`, whose value is the row's range and step, free among the others.
    pub fn new(label: impl Into<String>, face: Color32) -> Self {
        Self { label: label.into(), face, scale: None, follow: None, order: Order::Free, default: None, unset: None, falling: false }
    }

    /// How the value is kept among the others.
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    /// A value with its own `range` and `step`, which `follow` places on the row.
    pub fn follows(mut self, range: &RangeInclusive<f64>, step: f64, follow: Follow) -> Self {
        self.scale = Some(Scale::new(range, Some(step)));
        self.follow = Some(follow);
        self
    }

    /// The value a double-click on the thumb, or Ctrl/Cmd+Backspace while it has the focus, sets.
    pub fn default_value(mut self, value: f64) -> Self {
        self.default = Some(value);
        self
    }

    /// Where a value that isn't a finite number is drawn; the start of its range by default.
    pub fn unset_at(mut self, value: f64) -> Self {
        self.unset = Some(value);
        self
    }

    /// The value falls as the thumb moves right (a levels gamma): the arrow keys, Page keys, Home,
    /// End and the wheel still move the thumb the way they point.
    pub fn falling(mut self) -> Self {
        self.falling = true;
        self
    }
}

/// Triangle thumbs on one row, tip up, each a value of its own: the rows of a levels editor. It
/// reports the row's values as one array through the edit lifecycle, built like
/// [`RangeSlider`](super::RangeSlider): one gesture on the row, with the thumbs as its focus
/// owners. An edit writes only the edited thumb's value; the others stay as the gesture holds
/// them.
///
/// - Each thumb is a Tab stop and an AccessKit `Slider` named by its label, with its value and
///   bounds.
/// - Pointer (primary button): a press within `levels.hit` of a thumb grabs it, so it moves with
///   the pointer and doesn't jump; a press elsewhere moves the nearest thumb there. Where thumbs
///   overlap under the press, the drag's direction picks: left the first of them in row order,
///   right the last; a still press moves none. A press focuses the thumb it moves (the nearest
///   one while the drag hasn't picked), which keeps the focus after the release. A tap is a
///   [`Set`](crate::edit::EditEvent::Set) to what the same press and release give over two frames.
/// - A double-click on a thumb, or Ctrl/Cmd+Backspace while it has the focus, sets its default.
/// - Keys, the wheel and screen-reader requests move the focused thumb as on a
///   [`Slider`](super::Slider), within its bounds; a [falling](Thumb::falling) thumb's keys and
///   wheel follow the thumb, its screen-reader requests the value.
/// - The row keeps its geometry right to left: values grow to the right.
#[derive(Clone, Debug)]
#[must_use = "show the thumb row with `show`"]
pub(crate) struct ThumbRow<const N: usize> {
    id_salt: IdSalt,
    value: [f64; N],
    scale: Scale,
    thumbs: [Thumb; N],
    width: Option<f32>,
    policy: EditPolicy,
    enabled: bool,
    press_waits: bool,
}

/// What a [`ThumbRow`] shows and reports in a frame.
#[derive(Debug)]
pub(crate) struct ThumbRowOutput<const N: usize> {
    /// The frame's edit event; the response covers the row.
    pub edit: EditResponse<[f64; N]>,
    /// The values drawn: the gesture's, each in its range.
    pub shown: [f64; N],
    /// The gesture's values as they are (the host's while none runs), for fields that show a
    /// value outside the range as it is.
    pub values: [f64; N],
    /// The thumb a pointer drag is moving.
    pub dragged: Option<usize>,
}

/// What a press at a point moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    /// The thumb under the press, by the pointer's movement.
    Grab(usize),
    /// The nearest thumb, to the pointer: the press is off the thumbs.
    Jump(usize),
    /// Overlapping thumbs under the press: a left drag moves the first of them, a right drag the
    /// last. The nearest has the focus until the drag picks.
    Either { first: usize, last: usize, nearest: usize },
}

impl Press {
    /// The thumb the press focuses before the pointer moves.
    fn focus(self) -> usize {
        match self {
            Press::Grab(i) | Press::Jump(i) => i,
            Press::Either { nearest, .. } => nearest,
        }
    }

    /// The thumb the press moves with the pointer at `x`, from `origin`.
    fn thumb(self, origin: f32, x: f32) -> Option<usize> {
        match self {
            Press::Grab(i) | Press::Jump(i) => Some(i),
            Press::Either { first, .. } if x < origin => Some(first),
            Press::Either { last, .. } if x > origin => Some(last),
            Press::Either { .. } => None,
        }
    }

    /// The one thumb the press moves, before the drag picks among overlapping ones.
    fn single(self) -> Option<usize> {
        match self {
            Press::Grab(i) | Press::Jump(i) => Some(i),
            Press::Either { .. } => None,
        }
    }

    /// Whether the press lights thumb `i`.
    fn lights(self, i: usize) -> bool {
        match self {
            Press::Grab(t) | Press::Jump(t) => t == i,
            Press::Either { first, last, .. } => (first..=last).contains(&i),
        }
    }
}

/// The row's thumbs on its track.
struct Layout<'a, const N: usize> {
    thumbs: &'a [Thumb; N],
    scale: Scale,
    track: Track,
    /// Thumbs whose centres are closer than this overlap: a triangle's width.
    overlap: f32,
}

impl<const N: usize> Layout<'_, N> {
    fn scale_of(&self, i: usize) -> Scale {
        self.thumbs[i].scale.unwrap_or(self.scale)
    }

    /// The values as drawn: each in its range, and one that isn't finite where its thumb says.
    fn placed(&self, value: &[f64; N]) -> [f64; N] {
        std::array::from_fn(|i| {
            let scale = self.scale_of(i);
            match value[i] {
                v if v.is_finite() => scale.place(v),
                _ => self.thumbs[i].unset.map_or(scale.min, |u| scale.place(u)),
            }
        })
    }

    /// The row value thumb `i` sits at.
    fn at(&self, values: &[f64; N], i: usize) -> f64 {
        self.thumbs[i].follow.map_or(values[i], |f| (f.at)(values, values[i]))
    }

    fn xs(&self, values: &[f64; N]) -> [f32; N] {
        std::array::from_fn(|i| self.track.x_of(self.at(values, i)))
    }

    /// Where thumb `i` may go, with the others at `values`.
    fn bounds(&self, values: &[f64; N], i: usize) -> (f64, f64) {
        let scale = self.scale_of(i);
        let Order::Between { below, above, gap } = self.thumbs[i].order else { return (scale.min, scale.max) };
        let side = |j: Option<usize>, gap: f64, end: f64| j.and_then(|j| values.get(j)).map_or(end, |v| (v + gap).clamp(scale.min, scale.max));
        let (lo, hi) = (side(below, gap, scale.min), side(above, -gap, scale.max));
        (lo.min(hi), hi)
    }

    /// What a press at `x` moves, with the thumbs at `values`.
    fn press(&self, values: &[f64; N], x: f32) -> Option<Press> {
        let xs = self.xs(values);
        let d = |i: usize| (x - xs[i]).abs();
        let nearest = (0..N).min_by(|&a, &b| d(a).total_cmp(&d(b)))?;
        let at = xs[nearest];
        let on = d(nearest) <= self.track.reach;
        // Off the thumbs, thumbs stacked at one place: the side of the press picks, as a drag's
        // direction does on them.
        let group = |i: usize| if on { d(i) <= self.track.reach && (xs[i] - at).abs() < self.overlap } else { xs[i] == at };
        let first = (0..N).find(|&i| group(i)).unwrap_or(nearest);
        let last = (0..N).rev().find(|&i| group(i)).unwrap_or(nearest);
        Some(match (on, first == last) {
            (true, true) => Press::Grab(nearest),
            (true, false) => Press::Either { first, last, nearest },
            (false, _) if x < at => Press::Jump(first),
            (false, _) => Press::Jump(last),
        })
    }

    /// The values while `press` at `origin` is dragged to `x`: the moved thumb from `anchor`, the
    /// values the drag began from, the others at `raw`, the host's values when the gesture began
    /// (a gesture the host ended carries on from them, so a value set meanwhile stays). A grabbed
    /// thumb moves by the pointer's movement and stays where it was while the pointer is still; a
    /// jumping one goes to the pointer. The moved value snaps to its step and stays in its bounds
    /// among the others.
    fn moved(&self, anchor: &[f64; N], raw: &[f64; N], press: Press, origin: f32, x: f32) -> [f64; N] {
        let Some(i) = press.thumb(origin, x) else { return *raw };
        let grab = !matches!(press, Press::Jump(_));
        let mut out = *raw;
        if grab && x == origin {
            out[i] = anchor[i];
            return out;
        }
        let start = &self.placed(raw);
        let at = if grab { self.at(&self.placed(anchor), i) + self.track.value_per(x - origin) } else { self.track.value_at(x) };
        let value = self.thumbs[i].follow.map_or(at, |f| (f.value_at)(start, at));
        let (lo, hi) = self.bounds(start, i);
        out[i] = self.scale_of(i).snap(value).clamp(lo, hi);
        out
    }

    /// The values `raw` with thumb `i` at its default, if it has one.
    fn reset(&self, raw: &[f64; N], i: usize) -> Option<[f64; N]> {
        let default = self.thumbs[i].default.filter(|d| d.is_finite())?;
        let (lo, hi) = self.bounds(&self.placed(raw), i);
        let mut out = *raw;
        out[i] = self.scale_of(i).snap(default).clamp(lo, hi);
        Some(out)
    }
}

impl<const N: usize> ThumbRow<N> {
    /// A row of `thumbs` showing the host's values (`value[i]` is thumb `i`'s) on `range`, read as
    /// [`Slider::new`](super::Slider::new) reads it, with the `step` levels snap to. `id_salt`
    /// keeps the row's gesture and its thumbs' focus when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, value: [f64; N], range: &RangeInclusive<f64>, step: Option<f64>, thumbs: [Thumb; N]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            scale: Scale::new(range, step),
            thumbs,
            width: None,
            policy: EditPolicy::default(),
            enabled: true,
            press_waits: false,
        }
    }

    /// The row's width; the available width by default.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// How the gesture reports ([`EditPolicy`]).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether a press waits in this pass ([`press_turn`]), for a row that is a part of a
    /// composite reporting through one edit.
    pub fn press_waits(mut self, waits: bool) -> Self {
        self.press_waits = waits;
        self
    }

    /// Shows the row in `style` and returns what it reports.
    pub fn show(self, ui: &mut Ui, style: &ThumbRowStyle) -> ThumbRowOutput<N> {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (_, rect) = ui.allocate_space(vec2(width, style.height));
        let id = ui.make_persistent_id(self.id_salt);
        let options = InteractOptions { enabled: self.enabled, ..Default::default() };
        let response = ui.interact_opt(rect, id, Sense::DRAG, options);
        let sensing = Sensing::of(&response);
        let layout = Layout {
            thumbs: &self.thumbs,
            scale: self.scale,
            track: Track::inset(rect, style.inset(), style.hit, self.scale),
            overlap: style.thumb.x,
        };
        let ids: [Id; N] = std::array::from_fn(|i| Interactive::id_in(ui, id.with(("thumb", i))));
        let finite = self.value.iter().all(|v| v.is_finite());

        // The gesture lives on the row, which takes every press; the focus goes to the thumbs.
        let mut gesture = Gesture::load(ui, id, &self.value, self.policy).focus_owners(&ids).event_filter(KEY_FILTER).enabled(sensing.enabled);
        let (focus, active) = pointer_input(ui, &response, &layout, &mut gesture, PressInput { sensing, waits: self.press_waits, finite });
        let dragged = active.and_then(Press::single);
        let mut focus = focus.map(|i| ids[i]);
        if response.drag_stopped() && focus.is_none() {
            // A release away from the focused thumb would take its focus (egui's click-away).
            focus = ids.into_iter().find(|t| ui.memory(|m| m.has_focus(*t)));
        }

        for (i, &thumb) in ids.iter().enumerate() {
            let reset = layout.reset(gesture.shown(), i).filter(|_| sensing.enabled && ui.memory(|m| m.has_focus(thumb)));
            // Taken during a drag too, which it doesn't change, as the arrows are.
            if let Some(reset) = reset
                && ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Backspace))
                && !sensing.dragging
            {
                gesture.set(reset);
            }
            let values = layout.placed(gesture.shown());
            let sensing = Sensing { falling: self.thumbs[i].falling, ..sensing };
            if let Some(v) = stepped(ui, thumb, sensing, None, values[i], layout.bounds(&values, i), &layout.scale_of(i)) {
                let mut out = *gesture.shown();
                out[i] = v;
                gesture.set(out);
            }
        }

        let values = *gesture.shown();
        let shown = layout.placed(&values);
        let xs = layout.xs(&shown);
        let thumb_rect = |x: f32| Rect::from_min_size(pos2(x - style.thumb.x / 2.0, rect.top()), style.thumb);
        let thumbs: [Response; N] = std::array::from_fn(|i| {
            let (lo, hi) = layout.bounds(&shown, i);
            Interactive::new(id.with(("thumb", i)), Role::Slider, self.thumbs[i].label.as_str())
                .sense(Sense::FOCUSABLE)
                .enabled(sensing.enabled)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, thumb_rect(xs[i]))
                .numeric(shown[i], lo..=hi, layout.scale_of(i).key_step())
                .finish(ui)
        });
        if let Some(thumb) = focus
            && !ui.memory(|m| m.has_focus(thumb))
        {
            ui.memory_mut(|m| m.request_focus(thumb));
        }

        if ui.is_rect_visible(rect) {
            // The thumbs a press would move light up while hovered, and the dragged one while pressed.
            let lit = active.or_else(|| response.hover_pos().and_then(|p| layout.press(&shown, p.x)));
            let down = response.is_pointer_button_down_on();
            for (i, thumb) in thumbs.iter().enumerate() {
                let lit = lit.is_some_and(|p| p.lights(i));
                let paint = TrianglePaint {
                    face: self.thumbs[i].face,
                    edge: if sensing.enabled { style.edge } else { style.disabled_edge },
                    ring: StateLayer::color_of(ui, &style.ring, thumb.id, sensing.enabled, lit, lit && down),
                };
                let bounds = paint_triangle(ui, style, pos2(xs[i], rect.top()), paint);
                FocusRing::paint_radius(ui, bounds, CornerRadius::ZERO, thumb);
            }
        }
        ThumbRowOutput { edit: EditResponse::new(response, gesture.finish()), shown, values, dragged }
    }
}

/// What the row's pointer input needs to know besides the pointer.
#[derive(Clone, Copy, Debug)]
struct PressInput {
    sensing: Sensing,
    /// A press waits in this pass ([`press_turn`]).
    waits: bool,
    /// Every host value is a finite number, so a press may begin a gesture.
    finite: bool,
}

/// Presses, drags, the release and taps on the row: the thumb to focus, and what a drag under way
/// moves (the thumb it picked, or overlapping thumbs while it hasn't).
fn pointer_input<const N: usize>(
    ui: &Ui,
    response: &Response,
    layout: &Layout<'_, N>,
    gesture: &mut Gesture<[f64; N]>,
    input: PressInput,
) -> (Option<usize>, Option<Press>) {
    let modifiers = ui.input(|i| i.modifiers);
    let tapped = tap(ui, response);
    let pressed = response.drag_started_by(PointerButton::Primary).then(|| pointer(ui, response).map(|(_, origin)| origin)).flatten();
    let turn = press_turn(ui, response.id, input.waits, pressed.or(tapped));
    let mut focus = None;
    if let PressTurn::Held(at) = turn
        && input.finite
    {
        gesture.begin();
        if let Some(anchor) = gesture.anchor().copied()
            && let Some(press) = layout.press(&layout.placed(&anchor), at.x)
        {
            focus = Some(press.focus());
            gesture.update(layout.moved(&anchor, &anchor, press, at.x, at.x), modifiers);
        }
        if !response.dragged_by(PointerButton::Primary) {
            gesture.commit();
        }
    }
    let waits = matches!(turn, PressTurn::Waits);
    if let Some(origin) = pressed {
        focus = layout.press(&layout.placed(gesture.shown()), origin.x).map(Press::focus);
        if input.finite && !waits {
            gesture.begin();
        }
    }
    let mut active = None;
    if response.dragged_by(PointerButton::Primary)
        && let Some(anchor) = gesture.anchor().copied()
        && let Some(start) = gesture.start().copied()
        && let Some((pos, origin)) = pointer(ui, response)
        && let Some(press) = layout.press(&layout.placed(&anchor), origin.x)
    {
        active = Some(press);
        if let (Press::Either { .. }, Some(i)) = (press, press.thumb(origin.x, pos.x)) {
            focus = Some(i);
            active = Some(Press::Grab(i));
        }
        gesture.update(layout.moved(&anchor, &start, press, origin.x, pos.x), modifiers);
    }
    // The reset ends the second click's gesture, or is its own Set.
    let double = double_click(ui, response, layout, gesture, input.sensing);
    if response.drag_stopped() {
        gesture.commit();
    }
    if let Some(at) = tapped {
        let raw = *gesture.shown();
        if let Some(press) = layout.press(&layout.placed(&raw), at.x) {
            focus = Some(press.focus());
            if double.is_none() && !waits {
                gesture.set(layout.moved(&raw, &raw, press, at.x, at.x));
            }
        }
    }
    (double.or(focus), active)
}

/// A double-click's second click on the row sets the thumb under it to its default: the first
/// click has moved the nearest thumb there. Returns the thumb.
fn double_click<const N: usize>(
    ui: &Ui,
    response: &Response,
    layout: &Layout<'_, N>,
    gesture: &mut Gesture<[f64; N]>,
    sensing: Sensing,
) -> Option<usize> {
    if !(sensing.enabled && sensing.hovered && ui.input(|i| i.pointer.button_double_clicked(PointerButton::Primary))) {
        return None;
    }
    let i = layout.press(&layout.placed(gesture.shown()), response.hover_pos()?.x)?.focus();
    gesture.set(layout.reset(gesture.shown(), i)?);
    Some(i)
}

/// A thumb's colours now.
#[derive(Clone, Copy, Debug)]
struct TrianglePaint {
    face: Color32,
    edge: Color32,
    ring: Color32,
}

/// Paints a triangle thumb, tip up at `tip`, and returns its bounding rect.
fn paint_triangle(ui: &Ui, style: &ThumbRowStyle, tip: Pos2, paint: TrianglePaint) -> Rect {
    let (w, h) = (style.thumb.x, style.thumb.y);
    let points = [tip, pos2(tip.x + w / 2.0, tip.y + h), pos2(tip.x - w / 2.0, tip.y + h)];
    if paint.ring != Color32::TRANSPARENT {
        // The triangle grown by the ring and the outer half of the edge on every side: its sides
        // move out by `out` when it scales about its incentre by (r + out) / r.
        let side = (w * w / 4.0 + h * h).sqrt();
        let r = w * h / (w + 2.0 * side);
        let centre = pos2(tip.x, tip.y + h - r);
        let k = (r + EDGE / 2.0 + style.ring_width) / r;
        let ring = points.map(|p| centre + (p - centre) * k).to_vec();
        ui.painter().add(Shape::convex_polygon(ring, paint.ring, Stroke::NONE));
    }
    ui.painter().add(Shape::convex_polygon(points.to_vec(), paint.face, Stroke::new(EDGE, paint.edge)));
    Rect::from_min_size(pos2(tip.x - w / 2.0, tip.y), vec2(w, h))
}
