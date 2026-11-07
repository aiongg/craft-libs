//! The canvas controls: leaves a `Canvas` body shows (`#[component(canvas_control)]`).

mod words;

/// A canvas-control type, as `#[component(canvas_control)]` registers it, so the canvas-control cases run over it.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct CanvasControl {
    /// The component's type name.
    pub type_name: &'static str,
}

impl CanvasControl {
    /// A registration for the type named `type_name`.
    pub const fn new(type_name: &'static str) -> Self {
        CanvasControl { type_name }
    }
}

/// Every registered canvas-control type in the running binary.
#[cfg(feature = "testing")]
#[linkme::distributed_slice]
pub static CANVAS_CONTROLS: [CanvasControl];
