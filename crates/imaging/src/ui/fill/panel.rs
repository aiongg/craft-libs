//! [`FillPanel`]: the Content-Aware Fill panel (task WS-CA2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::FillLabels;
use super::session::{FillReset, FillSettings, SamplingMode};

/// What [`FillPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FillPanelOutput {
    pub response: Response,
    /// A setting changed (written to the settings): the tint and the preview follow.
    pub changed: bool,
    pub dragging: bool,
    /// A sampling option picked (the session changes: `FillSession::set_sampling`).
    pub sampling: Option<SamplingMode>,
    /// A section's Reset pressed.
    pub reset: Option<FillReset>,
    /// The Color swatch pressed: the workspace asks the app for a colour (`ColorRequest`, key
    /// `fill.overlayColor`).
    pub pick_color: bool,
}

/// The settings column headed "Content-Aware Fill" (lead call 10), top to bottom:
///
/// - **Sampling Area Overlay** (collapsed at first): Show Sampling Area, Opacity (0..=100 %),
///   Color (a swatch), Indicates (Sampling Area, Excluded Area).
/// - **Sampling Area Options** with a Reset icon button at the header's trailing end: Auto,
///   Rectangular, Custom (a segmented choice), Sample All Layers.
/// - **Fill Settings** with its Reset: Color Adaptation (None, Default, High, Very High), Rotation
///   Adaptation (None, Low, Medium, High, Full), Scale, Mirror.
/// - **Output Settings**: Output To (Current Layer, New Layer, Duplicate Layer).
/// - The error line (a limit, a failed Apply or preview) in `status.bad.fg`, caption role.
///
/// The footer (Reset at the leading end; OK, Apply, Cancel in the platform's order, lead call 12)
/// is the side column's, drawn by the workspace.
#[expect(dead_code, reason = "skeleton: `show` (task WS-CA2) reads the fields")]
pub struct FillPanel<'a> {
    id_salt: IdSalt,
    settings: &'a mut FillSettings,
    sampling: SamplingMode,
    error: Option<&'a str>,
    labels: &'a FillLabels,
}

impl<'a> FillPanel<'a> {
    /// The panel over `settings`, with the session's `sampling` option shown as picked.
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut FillSettings, sampling: SamplingMode, labels: &'a FillLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, sampling, error: None, labels }
    }

    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA2")]
    pub fn show(self, ui: &mut Ui) -> FillPanelOutput {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:118 `caf_props_panel`.
        todo!()
    }
}
