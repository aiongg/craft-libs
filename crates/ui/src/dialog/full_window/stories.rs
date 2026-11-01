//! Stories of the full-window regions (stub; task KS3 writes them and RG2 registers them).
//!
//! Every story string goes through story words; the image is a fake `RasterSource` (a gradient
//! with shapes and a transparent corner), so every state shows without an app.
//!
//! - `patterns/full-window/frame`: title strip, options bar, a `ToolRail` with a group gap, one
//!   Wide `FullWindowSide` with header and footer, the canvas. Args: footer at top or bottom, a
//!   second side column, Size Compact and Comfortable, right-to-left.
//! - `patterns/full-window/canvas`: fit, zoom steps, pan, the Hand and Zoom tools, the
//!   checkerboard, a texel-scaled `RasterOverlay` with a tint, the `FullWindowZoomBar` with a
//!   progress bar and with a note, `PreviewStatus` pending and failed.
//! - `patterns/full-window/rail-in-dialog`: `ToolRail` with group gaps and a shared-letter slot
//!   inside a `FullWindowDialog`, `pick_by_key` on.
