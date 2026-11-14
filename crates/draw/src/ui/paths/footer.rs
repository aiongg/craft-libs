//! The footer of the Paths panel.

/// A button of the Paths footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PathsFooterButton {
    /// Fill the path with the foreground colour; disabled while the document cannot be filled.
    FillPath,
    /// Stroke the path.
    StrokePath,
    /// Load the path as a selection; a row dropped on it is loaded.
    LoadSelection,
    /// Make a work path from the selection.
    MakeWorkPath,
    /// Create a new path; a row dropped on it is duplicated.
    New,
    /// Delete the path; a row dropped on it is deleted.
    Delete,
}
