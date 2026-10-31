//! Photoshop keyboard shortcut sets (`.kys`, Edit › Keyboard Shortcuts › Save Set, and the live
//! `Keyboard Shortcuts.psp`, the same XML): read into a [`ForeignSet`] for [`crate::plan`], and
//! written from a keymap for the dialog's Set row export.
//!
//! A `.kys` is `<command kind="static|dynamic" name="Gaussian Blur...">` rows, each with one or
//! more `<shortcut>Control+Opt+Cmd+G</shortcut>`, then `<tool>` keys. Photoshop names a command by
//! its menu label alone, so [`crate::plan`] matches by label. A set saved on Windows spells the
//! command modifier `Ctrl` ([`crate::Flavor::Windows`]); a Mac set's `Control` is the Control key.
//! Sets the suite writes carry each command's id (`craft-id`, which Photoshop ignores), so they
//! read back exactly.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::import::ForeignSet;
use crate::keymap::Keymap;

/// A set is a few tens of kilobytes; a larger file is not one and is not parsed.
// pc: crates/engine/src/kys.rs:26 `MAX_BYTES`.
pub const MAX_BYTES: usize = 4 << 20;

/// The attribute naming the suite's command id on the rows it writes.
// pc: crates/engine/src/kys.rs:260 `PC_ID` ("pc-id"; read too, so PhotoCraft's own sets keep
// matching exactly).
pub const ID_ATTRIBUTE: &str = "craft-id";

/// Why a file isn't a readable set.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KysError {
    /// Larger than [`MAX_BYTES`].
    TooLarge,
    /// Not XML, or not a `photoshop-keyboard-shortcuts` root; the parser's message.
    NotASet(String),
}

impl std::fmt::Display for KysError {
    // new: "The file is larger than a keyboard shortcut set can be." / "The file is not a
    // Photoshop keyboard shortcut set: {message}".
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for KysError {}

/// Reads a `.kys` or `Keyboard Shortcuts.psp`: the set's name (from the root's `filename`,
/// `$$$/FileName/Presets/KeyboardCustomization/{name}={name}`), one [`crate::ForeignEntry`] per
/// command row with a name (its keys in the file's flavour, an id from [`ID_ATTRIBUTE`] or
/// `pc-id` as the entry's path when present), and the `<tool>` rows as tool entries. Rows without
/// a name are skipped; rows without a shortcut stay, with no keys.
// pc: crates/engine/src/kys.rs:66 `parse`, :50 `filename_to_name`, :128 `to_ours`.
pub fn read(text: &str) -> Result<ForeignSet, KysError> {
    todo!()
}

/// A `.kys` file named `name` with every command and tool of `keymap` that has a binding:
/// commands then tools, labels with Photoshop's `...` for `…`, keys in the spelling Photoshop's
/// macOS sets use (`Control+Opt+Shift+Cmd+G`), and [`ID_ATTRIBUTE`] on each row. Double presses
/// and scoped bindings (which Photoshop can't express) are left out.
// pc: crates/engine/src/kys.rs:296 `write`, :273 `photoshop_key`.
pub fn write(name: &str, keymap: &Keymap) -> String {
    todo!()
}
