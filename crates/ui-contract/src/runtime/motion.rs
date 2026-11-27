//! The motion service (13): the motion constants, and the motions a component's runs drive. A motion lives in the
//! node's memory, so it goes with the node. A run names a target and gets the value at the frame time; while a value
//! it showed still moves, the run asks its node to build again in the next frame (the run's wake reaches
//! `BuildCx::request_build_at` of the running node), so only the animating node runs, once per frame, and a settled
//! one asks for nothing. Timed wakes (`wake_at`, a stepped loop, a caret's blink) ask for the build at their moment
//! only. Paint-time motion (the fades between pointer looks, `fade_to`) is in `contract::rt::motion`, on the same
//! tween type.

use crate::contract::Context;
use crate::primitives::Id;
use crate::runtime::env::MotionPolicy;
use crate::runtime::memory::{MemoryStage, sub};
use craft_behavior::{MotionKind, Playback};
use craft_theme::{Motion, Tokens};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::hash::Hash;
use std::time::Duration;

pub use crate::contract::rt::motion::{Interpolate, Tween, loop_phase};
/// The instant type [`now`] returns: `std::time::Instant` on native targets, a clock that works in the browser on
/// wasm.
pub use web_time::Instant;

/// Which token of `MotionTokens` gives a spec its duration and curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::contract::MemoProps)]
enum Token {
    Hover,
    Open,
    Move,
    Caret,
    Outline,
}

/// How a value approaches its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::contract::MemoProps)]
pub(crate) enum Curve {
    /// From where it was to the new target over the token's duration.
    Tween,
    /// Toward a target that moves every frame, by exponential smoothing with the token's duration as its settle time.
    Follow,
}

/// A motion constant: a component picks one from `spec`, never writes seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::contract::MemoProps)]
pub struct MotionSpec {
    token: Token,
    kind: MotionKind,
    curve: Curve,
}

impl MotionSpec {
    const fn new(token: Token, kind: MotionKind, curve: Curve) -> Self {
        MotionSpec { token, kind, curve }
    }

    /// The kind that picks this spec's row of the motion table.
    pub const fn kind(self) -> MotionKind {
        self.kind
    }

    /// The motion of this spec's token in `tokens`: what ui-rt's paint phase evaluates at the frame's time.
    pub(crate) fn motion_of(self, tokens: &Tokens) -> Motion {
        let m = &tokens.motion;
        match self.token {
            Token::Hover => m.hover,
            Token::Open => m.open,
            Token::Move => m.r#move,
            Token::Caret => m.caret,
            Token::Outline => m.outline,
        }
    }

    fn motion(self, policy: &MotionPolicy) -> Motion {
        let tokens = &policy.tokens;
        match self.token {
            Token::Hover => tokens.hover,
            Token::Open => tokens.open,
            Token::Move => tokens.r#move,
            Token::Caret => tokens.caret,
            Token::Outline => tokens.outline,
        }
    }
}

/// The motion constants.
pub mod spec {
    use super::{Curve, MotionKind, MotionSpec, Token};

    /// Hover and press feedback.
    pub const HOVER: MotionSpec = MotionSpec::new(Token::Hover, MotionKind::Fade, Curve::Tween);
    /// A check, a toggle or a selection turning on or off.
    pub const CHECK: MotionSpec = MotionSpec::new(Token::Hover, MotionKind::Fade, Curve::Tween);
    /// A popup's opacity.
    pub const OPEN_FADE: MotionSpec = MotionSpec::new(Token::Open, MotionKind::Fade, Curve::Tween);
    /// A popup's travel while it opens.
    pub const OPEN_RISE: MotionSpec = MotionSpec::new(Token::Open, MotionKind::Travel, Curve::Tween);
    /// An element moving within the screen.
    pub const MOVE: MotionSpec = MotionSpec::new(Token::Move, MotionKind::Travel, Curve::Tween);
    /// A text caret gliding to its new place.
    pub const CARET: MotionSpec = MotionSpec::new(Token::Caret, MotionKind::Travel, Curve::Tween);
    /// A selection outline gliding to its new cells.
    pub const OUTLINE: MotionSpec = MotionSpec::new(Token::Outline, MotionKind::Travel, Curve::Tween);
    /// A target that moves every frame: zoom ease, remote cursors, autoscroll.
    pub const FOLLOW: MotionSpec = MotionSpec::new(Token::Move, MotionKind::Travel, Curve::Follow);
    /// A spinner, marching ants or a pulse.
    pub const LOOP: MotionSpec = MotionSpec::new(Token::Move, MotionKind::Loop, Curve::Tween);
    /// A text caret's blink.
    pub const BLINK: MotionSpec = MotionSpec::new(Token::Caret, MotionKind::Blink, Curve::Tween);
}

