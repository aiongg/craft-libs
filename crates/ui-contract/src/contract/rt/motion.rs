//! Motion on paint-time reads (docs/contract-rt.md sections 8 and 10, "Motion"), and the one tween
//! type every motion runs on: the paint-time fades here and the named motions of a run
//! (`runtime::motion::Animate`).
//!
//! A run decides targets; ui-rt's paint phase evaluates the animation. A node keeps what a run
//! recorded (paint variants, a target opacity) and the paint phase shows the value at the frame's
//! time: a fade between two kept variants, an opacity tween. While an animation runs the node reads
//! the frame time in paint, so ui-rt repaints that node (and nothing else) each frame and asks the
//! platform for frames; the paint that finds it finished reads nothing, so no frame is asked for
//! and a finished animation costs nothing. No hook runs at any point.
//!
//! The mode (full, reduced, off) and the durations come from env cells read in paint, so a change of
//! either repaints the animating nodes only.

use std::cell::RefCell;

use craft_behavior::{MotionKind, MotionMode, Playback};
use craft_theme::{Motion, Rgba, Tokens};
use craft_ui_rt::{DisplayList, Item, PaintCx, Pos2, Rect, Stroke, Vec2};

use super::env::EnvCells;
use super::node::{Kept, Pointer, Variant};
pub(crate) use crate::runtime::motion::{MotionSpec, spec};

/// A value a tween can run over.
pub trait Interpolate: Copy + PartialEq + 'static {
    /// The value `t` of the way from `a` to `b`; `t` is 0 to 1.
    fn lerp(a: Self, b: Self, t: f32) -> Self;
    /// How far apart two values are, in the value's own unit.
    fn distance(a: Self, b: Self) -> f32;
}

impl Interpolate for f32 {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        a + (b - a) * t
    }

    fn distance(a: Self, b: Self) -> f32 {
        (a - b).abs()
    }
}

impl Interpolate for Vec2 {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        a + (b - a) * t
    }

    fn distance(a: Self, b: Self) -> f32 {
        (a - b).length()
    }
}

impl Interpolate for Pos2 {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        a + (b - a) * t
    }

    fn distance(a: Self, b: Self) -> f32 {
        (a - b).length()
    }
}

impl Interpolate for Rect {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        Rect::from_min_max(<Pos2 as Interpolate>::lerp(a.min, b.min, t), <Pos2 as Interpolate>::lerp(a.max, b.max, t))
    }

    fn distance(a: Self, b: Self) -> f32 {
        <Pos2 as Interpolate>::distance(a.min, b.min).max(<Pos2 as Interpolate>::distance(a.max, b.max))
    }
}

/// A distance under this ends a follow; a thousandth of a point or of a unit fraction is below anything painted.
const FOLLOW_EPSILON: f32 = 0.001;

/// Time constants a follow's settle time spans: 95% of the way after the motion's duration.
const FOLLOW_TIME_CONSTANTS: f32 = 3.0;

/// A running animation of one value. It starts at the first evaluation after it was retargeted, so
/// a paint-time tween's run names only the target. Evaluating at the same time twice gives the
/// same value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween<T: Interpolate> {
    from: T,
    to: T,
    started: Option<f64>,
    shown: T,
}

impl<T: Interpolate> Tween<T> {
    /// A tween at rest at `value`.
    pub fn at(value: T) -> Self {
        Tween { from: value, to: value, started: None, shown: value }
    }

    /// The value it runs toward.
    pub fn target(&self) -> T {
        self.to
    }

    /// The value last evaluated.
    pub fn shown(&self) -> T {
        self.shown
    }

    /// Whether the shown value is not yet the target.
    pub fn is_running(&self) -> bool {
        self.shown != self.to
    }

    /// Runs toward `to` from the value shown, from the next evaluation on.
    pub fn retarget(&mut self, to: T) {
        if to != self.to {
            self.from = self.shown;
            self.to = to;
            self.started = None;
        }
    }

    /// The value at `now`. Under a playback that does not run, or with no duration, it is the target.
    pub fn value_at(&mut self, now: f64, motion: Motion, playback: Playback) -> T {
        if self.shown == self.to {
            return self.to;
        }
        let started = *self.started.get_or_insert(now);
        let seconds = f64::from(motion.millis) / 1000.0;
        let progress = if playback != Playback::Run || seconds <= 0.0 { 1.0 } else { ((now - started) / seconds) as f32 };
        self.shown = if progress >= 1.0 { self.to } else { T::lerp(self.from, self.to, motion.easing.at(progress.max(0.0))) };
        self.shown
    }

