//! The monitor type of persisted layouts.

/// A display in desktop logical points, the one monitor type of saved data. Applications on an egui with monitor
/// reports fill their monitor list with these; a native floating window stores the one it is on.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct Monitor {
    /// Display name from the OS, when it reports one.
    pub name: Option<String>,
    /// `[x, y, width, height]` on the desktop.
    pub rect: [f32; 4],
    /// Physical pixels per logical point.
    pub scale: f32,
}

impl Monitor {
    /// A display at `rect` with `scale` physical pixels per logical point.
    pub fn new(name: Option<String>, rect: [f32; 4], scale: f32) -> Self {
        Self { name, rect, scale }
    }
}
