//! Popup handles and the `Overlays` trait: a component's popups and their open state, kept on its node
//! (`contract::rt::overlay::PopupStates`, taken into the run's `CxCore::popups`), and hover dwell on build wakes.

use super::opener::Trigger;
use crate::contract::cx::CxCore;
use crate::contract::{BindMut, Component, Cx, Id};
use crate::runtime::memory::{MemoryStage, sub};
use craft_behavior::{EditEvent, Intent, Mixed};

/// An id and a name; `Copy`, never a snapshot of the open state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupHandle {
    pub(super) id: Id,
    pub(super) name: &'static str,
}

impl PopupHandle {
    pub(crate) fn new(opener: Id, name: &'static str) -> Self {
        PopupHandle { id: sub(opener, ("popup", name)), name }
    }

    /// The popup of the repeat `index` of this name: a component with one popup per part or per entry gives each its
    /// own. Index 0 is the handle `cx.popup(name)` returns.
    #[must_use]
    pub fn at(self, index: usize) -> PopupHandle {
        match index {
            0 => self,
            _ => PopupHandle { id: sub(self.id, ("at", index)), name: self.name },
        }
    }

    /// The popup's id: its open state's and its layer child's.
    pub fn id(self) -> Id {
        self.id
    }

    /// The name given to `cx.popup`.
    pub fn name(self) -> &'static str {
        self.name
    }
}

/// The seconds a press must be held for `Trigger::Hold`.
pub const HOLD: f64 = 0.5;

/// The `Cx` trait that names popups.
pub trait Overlays {
    /// The handle of the popup `name` of this component. Its open state is kept on the component's node from the
    /// first time a handle opens it.
    fn popup(&mut self, name: &'static str) -> PopupHandle;
}

impl Overlays for CxCore<'_> {
    fn popup(&mut self, name: &'static str) -> PopupHandle {
        PopupHandle::new(self.id, name)
    }
}

impl<C: Component> Overlays for Cx<'_, C> {
    fn popup(&mut self, name: &'static str) -> PopupHandle {
        self.core.popup(name)
    }
}

impl PopupHandle {
    /// Toggles or opens the popup from `trigger` at step 9. An opener whose click toggled its popup keeps the layer
    /// chain it sits in open (`Dismiss::on_activate`).
    pub fn trigger<C: Component>(self, trigger: Trigger, cx: &mut Cx<'_, C>) {
        debug_assert!(C::OVERLAYS, "`{}` hosts a popup but sets `OVERLAYS = false`", std::any::type_name::<C>());
        let core = &mut *cx.core;
        // Only a hover popup reads the opener's hover and focus look: a read here would run every opener per hover move.
        let resting = trigger == Trigger::Hover && (core.response.hovered() || core.focus.visible);
        if trigger == Trigger::Context {
            core.popups.offer_context_menu();
        }
        let toggled = match trigger {
            Trigger::Click => core.activation.is_some(),
            Trigger::Press => {
                matches!(core.press.phase, craft_behavior::PressPhase::Down { start, .. } if start.button == craft_behavior::PointerButton::Primary)
            }
            Trigger::Context => core.intents.take(Intent::ContextMenu) || (core.resolved.enabled && core.response.secondary_clicked()),
            Trigger::Hover => dwell(core, self.id, "dwell", resting, crate::contract::tooltip::DELAY),
            Trigger::Hold => {
                let held = core.press.phase.is_press();
                dwell(core, self.id, "dwell", held, HOLD)
            }
            Trigger::Manual => false,
        };
        let open = core.popups.is_open(self.id);
        if toggled {
            if trigger == Trigger::Click {
                core.keep_open = true;
            }
            let toggles = matches!(trigger, Trigger::Click | Trigger::Press);
            core.popups.set(self.id, if toggles { !open } else { true });
        } else if trigger != Trigger::Manual && core.intents.take(Intent::Open) {
            core.popups.set(self.id, true);
        } else if trigger == Trigger::Hover && open {
            // The pointer may cross the gap between the opener and the popup: the popup closes after a grace period
            // with the pointer on neither.
            let over = resting || core.popups.hovered(self.id);
            if dwell(core, self.id, "leave", !over, crate::contract::tooltip::GRACE) {
                core.popups.set(self.id, false);
            }
        }
        if trigger == Trigger::Hover {
            core.popups.watch_hover(self.id);
        }
    }

