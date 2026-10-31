//! Canvas overlays: what the design system draws over a document (marquees, guides, rulers) and
//! the foundation they share. Overlays draw and sense; the document, its undo and the tool logic stay in the app.
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
//! - [`CanvasGrid`]: major and minor grid lines over a document, solid, dashed, dotted or as crossings.
//! - [`Guides`]: ruler guides; [`SmartGuides`]: alignment lines and distances during a move.
//! - [`quad`]: transform box geometry, where a box goes when a part of it is dragged.
//! - [`PartFocus`]: one Tab stop for an overlay made of parts, with Page keys to pick one.
//! - [`ReferencePoint`]: the 3 × 3 picker of a transform box's reference point.
//! - [`turn_cursor`]: the rotate pointer outside a transform box's corners; [`distort_cursor`],
//!   [`skew_cursor`] and [`selection_cursor`] are the other painted pointers; [`PointerInk`] says
//!   whether the system pointer stays or the pointer is painted whole.
//!
//! - [`ToolHost`] and [`CanvasTool`]: the on-canvas tool host one tool at a time runs on; [`ToolCursor`]
//!   and [`CursorHost`]: the pointer over the canvas; [`PenInput`]: pen pressure, tilt and the eraser end;
//!   [`SnapTargets`]: snapping geometry.
//! - [`zoom`]: the zoom range and ladder ([`ZoomLadder`]), fit and fill, zoom about a point,
//!   [`ZoomAnimation`], the Zoom tool's [`HoldZoom`] and [`ZoomDrag`], [`ZoomWindowFit`].
//! - [`navigate`]: wheel, pinch and twist, page keys, flick pan, [`Autoscroll`], [`RotateView`],
//!   all under the user's [`NavigationPrefs`]; [`ViewHistory`]: Go Back and Go Forward.
//! - [`MomentaryKeys`] and [`SpringTool`]: temporary tools while a key is held.
//! - [`RasterDisplay`]: the CPU raster path over a [`RasterSource`], with its [`Checkerboard`];
//!   [`PixelGrid`]: the pixel grid above 500 %.
//! - [`Pasteboard`]: the area around the pages and their [`PageFrame`]s; [`SplitViews`]: 1, 2 or
//!   4 views of one document.
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

pub mod claim;
pub(crate) mod core_stories;
mod crisp;
mod cursor;
#[cfg(test)]
mod cursor_tests;
mod grid;
pub mod guides;
pub mod hit;
pub(crate) mod marquee;
mod momentary;
#[cfg(test)]
mod momentary_tests;
pub mod navigate;
#[cfg(test)]
mod navigate_tests;
mod part_focus;
mod pasteboard;
#[cfg(test)]
mod pasteboard_tests;
mod pixel_grid;
#[cfg(test)]
mod pixel_grid_tests;
mod pointer;
#[cfg(test)]
mod pointer_tests;
pub mod quad;
mod raster;
#[cfg(test)]
mod raster_tests;
mod readout;
pub(crate) mod ref_point;
pub(crate) mod rulers;
mod scrollbars;
mod selection_bar;
#[cfg(test)]
mod selection_bar_tests;
mod snap;
#[cfg(test)]
mod snap_tests;
mod split;
#[cfg(test)]
mod split_tests;
pub(crate) mod story;
#[cfg(test)]
mod tests;
mod tool_host;
#[cfg(test)]
mod tool_host_tests;
mod tool_pill;
#[cfg(test)]
mod tool_pill_tests;
mod turn;
mod units;
mod view;
mod view_history;
#[cfg(test)]
mod view_history_tests;
pub mod zoom;
#[cfg(test)]
mod zoom_tests;

pub use crate::edit::order::Turn;
pub use ::kurbo;
pub use claim::Claim;
pub use crisp::{crisp, crisp_shape};
pub use cursor::{CursorBadge, CursorHost, CursorPrefs, CursorStyle, PaintingCursor, ToolCursor};
pub use grid::{CanvasGrid, GridLine, GridStyle};
pub use guides::{Guide, GuideLine, GuideNames, GuidePlace, GuideStyle, Guides, GuidesResponse, Measure, Orientation, SmartGuides, SmartLine};
pub use marquee::{
    Constraint, Marquee, MarqueeDrag, MarqueeLook, MarqueeRect, MarqueeResponse, MarqueeShape, MarqueeStyle, fixed_size_pixels, marquee_pixels,
    square_pixels,
};
pub use momentary::{MomentaryKeys, MomentaryTool, SpringTool};
pub use navigate::{Autoscroll, HandTool, NavigationLabels, NavigationPrefs, RotateView, RotateViewTool, ZoomTool};
pub use part_focus::{Part, PartFocus, PartFocusResponse};
pub use pasteboard::{PageBorder, PageFrame, Pasteboard, PasteboardColor, PasteboardResponse, PasteboardStyle};
pub use pixel_grid::{GridSegment, PixelGrid};
pub use pointer::{CanvasButtons, PenFeed, PenInput, PenSample};
pub use raster::{CheckerColors, CheckerSize, Checkerboard, DetailPlan, MAX_TEXTURE, PixelRect, RasterDisplay, RasterResponse, RasterSource};
pub use readout::{Readout, ReadoutStyle};
pub use ref_point::{RefPointLabels, RefSize, ReferencePoint, ReferencePointStyle};
pub use rulers::{RulerNames, RulerPart, RulerStyle, Rulers, RulersResponse};
pub use scrollbars::{CanvasScrollbars, ScrollbarNames, ScrollbarStyle, ScrollbarsResponse};
pub use snap::{SNAP_PX, SnapGesture, SnapKind, SnapLine, SnapOptions, SnapTarget, SnapTargets, union as snap_union};
pub use split::{SplitLabels, SplitLayout, SplitPane, SplitResponse, SplitViews};
pub use tool_host::{CanvasTool, NoTool, SnapAt, ToolAction, ToolContext, ToolGesture, ToolHost, ToolHostResponse, ToolInput, ToolKey, ToolPointer};
pub use selection_bar::{SelectionBar, SelectionBarLabels, SelectionBarResponse};
pub use tool_pill::{PillPlace, ToolPill, ToolPillLabels, ToolPillResponse};
pub use turn::{
    PointerInk, SelectionBadge, distort_cursor, selection_badge_at, selection_badge_beside_pointer, selection_cursor, skew_cursor, turn_cursor,
    turn_cursor_with,
};
pub use units::{RulerUnits, Ticks};
pub use view::{CanvasView, resize_cursor};
pub use view_history::{ViewHistory, ViewPlace};
pub use zoom::{HoldZoom, ZoomAnimation, ZoomDrag, ZoomLadder, ZoomWindowFit};
