//! The key values the contract keeps: the `Intents` of the focused component, a menu-bar key and an arrow nudge. The
//! key routing is `contract::rt::keys`.

use craft_keymap::{Key, Mods};
use craft_ui_rt::Vec2;
pub use intents::Intents;
pub(crate) use intents::Lent;

mod intents;

/// A menu-bar key of an in-window bar (spec 9.7). F10 reaches the bar on every OS; Alt keys only off a Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuKey {
    /// F10 or a lone Alt press: focus the menu bar.
    Focus,
    /// Alt and a letter: open the menu with that mnemonic.
    Letter(char),
}

/// An arrow press as a nudge of content: its unit direction, its modifiers and the keys down when it went down.
#[derive(Clone, Debug)]
pub(crate) struct Nudge {
    pub(crate) direction: Vec2,
    pub(crate) mods: Mods,
    pub(crate) held: Vec<Key>,
}

/// The press or text an intent came from.
#[derive(Clone, Copy, Debug)]
struct Source {
    /// Its place among the frame's events ([`Batch::place`]).
    place: usize,
    /// Shift was held at the press; never for a text.
    shift: bool,
}
