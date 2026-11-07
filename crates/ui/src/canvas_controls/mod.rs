//! The canvas controls: leaves a `Canvas` body shows (`#[component(canvas_control)]`).

mod words;

// Selection outlines, grids and previews (canvas-controls-b).
mod bands;
mod grid;
mod line_preview;
mod marquee;
mod outline;
mod pixel_grid;

pub use bands::{range_bands, rect_around};
pub use grid::{CanvasGrid, GridAxis, GridLine, GridLines, GridStyle, MAX_LINES};
pub use line_preview::{LineGuideStyle, StraightLinePreview};
pub use marquee::{Marquee, MarqueeLook, MarqueeShape, MarqueeStyle, Run};
pub use outline::{HoverMask, HoverOutline, HoverShape, OutlineStyle, SelectionOutline, SelectionOutlineLook};
pub use pixel_grid::{GridSegment, PixelGrid, PixelGridStyle};
pub use words::SelectionWords;
// End of canvas-controls-b.
