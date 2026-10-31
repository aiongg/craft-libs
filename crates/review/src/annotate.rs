//! Annotations on a page: Acrobat's kinds and tools, their geometry and handles, and the edits a
//! drawing or editing gesture reports (N5.03). Coordinates are page units (points, y down), as
//! the canvas tool host gives them.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use kurbo::{Point, Rect, Vec2};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct AnnotationId(pub u64);

/// What an annotation is (PDF annotation subtypes as Acrobat names them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AnnotationKind {
    /// A sticky note.
    #[default]
    Note,
    TextBox,
    Callout,
    /// A file attachment.
    Attach,
    Highlight,
    Underline,
    StrikeOut,
    Squiggly,
    /// Insert Text at a caret.
    Caret,
    /// Strike-through with replacement text.
    ReplaceText,
    /// Freehand drawing.
    Ink,
    Line,
    Arrow,
    Rectangle,
    Oval,
    PolyLine,
    Polygon,
    Cloud,
}

/// A tool of the annotation toolbar: a kind to draw, the Eraser (removes Ink strokes it
/// crosses), or Select (edit existing annotations).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnnotationTool {
    #[default]
    Select,
    Draw(AnnotationKind),
    Eraser,
}

/// The toolbar's flyout groups, in Acrobat's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnnotationGroup {
    Comment,
    Highlight,
    Draw,
}

impl AnnotationGroup {
    pub const ALL: [AnnotationGroup; 3] = [AnnotationGroup::Comment, AnnotationGroup::Highlight, AnnotationGroup::Draw];

    /// The group's tools in Acrobat's order (PdfCraft `tools/comments.rs:81` `GROUPS`).
    pub fn tools(self) -> &'static [AnnotationTool] {
        use AnnotationKind as K;
        use AnnotationTool::Draw as D;
        match self {
            AnnotationGroup::Comment => &[D(K::Note), D(K::TextBox), D(K::Callout), D(K::Attach)],
            AnnotationGroup::Highlight => &[D(K::Highlight), D(K::Underline), D(K::StrikeOut), D(K::Squiggly), D(K::Caret), D(K::ReplaceText)],
            AnnotationGroup::Draw => {
                &[D(K::Ink), D(K::Line), D(K::Arrow), D(K::Rectangle), D(K::Oval), D(K::PolyLine), D(K::Polygon), D(K::Cloud), AnnotationTool::Eraser]
            }
        }
    }
}

impl AnnotationKind {
    /// Text markup: drawn over page text by selecting it (Highlight … Replace Text).
    pub fn is_markup(self) -> bool {
        matches!(
            self,
            AnnotationKind::Highlight
                | AnnotationKind::Underline
                | AnnotationKind::StrikeOut
                | AnnotationKind::Squiggly
                | AnnotationKind::ReplaceText
        )
    }

    /// Kinds drawn by points (Line, Arrow, PolyLine, Polygon, Cloud, Ink): reshaped by moving a
    /// point; the others resize by their box.
    pub fn is_pointwise(self) -> bool {
        matches!(
            self,
            AnnotationKind::Line
                | AnnotationKind::Arrow
                | AnnotationKind::PolyLine
                | AnnotationKind::Polygon
                | AnnotationKind::Cloud
                | AnnotationKind::Ink
        )
    }

    /// The English name the card and tooltips show ("Note", "Text box", "Strikethrough",
    /// "Rectangle"; PdfCraft's reader-friendly names).
    // pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:25 `subtype_label`.
    pub fn label(self) -> &'static str {
        todo!()
    }
}

/// An annotation as the overlay draws and edits it; the app maps its document's annotations to
/// these and back.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Annotation {
    pub id: AnnotationId,
    pub page: usize,
    pub kind: AnnotationKind,
    /// The bounding box on the page.
    pub rect: Rect,
    /// The defining points of pointwise kinds, and the quads of markups (four points each).
    pub points: Vec<Point>,
    /// Ink strokes.
    pub strokes: Vec<Vec<Point>>,
    /// Straight RGB.
    pub color: [u8; 3],
    pub opacity: f32,
    pub stroke_width: f64,
    pub text: String,
    pub locked: bool,
}

/// A handle of a selected annotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnnotationHandle {
    /// One of the eight box handles, clockwise from the top-left: `(-1, -1)`, `(0, -1)`, `(1, -1)`,
    /// `(1, 0)`, `(1, 1)`, `(0, 1)`, `(-1, 1)`, `(-1, 0)` (PdfCraft `geometry.rs:50`).
    Box(i8, i8),
    /// The defining point `i` of a pointwise kind.
    Point(usize),
}

/// What a point on the page hits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnnotationHit {
    Handle(AnnotationId, AnnotationHandle),
    Body(AnnotationId),
}

/// The handles `annotation` shows when selected: its points for pointwise kinds, else the eight
/// box handles (none for markups and locked annotations).
pub fn handles(annotation: &Annotation) -> Vec<(AnnotationHandle, Point)> {
    todo!()
}

/// The topmost hit at `at` within `tolerance` page units: a handle of a selected annotation
/// first, then any annotation's body (strokes and lines by distance, boxes by containment),
/// last drawn first.
pub fn hit(annotations: &[Annotation], selected: &[AnnotationId], at: Point, tolerance: f64) -> Option<AnnotationHit> {
    todo!()
}

/// `rect` with handle `(hx, hy)` moved by `delta`; with `aspect` (Shift on a corner, or a kind
/// that keeps its aspect) the ratio is kept; never inverted below one unit.
// pdfcraft: crates/ui-egui/src/tools/comments/geometry.rs:72 `resized`, `:68` `corner_ratio`.
pub fn resized(rect: Rect, handle: (i8, i8), delta: Vec2, aspect: Option<f64>) -> Rect {
    todo!()
}

/// `points` with point `i` moved to `to`; with `shift` the segment from its neighbour snaps to
/// 45° steps.
// pdfcraft: crates/ui-egui/src/tools/comments/geometry.rs:37 `reshaped`.
pub fn reshaped(points: &[Point], i: usize, to: Point, shift: bool) -> Vec<Point> {
    todo!()
}

/// What an annotation gesture asks of the app. Moves and reshapes are reported as the value of
/// an `EditEvent` gesture (one undo step per drag).
#[derive(Clone, Debug, PartialEq)]
pub enum AnnotationEdit {
    /// A new annotation drawn with a tool; for a Note or Text box the app opens the composer
    /// ([`CommentComposer`](crate::ui::CommentComposer)) on it.
    Create(Annotation),
    Move {
        ids: Vec<AnnotationId>,
        delta: Vec2,
    },
    Resize {
        id: AnnotationId,
        rect: Rect,
    },
    Reshape {
        id: AnnotationId,
        points: Vec<Point>,
    },
    Delete(Vec<AnnotationId>),
    /// The Eraser crossed these strokes of an Ink annotation.
    Erase {
        id: AnnotationId,
        strokes: Vec<usize>,
    },
    Select(Vec<AnnotationId>),
}
