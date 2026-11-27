//! Step 12, state: the flags and interaction state a look resolves against.

use super::paint::surface_ink;
use crate::contract::{Component, Cx, Hover, Validity};
use crate::style::{Flags, StateSet, StyleView};
use craft_behavior::Input;
use craft_theme::{State, StateMix};

/// Whether a control's value differs from its reset value, and the reset value as the control words
/// it (10.2): what its label scope shows. Kept in the node's memory, where the label reads it.
#[derive(Clone, Default, Debug, PartialEq)]
pub(crate) struct ResetFacts {
    pub differs: bool,
    pub default: Option<String>,
}

/// Step 12: a control with a reset value records whether its value differs from it.
pub(crate) fn register_reset<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) {
    if matches!(C::INPUT, Input::Editor) {
        return;
    }
    let Some(reset) = comp.bind().and_then(|bind| bind.reset_value().cloned()) else { return };
    let differs = cx.mixed || cx.shown.as_deref() != Some(&reset);
    let default = comp.word_value(&reset, cx);
    let id = crate::runtime::memory::sub(cx.core.id, "reset");
    *cx.core.memory.shown::<ResetFacts>(id) = ResetFacts { differs, default };
}

/// The component's shown value when it is a `bool`: a two-state value, on or off.
pub(crate) fn shown_bool<C: Component>(cx: &Cx<'_, C>) -> Option<bool> {
    cx.shown.as_deref().and_then(|value| (value as &dyn std::any::Any).downcast_ref::<bool>()).copied()
}

/// The flags of step 12 that the component's own inputs give, for a run, a face and a collection member alike: its
/// own `flags`, READ_ONLY and SELECTED from its props, CHECKED from a shown `bool`, MIXED from a mixed binding and the
/// binding's validity.
pub(crate) fn value_flags<C: Component>(comp: &C, cx: &Cx<'_, C>) -> Flags {
    let mut flags = comp.flags();
    if cx.core.resolved.read_only {
        flags |= Flags::READ_ONLY;
    }
    if cx.core.resolved.selected == Some(true) {
        flags |= Flags::SELECTED;
    }
    if shown_bool(cx) == Some(true) {
        flags |= Flags::CHECKED;
    }
    if cx.mixed {
        flags |= Flags::MIXED;
    }
    match &cx.validity {
        Validity::Refuse(_) | Validity::Error(_) => flags |= Flags::INVALID,
        Validity::Warn(_) => flags |= Flags::WARN,
        _ => {}
    }
    flags
}

/// Step 12, state, for one recorded look: flags first (`value_flags`, OPEN from a popup shown this
/// run, CHECKED from a command's checked state), then the interaction state of the look
/// (`hovered`, `pressed`); the ink of the surface, else `inherited`. A look is discrete: ui-rt fades
/// between the kept looks at paint time, so no tween runs here.
pub(crate) fn state<C: Component>(comp: &C, cx: &mut Cx<'_, C>, open: bool, hovered: bool, pressed: bool, inherited: crate::style::Ink) {
    let mut flags = value_flags(comp, cx);
    let core = &mut *cx.core;
    if open {
        flags |= Flags::OPEN;
    }
    if core.command.as_deref().and_then(|command| command.checked) == Some(true) {
        flags |= Flags::CHECKED;
    }
    if core.text_invalid {
        flags |= Flags::INVALID;
    }
    // A faded component shows no hover or press: its rest look is its disabled look.
    let (hovered, pressed) = if core.faded { (false, false) } else { (hovered, pressed || core.press.phase.is_press()) };
    if core.focus.focused {
        flags |= Flags::FOCUSED;
    }
    if core.focus.visible {
        flags |= Flags::FOCUS_VISIBLE;
    }
    let state = if !core.looks_enabled() {
        State::Disabled
    } else if !core.resolved.enabled {
        State::Rest
    } else if pressed || flags.contains(Flags::OPEN) {
        State::Pressed
    } else if hovered {
        State::Hover
    } else {
        State::Rest
    };
    if !core.face_mode {
        let report = Hover { over: hovered, entered: false, left: false, clicked: hovered && core.response.clicked() };
        *core.memory.shown::<Hover>(super::ShownState::hover_slot(core.id)) = report;
    }
    let on = flags.intersects(Flags::ON);
    let t = StateMix::new(f32::from(u8::from(hovered)), f32::from(u8::from(pressed)), f32::from(u8::from(on)));
    core.state = StateSet::new(state, flags, t);
    core.ink = surface_ink(cx.style.surface(), &core.state, inherited);
    core.stated = true;
}
