//! [`WideAnglePanel`]: Adaptive Wide Angle's settings column (task WS-WA1).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::WideAngleLabels;
use super::session::{WideAngleConstraintEdit, WideAngleSession};

/// What [`WideAnglePanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WideAnglePanelOutput {
    pub response: Response,
    /// The correction values changed (written to the session).
    pub changed: bool,
    pub dragging: bool,
    /// A row's Orientation or Delete.
    pub edit: Option<WideAngleConstraintEdit>,
}

/// The settings column (Standard), top to bottom: **Correction** (Auto, Fisheye, Perspective, Full
/// Spherical), Scale (50..=150 %), Focal Length (0..=200 mm), Crop Factor (0.1..=10); **Constraints
/// (N)** (a `Counted` header): one row per constraint with its number, an Orientation select and
/// Delete; Preview (a check box: the corrected preview or the source); the hint "Drag on the image
/// to add a constraint; Shift for horizontal/vertical; right-click a constraint to change or delete
/// it." in the caption role, `text.faint`. The footer (OK, Cancel at the bottom) is the column's;
/// PhotoCraft's full-width footer and its "N ms · straightness" text are gone (lead calls 11, 14).
#[expect(dead_code, reason = "skeleton: `show` (task WS-WA1) reads the fields")]
pub struct WideAnglePanel<'a> {
    id_salt: IdSalt,
    session: &'a mut WideAngleSession,
    preview: &'a mut bool,
    labels: &'a WideAngleLabels,
}

impl<'a> WideAnglePanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, session: &'a mut WideAngleSession, preview: &'a mut bool, labels: &'a WideAngleLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), session, preview, labels }
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-WA1")]
    pub fn show(self, ui: &mut Ui) -> WideAnglePanelOutput {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:363-437 (Correction, the sliders, the
        // constraint rows, Preview, the hint).
        todo!()
    }
}
