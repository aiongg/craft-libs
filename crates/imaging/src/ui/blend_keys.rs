//! The blend mode keys, as in Photoshop: ⇧+ and ⇧− step through the target's modes, ⇧⌥ plus a
//! letter picks one ([`BlendKeys`]).
//!
//! The app lists the modes its target accepts ([`BlendKeyEntry`]: a painting tool's own Mode menu,
//! or the layer modes) with its own key type; the library reads the keys and returns the entry to
//! set. Which target a tool has, the engine command and its undo step stay the app's.
//!
//! ```ignore
//! // Before the single-letter tool keys.
//! let entries = [BlendKeyEntry::new(Mode::Normal, BlendShortcut::Normal), BlendKeyEntry::new(Mode::Multiply, BlendShortcut::Multiply)];
//! if let Some(pick) = BlendKeys::new(&entries).current(&layer.blend).consume(ctx) {
//!     app.set_blend(pick.into_key()); // one undo step
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Context, Key};

use craft_ui::focus::key_owner::OwnerGate;

/// Photoshop's ⇧⌥ letter positions: the 26 lettered blend modes. Darker Color, Lighter Color,
/// Subtract and Divide have no letter; list them with [`BlendKeyEntry::unlettered`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BlendShortcut {
    /// ⇧⌥N.
    Normal,
    /// ⇧⌥I.
    Dissolve,
    /// ⇧⌥Q: painting tools only.
    Behind,
    /// ⇧⌥R: painting tools only (paints transparency).
    Clear,
    /// ⇧⌥P: groups only.
    PassThrough,
    /// ⇧⌥K.
    Darken,
    /// ⇧⌥M.
    Multiply,
    /// ⇧⌥B.
    ColorBurn,
    /// ⇧⌥A.
    LinearBurn,
    /// ⇧⌥G.
    Lighten,
    /// ⇧⌥S.
    Screen,
    /// ⇧⌥D.
    ColorDodge,
    /// ⇧⌥W.
    LinearDodge,
    /// ⇧⌥O.
    Overlay,
    /// ⇧⌥F.
    SoftLight,
    /// ⇧⌥H.
    HardLight,
    /// ⇧⌥V.
    VividLight,
    /// ⇧⌥J.
    LinearLight,
    /// ⇧⌥Z.
    PinLight,
    /// ⇧⌥L.
    HardMix,
    /// ⇧⌥E.
    Difference,
    /// ⇧⌥X.
    Exclusion,
    /// ⇧⌥U.
    Hue,
    /// ⇧⌥T.
    Saturation,
    /// ⇧⌥C.
    Color,
    /// ⇧⌥Y.
    Luminosity,
}

impl BlendShortcut {
    /// Every position, in Photoshop's menu order (as the variants).
    pub const ALL: [BlendShortcut; 26] = [
        BlendShortcut::Normal,
        BlendShortcut::Dissolve,
        BlendShortcut::Behind,
        BlendShortcut::Clear,
        BlendShortcut::PassThrough,
        BlendShortcut::Darken,
        BlendShortcut::Multiply,
        BlendShortcut::ColorBurn,
        BlendShortcut::LinearBurn,
        BlendShortcut::Lighten,
        BlendShortcut::Screen,
        BlendShortcut::ColorDodge,
        BlendShortcut::LinearDodge,
        BlendShortcut::Overlay,
        BlendShortcut::SoftLight,
        BlendShortcut::HardLight,
        BlendShortcut::VividLight,
        BlendShortcut::LinearLight,
        BlendShortcut::PinLight,
        BlendShortcut::HardMix,
        BlendShortcut::Difference,
        BlendShortcut::Exclusion,
        BlendShortcut::Hue,
        BlendShortcut::Saturation,
        BlendShortcut::Color,
        BlendShortcut::Luminosity,
    ];

    /// The position's letter key, as each variant's doc says.
    pub fn letter(self) -> Key {
        // pc: crates/ui-egui/src/input/blend_keys.rs:41-72 `letter_pick` (the table).
        todo!()
    }

    /// The position whose letter is `key`, or `None` for any other key.
    pub fn from_key(key: Key) -> Option<BlendShortcut> {
        // pc: crates/ui-egui/src/input/blend_keys.rs:41-72 `letter_pick`.
        todo!()
    }
}

/// One mode the target accepts, in the order of its menu: the app's key of the mode and its ⇧⌥
/// letter position, if it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlendKeyEntry<K> {
    pub key: K,
    pub shortcut: Option<BlendShortcut>,
}

impl<K> BlendKeyEntry<K> {
    /// The mode `key` at letter position `shortcut`.
    pub fn new(key: K, shortcut: BlendShortcut) -> Self {
        Self { key, shortcut: Some(shortcut) }
    }

    /// A mode without a letter (Darker Color, Lighter Color, Subtract, Divide): ⇧+ and ⇧− reach
    /// it, no letter does.
    pub fn unlettered(key: K) -> Self {
        Self { key, shortcut: None }
    }

