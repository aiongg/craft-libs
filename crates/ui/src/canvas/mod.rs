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
//! - [`Guides`]: ruler guides; [`SmartGuides`]: alignment lines and distances during a move.
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

mod claim;
mod crisp;
pub(crate) mod guides;
pub mod hit;
pub(crate) mod marquee;
mod readout;
pub(crate) mod rulers;
pub(crate) mod story;
#[cfg(test)]
mod tests;
mod units;
mod view;

pub use ::kurbo;
pub use claim::Claim;
pub use crisp::{crisp, crisp_shape};
pub use guides::{Guide, GuideLine, GuidePlace, GuideStyle, Guides, GuidesResponse, Measure, Orientation, SmartGuides, SmartLine};
pub use marquee::{Constraint, Marquee, MarqueeDrag, MarqueeLook, MarqueeRect, MarqueeResponse, MarqueeShape, MarqueeStyle};
pub use readout::{Readout, ReadoutStyle};
pub use rulers::{RulerPart, RulerStyle, Rulers, RulersResponse};
pub use units::{RulerUnits, Ticks};
pub use view::{CanvasView, resize_cursor};
