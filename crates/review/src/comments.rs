//! Comment threads: where they are, their messages, reactions and review status, and the
//! panel's sort and search (N5.01, N5.02).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct ThreadId(pub u64);

/// Where a thread is.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
}

/// Acrobat's review statuses, set as a status reply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Reaction {
    pub emoji: String,
    pub people: Vec<String>,
}

/// One message in a thread; the first is the comment, the rest replies.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommentThread {
    pub id: ThreadId,
    pub anchor: CommentAnchor,
    /// The annotation kind the thread hangs on, for the card's icon and the Type sort ("Note",
    /// "Highlight"); empty for plain comments.
    pub kind: String,
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

/// How the panel orders threads (Acrobat's Sort by).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CommentSort {
    #[default]
    Page,
    Author,
    Date,
    Type,
    Color,
}

/// The order of `threads` by `sort`, as indices: page then position on the page (top to bottom),
/// author A–Z, newest first, kind, colour; ties keep document order.
// pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:115 `show` (sorting),
// `tools/comments/state.rs:297` `SortBy`.
pub fn sort_threads(threads: &[CommentThread], sort: CommentSort) -> Vec<usize> {
    todo!()
}

/// Whether `thread` matches the search `query`: case-insensitive, in any message's text or
/// author, or the kind.
// pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:104 `matches`.
pub fn matches(thread: &CommentThread, query: &str) -> bool {
    todo!()
}

/// "Ana María Ruiz" → "AM": the first letters of the first two words, upper case; none gives
/// "?".
// pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:72 `initials`.
pub fn initials(name: &str) -> String {
    todo!()
}
