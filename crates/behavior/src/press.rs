//! Pointer values and the press lifecycle (8.1, 8.2), egui-free: the runtime converts egui's events once per
//! pass and feeds them to a [`PressMachine`].

use crate::sense::Sense;
use bitflags::bitflags;
use craft_keymap::Mods;
use emath::{Pos2, Vec2};

/// A pointer button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PointerButton {
    /// The left button, a touch contact or a pen tip.
    #[default]
    Primary,
    /// The right button.
    Secondary,
    /// The wheel button.
    Middle,
    /// The first extra button, "back".
    Extra1,
    /// The second extra button, "forward".
    Extra2,
}

bitflags! {
    /// The pointer buttons held.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Buttons: u8 {
        /// [`PointerButton::Primary`].
        const PRIMARY = 1;
        /// [`PointerButton::Secondary`].
        const SECONDARY = 2;
        /// [`PointerButton::Middle`].
        const MIDDLE = 4;
        /// [`PointerButton::Extra1`].
        const EXTRA1 = 8;
        /// [`PointerButton::Extra2`].
        const EXTRA2 = 16;
    }
}

impl Buttons {
    /// The flag of one button.
    pub const fn of(button: PointerButton) -> Buttons {
        match button {
            PointerButton::Primary => Buttons::PRIMARY,
            PointerButton::Secondary => Buttons::SECONDARY,
            PointerButton::Middle => Buttons::MIDDLE,
            PointerButton::Extra1 => Buttons::EXTRA1,
            PointerButton::Extra2 => Buttons::EXTRA2,
        }
    }
}

/// What kind of device the pointer is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PointerKind {
    /// A mouse or trackpad.
    #[default]
    Mouse,
    /// A pen.
    Pen,
    /// A finger.
    Touch,
}

/// Where and when a press began.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PressStart {
    /// The pointer position at the press.
    pub origin: Pos2,
    /// The button that went down.
    pub button: PointerButton,
    /// The pass time of the press, in seconds.
    pub time: f64,
    /// 1 for a single press, 2 for a double press, and so on.
    pub clicks: u32,
}

impl PressStart {
    /// A press of `button` at `origin`.
    pub const fn new(origin: Pos2, button: PointerButton, time: f64, clicks: u32) -> Self {
        PressStart { origin, button, time, clicks }
    }
}

/// Pressure and tilt of the current contact.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PointerSample {
    /// 0 to 1.
    pub pressure: f32,
    /// Tilt around x and y, in radians.
    pub tilt: [f32; 2],
    /// Barrel rotation, in radians.
    pub rotation: f32,
    /// The eraser end of a pen.
    pub eraser: bool,
}

impl PointerSample {
    /// A sample with no tilt or rotation.
    pub const fn new(pressure: f32) -> Self {
        PointerSample { pressure, tilt: [0.0, 0.0], rotation: 0.0, eraser: false }
    }
}

/// A two-finger gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MultiTouch {
    /// Pan since the last pass.
    pub translation: Vec2,
    /// Zoom factor since the last pass.
    pub zoom: f32,
    /// Rotation since the last pass, in radians.
    pub rotation: f32,
    /// The centre of the fingers.
    pub center: Pos2,
}

impl MultiTouch {
    /// A gesture around `center`.
    pub const fn new(translation: Vec2, zoom: f32, rotation: f32, center: Pos2) -> Self {
        MultiTouch { translation, zoom, rotation, center }
    }
}

/// Why a captured press ended without a release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PressCancel {
    /// Escape.
    Escape,
    /// The window lost focus.
    FocusLost,
    /// The part turned disabled.
    Disabled,
    /// The owner was not drawn for a pass.
    Removed,
}

