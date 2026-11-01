//! Comment threads: where they are, their messages, reactions and review status, and the
//! panel's sort and search (N5.01, N5.02).

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
    /// timeline.
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
/// author A–Z, newest first, kind, colour; ties keep document order.
// pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:115 `show` (sorting),
// `tools/comments/state.rs:297` `SortBy`.
pub fn sort_threads(threads: &[CommentThread], sort: CommentSort) -> Vec<usize> {
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
        CommentSort::Type => order.sort_by(|&a, &b| threads[a].kind.cmp(&threads[b].kind)),
        CommentSort::Color => order.sort_by_key(|&i| threads[i].color),
    }
    order
}

/// Whether `thread` matches the search `query`: case-insensitive, in any message's text or
/// author, or the kind.
// pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:104 `matches`.
pub fn matches(thread: &CommentThread, query: &str) -> bool {
    let query = query.to_lowercase();
    let has = |text: &str| text.to_lowercase().contains(&query);
    has(&thread.kind) || thread.messages.iter().any(|m| has(&m.text) || has(&m.author))
}

/// "Ana María Ruiz" → "AM": the first letters of the first two words, upper case; none gives
/// "?".
// pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:72 `initials`.
pub fn initials(name: &str) -> String {
    let letters: String = name.split_whitespace().filter_map(|word| word.chars().next()).take(2).collect();
    if letters.is_empty() { "?".into() } else { letters.to_uppercase() }
}