    /// Behind (⇧⌥Q), for a painting tool that offers it. Without such an entry ⇧⌥Q is consumed
    /// and does nothing.
    pub fn behind(key: K) -> Self {
        Self::new(key, BlendShortcut::Behind)
    }

    /// Clear (⇧⌥R), for a painting tool that offers it (PhotoCraft's Brush, Pencil and Paint
    /// Bucket list it after Dissolve). Without such an entry ⇧⌥R is consumed and does nothing.
    pub fn clear(key: K) -> Self {
        Self::new(key, BlendShortcut::Clear)
    }
}

/// What a blend mode key picked: the entry's key, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlendKeyPick<K> {
    /// ⇧+ or ⇧− stepped to this entry.
    Stepped(K),
    /// A ⇧⌥ letter picked this entry.
    Lettered(K),
}

impl<K> BlendKeyPick<K> {
    /// The picked entry's key.
    pub fn key(&self) -> &K {
        match self {
            BlendKeyPick::Stepped(key) | BlendKeyPick::Lettered(key) => key,
        }
    }

    /// The picked entry's key, taken.
    pub fn into_key(self) -> K {
        match self {
            BlendKeyPick::Stepped(key) | BlendKeyPick::Lettered(key) => key,
        }
    }
}

/// The blend mode keys over the app's list of `entries`.
///
/// - ⇧+ and ⇧− (⇧= is + on a US layout; a layout that types + and − with Shift arrives as
///   [`Key::Plus`] and [`Key::Minus`]) step to the next or previous entry, wrapping at either end;
///   a [`current`](Self::current) that isn't listed (or none) steps from the top (+) or the bottom
///   (−).
/// - ⇧⌥ plus a letter picks the entry whose [`BlendShortcut`] has that letter: the layout's letter
///   first ([`Key`] of the event; ⇧⌥Z on a German keyboard is Pin Light), then the physical key
///   (on a Mac ⌥ turns the letter into another character, ⇧⌥M types Â). A letter of the table
///   whose mode isn't listed (Clear or Behind for layers, Pass Through for a painting tool,
///   Overlay for the Healing Brush) is consumed and picks nothing. Any other letter is left alone.
/// - Neither ⌘ nor Ctrl may be held. Presses are read and removed from egui's input only while
///   the app owns the keyboard ([`KeyOwner::App`](craft_ui::focus::key_owner::KeyOwner::App):
///   no text field, menu, open list or dialog has it), so an open Blend Mode list keeps its arrows
///   and letters. Call it before the single-letter tool keys, which would take ⇧⌥M as M.
/// - Several presses in one pass combine: steps count from the previous press's entry, and the
///   last press's entry is the pick, so the app makes one undo step.
/// - No AccessKit node: the app's blend control announces the new mode. Keys are not directional,
///   so nothing mirrors in a right-to-left interface.
#[must_use = "read the keys with `consume`"]
pub struct BlendKeys<'a, K> {
    entries: &'a [BlendKeyEntry<K>],
    current: Option<&'a K>,
    gate: OwnerGate,
}

impl<'a, K: Clone + PartialEq> BlendKeys<'a, K> {
    /// The keys over `entries`, in menu order.
    pub fn new(entries: &'a [BlendKeyEntry<K>]) -> Self {
        Self { entries, current: None, gate: OwnerGate::default() }
    }

    /// The target's mode now, where ⇧+ and ⇧− step from.
    pub fn current(mut self, current: &'a K) -> Self {
        self.current = Some(current);
        self
    }

    /// The app's own modal state for [`key_owner::owner`](craft_ui::focus::key_owner::owner) (a
    /// full-window workspace, a dialog drawn without an egui modal layer); none by default.
    pub fn gate(mut self, gate: OwnerGate) -> Self {
        self.gate = gate;
        self
    }

    /// Reads and removes this pass's blend mode key presses (the type's rules) and returns the
    /// entry they pick, or `None`.
    pub fn consume(self, ctx: &Context) -> Option<BlendKeyPick<K>> {
        // pc: crates/ui-egui/src/input/blend_keys.rs:83 `handle` (:86-106: `retain` over pressed
        // keys with Shift and neither ⌘ nor Ctrl; ⌥ letters through `letter_pick(key)` then
        // `physical_key`; Plus or Equals +1, Minus −1), :18 `stepped`, :264 `pick_mode` (a mode
        // the menu lacks does nothing; Behind returns); crates/ui-egui/src/input/shortcuts.rs:859-861
        // (before the tool letters). new: `focus::key_owner::owner` must be `KeyOwner::App`.
        todo!()
    }
}

/// The entry `step` places after (or before, when negative) `current` in `entries`, wrapping; an
/// unlisted or absent `current` steps from the top (`step > 0`) or the bottom. `None` for an
/// empty list.
fn stepped<'e, K: PartialEq>(entries: &'e [BlendKeyEntry<K>], current: Option<&K>, step: i32) -> Option<&'e BlendKeyEntry<K>> {
    // pc: crates/ui-egui/src/input/blend_keys.rs:18 `stepped` (`rem_euclid`; unlisted: 0 or n − 1).
    todo!()
}
