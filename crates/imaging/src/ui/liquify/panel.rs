//! [`LiquifyPanel`]: Liquify's Properties panel (task WS-LQ2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::LiquifyLabels;
use super::session::{LiquifyGlobal, LiquifySettings};

/// What [`LiquifyPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct LiquifyPanelOutput {
    pub response: Response,
    /// A setting changed this pass (already written to the settings).
    pub changed: bool,
    /// A slider is being dragged: the workspace's history waits.
    pub dragging: bool,
    /// Reconstruct (with the Amount), None, Mask All or Invert All pressed.
    pub global: Option<LiquifyGlobal>,
    /// Restore All pressed.
    pub restore_all: bool,
    /// Show Backdrop turned on, or Use changed while it is on: the backdrop to ask the app for
    /// (`None`: All Layers).
    pub backdrop_wanted: Option<Option<u64>>,
}

/// The Properties panel in the side column, top to bottom, as PhotoCraft (and Photoshop) order it:
///
/// - **Brush Tool Options** (`SectionHeader`): `SliderRow`s Size (1..=3000, the
///   [`LiquifyBrush::SLIDER_MAX`](super::LiquifyBrush::SLIDER_MAX)), Density, Pressure, Rate
///   (0..=100).
/// - **Brush Reconstruct Options**: Amount (0..=100 %); buttons Reconstruct and Restore All.
/// - **Mask Options**: buttons None, Mask All, Invert All.
/// - **View Options**: Show Mesh; Mesh Size (Small, Medium, Large); Show Mask; Show Backdrop and,
///   while it is on, Use (All Layers, then the source's layers bottom to top), Mode (In Front,
///   Behind, Blend), Opacity (0..=100).
///
/// PhotoCraft's stats line ("N stroke(s) · field … · dab … ms · preview … ms") is not drawn (lead
/// call 14). Every control is a Tab stop in that order; sliders report drags through `dragging`.
#[expect(dead_code, reason = "skeleton: `show` (task WS-LQ2) reads the fields")]
pub struct LiquifyPanel<'a> {
    id_salt: IdSalt,
    settings: &'a mut LiquifySettings,
    layers: &'a [(u64, String)],
    labels: &'a LiquifyLabels,
}

impl<'a> LiquifyPanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut LiquifySettings, labels: &'a LiquifyLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, layers: &[], labels }
    }

    /// Show Backdrop › Use's layers, bottom to top ([`LiquifySource::layers`](super::LiquifySource::layers)).
    pub fn layers(mut self, layers: &'a [(u64, String)]) -> Self {
        self.layers = layers;
        self
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ2")]
    pub fn show(self, ui: &mut Ui) -> LiquifyPanelOutput {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:995-1059 (the four sections).
        todo!()
    }
}