/// One part's press state in one pass.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PressPhase {
    /// Nothing under the pointer.
    Idle,
    /// The pointer is over the part.
    Hover,
    /// The button went down on the part.
    #[non_exhaustive]
    Down {
        /// The press.
        start: PressStart,
    },
    /// The press moved past slop on a part that senses a drag.
    #[non_exhaustive]
    DragStart {
        /// The press.
        start: PressStart,
        /// The pointer position.
        pos: Pos2,
    },
    /// The drag continues.
    #[non_exhaustive]
    Drag {
        /// The press.
        start: PressStart,
        /// The pointer position.
        pos: Pos2,
        /// The pointer movement of this pass.
        delta: Vec2,
    },
    /// The button went up.
    #[non_exhaustive]
    Release {
        /// The press.
        start: PressStart,
        /// The pointer position at the release.
        pos: Pos2,
        /// A release within slop of a part that senses a click.
        click: bool,
    },
    /// A captured press ended without a release.
    #[non_exhaustive]
    Cancel {
        /// The press.
        start: PressStart,
        /// Why it ended.
        reason: PressCancel,
    },
}

impl PressPhase {
    /// A press is in progress or just ended: not `Idle` or `Hover`.
    pub const fn is_press(&self) -> bool {
        !matches!(self, PressPhase::Idle | PressPhase::Hover)
    }

    /// The press the phase belongs to.
    pub const fn start(&self) -> Option<PressStart> {
        match self {
            PressPhase::Down { start }
            | PressPhase::DragStart { start, .. }
            | PressPhase::Drag { start, .. }
            | PressPhase::Release { start, .. }
            | PressPhase::Cancel { start, .. } => Some(*start),
            PressPhase::Idle | PressPhase::Hover => None,
        }
    }

    /// Whether the phase is a drag in progress.
    pub const fn is_drag(&self) -> bool {
        matches!(self, PressPhase::DragStart { .. } | PressPhase::Drag { .. })
    }

    /// Whether the phase is a click: a release within slop.
    pub const fn is_click(&self) -> bool {
        matches!(self, PressPhase::Release { click: true, .. })
    }

    /// The phase with its positions mapped by `point` and its drag delta by `delta`, for a part whose space
    /// differs from the screen's.
    pub fn mapped(self, point: impl Fn(Pos2) -> Pos2, delta: impl Fn(Vec2) -> Vec2) -> PressPhase {
        let at = |mut start: PressStart| {
            start.origin = point(start.origin);
            start
        };
        match self {
            PressPhase::Idle | PressPhase::Hover => self,
            PressPhase::Down { start } => PressPhase::Down { start: at(start) },
            PressPhase::DragStart { start, pos } => PressPhase::DragStart { start: at(start), pos: point(pos) },
            PressPhase::Drag { start, pos, delta: d } => PressPhase::Drag { start: at(start), pos: point(pos), delta: delta(d) },
            PressPhase::Release { start, pos, click } => PressPhase::Release { start: at(start), pos: point(pos), click },
            PressPhase::Cancel { start, reason } => PressPhase::Cancel { start: at(start), reason },
        }
    }
}

/// A part's phase, the modifiers held, the pen sample and whether it holds the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PressState {
    /// The phase.
    pub phase: PressPhase,
    /// The modifiers held.
    pub mods: Mods,
    /// Pressure and tilt of the current contact.
    pub sample: Option<PointerSample>,
    /// A drag holds the pointer: nothing else hovers.
    pub captured: bool,
}

impl PressState {
    /// A part with no press history.
    pub const IDLE: PressState = PressState::new(PressPhase::Idle);

    /// A state in `phase` with no modifiers, sample or capture.
    pub const fn new(phase: PressPhase) -> Self {
        PressState { phase, mods: Mods::NONE, sample: None, captured: false }
    }

    /// The modifiers held.
    pub const fn mods(mut self, mods: Mods) -> Self {
        self.mods = mods;
        self
    }

    /// The pen sample of the contact.
    pub const fn sample(mut self, sample: Option<PointerSample>) -> Self {
        self.sample = sample;
        self
    }

    /// Whether a drag holds the pointer.
    pub const fn captured(mut self, captured: bool) -> Self {
        self.captured = captured;
        self
    }
}

impl Default for PressState {
    fn default() -> Self {
        PressState::IDLE
    }
}

