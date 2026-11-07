//! The command board.

use std::sync::Arc;

/// A command named by its registry id.
// F3-LATER: runtime::commands (the registry, `CommandSet`, per-frame state and firing are F4)
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommandRef(pub &'static str);

impl CommandRef {
    /// The command with registry id `id`.
    pub const fn new(id: &'static str) -> Self {
        CommandRef(id)
    }
}

impl From<&'static str> for CommandRef {
    fn from(id: &'static str) -> Self {
        CommandRef(id)
    }
}

/// A bound command as the pipeline resolved it at step 1.
// F3-LATER: runtime::commands (title, label, shortcut, enabled and checked state)
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResolvedCommand {
    /// The title shown as the label.
    pub title: Arc<str>,
}
