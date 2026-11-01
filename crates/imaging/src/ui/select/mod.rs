//! The marquee and lasso selection tools.
//!
//! Each tool is a [`CanvasTool`] on the canvas's `ToolHost`: it draws its outline while the user
//! works and reports [`SelectEvent`]s, which the app drains with `take_events` after
//! `ToolHost::show` and turns into its own commands. No tool changes the document.
//!
//! - [`MarqueeTool`]: the Rectangular, Elliptical, Single Row and Single Column Marquees.
//! - [`LassoTool`]: freehand, with ⌥-latched straight segments.
//! - [`PolygonLassoTool`]: click by click, ⇧ at 45°.
//! - [`MagneticLassoTool`]: an outline that snaps to edges the app's [`EdgeTracer`] finds.
//!
//! Every tool keeps its state in itself between passes and asks the app's [`SelectionProbe`]
//! about the selection under the pointer. The app wraps the tool and the probe in a
//! [`SelectToolPass`] for each pass:
//!
//! ```ignore
//! let probe = DocSelection { doc: &doc };                  // the app's SelectionProbe
//! self.lasso.set_operation(bar.operation);                 // the options bar's mode
//! let mut pass = SelectToolPass::new(&mut self.lasso, &probe);
//! let _ = ToolHost::new(canvas_id).show(ui, &view, &canvas, &mut pass);
//! for event in self.lasso.take_events() {
//!     match event {
//!         SelectEvent::Commit { shape, operation } => app.run_select(shape, operation), // one undo step
//!         SelectEvent::Deselect => app.run("select.deselect"),
//!         SelectEvent::DragSelection(edit) => app.drag_selection(edit),                // EditEvent<SelectionDrag>
//!         _ => {}
//!     }
//! }
//! ```
//!
//! A press inside the selection drags it instead of drawing (Photoshop): the outline with a plain
//! press and the operation New, the selected pixels with ⌘ (⌘⌥ a copy), a floating piece with a
//! plain press. Click-driven tools (the Polygonal Lasso) need ⌘ for any of it, and never while an
//! outline is open. A ⌘ press outside the selection is [`ToolAction::Ignored`](craft_ui::canvas::ToolAction)
//! so the app's Move tool takes it (PhotoCraft's `cmd_moves`).
//!
//! [`CanvasTool`]: craft_ui::canvas::CanvasTool

use craft_ui::canvas::SelectionOperation;
use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::edit::EditEvent;
use craft_ui::localized::words::Counted;

mod drag;
mod lasso;
#[cfg(test)]
mod lasso_tests;
mod magnetic;
#[cfg(test)]
mod magnetic_tests;
mod marquee;
#[cfg(test)]
mod marquee_tests;
mod polygon;
#[cfg(test)]
mod polygon_tests;
pub(crate) mod stories;
mod style;

pub use drag::{InsideDrag, PressRule, cursor_over, drag_kind};
pub use lasso::LassoTool;
pub use magnetic::{EdgeSearch, EdgeTracer, MagneticLassoTool, MagneticSettings};
pub use marquee::{MarqueeKind, MarqueeTool};
pub use polygon::PolygonLassoTool;
pub use style::SelectToolStyle;

/// The point count of an outline being drawn, for the status line and the outline's AccessKit
/// value ("5 points").
pub(crate) const OUTLINE_POINTS: Counted = Counted::new("select.outlinePoints", "{n} point", "{n} points");

craft_ui::words! {
    /// The words of the selection tools, English by default.
    #[non_exhaustive]
    pub struct SelectLabels {
        /// The accessible name of an outline being drawn.
        outline: "select.outline" = "Selection outline";
        /// The marquee readout's width row.
        width: "select.width" = "W:";
        /// The marquee readout's height row.
        height: "select.height" = "H:";
    }
}

/// The selection a tool made, in document units. Anti-alias and Feather are the app's options and
/// ride with its command, not with the shape.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SelectionShape {
    /// A rectangle on whole pixels: the Rectangular Marquee, and the Single Row and Single Column
    /// Marquees' one-pixel line across the document.
    Rectangle(Rect),
    /// The ellipse inscribed in a rectangle on whole pixels: the Elliptical Marquee.
    Ellipse(Rect),
    /// A closed polygon of 3 or more points (the last joins the first): the Lasso, the Polygonal
    /// Lasso and the Magnetic Lasso.
    Polygon(Vec<Point>),
}

