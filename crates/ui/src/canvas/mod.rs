//! Canvas overlays: what the design system draws over a document (marquees, guides, rulers,
//! transform boxes, paths) and the foundation they share. Overlays draw and sense; the document,
//! its undo and the tool logic stay in the app.
//!
//! - [`CanvasView`]: the mapping between document and screen (centre, zoom, rotation, flip,
//!   pixel aspect). Apps convert their own view type into it once per frame.
//! - [`kurbo`]: document geometry (`Point`, `Vec2`, `Rect`, `Affine`), in f64. Screen geometry is
//!   egui's.
//! - [`crisp`]: horizontal and vertical lines on whole device pixels.
//! - [`Claim`]: which overlay owns the current press on a canvas, so one press moves one thing.
//! - [`hit`]: distances from the pointer to screen geometry, for hit tests that follow a rotated
//!   view.
//! - [`resize_cursor`]: the resize arrow for a document direction as the view shows it.
//! - [`Readout`]: labelled values beside the pointer during a gesture.
//! - [`RulerUnits`]: the ruler unit of a document axis, for ruler ticks and guide values.
//! - [`Rulers`]: the strips along the top and left of a canvas, with the zero point's corner.
//! - [`CanvasScrollbars`]: the bars along the bottom and right of a canvas, and the clamp that keeps
//!   the view on the document when overscroll is off.
//! - [`Guides`]: ruler guides; [`SmartGuides`]: alignment lines and distances during a move.
//! - [`quad`]: transform box geometry, where a box goes when a part of it is dragged.
//! - [`PartFocus`]: one Tab stop for an overlay made of parts, with Page keys to pick one.
//! - [`TransformFields`]: the options bar's X, Y, W, H, angle and skew fields of a transform box;
//!   [`ReferencePoint`]: the 3 × 3 picker of its reference point.
//! - [`turn_cursor`]: the rotate pointer outside a transform box's corners; [`distort_cursor`],
//!   [`skew_cursor`] and [`selection_cursor`] are the other painted pointers; [`PointerInk`] says
//!   whether the system pointer stays or the pointer is painted whole.
//! - [`TransformBox`]: the box of Free Transform, its handles and reference point.
//! - [`BezierOverlay`]: paths with anchors and direction handles, edited as Direct Selection and
//!   Convert Point edit them; [`EditPath`] and its parts are the paths overlays share.
//! - [`WarpGrid`]: a warp's mesh ([`WarpMesh`]), its anchors and handles dragged and nudged.
//! - [`GradientOverlay`]: a gradient's line with its end handles, colour stops and midpoints;
//!   [`gradient_line`] is where each sits.
//! - [`PenDraft`]: the path being drawn with the Pen, its gestures and keys.
//!
//! Nudging with the arrow keys is [`behavior::nudge`](crate::behavior::nudge).
//!
//! Overlays sense through the host's canvas `Response` (one `interact` over the canvas rect)
//! rather than widgets of their own, and run before the host's tool input, in priority order:
//! handles, guides, then the tool. Every tolerance is in screen points. Nothing on the canvas
//! mirrors in a right-to-left interface ([`Unmirrored::Canvas`](crate::direction::Unmirrored)).
//!
//! ```
//! use craft_ui::canvas::{CanvasView, Readout, kurbo::Point};
//! # egui::__run_test_ui(|ui| {
//! let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
//! let view = CanvasView { rotation: 30.0, ..CanvasView::new(rect, Point::new(320.0, 200.0), 0.5) };
//! if let Some(pointer) = canvas.hover_pos() {
//!     let at = view.to_doc(pointer);
//!     Readout::new("pointer").row("X:", format!("{:.0}", at.x)).row("Y:", format!("{:.0}", at.y)).show(ui.ctx(), pointer);
//! }
//! # });
//! ```

pub(crate) mod bezier;
mod claim;
mod crisp;
pub mod gradient_line;
pub(crate) mod gradient_overlay;
pub(crate) mod guides;
pub mod hit;
pub(crate) mod marquee;
mod part_focus;
pub(crate) mod pen;
pub mod quad;
mod readout;
pub(crate) mod ref_point;
pub(crate) mod rulers;
mod scrollbars;
pub(crate) mod story;
#[cfg(test)]
mod tests;
pub(crate) mod transform_box;
pub(crate) mod transform_fields;
mod turn;
mod units;
mod view;
pub(crate) mod warp;

pub use crate::edit::order::Turn;
pub use ::kurbo;
pub use bezier::{
    BezierHit, BezierNames, BezierOverlay, BezierResponse, BezierStyle, BezierTool, EditPath, EditSubpath, HandleSide, HandlesShown, Knot, KnotRef,
    MAX_ANCHORS, PathEdit, PathSelection,
};
pub use claim::Claim;
pub use crisp::{crisp, crisp_shape};
pub use gradient_line::{GradientExtent, GradientLine, LineGradient, LinePart};
pub use gradient_overlay::{GradientLineNames, GradientLineStyle, GradientOverlay, GradientOverlayResponse, LineCursor};
pub use guides::{Guide, GuideLine, GuideNames, GuidePlace, GuideStyle, Guides, GuidesResponse, Measure, Orientation, SmartGuides, SmartLine};
pub use marquee::{
    Constraint, Marquee, MarqueeDrag, MarqueeLook, MarqueeRect, MarqueeResponse, MarqueeShape, MarqueeStyle, fixed_size_pixels, marquee_pixels,
    square_pixels,
};
pub use part_focus::{Part, PartFocus, PartFocusResponse};
pub use pen::{PenDraft, PenNames, PenResponse, PenStyle};
pub use readout::{Readout, ReadoutStyle};
pub use ref_point::{RefSize, ReferencePoint, ReferencePointStyle};
pub use rulers::{RulerNames, RulerPart, RulerStyle, Rulers, RulersResponse};
pub use scrollbars::{CanvasScrollbars, ScrollbarNames, ScrollbarStyle, ScrollbarsResponse};
pub use transform_box::{RotateZone, TransformBox, TransformBoxResponse, TransformLabels, TransformNames, TransformStyle};
pub use transform_fields::{FieldLabel, SizeUnit, TransformFieldLabels, TransformFields, TransformRow};
pub use turn::{
    PointerInk, SelectionBadge, distort_cursor, selection_badge_at, selection_badge_beside_pointer, selection_cursor, skew_cursor, turn_cursor,
    turn_cursor_with,
};
pub use units::{RulerUnits, Ticks};
pub use view::{CanvasView, resize_cursor};
pub use warp::{WarpEdit, WarpGrid, WarpMesh, WarpNames, WarpResponse, WarpSide, WarpSplit, WarpStyle};
