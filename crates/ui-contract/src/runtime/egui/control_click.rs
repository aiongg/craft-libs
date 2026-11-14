//! macOS Control-click: a primary press with Control held is a secondary press.

/// The state that keeps a Control-click's events secondary until its release.
#[derive(Default)]
#[allow(dead_code)] // P3(canvas-widgets-w4): tp.T39
pub(crate) struct ControlClick;

#[allow(dead_code)] // P3(canvas-widgets-w4): tp.T39
impl ControlClick {
    /// On macOS a primary press with Control and neither Option nor Command becomes a secondary press, and its events
    /// stay secondary until the release; Control-Option and Control-Command presses stay primary.
    pub(crate) fn normalize(&mut self, events: &mut [egui::Event], macos: bool) {
        let _ = (events, macos);
        todo!("tp.T39")
    }
}
