//! Step 8, sense: the state a part's look resolves against.

use crate::style::{Flags, StateSet};
use craft_theme::{State, StateMix};

/// The part states: each part's press and hover from the sensing core (none without `pointer`: the face of a faded
/// host), with its own flags.
pub(crate) fn part_state(result: &crate::primitives::PartResult, enabled: bool, pointer: bool, focus_visible: Option<bool>) -> StateSet {
    let mut flags = result.flags;
    if let Some(visible) = focus_visible {
        flags |= Flags::FOCUSED;
        if visible {
            flags |= Flags::FOCUS_VISIBLE;
        }
    }
    let (hovered, pressed) = if pointer { (result.hovered, result.pressed()) } else { (false, false) };
    let state = if !enabled || result.inactive {
        State::Disabled
    } else if pressed {
        State::Pressed
    } else if hovered {
        State::Hover
    } else {
        State::Rest
    };
    let on = flags.intersects(Flags::ON);
    StateSet::new(state, flags, StateMix::new(f32::from(u8::from(hovered)), f32::from(u8::from(pressed)), f32::from(u8::from(on))))
}