/// What a press inside the selection drags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DragKind {
    /// The outline alone (PhotoCraft's `select.transformSelection`).
    Outline,
    /// The selected pixels, or the floating piece: ⌘ cuts them out first, ⌘⌥ (`copy`) lifts a
    /// copy, a floating piece moves with a plain press (PhotoCraft's `select.float`).
    Pixels { copy: bool },
}

/// The value of a selection drag: what moves and by how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SelectionDrag {
    pub kind: DragKind,
    /// The offset from the press in whole document pixels (rounded, each within ±10⁷). The ants
    /// follow it through `SelectionOverlay::offset` while the drag runs.
    pub delta: [i32; 2],
}

/// What a selection tool reports. The app drains them after `ToolHost::show`, in order.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SelectEvent {
    /// An outline closed or a marquee was released: select `shape` with `operation` (latched at the
    /// press, [`SelectionOperation::resolve`]). One undo step.
    Commit { shape: SelectionShape, operation: SelectionOperation },
    /// A click that drew nothing (a marquee released within its click slop): deselect. One undo
    /// step (PhotoCraft's `select.deselect`).
    Deselect,
    /// A press inside the selection dragged it: `Begin` at the press, `Update` as it moves,
    /// `Commit` at the release (one undo step; `changed` false when the delta is 0, where the app
    /// deselects, or drops a floating piece that never moved, PhotoCraft's rule), `Cancel` on Esc.
    DragSelection(EditEvent<SelectionDrag>),
}

/// What a selection tool is doing, for the status line and a test.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum SelectPhase {
    /// Nothing under way.
    #[default]
    Idle,
    /// A marquee being dragged: the rectangle the release would select (whole pixels), `None`
    /// while the drag is still a click.
    Marquee { rect: Option<Rect> },
    /// An outline being drawn with `points` points so far.
    Outline { points: usize },
    /// The selection being dragged.
    Dragging(DragKind),
}

/// What a selection tool asks the app about the active document. The app implements it over its
/// document for one pass; the library never names a document type (lead call 3).
pub trait SelectionProbe {
    /// The active document's identity: an outline being drawn is dropped when it changes
    /// ([`OutlineDraft::for_document`](craft_ui::canvas::draft::OutlineDraft::for_document)).
    fn document(&self) -> u64;

    /// The document's size in pixels: the Single Row and Single Column Marquees span it.
    fn size(&self) -> [u32; 2];

    /// Whether anything is selected: ⌥ with nothing selected is not subtract
    /// ([`SelectionOperation::resolve`]).
    fn has_selection(&self) -> bool;

    /// Whether document point `at` is inside the selection (coverage above 0, PhotoCraft's
    /// `covers`).
    fn inside_selection(&self, at: Point) -> bool;

    /// Whether a floating piece (pixels lifted by a selection drag, not yet dropped) is on the
    /// canvas. Default: none.
    fn floating(&self) -> bool {
        false
    }

    /// Whether `at` is on the floating piece where it is shown now (the app subtracts the piece's
    /// offset). A press there drags the piece; one in the margin the cut left starts a new
    /// selection. Default: [`inside_selection`](Self::inside_selection).
    fn on_floating(&self, at: Point) -> bool {
        self.inside_selection(at)
    }
}

/// A selection tool with this pass's [`SelectionProbe`]: the [`CanvasTool`] the app hands to
/// `ToolHost::show`. Built every pass; the tool keeps its state, the probe borrows the document.
///
/// ```
/// use craft_imaging::ui::select::{LassoTool, SelectToolPass, SelectionProbe};
/// use craft_ui::canvas::CanvasTool;
/// use craft_ui::canvas::kurbo::Point;
///
/// struct Nothing;
/// impl SelectionProbe for Nothing {
///     fn document(&self) -> u64 { 1 }
///     fn size(&self) -> [u32; 2] { [100, 100] }
///     fn has_selection(&self) -> bool { false }
///     fn inside_selection(&self, _: Point) -> bool { false }
/// }
///
/// let mut lasso = LassoTool::new();
/// let mut pass = SelectToolPass::new(&mut lasso, &Nothing);
/// let _: &mut dyn CanvasTool = &mut pass;
/// ```
///
/// [`CanvasTool`]: craft_ui::canvas::CanvasTool
pub struct SelectToolPass<'a, T> {
    tool: &'a mut T,
    probe: &'a dyn SelectionProbe,
}

impl<'a, T> SelectToolPass<'a, T> {
    /// `tool` asking `probe` this pass.
    pub fn new(tool: &'a mut T, probe: &'a dyn SelectionProbe) -> Self {
        Self { tool, probe }
    }
}