/// The longest gap between presses that still count as one multi-click, in seconds.
pub const MULTI_CLICK_TIME: f64 = 0.3;

/// The click count of a press at `pos` and `time`, given the previous press `(pos, time, clicks)`: one more when
/// it follows within [`MULTI_CLICK_TIME`] and `slop` of the previous one, else 1.
pub fn count_clicks(previous: Option<(Pos2, f64, u32)>, pos: Pos2, time: f64, slop: f32) -> u32 {
    match previous {
        Some((place, when, count)) if time - when <= MULTI_CLICK_TIME && place.distance(pos) <= slop => count + 1,
        _ => 1,
    }
}

/// A primary-button event of one pass, in the order it arrived.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PointerEvent {
    /// The pointer moved.
    Moved(Pos2),
    /// The primary button went down.
    Pressed {
        /// Where.
        pos: Pos2,
        /// The pass time.
        time: f64,
        /// 1 for a single press, 2 for a double press, and so on; see [`count_clicks`].
        clicks: u32,
    },
    /// The primary button went up.
    Released {
        /// Where.
        pos: Pos2,
    },
}

/// What the machine reads of one pass.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct PassInput<'a> {
    /// The pass's pointer events.
    pub events: &'a [PointerEvent],
    /// The movement that turns a press into a drag.
    pub slop: f32,
    /// The latest pointer position when the pass began.
    pub pos: Pos2,
    /// The pointer movement of the pass.
    pub delta: Vec2,
    /// Whether the primary button is down when the pass ends.
    pub held: bool,
    /// A reason the captured press ends now.
    pub cancel: Option<PressCancel>,
}

impl<'a> PassInput<'a> {
    /// A pass with these events, no movement and no cancel.
    pub const fn new(events: &'a [PointerEvent], slop: f32, pos: Pos2, held: bool) -> Self {
        PassInput { events, slop, pos, delta: Vec2::ZERO, held, cancel: None }
    }

    /// The pointer movement of the pass.
    pub const fn delta(mut self, delta: Vec2) -> Self {
        self.delta = delta;
        self
    }

    /// A reason the captured press ends now.
    pub const fn cancel(mut self, cancel: Option<PressCancel>) -> Self {
        self.cancel = cancel;
        self
    }
}

/// What one pass did to the press.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PressStep<T> {
    /// The press winner while a press is open, or the one released or cancelled in this pass.
    pub target: Option<T>,
    /// The target's phase, in screen space.
    pub phase: Option<PressPhase>,
    /// A drag is in progress or ended in this pass.
    pub dragging: bool,
    /// The target released as a click in this pass.
    pub activated: Option<T>,
}

impl<T> Default for PressStep<T> {
    fn default() -> Self {
        PressStep { target: None, phase: None, dragging: false, activated: None }
    }
}

#[derive(Clone, Copy, Debug)]
struct Open<T> {
    target: T,
    sense: Sense,
    start: PressStart,
    dragging: bool,
    /// Moved past slop on a target that senses no drag: the click is gone and nothing starts.
    abandoned: bool,
}

/// The press lifecycle of one root: the press winner it picked, kept until release or cancel.
#[derive(Clone, Copy, Debug)]
pub struct PressMachine<T> {
    open: Option<Open<T>>,
    /// The last press ended in a cancel: its later events belong to nobody.
    dead: bool,
}

impl<T> Default for PressMachine<T> {
    fn default() -> Self {
        PressMachine { open: None, dead: false }
    }
}

impl<T: Copy> PressMachine<T> {
    /// A machine with no press.
    pub const fn new() -> Self {
        PressMachine { open: None, dead: false }
    }

    /// The target of the open press.
    pub fn target(&self) -> Option<T> {
        self.open.map(|open| open.target)
    }

    /// Whether a drag holds the pointer.
    pub fn dragging(&self) -> bool {
        self.open.is_some_and(|open| open.dragging)
    }