    /// The value at `now` of a follow toward `to`, a target that may move every frame: exponential
    /// smoothing from the value shown, with the motion's duration as its settle time, so a moved
    /// target is followed from where the value is and never restarts. The first evaluation after a
    /// rest moves nothing and starts the clock. Under a playback that does not run, or with no
    /// duration, it is the target.
    pub fn follow(&mut self, to: T, now: f64, motion: Motion, playback: Playback) -> T {
        self.to = to;
        let seconds = f64::from(motion.millis) / 1000.0;
        if self.shown == to || playback != Playback::Run || seconds <= 0.0 {
            self.shown = to;
            self.started = None;
            return to;
        }
        let last = self.started.replace(now).unwrap_or(now);
        let dt = (now - last).max(0.0) as f32;
        let rate = 1.0 - (-dt * FOLLOW_TIME_CONSTANTS / seconds as f32).exp();
        let next = T::lerp(self.shown, to, rate);
        self.shown = if T::distance(next, to) < FOLLOW_EPSILON { to } else { next };
        self.shown
    }
}

/// How far into a loop of `period` seconds the frame time `now` is, 0 to 1: the paint-time read of a
/// spinner or marching ants. `None` when the mode paints loops still.
pub fn loop_phase(mode: MotionMode, now: f64, period: f64) -> Option<f32> {
    (mode.playback(MotionKind::Loop) == Playback::Run && period > 0.0).then(|| (now / period).fract() as f32)
}

/// What a node's paint knows about its fade between pointer looks.
#[derive(Default)]
pub(crate) struct Fade {
    /// The look the node shows or fades to; `None` before the first paint, which shows its look at
    /// once.
    to: Option<Pointer>,
    /// The lists the fade runs from.
    from: Option<(DisplayList, DisplayList)>,
    started: Option<f64>,
}

/// The target opacity a run set and the tween that shows it.
pub(crate) struct Level {
    pub(crate) tween: Tween<f32>,
    pub(crate) spec: MotionSpec,
}

impl Level {
    /// Aims the tween at `target`; the first target is shown at once.
    pub(crate) fn aim(slot: &mut Option<Level>, target: f32, spec: MotionSpec) {
        match slot {
            Some(level) => {
                level.tween.retarget(target);
                level.spec = spec;
            }
            None => *slot = Some(Level { tween: Tween::at(target), spec }),
        }
    }
}

fn variant(kept: &Kept, look: Pointer) -> &Variant {
    kept.variants.iter().find(|v| v.look == look).unwrap_or(&kept.variants[0])
}

/// Paints the node's kept looks for `look`, blended while a fade runs; reads the frame time only
/// while it does.
pub(crate) fn paint_looks(kept: &Kept, cx: &mut PaintCx<'_>, look: Pointer) {
    if kept.variants.len() < 2 {
        if let Some(v) = kept.variants.first() {
            extend(cx, &v.content, &v.overlay);
        }
        return;
    }
    let (motion, playback) = fade_motion(cx);
    let now = cx.time_until(|_| None);
    let mut fade = kept.fade.borrow_mut();
    let target = variant(kept, look);
    if fade.to.is_some_and(|prev| prev != look) {
        let shown = shown_lists(&fade, kept, now, motion, playback);
        fade.from = Some(shown);
        fade.started = None;
    }
    fade.to = Some(look);
    let t = match (&fade.from, playback) {
        (None, _) | (_, Playback::Snap) => None,
        (Some(_), _) => {
            let started = *fade.started.get_or_insert(now);
            let seconds = f64::from(motion.millis) / 1000.0;
            let progress = if seconds > 0.0 { ((now - started) / seconds) as f32 } else { 1.0 };
            (progress < 1.0).then_some(motion.easing.at(progress.max(0.0)))
        }
    };
    match (&fade.from, t) {
        (Some((content, overlay)), Some(t)) => {
            cx.time();
            let content = blend_lists(content, &target.content, t);
            let overlay = blend_lists(overlay, &target.overlay, t);
            extend(cx, &content, &overlay);
        }
        _ => {
            fade.from = None;
            fade.started = None;
            extend(cx, &target.content, &target.overlay);
        }
    }
}

