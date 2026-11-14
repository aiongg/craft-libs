//! [`VectorModeSelect`]: the Shape, Path and Pixels mode choice of the vector tools.

use super::VectorMode;
use craft_ui::contract::{Bind, EditResponse};
use egui::Ui;

/// The mode select of the vector tools' bars; a mode in `offered` with `false` shows disabled.
#[must_use = "show the select with `show`"]
#[derive(craft_ui::contract::Pattern)]
pub struct VectorModeSelect<'a> {
    mode: Bind<'a, VectorMode>,
    offered: &'a [(VectorMode, bool)],
}

impl<'a> VectorModeSelect<'a> {
    /// The select of `mode` over the `offered` modes, each with whether it is enabled.
    pub fn new(mode: Bind<'a, VectorMode>, offered: &'a [(VectorMode, bool)]) -> Self {
        VectorModeSelect { mode, offered }
    }

    /// Shows the select and reports the picked mode.
    pub fn show(self, ui: &mut Ui) -> EditResponse<VectorMode> {
        let VectorModeSelect { mode, offered } = self;
        let _ = (ui, mode, offered);
        todo!("tl.T29")
    }
}
