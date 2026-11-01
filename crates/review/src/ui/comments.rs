//! The Comments panel (N5.01).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use crate::comments::{CommentSort, CommentThread, ReviewStatus, ThreadId};

craft_ui::words! {
    /// The panel's words, English by default (PdfCraft's, Acrobat's).
    #[non_exhaustive]
    pub struct CommentsLabels {
        title: "comments.title" = "Comments";
        replies: "comments.replies" = "{n} replies";
        search: "comments.search" = "Search comments";
        sort_by: "comments.sortBy" = "Sort by";
        sort_page: "comments.sortPage" = "Page";
        sort_author: "comments.sortAuthor" = "Author";
        sort_date: "comments.sortDate" = "Date";
        sort_type: "comments.sortType" = "Type";
        sort_color: "comments.sortColor" = "Color";
        add_comment: "comments.add" = "Add a comment";
        reply: "comments.reply" = "Add a reply";
        post: "comments.post" = "Post";
        cancel: "comments.cancel" = "Cancel";
        more: "comments.more" = "More";
        edit: "comments.edit" = "Edit";
        set_status: "comments.setStatus" = "Set status";
        delete: "comments.delete" = "Delete";
        copy_text: "comments.copyText" = "Copy text";
        mark: "comments.mark" = "Mark with checkmark";
        unmark: "comments.unmark" = "Remove checkmark";
        marked: "comments.marked" = "Marked";
        locked: "comments.locked" = "Locked";
        unknown_author: "comments.unknownAuthor" = "Unknown author";
        page: "comments.page" = "Page {n}";
        status_none: "comments.statusNone" = "None";
        accepted: "comments.accepted" = "Accepted";
        rejected: "comments.rejected" = "Rejected";
        cancelled: "comments.cancelled" = "Cancelled";
        completed: "comments.completed" = "Completed";
        empty: "comments.empty" = "No comments yet.";
    }
}

/// What the panel keeps between passes; the app keeps one per document window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommentsPanelState {
    pub search: String,
    pub sort: CommentSort,
    pub selected: Option<ThreadId>,
    /// The message being edited inline: (thread, message index, text).
    pub editing: Option<(ThreadId, usize, String)>,
    /// The selected card's reply draft.
    pub reply: String,
    /// The "Add a comment" box's draft.
    pub new_comment: String,
}

/// What the panel asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum CommentEdit {
    /// "Add a comment": a sticky note on the current page.
    Add {
        text: String,
    },
    Reply {
        thread: ThreadId,
        text: String,
    },
    /// Message `index` (0: the comment) edited.
    Edit {
        thread: ThreadId,
        index: usize,
        text: String,
    },
    /// Message `index` deleted; 0 deletes the thread.
    Delete {
        thread: ThreadId,
        index: usize,
    },
    SetStatus {
        thread: ThreadId,
        status: ReviewStatus,
    },
    Mark {
        thread: ThreadId,
        marked: bool,
    },
    /// A card was selected: the app selects the annotation on the page and scrolls to it.
    Select(ThreadId),
    /// Copy text of message `index`; the app puts it on the clipboard.
    CopyText {
        thread: ThreadId,
        index: usize,
    },
}

/// Acrobat's right-hand comment list (PdfCraft).
///
/// - Header "Comments N" with "N replies" under it, search (filters by [`matches`](crate::comments::matches)),
///   the Sort by menu.
/// - "Add a comment" box: Enter (Cmd+Enter in a multi-line draft) posts a sticky note.
/// - Cards grouped by page under "Page N" headings (by [`sort_threads`](crate::comments::sort_threads)): an
///   avatar with the kind's icon in the annotation colour, author, modified date (relative
///   under a week with [`relative_age`](craft_ui::localized::relative_age) in
///   [`AgeStyle::Short`](craft_ui::localized::AgeStyle), then the date through craft-i18n's
///   `I18n::dates()`, as `AGE_SHOWN_AS_DATE` describes), the text; a status badge (Accepted:
///   thumbs-up; Rejected: thumbs-down; Cancelled: circle-x; Completed: check; colours
///   `review.status_*` of [`ReviewStyle`](super::ReviewStyle)); the checkmark (a toggle on the
///   selected card, a mark otherwise); a lock badge; replies under it, each with its own "…"
///   ([`PressMenu`](craft_ui::menu::PressMenu): Edit, Copy text, Delete).
/// - The selected card has the `review.card_accent` bar, the reply box and "…" (Edit, Set
///   status, Delete). Locked threads and `read_only` hide the editing items.
/// - Keyboard: Up / Down move between cards (selecting them), Enter focuses the reply box, Esc
///   leaves an inline edit; every button is a Tab stop. AccessKit: a list of cards, each named
///   by author and text.
pub struct CommentsPanel<'a> {
    id_salt: IdSalt,
    threads: &'a [CommentThread],
    state: &'a mut CommentsPanelState,
    me: &'a str,
    read_only: bool,
    labels: Option<CommentsLabels>,
}

impl<'a> CommentsPanel<'a> {
    /// `me`: the author name new comments and replies get (the app's identity setting).
    pub fn new(id_salt: impl AsIdSalt, threads: &'a [CommentThread], state: &'a mut CommentsPanelState, me: &'a str) -> Self {
        Self { id_salt: IdSalt::new(id_salt), threads, state, me, read_only: false, labels: None }
    }

    /// No posting or editing (a document that forbids comments).
    pub fn read_only(mut self, on: bool) -> Self {
        self.read_only = on;
        self
    }

    pub fn labels(mut self, labels: CommentsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:115 `show`, `:311` `card`, `:541`
    // `inline_editor`, `:568` `reply_menu`, `:600` `avatar`, `:626` `header_controls`;
    // `tools/comments/menu.rs:368` `status_badge`; deckcraft: crates/ui-egui/src/panels/comments.rs:67.
    pub fn show(self, ui: &mut Ui) -> CommentsPanelOutput {
        todo!()
    }
}

/// What the Comments panel did this pass.
#[must_use]
pub struct CommentsPanelOutput {
    pub response: Response,
    /// This pass's edits, in order.
    pub edits: Vec<CommentEdit>,
}
