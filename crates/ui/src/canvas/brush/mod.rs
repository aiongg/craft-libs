//! The brush tip on the canvas: its cursor outline and the resize gesture.
//!
//! Every app with a brush (PhotoCraft's painting tools and Quick Selection, the imaging library's
//! Liquify, Select and Mask and Content-Aware Fill brushes, LightCraft's masks) describes its tip
//! once and gets Photoshop's pointers and on-canvas resize. The engine computes the numbers (the
//! 50 % contour, a traced outline, the falloff); the library never sees an engine type.
//!
//! - [`BrushTip`], [`BrushTipOutline`], [`TipRings`]: the tip as the cursor draws it: a round tip's
//!   two radii, a traced shape's rings, or an aliased tip's whole-pixel cell.
//! - [`TipPose`]: the pen's tilt and rotation as the tip's axes in document space.
//! - [`BrushCursor`]: the [`ToolCursor`](super::ToolCursor) for the tip under the user's
//!   [`CursorOptions`](super::CursorOptions) (Standard, Precise, Normal or Full Size Brush Tip,
//!   Caps Lock, Show Only Crosshair While Painting), through zoom, rotation, flip and pixel aspect.
//! - [`BrushResize`]: Photoshop's resize drag (Alt + right-drag, or Ctrl + Alt + drag): left and
//!   right set the diameter, up and down the hardness ([`BrushLevel`]), reported as
//!   [`EditEvent<BrushDimensions>`](crate::edit::EditEvent).
//! - [`TipFalloff`]: the tip's falloff for the resize preview.
//! - [`BrushStyle`]: the sizes and colours, from the `cursor` and `brush` token families.
//!
//! ```no_run
//! use craft_ui::canvas::brush::{BrushCursor, BrushStyle, BrushTip, TipPose};
//! use craft_ui::canvas::{CanvasTool, CursorOptions, ToolContext, ToolCursor};
//! # struct BrushTool { tip: BrushTip, pose: TipPose, options: CursorOptions, stroking: bool }
//! # impl BrushTool {
//! fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
//!     BrushCursor::new(&self.tip).pose(self.pose).painting(self.stroking)
//!         .cursor(&BrushStyle::of(cx.tokens), &self.options, cx.view, cx.caps_lock)
//! }
//! # }
//! ```

mod cursor;
#[cfg(test)]
mod cursor_tests;
mod falloff;
mod pixel_tip;
#[cfg(test)]
mod pixel_tip_tests;
mod resize;
#[cfg(test)]
mod resize_tests;
pub(crate) mod stories;
mod style;
mod tip;

pub use cursor::BrushCursor;
pub use falloff::TipFalloff;
pub use resize::{BrushDimensions, BrushLevel, BrushResize, BrushResizeLabels, BrushResizeOptions};
pub use style::BrushStyle;
pub use tip::{BrushTip, BrushTipOutline, TipPose, TipRings};
