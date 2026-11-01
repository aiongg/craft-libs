//! [`PerspectiveBar`]: Perspective Warp's options bar (task TR-T13).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T13)")]

use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::localized::words::Counted;
use craft_ui::words;

use super::{PerspectiveEdit, PerspectiveSession};

/// The plane count in Layout mode: "{n} plane", "{n} planes" (registered by RG1).
pub(crate) const PLANES: Counted = Counted::new("perspectiveBar.planes", "{n} plane", "{n} planes");

words! {
    /// The words of [`PerspectiveBar`] (PhotoCraft's Perspective Warp bar). `Default` is English.
    pub struct PerspectiveBarLabels {
        layout: "perspectiveBar.layout" = "Layout";
        warp: "perspectiveBar.warp" = "Warp";
        mode: "perspectiveBar.mode" = "Mode";
        remove_planes: "perspectiveBar.removePlanes" = "Remove Planes";
        straighten: "perspectiveBar.straighten" = "Straighten";
        straighten_tip: "perspectiveBar.straightenTip" = "Automatically straighten near-vertical line segments";
        level: "perspectiveBar.level" = "Level";
        level_tip: "perspectiveBar.levelTip" = "Automatically level near-horizontal line segments";
        both: "perspectiveBar.both" = "Both";
        both_tip: "perspectiveBar.bothTip" = "Automatically straighten and level";
        commit: "perspectiveBar.commit" = "Commit Perspective Warp";
        cancel: "perspectiveBar.cancel" = "Cancel Perspective Warp";
    }
}

/// Perspective Warp's options bar, in Tab order: the Layout and Warp tabs (`Segmented`, LC30;
/// Warp disabled with no plane); in Layout the plane count ("2 planes", through [`Counted`]) and
/// Remove Planes; in Warp Straighten, Level and Both with their tooltips; then cancel and commit at
/// the reading end (`OptionsBar::commit_cancel`). Every control is an ordinary focusable control;
/// the bar never changes the session.
pub struct PerspectiveBar<'a> {
    id_salt: IdSalt,
    session: &'a PerspectiveSession,
    labels: PerspectiveBarLabels,
}

/// What the bar reported this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PerspectiveBarOutput {
    /// The edits in order; the app applies each with [`PerspectiveSession::apply`].
    pub edits: Vec<PerspectiveEdit>,
    /// Commit pressed.
    pub apply: bool,
    /// Cancel pressed.
    pub discard: bool,
}

impl<'a> PerspectiveBar<'a> {
    /// The bar of `session`.
    pub fn new(id_salt: impl AsIdSalt, session: &'a PerspectiveSession) -> Self {
        Self { id_salt: IdSalt::new(id_salt), session, labels: PerspectiveBarLabels::default() }
    }

    pub fn labels(mut self, labels: PerspectiveBarLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Draws the bar into the options bar's `ui`.
    pub fn show(self, ui: &mut Ui) -> PerspectiveBarOutput {
        // pc: crates/ui-egui/src/tools/perspective_ui.rs:310 (`options_bar`).
        todo!()
    }
}
