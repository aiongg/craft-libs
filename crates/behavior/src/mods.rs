//! Modifier keys held with a pointer press or a key.

/// Modifier keys held.
// F2-MERGE: F2d (craft_keymap::Mods, same fields; craft-behavior takes that dependency once the design lint's
// egui-linked rule ignores optional dependencies)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Mods {
    /// The command modifier: Cmd on macOS, Ctrl on Windows and Linux.
    pub cmd: bool,
    /// The Control key on macOS. Off macOS it folds into `cmd`.
    pub ctrl: bool,
    /// Alt, Option on macOS.
    pub alt: bool,
    /// The Shift key.
    pub shift: bool,
}

impl Mods {
    /// Modifiers with the given keys held.
    pub const fn new(cmd: bool, ctrl: bool, alt: bool, shift: bool) -> Self {
        Mods { cmd, ctrl, alt, shift }
    }

    /// No modifiers.
    pub const NONE: Mods = Mods { cmd: false, ctrl: false, alt: false, shift: false };
    /// Only `cmd`.
    pub const CMD: Mods = Mods { cmd: true, ..Mods::NONE };
    /// Only `ctrl`.
    pub const CTRL: Mods = Mods { ctrl: true, ..Mods::NONE };
    /// Only `alt`.
    pub const ALT: Mods = Mods { alt: true, ..Mods::NONE };
    /// Only `shift`.
    pub const SHIFT: Mods = Mods { shift: true, ..Mods::NONE };

    /// Whether no modifier is held.
    pub const fn is_none(self) -> bool {
        !(self.cmd || self.ctrl || self.alt || self.shift)
    }
}
