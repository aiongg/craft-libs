//! The viewer's commands. Every action of a panel menu or the options bar is one of them, so a key, the palette
//! or a script reaches it too.

use craft_ui::contract::CommandSet;

/// The viewer's app-scope commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "gallery")]
#[non_exhaustive]
pub enum GalleryCommands {
    /// Put the panels back at their default sizes.
    #[command(label = "Reset Layout")]
    ResetLayout,
    /// Show every story of the chosen component with fresh state.
    #[command(label = "Reset Stories")]
    ResetStories,
}