/// The lists the node shows at `now`: the running fade's blend, or the settled look's lists.
fn shown_lists(fade: &Fade, kept: &Kept, now: f64, motion: Motion, playback: Playback) -> (DisplayList, DisplayList) {
    let settled = |look| {
        let v = variant(kept, look);
        (v.content.clone(), v.overlay.clone())
    };
    let Some(to) = fade.to else { return settled(Pointer::Rest) };
    let (Some((content, overlay)), Some(started)) = (&fade.from, fade.started) else { return settled(to) };
    let seconds = f64::from(motion.millis) / 1000.0;
    let progress = if playback == Playback::Snap || seconds <= 0.0 { 1.0 } else { ((now - started) / seconds) as f32 };
    if progress >= 1.0 {
        return settled(to);
    }
    let t = motion.easing.at(progress.max(0.0));
    let v = variant(kept, to);
    (blend_lists(content, &v.content, t), blend_lists(overlay, &v.overlay, t))
}

fn extend(cx: &mut PaintCx<'_>, content: &DisplayList, overlay: &DisplayList) {
    cx.extend(content);
    if !overlay.is_empty() {
        cx.over(|cx| cx.extend(overlay));
    }
}

/// The hover motion and what the env's mode makes of a fade; both read in paint.
fn fade_motion(cx: &mut PaintCx<'_>) -> (Motion, Playback) {
    match cx.provided::<EnvCells>() {
        Some(cells) => {
            let tokens = cells.tokens.get(cx);
            let mode = cells.motion.get(cx);
            (spec::HOVER.motion_of(&tokens), mode.playback(MotionKind::Fade))
        }
        None => {
            let tokens = cx.tokens();
            (spec::HOVER.motion_of(&tokens), Playback::Run)
        }
    }
}

/// Applies the node's opacity tween to `cx` and reads the frame time while it runs.
pub(crate) fn paint_level(kept: &Kept, cx: &mut PaintCx<'_>) {
    let mut level = kept.level.borrow_mut();
    let Some(level) = level.as_mut() else { return };
    if level.tween.target() == 1.0 && !level.tween.is_running() {
        return;
    }
    let (spec, tokens, mode) = match cx.provided::<EnvCells>() {
        Some(cells) => (level.spec, cells.tokens.get(cx), cells.motion.get(cx)),
        None => (level.spec, cx.tokens(), MotionMode::Full),
    };
    let now = cx.time_until(|_| None);
    let opacity = level.tween.value_at(now, spec.motion_of(&tokens), mode.playback(spec.kind()));
    if level.tween.is_running() {
        cx.time();
    }
    cx.opacity(opacity);
}

/// The two lists blended `t` of the way: item by item when their structure matches (the same items
/// with other colours, strokes or opacities), otherwise `from` until the middle and `to` after it.
pub(crate) fn blend_lists(from: &DisplayList, to: &DisplayList, t: f32) -> DisplayList {
    if from.items.len() == to.items.len() {
        let items: Option<Vec<Item>> = from.items.iter().zip(&to.items).map(|(a, b)| blend_item(a, b, t)).collect();
        if let Some(items) = items {
            return DisplayList { items };
        }
    }
    if t < 0.5 { from.clone() } else { to.clone() }
}