/// How long a blinking caret shows or hides its stroke, in seconds: the platform's usual caret rate.
const BLINK_HALF_PERIOD: f64 = 0.53;

/// Moves `tween` toward `target` by `spec` under `policy` and gives its value at `now`; where the mode does not run
/// `spec`'s kind it is at the target at once. Every named motion, keyed or not, steps through here.
fn advance<T: Interpolate>(policy: &MotionPolicy, tween: &mut Tween<T>, target: T, spec: MotionSpec, now: f64) -> T {
    let (motion, playback) = (spec.motion(policy), policy.mode.playback(spec.kind));
    match spec.curve {
        Curve::Tween => {
            tween.retarget(target);
            tween.value_at(now, motion, playback)
        }
        Curve::Follow => tween.follow(target, now, motion, playback),
    }
}

/// An open-close presence: `t` is 0 closed and 1 open, and a closing one stays visible until `t` reaches 0.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Presence {
    /// The animated openness, 0 to 1.
    pub t: f32,
    /// Whether anything of it should be shown.
    pub visible: bool,
    /// Closing but still fading out.
    pub exiting: bool,
}

impl Presence {
    /// The presence of an openness `t` whose target is open or closed.
    fn at(t: f32, open: bool) -> Self {
        Presence { t, visible: open || t > 0.0, exiting: !open && t > 0.0 }
    }
}

/// One key of a keyed presence: the id of its key, its openness, the frame time of the run that last drove it open,
/// and what the caller keeps with it while it shows.
#[derive(Clone)]
struct KeyedEntry<P> {
    key: Id,
    tween: Tween<f32>,
    /// A key that no run at the running frame time drove open is closing.
    open_in: Option<f64>,
    payload: P,
}

/// The keys of one keyed presence name, in the order they first opened, and the frame time the name was first driven.
#[derive(Clone)]
struct Keyed<P> {
    born: f64,
    entries: Vec<KeyedEntry<P>>,
}

impl<P> Keyed<P> {
    fn moving(&self) -> bool {
        self.entries.iter().any(|entry| entry.tween.is_running())
    }
}

/// Steps the keys of `keyed` that `open` does not hold open toward closed at `now`, from where they are; drops the
/// ones that faded out and gives the others with the payload each was last given.
fn closing<P: Clone>(
    keyed: &mut Keyed<P>,
    policy: &MotionPolicy,
    spec: MotionSpec,
    now: f64,
    open: impl Fn(&KeyedEntry<P>) -> bool,
) -> Vec<(Presence, P)> {
    let mut out = Vec::new();
    keyed.entries.retain_mut(|entry| {
        if open(entry) {
            return true;
        }
        entry.open_in = None;
        let presence = Presence::at(advance(policy, &mut entry.tween, 0.0, spec, now), false);
        if presence.visible {
            out.push((presence, entry.payload.clone()));
        }
        presence.visible
    });
    out
}

/// A node's motion memory: its named tweens and keyed presences, by type and slot id. It lives in the node's
/// [`MemoryStage`], so it goes with the node. The hooks drive it through `&self` contexts (a paint gets `&Cx`), so it
/// sits in a cell; a hook holds no borrow of it across calls.
#[derive(Default)]
pub(crate) struct Motions(RefCell<Vec<(TypeId, Id, Box<dyn Any>)>>);

