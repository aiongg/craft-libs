//! The accessibility bridge (N3.05): a page view draws pages as pictures, so on its own it is one
//! opaque widget to a screen reader. [`PagesAccess`] describes the text under it as an AccessKit
//! tree, the way a browser describes a page: the view is the Document node (with the caret and
//! selection as an AccessKit text selection), each paragraph a Paragraph, Heading (with its
//! level) or ListItem (grouped in Lists, with its ListMarker), holding one TextRun per laid-out
//! line with its characters' screen positions; tables are Table › Row › Cell, pictures Images
//! named by their alt text, hyperlinks Links around their runs.
//!
//! The app describes the pages it was asked for through [`AccessDescriber`]; the bridge builds
//! nothing unless a screen reader (or a test) asked egui for AccessKit output, describes only the
//! pages on screen and one either side, caps the tree at [`MAX_NODES`], and rebuilds only when
//! the app's revision, the selection, the described pages or their place on screen changed.
//!
//! ```ignore
//! let view = PageView::new("doc", &mut pages, &mut state).name(&doc.title).show(ui);
//! PagesAccess::new(view.response.id, &view.pages).revision(doc.layout_revision()).show(ui.ctx(), &mut DocDescriber(&doc));
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Context, Id, Rect};

use super::view::PlacedPage;

/// The most nodes described at once (a dense page has a few hundred).
pub const MAX_NODES: usize = 6000;
/// AccessKit's word starts are `u8`s: a text run holds at most this many characters; a longer
/// line is split into runs.
pub const MAX_RUN: usize = 255;
/// The deepest table nesting described.
pub const MAX_DEPTH: usize = 16;

/// What a block node is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockRole {
    Paragraph,
    /// A heading at outline level 1–9.
    Heading(u8),
    /// A list item at list level 0–8, with its marker text ("1.", "•").
    ListItem {
        level: u8,
        marker: String,
    },
    Table,
    Row,
    Cell,
    /// A picture or chart named by its alt text.
    Image(String),
    /// A hyperlink to `url`.
    Link(String),
}

/// A handle to a node the describer added, for adding its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccessNode(u32);

/// A text position the app names: (node of the text run, char offset in it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessPosition {
    pub run: AccessNode,
    pub offset: usize,
}

/// Collects one pass's description. Every method returns `None` once [`MAX_NODES`] is reached
/// (or the parent is past [`MAX_DEPTH`] tables deep), and the describer stops.
pub struct AccessBuilder {
    nodes: Vec<(Option<AccessNode>, BlockRole)>,
    full: bool,
}

impl AccessBuilder {
    /// A block under `parent` (`None`: under the document). Consecutive list items at one level
    /// are grouped into a List node by the builder.
    // wordcraft: crates/ui-egui/src/a11y.rs:239 `Builder::container`, :274 `para_node`.
    pub fn block(&mut self, parent: Option<AccessNode>, role: BlockRole) -> Option<AccessNode> {
        todo!()
    }

    /// One laid-out line's text under `parent`, with each character's screen x and width and the
    /// line's screen rectangle; word starts are worked out from the text.
    // wordcraft: crates/ui-egui/src/a11y.rs:431 `Builder::flush`, :501 `push_char`, :515 `word_starts`.
    pub fn line(&mut self, parent: AccessNode, text: &str, chars: &[(f32, f32)], line: Rect) -> Option<AccessNode> {
        todo!()
    }
}

/// The app's side: it describes pages of its layout.
pub trait AccessDescriber {
    /// Describe pages `pages` (their placement on screen in `placed`) into `out`, in reading
    /// order.
    fn describe(&mut self, placed: &[PlacedPage], out: &mut AccessBuilder);

    /// The caret and the selection's other end, if the document has a text selection on the
    /// described pages.
    fn selection(&self) -> Option<(AccessPosition, AccessPosition)>;
}

/// The bridge, built per pass after the [`PageView`](super::view::PageView).
pub struct PagesAccess<'a> {
    view: Id,
    placed: &'a [PlacedPage],
    revision: u64,
}

impl<'a> PagesAccess<'a> {
    /// Describes the pages `placed` under the view widget `view`.
    pub fn new(view: Id, placed: &'a [PlacedPage]) -> Self {
        PagesAccess { view, placed, revision: 0 }
    }

    /// The app's layout revision: a new value rebuilds the tree.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    /// Describe, or reuse last pass's tree. Nodes hang under the view's Document node (egui puts
    /// new nodes under the window root; the bridge moves them).
    // wordcraft: crates/ui-egui/src/a11y.rs:87 `expose`, :138 `build`.
    pub fn show(self, ctx: &Context, describer: &mut dyn AccessDescriber) {
        todo!()
    }
}
