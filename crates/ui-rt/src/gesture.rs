//! Wheel steps, touchpad pinches and two-finger touch gestures, as nodes receive them
//! ([`crate::WheelInput`]).
//!
//! Which wheel steps zoom is decided here, once, as craft's egui runtime decides it: a step with
//! the zoom modifiers held zooms instead of scrolling, and off macOS a fractional Ctrl+wheel step
//! (or a Linux point-unit one) is a touchpad pinch its driver sent as a wheel, with a whole notch
//! within [`PINCH_CONTINUES`] of such a step still part of the pinch. A device that reports its own
//! pinch ([`crate::WindowEvent::Pinch`]) ends that run.
//!
//! Two touch contacts down at once are a pinch: the press of the first is cancelled, and every
//! step of the two goes to the node that took the gesture's start under their centre.

use craft_behavior::{PressCancel, Sense, WheelPhase, WheelSource, WheelUnit};
use craft_host::target::Os;
use craft_keymap::chord::Mods;
use emath::{Pos2, Vec2};

use crate::event::{Event, WheelInput};
use crate::id::{NodeId, WindowId};
use crate::platform::PointerEvent;
use crate::runtime::Runtime;

/// How long after a pinch step a whole Ctrl+wheel notch still belongs to the pinch, in seconds:
/// touchpad drivers that send a pinch as Ctrl+wheel round some of its steps to whole notches.
pub const PINCH_CONTINUES: f64 = 0.25;

/// How the runtime reads wheel steps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelOptions {
    /// A step with any of these held zooms and never scrolls. `Mods::CMD`: ⌘ on macOS, Ctrl
    /// elsewhere.
    pub zoom_mods: Mods,
    /// Off macOS, fractional Ctrl+wheel steps (and Linux point-unit ones) are a pinch whatever
    /// `zoom_mods` says.
    pub ctrl_wheel_pinch: bool,
    /// A step of `p` points zooms by `exp(zoom_speed * p)`.
    pub zoom_speed: f32,
    /// Points per line, for the zoom of line-unit steps.
    pub line_points: f32,
}

impl Default for WheelOptions {
    fn default() -> WheelOptions {
        WheelOptions {
            zoom_mods: Mods::CMD,
            ctrl_wheel_pinch: true,
            zoom_speed: 1.0 / 200.0,
            line_points: if cfg!(target_arch = "wasm32") { 8.0 } else { 40.0 },
        }
    }
}

/// Two touch contacts held as one gesture.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TouchPinch {
    ids: [u64; 2],
    /// The node that took the gesture's start.
    owner: Option<NodeId>,
    hit: Option<crate::hit::Hit>,
    centre: Pos2,
    distance: f32,
    angle: f32,
}

/// One window's touch contacts and pinch state.
#[derive(Default)]
pub(crate) struct Touches {
    contacts: smallvec::SmallVec<[(u64, Pos2); 4]>,
    pinch: Option<TouchPinch>,
    /// A pinch ended while contacts stay down: none of them presses until all are lifted.
    spent: bool,
    /// The input time of the last Ctrl+wheel step read as a pinch.
    last_pinch: Option<f64>,
    /// The contacts down since the first landed, while they may still end as a multi-finger tap.
    tap: Option<TapTrack>,
}

/// A touch gesture that may end as a multi-finger tap ([`Event::MultiTap`]): when its first finger landed, where each
/// landed, the most fingers down at once and the node the gesture's start went to. It ends when a finger moves past
/// [`craft_behavior::MULTI_TAP_SLOP`] or a contact is cancelled; the last lift taps when two or more fingers were down
/// and none stayed past [`craft_behavior::MULTI_TAP_TIME`].
#[derive(Clone, Debug)]
struct TapTrack {
    since: f64,
    landed: smallvec::SmallVec<[(u64, Pos2); 4]>,
    fingers: u8,
    owner: Option<NodeId>,
}

impl Touches {
    fn pos(&self, id: u64) -> Option<Pos2> {
        self.contacts.iter().find(|c| c.0 == id).map(|c| c.1)
    }
}

fn metrics(a: Pos2, b: Pos2) -> (Pos2, f32, f32) {
    let d = b - a;
    (a + d / 2.0, d.length(), d.y.atan2(d.x))
}

impl Runtime {
    /// Routes a wheel step (zoom 1, no hit yet): the zoom rule, then the topmost wheel shape and
    /// its ancestors.
    pub(crate) fn route_wheel(&mut self, window: WindowId, mut input: WheelInput) {
        if let Some(zoom) = self.wheel_zoom(window, input.delta, input.unit, input.mods) {
            input.zoom = zoom;
            input.delta = Vec2::ZERO;
        }
        self.send_wheel(window, input);
    }