impl Motions {
    /// The slot of type `T` under `id`, moved out.
    fn take<T: 'static>(&self, id: Id) -> Option<T> {
        let mut slots = self.0.borrow_mut();
        let at = slots.iter().position(|(t, i, _)| *t == TypeId::of::<T>() && *i == id)?;
        slots.swap_remove(at).2.downcast::<T>().ok().map(|value| *value)
    }

    /// A copy of the slot of type `T` under `id`.
    fn peek<T: Clone + 'static>(&self, id: Id) -> Option<T> {
        let slots = self.0.borrow();
        slots.iter().find(|(t, i, _)| *t == TypeId::of::<T>() && *i == id).and_then(|(_, _, value)| value.downcast_ref::<T>()).cloned()
    }

    /// Puts `value` in the slot of type `T` under `id`.
    fn put<T: 'static>(&self, id: Id, value: T) {
        let mut slots = self.0.borrow_mut();
        match slots.iter_mut().find(|(t, i, _)| *t == TypeId::of::<T>() && *i == id) {
            Some(slot) => slot.2 = Box::new(value),
            None => slots.push((TypeId::of::<T>(), id, Box::new(value))),
        }
    }
}

/// The wall clock's reading, for a host that drives a store on real time (`ShellApp::tick`). It is the one place
/// outside a run that reads the clock.
pub fn now() -> Instant {
    Instant::now()
}

/// The slot of a component's own `anim` name.
fn anim_slot(base: Id, name: &'static str) -> Id {
    sub(base, ("anim", name))
}

/// The slot of the keyed presence `name` of the component `base`.
fn keyed_slot(base: Id, name: &'static str) -> Id {
    sub(base, ("presence_keyed", name))
}

/// Everything an `Animate` call needs from the context type that makes it: the node's memory stage (its motion
/// memory and the window's context), the component's id the slots hang under, and the motion policy of its scope.
pub(crate) struct Driver<'a> {
    pub(crate) stage: &'a MemoryStage,
    pub(crate) base: Id,
    pub(crate) policy: MotionPolicy,
}

