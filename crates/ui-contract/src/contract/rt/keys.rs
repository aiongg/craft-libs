//! The key stage on craft-ui-rt (docs/contract-rt.md sections 4 and 10). ui-rt routes a platform
//! key event to the focused node and bubbles it up the node's ancestors until one handles it, then
//! to the window root. Each contract node claims at event time from what its last run kept, so no
//! run decides where a key goes:
//!
//! 1. The focused node claims the keys its `KeyUse` takes and an Escape its run held; the key is
//!    queued on the node (`NodeInput::Key`), which marks its path, so its run sees the key in the
//!    frame it arrived.
//! 2. A roving group's owner takes the moves its `Roving` takes ([`super::focus::rove`]).
//! 3. A trapping scope wraps Tab among its stops; a layer child whose scope cancels closes on
//!    Escape (the cancel rule: the innermost claimant first).
//! 4. The window root takes Tab and F6 ([`super::focus::window_keys`]).
//! 5. An Escape nothing took dismisses the topmost layer child that asks (ui-rt).

use craft_keymap::Chord;
use craft_ui_rt::{Key, KeyInput};

use super::focus::FocusKept;
use crate::runtime::focus::{KeepKeys, KeyUse};
use craft_behavior::OwnerClass;

/// A key the focused node took, as its next run sees it in `Cx::input`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPress {
    /// The key.
    pub key: Key,
    /// The modifiers held.
    pub mods: craft_ui_rt::Mods,
    /// The key repeats while held.
    pub repeat: bool,
    /// The key activates the node (Enter or Space as its `KeyUse::activate` says): the run's
    /// response reports a click.
    pub activates: bool,
}

/// Whether the focused node with `focus` takes `k`, and how: `None` passes it up the path.
pub(crate) fn take(focus: &FocusKept, k: &KeyInput) -> Option<KeyPress> {
    let keys = focus.stop.as_ref()?;
    if !k.down {
        return None;
    }
    let press = |activates| Some(KeyPress { key: k.key, mods: k.mods, repeat: k.repeat, activates });
    if activates(keys, k) {
        return press(true);
    }
    keeps(keys, focus.holds_escape, k).then(|| press(false)).flatten()
}

fn activates(keys: &KeyUse, k: &KeyInput) -> bool {
    let plain = k.mods.is_none();
    match k.key {
        Key::Enter => plain && keys.activate.enter() && !keys.submits,
        Key::Space => plain && keys.activate.space(),
        _ => false,
    }
}

/// The keys a focused component keeps from the stage above it.
fn keeps(keys: &KeyUse, holds_escape: bool, k: &KeyInput) -> bool {
    let m = k.mods;
    let chord = Chord::new(m, k.key);
    if keys.claims.contains(&chord) {
        return true;
    }
    let commandless = !(m.cmd || m.ctrl);
    let arrows = matches!(k.key, Key::Up | Key::Down | Key::Left | Key::Right);
    let typing = matches!(keys.owner, OwnerClass::Text | OwnerClass::Recorder);
    match k.key {
        Key::Escape => holds_escape,
        Key::Tab => keys.keep == KeepKeys::Tab && !(m.cmd || m.ctrl || m.alt),
        Key::Enter => keys.submits || typing,
        Key::Left | Key::Right => matches!(keys.keep, KeepKeys::InlineArrows | KeepKeys::BlockArrows) || keys.moves || keys.value || keys.nudge,
        _ if arrows => keys.keep == KeepKeys::BlockArrows || keys.moves || keys.value || keys.nudge,
        Key::Home | Key::End | Key::PageUp | Key::PageDown => keys.moves || typing,
        Key::Delete | Key::Backspace => keys.deletes,
        _ => typing && commandless,
    }
}
