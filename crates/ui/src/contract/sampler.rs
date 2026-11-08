//! The canvas reach under a modal layer, as a component asks for it (15.5, 8.3); the runtime side is `runtime::sampler`.

use super::{Component, Cx};
use crate::runtime::sampler::{self, Armed, Reach};
use egui::Id;

impl<C: Component> Cx<'_, C> {
    /// Arms a sampler for `canvas` (the canvas component's id): while a modal layer is open, primary presses on the
    /// canvas pass through it to the canvas, and so does the view navigation (`keep_view_open`). The arming control
    /// calls it in every pass while it is armed. The reach holds for this pass and the next.
    pub fn arm_sampler(&mut self, canvas: Id) {
        sampler::register(&self.core.ctx, Armed { canvas, reach: Reach::Sample });
    }

    /// Keeps the view of `canvas` (the canvas component's id) usable under a modal layer: the wheel scrolls and zooms
    /// it, and a middle press pans it, as in Photoshop's Color Picker and Levels dialogs. Its other input stays
    /// blocked. The dialog calls it in every pass while it is open.
    pub fn keep_view_open(&mut self, canvas: Id) {
        sampler::register(&self.core.ctx, Armed { canvas, reach: Reach::View });
    }
}
