//! [`PinWarpBar`]: Puppet Warp's options bar (task TR-T12).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T12)")]

use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::words;

use super::{PinEdit, PinWarpSession};

words! {
    /// The words of [`PinWarpBar`] (PhotoCraft's Puppet Warp bar). `Default` is English.
    pub struct PinWarpBarLabels {
        mode: "pinWarpBar.mode" = "Mode:";
        rigid: "pinWarpBar.rigid" = "Rigid";
        normal: "pinWarpBar.normal" = "Normal";
        distort: "pinWarpBar.distort" = "Distort";
        density: "pinWarpBar.density" = "Density:";
        fewer_points: "pinWarpBar.fewerPoints" = "Fewer Points";
        more_points: "pinWarpBar.morePoints" = "More Points";
        expansion: "pinWarpBar.expansion" = "Expansion:";
        show_mesh: "pinWarpBar.showMesh" = "Show Mesh";
        pin_depth: "pinWarpBar.pinDepth" = "Pin Depth:";
        forward: "pinWarpBar.forward" = "Bring the selected pin forward";
        backward: "pinWarpBar.backward" = "Send the selected pin backward";
        rotate: "pinWarpBar.rotate" = "Rotate:";
        rotation: "pinWarpBar.rotation" = "Rotation";
        auto: "pinWarpBar.auto" = "Auto";
        fixed: "pinWarpBar.fixed" = "Fixed";
        remove_all: "pinWarpBar.removeAll" = "Remove All Pins";
        commit: "pinWarpBar.commit" = "Commit Puppet Warp";
        cancel: "pinWarpBar.cancel" = "Cancel Puppet Warp";
    }
}

/// Puppet Warp's options bar, in Tab order: Mode select (Rigid, Normal, Distort), Density select
/// (Fewer Points, Normal, More Points), Expansion field (−50 to 100 px), the Show Mesh check; with
/// a selected pin, Pin Depth + and − buttons, the Rotate select (Auto, Fixed) and, when Fixed, its
/// degrees field (−360 to 360); Remove All Pins; then cancel and commit at the reading end
/// (`OptionsBar::commit_cancel`). With several pins selected the depth and rotation act on the
/// first. Every control is an ordinary focusable control; the bar never changes the session.
pub struct PinWarpBar<'a> {
    id_salt: IdSalt,
    session: &'a PinWarpSession,
    labels: PinWarpBarLabels,
    notice: Option<&'a str>,
}

/// What the bar reported this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PinWarpBarOutput {
    /// The edits in order; the app applies each with [`PinWarpSession::apply`].
    pub edits: Vec<PinEdit>,
    /// Commit pressed.
    pub apply: bool,
    /// Cancel pressed.
    pub discard: bool,
}

impl<'a> PinWarpBar<'a> {
    /// The bar of `session`.
    pub fn new(id_salt: impl AsIdSalt, session: &'a PinWarpSession) -> Self {
        Self { id_salt: IdSalt::new(id_salt), session, labels: PinWarpBarLabels::default(), notice: None }
    }

    pub fn labels(mut self, labels: PinWarpBarLabels) -> Self {
        self.labels = labels;
        self
    }

    /// A refused remesh's text from [`PinWarpSession::apply`], shown after the options until the
    /// app clears it.
    pub fn notice(mut self, text: Option<&'a str>) -> Self {
        self.notice = text;
        self
    }

    /// Draws the bar into the options bar's `ui`.
    pub fn show(self, ui: &mut Ui) -> PinWarpBarOutput {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:395 (`options_bar`).
        todo!()
    }
}
