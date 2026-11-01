//! The revisions panel: a compared copy's changes to accept or reject (N5.10).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use crate::revisions::{Revision, RevisionAction};

craft_ui::words! {
    /// The panel's words, English by default (DeckCraft's, PowerPoint's).
    #[non_exhaustive]
    pub struct RevisionsLabels {
        compared_with: "revisions.comparedWith" = "Compared with {name}";
        by_position: "revisions.byPosition" = "Not a copy of this presentation: slides are matched by position.";
        accept_all: "revisions.acceptAll" = "Accept All ({n})";
        reject_all: "revisions.rejectAll" = "Reject All ({n})";
        accept: "revisions.accept" = "Accept";
        reject: "revisions.reject" = "Reject";
        none_left: "revisions.noneLeft" = "No differences left.";
        end_review: "revisions.endReview" = "End Review";
    }
}

/// PowerPoint's Compare pane (DeckCraft): "Compared with {name}" (and the by-position note),
/// Accept All (n) and Reject All (n), then a card per change: a colour bar
/// (`review.revision_bar_*` in `review.revision_inserted`, `_deleted` or `_changed` of
/// [`ReviewStyle`](super::ReviewStyle)), the heading as a [`Link`](craft_ui::controls::Link)
/// (goes there), Accept and Reject at the right, up to eight detail lines with bullets. With no
/// changes: "No differences left." and End Review. Every link and button is a Tab stop in reading
/// order; counts through [`Counted`](craft_ui::localized::words::Counted) words.
pub struct RevisionsPanel<'a> {
    id_salt: IdSalt,
    compared_with: &'a str,
    by_position: bool,
    revisions: &'a [Revision],
    labels: Option<RevisionsLabels>,
}

impl<'a> RevisionsPanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, compared_with: &'a str, revisions: &'a [Revision]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), compared_with, by_position: false, revisions, labels: None }
    }

    /// The copy didn't match by identity; changes are by position.
    pub fn by_position(mut self, on: bool) -> Self {
        self.by_position = on;
        self
    }

    pub fn labels(mut self, labels: RevisionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // deckcraft: crates/ui-egui/src/panels/comments.rs:5 `revisions`.
    pub fn show(self, ui: &mut Ui) -> RevisionsPanelOutput {
        todo!()
    }
}

/// What the Revisions panel did this pass.
#[must_use]
pub struct RevisionsPanelOutput {
    pub response: Response,
    pub action: Option<RevisionAction>,
}