impl Driver<'_> {
    fn motions(&self) -> &Motions {
        &self.stage.motion
    }

    fn context(&self) -> &Context {
        self.stage.ctx()
    }

    /// The motion in slot `id`: moved out for a run, a copy for a measuring run, which stores nothing.
    fn load<T: Clone + 'static>(&self, id: Id) -> Option<T> {
        if self.stage.is_peek() { self.motions().peek(id) } else { self.motions().take(id) }
    }

    /// Puts a motion back after a run; a measuring run stores nothing.
    fn store<T: 'static>(&self, id: Id, value: T) {
        if !self.stage.is_peek() {
            self.motions().put(id, value);
        }
    }

    /// The node builds again in the next frame: a value this run showed is still moving. A measuring run reaches no
    /// node and asks for nothing.
    fn owe(&self) {
        if !self.stage.is_peek() {
            self.context().request_repaint();
        }
    }

    pub(crate) fn anim<T: Interpolate>(&self, name: &'static str, target: T, spec: MotionSpec) -> T {
        let id = anim_slot(self.base, name);
        let mut tween = self.load::<Tween<T>>(id).unwrap_or_else(|| Tween::at(target));
        let shown = advance(&self.policy, &mut tween, target, spec, self.now());
        if tween.is_running() {
            self.owe();
        }
        self.store(id, tween);
        shown
    }

    pub(crate) fn anim_bool(&self, name: &'static str, on: bool, spec: MotionSpec) -> f32 {
        self.anim(name, f32::from(u8::from(on)), spec)
    }

    /// The presence of the unkeyed `name`: the one-key case of `presence_keyed`, in a plain tween slot.
    pub(crate) fn presence(&self, name: &'static str, open: bool, spec: MotionSpec) -> Presence {
        Presence::at(self.anim_bool(name, open, spec), open)
    }

    /// Puts the keys of `name` back after a run and owes the next frame while one of them moves.
    fn store_keyed<P: 'static>(&self, id: Id, keyed: Keyed<P>) {
        if keyed.moving() {
            self.owe();
        }
        self.store(id, keyed);
    }

    /// The presence of `key` among the keys of `name`, which keeps `payload` with it while it is visible. At the frame
    /// time the name is first driven every key starts at its target, as `anim` does; a key that appears later enters
    /// from closed. Runs at one frame time step to the same value.
    pub(crate) fn presence_keyed<P: Clone + 'static>(&self, name: &'static str, key: Id, open: bool, payload: P, spec: MotionSpec) -> Presence {
        let id = keyed_slot(self.base, name);
        let now = self.now();
        let mut keyed = self.load::<Keyed<P>>(id).unwrap_or(Keyed { born: now, entries: Vec::new() });
        let target = f32::from(u8::from(open));
        let at = match keyed.entries.iter().position(|entry| entry.key == key) {
            Some(at) => at,
            None => {
                let start = if keyed.born == now { target } else { 0.0 };
                keyed.entries.push(KeyedEntry { key, tween: Tween::at(start), open_in: None, payload: payload.clone() });
                keyed.entries.len() - 1
            }
        };
        let entry = &mut keyed.entries[at];
        let presence = Presence::at(advance(&self.policy, &mut entry.tween, target, spec, now), open);
        entry.open_in = open.then_some(now);
        entry.payload = payload;
        if !presence.visible {
            keyed.entries.retain(|entry| entry.key != key);
        }
        self.store_keyed(id, keyed);
        presence
    }

    /// The keys of `name` that no run at this frame time drove open, closing from where they are, each with the
    /// payload it was last given; a key is dropped, payload and all, once it has faded out. Read it after driving
    /// this frame's open keys.
    pub(crate) fn exiting<P: Clone + 'static>(&self, name: &'static str, spec: MotionSpec) -> Vec<(Presence, P)> {
        let id = keyed_slot(self.base, name);
        let Some(mut keyed) = self.load::<Keyed<P>>(id) else { return Vec::new() };
        let now = self.now();
        let out = closing(&mut keyed, &self.policy, spec, now, |entry| entry.open_in == Some(now));
        self.store_keyed(id, keyed);
        out
    }

    pub(crate) fn jump<T: Interpolate>(&self, name: &'static str, value: T) {
        self.store(anim_slot(self.base, name), Tween::at(value));
    }

    /// The 0 to 1 position of a loop of `period` seconds at the frame time, stepped every `tick` seconds when given:
    /// a stepped loop asks for the build at its next step, an unstepped one for the next frame.
    pub(crate) fn phase(&self, period: f64, tick: Option<f64>) -> Option<f32> {
        let now = self.now();
        let tick = tick.filter(|tick| *tick > 0.0);
        let at = tick.map_or(now, |tick| (now / tick).floor() * tick);
        let phase = loop_phase(self.policy.mode, at, period)?;
        match tick {
            Some(tick) => self.wake_at(at + tick),
            None => self.owe(),
        }
        Some(phase)
    }

    /// Whether a caret that last moved or was typed at `restart` shows its stroke now. It shows solid for a half
    /// period after `restart`, then alternates, and the node builds again at the next edge only; a mode whose Blink
    /// row is solid never hides it and asks for nothing.
    pub(crate) fn blink(&self, restart: f64) -> bool {
        if self.policy.mode.playback(spec::BLINK.kind()) != Playback::Run {
            return true;
        }
        let since = (self.now() - restart).max(0.0);
        let step = (since / BLINK_HALF_PERIOD).floor();
        self.wake_at(restart + (step + 1.0) * BLINK_HALF_PERIOD);
        step % 2.0 == 0.0
    }

    /// The frame time in seconds, untracked: the same for every run of one frame.
    pub(crate) fn now(&self) -> f64 {
        self.context().time()
    }

    /// The node builds again at frame time `at`; the earliest request of a run wins. A measuring run asks for
    /// nothing.
    pub(crate) fn wake_at(&self, at: f64) {
        if self.stage.is_peek() {
            return;
        }
        let after = (at - self.now()).max(0.0);
        if after.is_finite() {
            self.context().request_repaint_after(Duration::from_secs_f64(after));
        }
    }
}

/// The keyed presences of `name` in the motion memory `memory` of the component `base`, for the keys not among
/// `listed`, stepped toward closed at the frame time as `Animate::exiting` steps them, on a copy: nothing is stored
/// and nothing is woken. A collection's layout reads it (`StyleCx::exiting`).
pub(crate) fn peek_exiting<P: Clone + 'static>(
    memory: &MemoryStage,
    base: Id,
    policy: MotionPolicy,
    name: &'static str,
    spec: MotionSpec,
    listed: &[Id],
) -> Vec<(Presence, P)> {
    let Some(mut keyed) = memory.motion.peek::<Keyed<P>>(keyed_slot(base, name)) else { return Vec::new() };
    closing(&mut keyed, &policy, spec, memory.ctx().time(), |entry| listed.contains(&entry.key))
}

