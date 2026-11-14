//! The commands of the Paths panel.

use super::PathKind;
use craft_ui::contract::CommandSet;

/// The commands of the Paths panel: its panel-menu rows, its row menu and the footer's buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CommandSet)]
#[command(prefix = "pathsPanel", scope = "pathsPanel")]
#[non_exhaustive]
pub enum PathsCommands {
    /// Load the path as a selection.
    #[command(label = "Make Selection…")]
    MakeSelection,
    /// Fill the path with the foreground colour.
    #[command(label = "Fill Path…")]
    FillPath,
    /// Stroke the path with the current tool.
    #[command(label = "Stroke Path…")]
    StrokePath,
    /// Save the work path.
    #[command(label = "Save Path…")]
    SavePath,
    /// Duplicate the path.
    #[command(label = "Duplicate Path…")]
    DuplicatePath,
    /// Rename the path in place.
    #[command(label = "Rename Path…")]
    RenamePath,
    /// Delete the path.
    #[command(label = "Delete Path")]
    DeletePath,
}

impl PathsCommands {
    /// Whether the command applies to a targeted row of `kind` (`None`: no row targeted); Fill Path and Stroke Path
    /// need `can_fill`.
    pub fn applies_to(self, kind: Option<PathKind>, can_fill: bool) -> bool {
        let fills = matches!(self, PathsCommands::FillPath | PathsCommands::StrokePath);
        kind.is_some() && (can_fill || !fills)
    }
}
