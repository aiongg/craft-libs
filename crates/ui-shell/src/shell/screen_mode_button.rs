//! `ScreenModeButton`: the status bar's screen mode button, a flyout of the three modes.

use super::screen_mode::ScreenMode;
use egui::{Response, Ui};

/// A flyout button over the three screen modes, with `mode` marked: a click, Enter or Space cycles; a hold, a drag away
/// or a right click lists the three modes without cycling. The face is the Cycle command's icon and tooltip; disabled
/// while Cycle is disabled.
#[must_use = "show the button with `show`"]
#[derive(crate::contract::Pattern)]
pub struct ScreenModeButton {
    #[allow(dead_code)] // P3(panels-w2): pn.C16
    mode: ScreenMode,
}

/// What a [`ScreenModeButton`] reports.
#[derive(Debug)]
pub struct ScreenModeButtonOutput {
    /// The button's response.
    pub response: Response,
}

impl ScreenModeButton {
    /// A button showing `mode` as the current one.
    pub fn new(mode: ScreenMode) -> Self {
        ScreenModeButton { mode }
    }

    /// Shows the button in `ui`.
    pub fn show(self, ui: &mut Ui) -> ScreenModeButtonOutput {
        let _ = ui;
        todo!("pn.C16")
    }
}
