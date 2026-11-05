//! Comment threads: where they are, their messages, reactions and review status, and the
//! panel's sort, search and filter (N5.01, N5.02).

use craft_i18n::Folder;

use crate::annotate::AnnotationKind;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ThreadId(pub u64);

/// Where a thread is.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CommentAnchor {
    /// A point on a page, in page units (PdfCraft's sticky notes and markups).
    Page { page: usize, at: [f64; 2] },
    /// A point on the canvas, optionally inside a frame or node that carries it when it moves
    /// (FigCraft).
    Canvas { at: [f64; 2], node: Option<u64> },
    /// A slide (DeckCraft), optionally at a point on it.
    Slide { slide: usize, at: Option<[f64; 2]> },
    /// A time on a timeline (a review of a cut).
    Time { seconds: f64 },
}

impl CommentAnchor {
    /// The page or slide number the panel groups by; `None` for canvas and time anchors.
    pub fn page(&self) -> Option<usize> {
        match self {
            CommentAnchor::Page { page, .. } => Some(*page),
            CommentAnchor::Slide { slide, .. } => Some(*slide),
            _ => None,
        }
    }

    /// The position the Page sort orders by within a page: top to bottom, or earliest first on a
    /// timeline. Acrobat sorts comments by position on the page; PdfCraft keeps the order the
    /// document lists them in.
    fn vertical(&self) -> f64 {
        match self {
            CommentAnchor::Page { at, .. } | CommentAnchor::Canvas { at, .. } => at[1],
            CommentAnchor::Slide { at, .. } => at.map_or(0.0, |at| at[1]),
            CommentAnchor::Time { seconds } => *seconds,
        }
    }
}

/// Acrobat's review statuses, set as a status reply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ReviewStatus {
    #[default]
    None,
    Accepted,
    Rejected,
    Cancelled,
    Completed,
}

impl ReviewStatus {
    /// The statuses the Set Status menu offers, in Acrobat's order.
    pub const MENU: [ReviewStatus; 5] =
        [ReviewStatus::None, ReviewStatus::Accepted, ReviewStatus::Rejected, ReviewStatus::Cancelled, ReviewStatus::Completed];
}

/// An emoji reaction and who gave it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Reaction {
    pub emoji: String,
    pub people: Vec<String>,
}

/// One message in a thread; the first is the comment, the rest replies.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommentMessage {
    pub author: String,
    /// Unix seconds.
    pub created: i64,
    pub modified: Option<i64>,
    pub text: String,
    pub reactions: Vec<Reaction>,
    /// Read by the viewer (FigCraft's read state).
    pub read: bool,
}

/// A comment and its replies.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommentThread {
    pub id: ThreadId,
    pub anchor: CommentAnchor,
    /// The annotation kind the thread hangs on, for the card's icon, the Type sort and its
    /// heading; `None` for a plain comment.
    pub kind: Option<AnnotationKind>,
    /// The label colour of the annotation (straight RGB).
    pub color: Option<[u8; 3]>,
    pub messages: Vec<CommentMessage>,
    pub status: ReviewStatus,
    /// Acrobat's per-comment checkmark.
    pub marked: bool,
    pub locked: bool,
    /// Figma's resolve.
    pub resolved: bool,
}

impl CommentThread {
    /// Replies, not counting the comment itself.
    pub fn replies(&self) -> usize {
        self.messages.len().saturating_sub(1)
    }
}

/// What the panel hides (PdfCraft's Filter comments menu): a thread shows unless its type,
/// author, status, colour or checkmark is listed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommentFilter {
    pub hidden_types: Vec<Option<AnnotationKind>>,
    /// The author of the thread's first message; empty for none.
    pub hidden_authors: Vec<String>,
    pub hidden_statuses: Vec<ReviewStatus>,
    pub hidden_colors: Vec<Option<[u8; 3]>>,
    /// `true` hides the marked threads, `false` the unmarked.
    pub hidden_checks: Vec<bool>,
}

impl CommentFilter {
    /// Whether anything is hidden.
    pub fn is_active(&self) -> bool {
        *self != CommentFilter::default()
    }

    /// Shows everything again.
    pub fn clear(&mut self) {
        *self = CommentFilter::default();
    }

    /// Whether `thread` passes every list.
    pub fn shows(&self, thread: &CommentThread) -> bool {
        let author = thread.messages.first().map_or("", |m| m.author.as_str());
        !self.hidden_types.contains(&thread.kind)
            && !self.hidden_authors.iter().any(|hidden| hidden == author)
            && !self.hidden_statuses.contains(&thread.status)
            && !self.hidden_colors.contains(&thread.color)
            && !self.hidden_checks.contains(&thread.marked)
    }
}

/// How the panel orders threads (Acrobat's Sort by).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CommentSort {
    #[default]
    Page,
    Author,
    Date,
    Type,
    Color,
}

/// The order of `threads` by `sort`, as indices: page then position on the page (top to bottom),
/// author A–Z, newest first, kind (by `kind_name`, the reader's name of a kind), colour; ties keep
/// document order. A plain comment sorts before every kind.
// pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:115 `show` (sorting),
// `tools/comments/state.rs:297` `SortBy`.
pub fn sort_threads(threads: &[CommentThread], sort: CommentSort, kind_name: impl Fn(AnnotationKind) -> String) -> Vec<usize> {
    let mut order: Vec<usize> = (0..threads.len()).collect();
    fn first(thread: &CommentThread) -> Option<&CommentMessage> {
        thread.messages.first()
    }
    match sort {
        CommentSort::Page => order.sort_by(|&a, &b| {
            let (a, b) = (&threads[a], &threads[b]);
            a.anchor.page().cmp(&b.anchor.page()).then_with(|| a.anchor.vertical().total_cmp(&b.anchor.vertical()))
        }),
        CommentSort::Author => order.sort_by_key(|&i| first(&threads[i]).map(|m| m.author.to_lowercase()).unwrap_or_default()),
        CommentSort::Date => order.sort_by_key(|&i| std::cmp::Reverse(first(&threads[i]).map(|m| m.modified.unwrap_or(m.created)))),
        CommentSort::Type => order.sort_by_cached_key(|&i| threads[i].kind.map(&kind_name)),
        CommentSort::Color => order.sort_by_key(|&i| threads[i].color),
    }
    order
}

/// Whether `thread` matches the search `query`: in any message's text or author, or the reader's
/// name of the kind (`kind_name`), ignoring case (and the Turkish dotted and dotless i, as
/// PdfCraft's `fold_case` does) through `folder`.
// pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:104 `matches`, `:107` `any_contains`.
pub fn matches(thread: &CommentThread, query: &str, folder: &Folder, kind_name: impl Fn(AnnotationKind) -> String) -> bool {
    let has = |text: &str| folder.contains(text, query);
    thread.kind.is_some_and(|kind| has(&kind_name(kind))) || thread.messages.iter().any(|m| has(&m.text) || has(&m.author))
}