    pub(crate) fn route_touchpad_pinch(&mut self, window: WindowId, pos: Pos2, zoom: f32, rotation: f32, mods: Mods) {
        if let Some(t) = self.touches_mut(window) {
            t.last_pinch = None;
        }
        let zoom = if zoom.is_finite() && zoom > 0.0 { zoom } else { 1.0 };
        let rotation = if rotation.is_finite() { rotation } else { 0.0 };
        let input = WheelInput {
            pos,
            delta: Vec2::ZERO,
            unit: WheelUnit::Point,
            phase: WheelPhase::Move,
            source: WheelSource::Trackpad,
            zoom,
            rotation,
            touch: false,
            mods,
            hit: None,
        };
        self.send_wheel(window, input);
    }

    fn send_wheel(&mut self, window: WindowId, mut input: WheelInput) {
        let through = self.modal_reach(window).map(|r| r.node);
        let hit =
            self.windows.get(&window).and_then(|w| crate::hit::hit_test_through(&self.tree, w, input.pos, &|s| s.intersects(Sense::WHEELS), through));
        if let Some(hit) = hit {
            input.hit = Some(hit);
            self.dispatch(hit.node, &Event::Wheel(input), true);
        }
    }

    /// The zoom factor of a wheel step, or `None` when it scrolls.
    fn wheel_zoom(&mut self, window: WindowId, delta: Vec2, unit: WheelUnit, mods: Mods) -> Option<f32> {
        let opts = self.wheel;
        let page = self.windows.get(&window).map_or(0.0, |w| w.viewport.y);
        let points = match unit {
            WheelUnit::Line => delta * opts.line_points,
            WheelUnit::Page => delta * page,
            _ => delta,
        };
        let factor = (opts.zoom_speed * (points.x + points.y)).exp();
        let factor = (factor.is_finite() && factor > 0.0).then_some(factor);
        let (os, now) = (self.os, self.time);
        // Off macOS, Ctrl is `cmd`.
        if opts.ctrl_wheel_pinch && os != Os::Mac && mods.cmd && !mods.alt && !mods.shift {
            let t = self.touches_mut(window)?;
            let whole = unit == WheelUnit::Line && delta.x.fract() == 0.0 && delta.y.fract() == 0.0;
            let smooth = match unit {
                WheelUnit::Line => !whole,
                WheelUnit::Point => os == Os::Linux,
                _ => false,
            };
            let continues = whole && t.last_pinch.is_some_and(|last| now - last <= PINCH_CONTINUES);
            if smooth || continues {
                t.last_pinch = Some(now);
                return Some(factor.unwrap_or(1.0));
            }
        }
        let z = opts.zoom_mods;
        let zooms = (z.cmd && mods.cmd) || (z.ctrl && mods.ctrl) || (z.alt && mods.alt) || (z.shift && mods.shift);
        zooms.then(|| factor.unwrap_or(1.0))
    }

    fn touches_mut(&mut self, window: WindowId) -> Option<&mut Touches> {
        self.windows.get_mut(&window).map(|w| &mut w.input.touches)
    }

