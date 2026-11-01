//! The version history panel and the preview banner (N5.06).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Rect, Response, Ui};

use crate::versions::{Branch, BranchId, Version, VersionId};

craft_ui::words! {
    /// The panel's words, English by default (FigCraft's, Figma's).
    #[non_exhaustive]
    pub struct VersionsLabels {
        title: "versions.title" = "Version history";
        current: "versions.current" = "Current version";
        save: "versions.save" = "Save version";
        title_field: "versions.titleField" = "Title";
        description: "versions.description" = "Description";
        autosaves: "versions.autosaves" = "{n} autosave versions";
        restore: "versions.restore" = "Restore this version";
        rename: "versions.rename" = "Rename";
        duplicate: "versions.duplicate" = "Duplicate";
        delete: "versions.delete" = "Delete";
        delete_confirm: "versions.deleteConfirm" = "Delete this version?";
        branches: "versions.branches" = "Branches";
        new_branch: "versions.newBranch" = "Create branch";
        open: "versions.open" = "Open";
        review_merge: "versions.reviewMerge" = "Review and merge";
        archive: "versions.archive" = "Archive";
        previewing: "versions.previewing" = "You're viewing {title}";
        go_back: "versions.goBack" = "Back to current version";
        cancel: "versions.cancel" = "Cancel";
    }
}

/// What the panel asks of the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionAction {
    Save {
        title: String,
        description: String,
    },
    /// Show a version on the canvas, read-only.
    Preview(VersionId),
    /// Back to the current version.
    Current,
    Restore(VersionId),
    Rename {
        id: VersionId,
        title: String,
    },
    Duplicate(VersionId),
    Delete(VersionId),
    CreateBranch {
        name: String,
    },
    OpenBranch(BranchId),
    MergeBranch(BranchId),
    ArchiveBranch(BranchId),
}

/// Figma's Version history panel in the left sidebar (FigCraft): the branches above (create,
/// open, review and merge, archive), then Current version, then the history newest first
/// ([`history_rows`](crate::versions::history_rows)) on a timeline line with a dot per row (the
/// current one filled), each named version with its title, date and author (relative under a
/// week with [`relative_age`](craft_ui::localized::relative_age), then the date through
/// craft-i18n's `I18n::dates()`), autosave runs collapsed; per-row "…" with Restore, Rename,
/// Duplicate and Delete (an inline confirm). The history is the kernel's
/// [`List`](craft_ui::list::List) of [`ListRow`](craft_ui::list::ListRow)s (title and the date and
/// author as the detail line; the dot column painted in the row's leading slot at
/// `review.version_dot_x`, text from `review.version_text_x`): one Tab stop, Up / Down move,
/// Enter previews, F2 or Rename renames in place (`List::start_rename`), autosave groups are
/// tree rows that open and close. Save version opens an inline form (Title, Description; Enter
/// saves, Esc cancels). The branch and footer buttons are Tab stops.
pub struct VersionHistoryPanel<'a> {
    id_salt: IdSalt,
    versions: &'a [Version],
    branches: &'a [Branch],
    previewing: Option<VersionId>,
    labels: Option<VersionsLabels>,
}

impl<'a> VersionHistoryPanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, versions: &'a [Version], branches: &'a [Branch]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), versions, branches, previewing: None, labels: None }
    }

    pub fn previewing(mut self, version: Option<VersionId>) -> Self {
        self.previewing = version;
        self
    }

    pub fn labels(mut self, labels: VersionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // figcraft: crates/ui-egui/src/panels/versions.rs:111 `panel`, `:317` `form`, `:374`
    // `timeline`, `:393` `current_row`, `:408` `version_row`, `:471` `branch_row`, `:557`
    // `autosave_group`, `:582` `confirm_row`.
    pub fn show(self, ui: &mut Ui) -> VersionHistoryPanelOutput {
        todo!()
    }
}

/// What the Version history panel did this pass.
#[must_use]
pub struct VersionHistoryPanelOutput {
    pub response: Response,
    pub action: Option<VersionAction>,
}

/// The banner over the canvas while a version is previewed: "You're viewing {title}", Restore
/// this version and Back to current version. Two Tab stops; Esc goes back.
pub struct VersionPreviewBanner<'a> {
    id_salt: IdSalt,
    version: &'a Version,
    labels: Option<VersionsLabels>,
}

impl<'a> VersionPreviewBanner<'a> {
    pub fn new(id_salt: impl AsIdSalt, version: &'a Version) -> Self {
        Self { id_salt: IdSalt::new(id_salt), version, labels: None }
    }

    /// Drawn at the top of `canvas`.
    // figcraft: crates/ui-egui/src/panels/versions.rs:523 `preview_banner`.
    pub fn show(self, ui: &mut Ui, canvas: Rect) -> Option<VersionAction> {
        todo!()
    }
}