fn mix_f(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// A colour blended in premultiplied sRGB, as the stateful looks blend.
fn mix_rgba(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let pre = |c: Rgba| {
        let alpha = f32::from(c.0[3]) / 255.0;
        [f32::from(c.0[0]) * alpha, f32::from(c.0[1]) * alpha, f32::from(c.0[2]) * alpha, f32::from(c.0[3])]
    };
    let (pa, pb) = (pre(a), pre(b));
    let m = [0, 1, 2, 3].map(|i| mix_f(pa[i], pb[i], t));
    if m[3] <= 0.0 {
        return Rgba::TRANSPARENT;
    }
    let alpha = m[3] / 255.0;
    let ch = |v: f32| (v / alpha).round().clamp(0.0, 255.0) as u8;
    Rgba([ch(m[0]), ch(m[1]), ch(m[2]), m[3].round().clamp(0.0, 255.0) as u8])
}

fn mix_stroke(a: Option<Stroke>, b: Option<Stroke>, t: f32) -> Option<Option<Stroke>> {
    match (a, b) {
        (None, None) => Some(None),
        (Some(a), Some(b)) if a.align == b.align && a.dash == b.dash => {
            Some(Some(Stroke { width: mix_f(a.width, b.width, t), color: mix_rgba(a.color, b.color, t), ..a }))
        }
        _ => None,
    }
}

fn blend_item(a: &Item, b: &Item, t: f32) -> Option<Item> {
    Some(match (a, b) {
        (Item::Rect { rect: ra, radii: qa, fill: fa, stroke: sa }, Item::Rect { rect: rb, radii: qb, fill: fb, stroke: sb })
            if ra == rb && qa == qb =>
        {
            Item::Rect { rect: *ra, radii: *qa, fill: mix_rgba(*fa, *fb, t), stroke: mix_stroke(*sa, *sb, t)? }
        }
        (Item::Circle { center: ca, radius: ra, fill: fa, stroke: sa }, Item::Circle { center: cb, radius: rb, fill: fb, stroke: sb })
            if ca == cb && ra == rb =>
        {
            Item::Circle { center: *ca, radius: *ra, fill: mix_rgba(*fa, *fb, t), stroke: mix_stroke(*sa, *sb, t)? }
        }
        (Item::Line { from: fa, to: ta, stroke: sa }, Item::Line { from: fb, to: tb, stroke: sb }) if fa == fb && ta == tb => {
            Item::Line { from: *fa, to: *ta, stroke: mix_stroke(Some(*sa), Some(*sb), t)?? }
        }
        (
            Item::Shadow { rect: ra, radii: qa, offset: oa, blur: ba, spread: pa, color: ca },
            Item::Shadow { rect: rb, radii: qb, offset: ob, blur: bb, spread: pb, color: cb },
        ) if ra == rb && qa == qb && oa == ob && ba == bb && pa == pb => {
            Item::Shadow { rect: *ra, radii: *qa, offset: *oa, blur: *ba, spread: *pa, color: mix_rgba(*ca, *cb, t) }
        }
        (Item::Image { key: ka, rect: ra, uv: ua, tint: ca }, Item::Image { key: kb, rect: rb, uv: ub, tint: cb })
            if ka == kb && ra == rb && ua == ub =>
        {
            Item::Image { key: *ka, rect: *ra, uv: *ua, tint: mix_rgba(*ca, *cb, t) }
        }
        (Item::ImageMask { key: ka, rect: ra, uv: ua, color: ca }, Item::ImageMask { key: kb, rect: rb, uv: ub, color: cb })
            if ka == kb && ra == rb && ua == ub =>
        {
            Item::ImageMask { key: *ka, rect: *ra, uv: *ua, color: mix_rgba(*ca, *cb, t) }
        }
        (Item::VGradient { rect: ra, top: ta, bottom: ba }, Item::VGradient { rect: rb, top: tb, bottom: bb }) if ra == rb => {
            Item::VGradient { rect: *ra, top: mix_rgba(*ta, *tb, t), bottom: mix_rgba(*ba, *bb, t) }
        }
        // Two paints of one text shape it to equal layouts: the later handle stands for both.
        (
            Item::Text { origin: oa, size: sa, color: ca, highlight: ha, angle: aa, .. },
            Item::Text { layout, origin: ob, size: sb, color: cb, highlight: hb, angle: ab },
        ) if oa == ob && sa == sb && ha == hb && aa == ab => {
            let color = match (ca, cb) {
                (Some(a), Some(b)) => Some(mix_rgba(*a, *b, t)),
                (a, b) if a == b => *a,
                _ => return None,
            };
            Item::Text { layout: layout.clone(), origin: *ob, size: *sb, color, highlight: hb.clone(), angle: *ab }
        }
        (Item::Path { path: pa, fill: fa, stroke: sa }, Item::Path { path: pb, fill: fb, stroke: sb }) if std::sync::Arc::ptr_eq(pa, pb) => {
            let fill = match (fa, fb) {
                (Some(a), Some(b)) => Some(craft_ui_rt::Fill { color: mix_rgba(a.color, b.color, t), ..*a }),
                (None, None) => None,
                _ => return None,
            };
            Item::Path { path: pa.clone(), fill, stroke: mix_stroke(*sa, *sb, t)? }
        }
        (Item::Mesh(a), Item::Mesh(b)) if std::sync::Arc::ptr_eq(a, b) => Item::Mesh(a.clone()),
        (Item::Surface { key: ka, rect: ra }, Item::Surface { key: kb, rect: rb }) if ka == kb && ra == rb => b.clone(),
        (Item::PushClip(a), Item::PushClip(b)) if a == b => Item::PushClip(*a),
        (Item::PopClip, Item::PopClip) => Item::PopClip,
        (Item::PushOpacity { opacity: oa, bounds: ba }, Item::PushOpacity { opacity: ob, bounds: bb }) if ba == bb => {
            Item::PushOpacity { opacity: mix_f(*oa, *ob, t), bounds: *ba }
        }
        (Item::PopOpacity, Item::PopOpacity) => Item::PopOpacity,
        _ => return None,
    })
}

/// The tween state a node keeps: its fade between pointer looks and its opacity tween.
pub(crate) type FadeCell = RefCell<Fade>;