    /// Takes a touch contact's sample before the press routing. Returns true when the touch
    /// gesture used it, so it starts, moves or ends no press.
    pub(crate) fn route_touch(&mut self, window: WindowId, p: PointerEvent) -> bool {
        use crate::platform::PointerPhase as P;
        let now = self.time;
        let Some(t) = self.touches_mut(window) else { return true };
        match p.phase {
            P::Down => {
                if t.contacts.is_empty() {
                    t.tap = Some(TapTrack { since: now, landed: Default::default(), fingers: 0, owner: None });
                }
                if !t.contacts.iter().any(|c| c.0 == p.id) {
                    t.contacts.push((p.id, p.pos));
                }
                let down = t.contacts.len() as u8;
                if let Some(tap) = t.tap.as_mut() {
                    tap.landed.push((p.id, p.pos));
                    tap.fingers = tap.fingers.max(down);
                }
                if t.pinch.is_some() || t.spent {
                    return true;
                }
                if t.contacts.len() < 2 {
                    return false;
                }
                let (a, b) = (t.contacts[0], t.contacts[1]);
                let (centre, distance, angle) = metrics(a.1, b.1);
                t.pinch = Some(TouchPinch { ids: [a.0, b.0], owner: None, hit: None, centre, distance, angle });
                if self.input_press_kind(window) == Some(craft_behavior::PointerKind::Touch) {
                    self.cancel_press(window, PressCancel::Gesture);
                }
                let hit = self.hit(window, centre, &|s| s.intersects(Sense::WHEELS));
                let input = self.touch_input(centre, Vec2::ZERO, 1.0, 0.0, WheelPhase::Start, p.mods, hit);
                let owner = hit.and_then(|h| self.dispatch(h.node, &Event::Wheel(input), true));
                if let Some(t) = self.touches_mut(window) {
                    if let Some(pinch) = t.pinch.as_mut() {
                        pinch.owner = owner;
                        pinch.hit = hit;
                    }
                    if let Some(tap) = t.tap.as_mut() {
                        tap.owner = owner;
                    }
                }
                true
            }
            P::Move => {
                if let Some(c) = t.contacts.iter_mut().find(|c| c.0 == p.id) {
                    c.1 = p.pos;
                }
                let drifted = t
                    .tap
                    .as_ref()
                    .and_then(|tap| tap.landed.iter().find(|l| l.0 == p.id))
                    .is_some_and(|l| (p.pos - l.1).length() > craft_behavior::MULTI_TAP_SLOP);
                if drifted {
                    t.tap = None;
                }
                let Some(pinch) = t.pinch else { return t.spent && t.pos(p.id).is_some() };
                if !pinch.ids.contains(&p.id) {
                    return true;
                }
                let (Some(a), Some(b)) = (t.pos(pinch.ids[0]), t.pos(pinch.ids[1])) else { return true };
                let (centre, distance, angle) = metrics(a, b);
                let zoom = if pinch.distance > 0.0 && distance > 0.0 { distance / pinch.distance } else { 1.0 };
                let mut turn = angle - pinch.angle;
                if turn > std::f32::consts::PI {
                    turn -= std::f32::consts::TAU;
                } else if turn < -std::f32::consts::PI {
                    turn += std::f32::consts::TAU;
                }
                let pan = centre - pinch.centre;
                if let Some(p) = t.pinch.as_mut() {
                    (p.centre, p.distance, p.angle) = (centre, distance, angle);
                }
                if let Some(owner) = pinch.owner {
                    let input = self.touch_input(centre, pan, zoom, turn, WheelPhase::Move, p.mods, pinch.hit);
                    self.deliver(owner, pinch.hit.map_or(owner, |h| h.node), &Event::Wheel(input));
                }
                true
            }
            P::Up | P::Cancel | P::Leave => {
                t.contacts.retain(|c| c.0 != p.id);
                let empty = t.contacts.is_empty();
                if p.phase != P::Up {
                    t.tap = None;
                }
                let tapped =
                    (if empty { t.tap.take() } else { None }).filter(|tap| tap.fingers >= 2 && now - tap.since <= craft_behavior::MULTI_TAP_TIME);
                let ended = t.pinch.filter(|pinch| pinch.ids.contains(&p.id));
                let used = ended.is_some() || t.pinch.is_some() || t.spent;
                if ended.is_some() {
                    t.pinch = None;
                    t.spent = !empty;
                } else if empty {
                    t.spent = false;
                }
                if let Some(pinch) = ended
                    && let Some(owner) = pinch.owner
                {
                    let phase = if p.phase == P::Up { WheelPhase::End } else { WheelPhase::Cancel };
                    let input = self.touch_input(pinch.centre, Vec2::ZERO, 1.0, 0.0, phase, p.mods, pinch.hit);
                    self.deliver(owner, pinch.hit.map_or(owner, |h| h.node), &Event::Wheel(input));
                }
                if let Some(tap) = tapped {
                    self.multi_tap(window, &tap);
                }
                used
            }
        }
    }

    /// Sends a finished multi-finger tap to the node the gesture's start went to, else to the topmost node under the
    /// fingers' centre that takes presses, bubbling.
    fn multi_tap(&mut self, window: WindowId, tap: &TapTrack) {
        let n = tap.landed.len().max(1) as f32;
        let at = Pos2::ZERO + tap.landed.iter().fold(Vec2::ZERO, |sum, l| sum + l.1.to_vec2()) / n;
        let ev = Event::MultiTap(craft_behavior::MultiTap::new(tap.fingers, at));
        match tap.owner {
            Some(owner) => {
                self.deliver(owner, owner, &ev);
            }
            None => {
                if let Some(hit) = self.hit(window, at, &|s| s.takes_press()) {
                    let _ = self.dispatch(hit.node, &ev, true);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn touch_input(
        &self,
        pos: Pos2,
        delta: Vec2,
        zoom: f32,
        rotation: f32,
        phase: WheelPhase,
        mods: Mods,
        hit: Option<crate::hit::Hit>,
    ) -> WheelInput {
        WheelInput { pos, delta, unit: WheelUnit::Point, phase, source: WheelSource::Unknown, zoom, rotation, touch: true, mods, hit }
    }

    /// Forgets a node that took a touch gesture.
    pub(crate) fn forget_touch_node(&mut self, window: WindowId, id: NodeId) {
        if let Some(t) = self.touches_mut(window)
            && let Some(p) = t.pinch.as_mut()
            && p.owner == Some(id)
        {
            p.owner = None;
        }
    }
}
