//! [`VanishingPointBar`]: Vanishing Point's options bar (task WS-VP2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::VanishingPointLabels;
use super::editor::{PasteImage, VanishingPointOptions, VanishingTool};

/// What [`VanishingPointBar::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VanishingPointBarOutput {
    pub response: Response,
    /// An option changed (written to the options).
    pub changed: bool,
    /// The paste's Width changed (a session change: one undo step when the field settles).
    pub paste: Option<PasteImage>,
}

/// The options bar, full width under the title: for Stamp Diameter (1..=5000 px), Hardness,
/// Opacity (0..=100 %), Aligned; for every tool Grid Size (px); with an image on the clipboard
/// Paste and Width (percent of the plane); New Layer. PhotoCraft's tool pills and its ✓ and ⊘
/// buttons go: the strip holds the tools and the side column's footer OK and Cancel.
#[expect(dead_code, reason = "skeleton: `show` (task WS-VP2) reads the fields")]
pub struct VanishingPointBar<'a> {
    id_salt: IdSalt,
    tool: VanishingTool,
    options: &'a mut VanishingPointOptions,
    paste: PasteImage,
    has_clip: bool,
    labels: &'a VanishingPointLabels,
}

impl<'a> VanishingPointBar<'a> {
    pub fn new(
        id_salt: impl AsIdSalt,
        tool: VanishingTool,
        options: &'a mut VanishingPointOptions,
        paste: PasteImage,
        has_clip: bool,
        labels: &'a VanishingPointLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, options, paste, has_clip, labels }
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-VP2")]
    pub fn show(self, ui: &mut Ui) -> VanishingPointBarOutput {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1230 `options_bar`.
        todo!()
    }
}
