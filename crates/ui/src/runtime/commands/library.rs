//! The command sets the library itself declares.

use craft_behavior::EditCommand;

/// The edit commands (11.3). A focused component that lists one in `KEYS.edits` takes it as
/// `Intent::Edit`; otherwise it is an app command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, craft_ui_macros::CommandSet)]
#[command(prefix = "edit")]
pub enum EditCommands {
    /// Cut the selection.
    #[command(label = "Cut", icon = "scissors", default = "Cmd+X")]
    Cut,
    /// Copy the selection.
    #[command(label = "Copy", icon = "copy", default = "Cmd+C")]
    Copy,
    /// Paste the clipboard.
    #[command(label = "Paste", icon = "clipboard", default = "Cmd+V")]
    Paste,
    /// Select everything.
    #[command(label = "Select All", default = "Cmd+A")]
    SelectAll,
    /// Undo the last edit.
    #[command(label = "Undo", icon = "undo", default = "Cmd+Z")]
    Undo,
    /// Redo the last undone edit.
    #[command(label = "Redo", icon = "redo", default = "Cmd+Shift+Z")]
    Redo,
}

impl EditCommands {
    /// The intent a component takes this command as.
    pub const fn edit(self) -> EditCommand {
        match self {
            EditCommands::Cut => EditCommand::Cut,
            EditCommands::Copy => EditCommand::Copy,
            EditCommands::Paste => EditCommand::Paste,
            EditCommands::SelectAll => EditCommand::SelectAll,
            EditCommands::Undo => EditCommand::Undo,
            EditCommands::Redo => EditCommand::Redo,
        }
    }

    /// The command an edit intent comes from.
    pub const fn of(command: EditCommand) -> Option<Self> {
        Some(match command {
            EditCommand::Cut => EditCommands::Cut,
            EditCommand::Copy => EditCommands::Copy,
            EditCommand::Paste => EditCommands::Paste,
            EditCommand::SelectAll => EditCommands::SelectAll,
            EditCommand::Undo => EditCommands::Undo,
            EditCommand::Redo => EditCommands::Redo,
            _ => return None,
        })
    }
}

/// The `window` commands (16.3): a floating container moves and resizes from the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, craft_ui_macros::CommandSet)]
#[command(prefix = "window")]
pub enum WindowCommand {
    /// Move the focused window with the arrow keys.
    #[command(label = "Move", default = "Alt+F7")]
    Move,
    /// Resize the focused window with the arrow keys.
    #[command(label = "Size", default = "Alt+F8")]
    Size,
}
