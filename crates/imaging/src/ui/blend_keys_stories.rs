//! Stories of the blend mode keys (task ST-AD2): a stub, not registered. Registering it (task
//! RG2) adds `stories()` to the imaging stories and regenerates the registry dump; until then the
//! gallery would run the `todo!()` bodies.
//!
//! A fake layer and a fake brush tool, each with its own entry list (the brush's with Clear after
//! Dissolve), a switch between them, the current mode, and a log of the picks. Every string goes
//! through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `imaging/blend-keys` | ⇧+ and ⇧− stepping with wrap, ⇧⌥ letters, a letter the target lacks (consumed, logged as nothing), and a text field that keeps the keys |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the stories land")]

use craft_ui::story::Story;

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