    /// Runs one pass. `pick` chooses the press winner and what it senses at a press position; it returns `None`
    /// when no shape of this root takes the press.
    pub fn step(&mut self, input: PassInput<'_>, mut pick: impl FnMut(Pos2) -> Option<(T, Sense)>) -> PressStep<T> {
        let mut pos = input.pos;
        let mut drag_started = false;
        let mut released: Option<(T, PressStart, Pos2, bool, bool)> = None;
        let mut cancelled: Option<(T, PressStart, PressCancel)> = None;
        if let (Some(reason), Some(open)) = (input.cancel, self.open)
            && open.dragging
        {
            cancelled = Some((open.target, open.start, reason));
            self.open = None;
            self.dead = true;
        }
        for event in input.events {
            match *event {
                PointerEvent::Moved(at) => {
                    pos = at;
                    if let Some(open) = &mut self.open
                        && !open.dragging
                        && !open.abandoned
                        && at.distance(open.start.origin) > input.slop
                    {
                        if open.sense.contains(Sense::DRAG) {
                            open.dragging = true;
                            drag_started = true;
                        } else {
                            open.abandoned = true;
                        }
                    }
                }
                PointerEvent::Pressed { pos: at, time, clicks } => {
                    pos = at;
                    self.dead = false;
                    self.open = pick(at).map(|(target, sense)| Open {
                        target,
                        sense,
                        start: PressStart::new(at, PointerButton::Primary, time, clicks),
                        dragging: false,
                        abandoned: false,
                    });
                    drag_started = false;
                    released = None;
                }
                PointerEvent::Released { pos: at } => {
                    pos = at;
                    if let Some(open) = self.open.take() {
                        let click = !open.dragging && !open.abandoned && open.sense.contains(Sense::CLICK);
                        released = Some((open.target, open.start, at, open.dragging, click));
                    }
                }
            }
        }
        // A press that ended without a release event (pointer lost) is gone.
        if self.open.is_some() && !input.held && released.is_none() {
            self.open = None;
        }
        if let Some((target, start, reason)) = cancelled {
            let phase = PressPhase::Cancel { start, reason };
            return PressStep { target: Some(target), phase: Some(phase), dragging: false, activated: None };
        }
        if let Some((target, start, at, was_dragging, click)) = released {
            let phase = PressPhase::Release { start, pos: at, click };
            return PressStep { target: Some(target), phase: Some(phase), dragging: was_dragging, activated: click.then_some(target) };
        }
        let Some(open) = self.open else { return PressStep::default() };
        let phase = if drag_started {
            PressPhase::DragStart { start: open.start, pos }
        } else if open.dragging {
            PressPhase::Drag { start: open.start, pos, delta: input.delta }
        } else {
            PressPhase::Down { start: open.start }
        };
        PressStep { target: Some(open.target), phase: Some(phase), dragging: open.dragging, activated: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emath::pos2;

    const SLOP: f32 = 6.0;

    fn pass(machine: &mut PressMachine<u8>, events: &[PointerEvent], sense: Sense) -> PressStep<u8> {
        let held = machine.open.is_some() || matches!(events.last(), Some(PointerEvent::Pressed { .. }));
        machine.step(PassInput::new(events, SLOP, pos2(0.0, 0.0), held), |_| Some((1, sense)))
    }

    fn press(x: f32, time: f64) -> PointerEvent {
        PointerEvent::Pressed { pos: pos2(x, 0.0), time, clicks: 1 }
    }

    #[test]
    fn a_release_within_slop_is_a_click_whatever_its_duration() {
        let mut machine = PressMachine::new();
        pass(&mut machine, &[press(0.0, 1.0)], Sense::CLICK);
        let step = pass(&mut machine, &[PointerEvent::Released { pos: pos2(3.0, 0.0) }], Sense::CLICK);
        assert!(step.phase.is_some_and(|phase| phase.is_click()));
        assert_eq!(step.activated, Some(1));
        assert_eq!(machine.target(), None);
    }

    #[test]
    fn movement_past_slop_starts_a_drag_only_on_a_target_that_senses_one() {
        let mut machine = PressMachine::new();
        pass(&mut machine, &[press(0.0, 1.0)], Sense::CLICK | Sense::DRAG);
        let step = pass(&mut machine, &[PointerEvent::Moved(pos2(20.0, 0.0))], Sense::CLICK | Sense::DRAG);
        assert!(matches!(step.phase, Some(PressPhase::DragStart { .. })));
        let step = pass(&mut machine, &[PointerEvent::Released { pos: pos2(20.0, 0.0) }], Sense::CLICK | Sense::DRAG);
        assert!(matches!(step.phase, Some(PressPhase::Release { click: false, .. })));
        assert_eq!((step.activated, step.dragging), (None, true));

        pass(&mut machine, &[press(0.0, 2.0)], Sense::CLICK);
        let step = pass(&mut machine, &[PointerEvent::Moved(pos2(20.0, 0.0))], Sense::CLICK);
        assert!(matches!(step.phase, Some(PressPhase::Down { .. })), "a click-only target never starts a drag");
        let step = pass(&mut machine, &[PointerEvent::Released { pos: pos2(0.0, 0.0) }], Sense::CLICK);
        assert_eq!(step.activated, None, "a press that left the slop circle is no click, even when released at the origin");
    }

    #[test]
    fn a_press_no_shape_takes_opens_nothing() {
        let mut machine = PressMachine::<u8>::new();
        let step = machine.step(PassInput::new(&[press(0.0, 1.0)], SLOP, pos2(0.0, 0.0), true), |_| None);
        assert_eq!(step, PressStep::default());
    }

    #[test]
    fn escape_cancels_a_drag_once_and_the_later_release_belongs_to_nobody() {
        let mut machine = PressMachine::new();
        pass(&mut machine, &[press(0.0, 1.0)], Sense::DRAG);
        pass(&mut machine, &[PointerEvent::Moved(pos2(20.0, 0.0))], Sense::DRAG);
        let held = PassInput::new(&[], SLOP, pos2(20.0, 0.0), true).cancel(Some(PressCancel::Escape));
        let step = machine.step(held, |_| Some((1, Sense::DRAG)));
        assert!(matches!(step.phase, Some(PressPhase::Cancel { reason: PressCancel::Escape, .. })));
        let step = pass(&mut machine, &[PointerEvent::Released { pos: pos2(20.0, 0.0) }], Sense::DRAG);
        assert_eq!(step, PressStep::default());
    }

    #[test]
    fn a_cancel_before_the_drag_starts_leaves_the_press_alone() {
        let mut machine = PressMachine::new();
        pass(&mut machine, &[press(0.0, 1.0)], Sense::CLICK | Sense::DRAG);
        let held = PassInput::new(&[], SLOP, pos2(0.0, 0.0), true).cancel(Some(PressCancel::Escape));
        let step = machine.step(held, |_| Some((1, Sense::CLICK | Sense::DRAG)));
        assert!(matches!(step.phase, Some(PressPhase::Down { .. })));
    }

    #[test]
    fn presses_in_quick_succession_at_one_place_count_up() {
        let first = count_clicks(None, pos2(0.0, 0.0), 1.0, SLOP);
        let second = count_clicks(Some((pos2(0.0, 0.0), 1.0, first)), pos2(1.0, 0.0), 1.2, SLOP);
        let late = count_clicks(Some((pos2(0.0, 0.0), 1.2, second)), pos2(1.0, 0.0), 2.0, SLOP);
        let far = count_clicks(Some((pos2(0.0, 0.0), 1.2, second)), pos2(50.0, 0.0), 1.3, SLOP);
        assert_eq!((first, second, late, far), (1, 2, 1, 1));
    }

    #[test]
    fn a_press_lost_without_a_release_ends_quietly() {
        let mut machine = PressMachine::new();
        pass(&mut machine, &[press(0.0, 1.0)], Sense::CLICK);
        let lost = machine.step(PassInput::new(&[], SLOP, pos2(0.0, 0.0), false), |_| None);
        assert_eq!(lost, PressStep::default());
        assert_eq!(machine.target(), None);
    }
}