/// Motion on `Cx`, `BodyCx` and `ItemCx`. Every method takes `&self`: slots live in the node's motion memory under
/// the component's id and go with the node. A value that still moves makes the node build again in the next frame;
/// a settled one asks for nothing.
pub trait Animate: crate::runtime::memory::sealed::Sealed {
    /// `target` approached over `spec`'s duration; the first call returns `target` without animating.
    fn anim<T: Interpolate>(&self, name: &'static str, target: T, spec: MotionSpec) -> T;
    /// A 0 to 1 value approaching 1 when `on`, else 0.
    fn anim_bool(&self, name: &'static str, on: bool, spec: MotionSpec) -> f32;
    /// Open-close presence for something shown while it opens and fades out.
    fn presence(&self, name: &'static str, open: bool, spec: MotionSpec) -> Presence;
    /// Open-close presence of one of a set of things under `name`, told apart by `key`, such as the items of a list
    /// that enter and leave. `payload` is kept with the key while it is visible, so a thing its owner no longer has
    /// can still be drawn while it fades out (`exiting`). The first frame of `name` starts every key at its target;
    /// a key that appears later enters from closed.
    fn presence_keyed<P: Clone + Send + Sync + 'static>(
        &self,
        name: &'static str,
        key: impl Hash,
        open: bool,
        payload: P,
        spec: MotionSpec,
    ) -> Presence;
    /// The keys of `name` that no run of this frame drove open through `presence_keyed`, fading out, each with the
    /// payload it was last given; a key is forgotten once it has faded out. Call it after driving the frame's open
    /// keys.
    fn exiting<P: Clone + Send + Sync + 'static>(&self, name: &'static str, spec: MotionSpec) -> Vec<(Presence, P)>;
    /// Sets `name` to `value` with no animation.
    fn jump<T: Interpolate>(&self, name: &'static str, value: T);
    /// The 0 to 1 position of a loop of `period` seconds, stepped every `tick` seconds when given; `None` when
    /// the mode asks for the still frame.
    fn phase(&self, period: f64, tick: Option<f64>) -> Option<f32>;
    /// Whether a caret last moved at `restart` (a frame time) shows now: solid right after `restart`, then
    /// blinking; always solid where the motion table's Blink row is solid.
    fn blink(&self, restart: f64) -> bool;
    /// The frame time in seconds, the same for every run of one frame.
    fn now(&self) -> f64;
    /// Asks for a build of the node at frame time `at` (timers, holds, polls), whatever the motion mode.
    fn wake_at(&self, at: f64);
}

/// The id of a presence key: its hash, so a key needs no `Debug` to tell its presence apart.
pub(crate) fn key_id(key: &impl Hash) -> Id {
    Id::new(key)
}

/// The types that reach the motion service.
pub(crate) trait Drives {
    fn driver(&self) -> Driver<'_>;
}

impl<X: Drives + crate::runtime::memory::sealed::Sealed> Animate for X {
    fn anim<T: Interpolate>(&self, name: &'static str, target: T, spec: MotionSpec) -> T {
        self.driver().anim(name, target, spec)
    }

    fn anim_bool(&self, name: &'static str, on: bool, spec: MotionSpec) -> f32 {
        self.driver().anim_bool(name, on, spec)
    }

    fn presence(&self, name: &'static str, open: bool, spec: MotionSpec) -> Presence {
        self.driver().presence(name, open, spec)
    }

    fn presence_keyed<P: Clone + Send + Sync + 'static>(
        &self,
        name: &'static str,
        key: impl Hash,
        open: bool,
        payload: P,
        spec: MotionSpec,
    ) -> Presence {
        self.driver().presence_keyed(name, key_id(&key), open, payload, spec)
    }

    fn exiting<P: Clone + Send + Sync + 'static>(&self, name: &'static str, spec: MotionSpec) -> Vec<(Presence, P)> {
        self.driver().exiting(name, spec)
    }

    fn jump<T: Interpolate>(&self, name: &'static str, value: T) {
        self.driver().jump(name, value);
    }

    fn phase(&self, period: f64, tick: Option<f64>) -> Option<f32> {
        self.driver().phase(period, tick)
    }

    fn blink(&self, restart: f64) -> bool {
        self.driver().blink(restart)
    }

    fn now(&self) -> f64 {
        self.driver().now()
    }

    fn wake_at(&self, at: f64) {
        self.driver().wake_at(at);
    }
}