    /// Opens the popup. For `Trigger::Manual` this and the binding are the only ways it opens.
    pub fn open<C: Component>(self, cx: &mut Cx<'_, C>) {
        debug_assert!(C::OVERLAYS, "`{}` hosts a popup but sets `OVERLAYS = false`", std::any::type_name::<C>());
        cx.core.popups.set(self.id, true);
    }

    /// Closes the popup; `OverlayUi::closing` reports it as `Requested`.
    pub fn close<C: Component>(self, cx: &mut Cx<'_, C>) {
        debug_assert!(C::OVERLAYS, "`{}` hosts a popup but sets `OVERLAYS = false`", std::any::type_name::<C>());
        cx.core.popups.close(self.id, crate::runtime::overlay::CloseReason::Requested);
    }

    /// Whether a toggle, `open` or `close` in this run leaves the popup's open state different from the state the run
    /// began with. Valid from step 9.
    pub fn changed<C: Component>(self, cx: &mut Cx<'_, C>) -> bool {
        cx.core.popups.changed(self.id)
    }

    /// Binds the popup's open state to `open`: call it in `update`, after the toggles and closes the component makes.
    /// A change of the bound value since the last run opens or closes the popup; an open state that differs from the
    /// bound value is written to it. A value the bind refuses to take (a given one) never forces the popup. Returns
    /// whether the popup is open in this run.
    pub fn bind_open<C: Component>(self, cx: &mut Cx<'_, C>, mut open: BindMut<'_, bool>) -> bool {
        let began = cx.core.popups.began(self.id);
        let seen = cx.core.popups.bound(self.id).unwrap_or(began);
        let held = match open.read(cx) {
            Some(Mixed::Same(held)) => Some(held),
            _ => None,
        };
        if let Some(held) = held
            && held != seen
        {
            cx.core.popups.set(self.id, held);
        }
        let now = cx.core.popups.is_open(self.id);
        let mut seen = held;
        if let Some(held) = held
            && held != now
        {
            let label = cx.core.label_arc();
            if open.write(cx, &EditEvent::set(Mixed::Same(held), now, label, None)) {
                seen = Some(now);
            }
        }
        cx.core.popups.set_bound(self.id, seen);
        now
    }

    /// Whether the popup is open in this run, with the changes made so far.
    pub fn is_open<C: Component>(self, cx: &mut Cx<'_, C>) -> bool {
        debug_assert!(C::OVERLAYS, "`{}` hosts a popup but sets `OVERLAYS = false`", std::any::type_name::<C>());
        cx.core.popups.is_open(self.id)
    }

    pub(crate) fn is_open_in(self, core: &CxCore<'_>) -> bool {
        core.popups.is_open(self.id)
    }
}

#[derive(Clone, Copy, Default)]
struct Dwell(Option<f64>);

/// Whether `held` has lasted `delay` seconds, timed in the slot `name` of `id` in the node's memory. While it waits,
/// the node builds again at the deadline (`CxCore::wake_at`); nothing runs per frame meanwhile.
pub(crate) fn dwell(core: &mut CxCore<'_>, id: Id, name: &'static str, held: bool, delay: f64) -> bool {
    let now = core.context().time();
    let due = dwell_in(core.memory, now, id, name, held, delay);
    match due {
        Ok(fired) => fired,
        Err(at) => {
            core.wake_at(at);
            false
        }
    }
}

/// The dwell step on `stage` at time `now`: `Ok(true)` when it fired, `Ok(false)` when not held, `Err(due)` while it
/// waits for `due`.
pub(super) fn dwell_in(stage: &mut MemoryStage, now: f64, id: Id, name: &'static str, held: bool, delay: f64) -> Result<bool, f64> {
    let since = stage.shown::<Dwell>(sub(id, name));
    if !held {
        since.0 = None;
        return Ok(false);
    }
    let due = *since.0.get_or_insert(now) + delay;
    if now >= due {
        since.0 = None;
        return Ok(true);
    }
    Err(due)
}

impl CxCore<'_> {
    /// The node builds again at frame time `at` (the earliest asked in this run wins); the driver applies it after
    /// the run with `BuildCx::request_build_at`.
    pub(crate) fn wake_at(&self, at: f64) {
        let earliest = self.run.build_at.get().map_or(at, |was| was.min(at));
        self.run.build_at.set(Some(earliest));
    }
}

impl From<PopupHandle> for Id {
    fn from(popup: PopupHandle) -> Id {
        popup.id
    }
}
