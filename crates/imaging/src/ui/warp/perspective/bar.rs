//! [`PerspectiveBar`]: Perspective Warp's options bar (task TR-T13).

use egui::{AsIdSalt, IdSalt, Modifiers, Ui};

use craft_theme::ControlSize;
use craft_ui::controls::{Button, Segmented, TooltipExt as _};
use craft_ui::edit::EditEvent;
use craft_ui::localized::Words;
use craft_ui::localized::words::Counted;
use craft_ui::patterns::OptionsBar;
use craft_ui::words;

use super::{PerspectiveEdit, PerspectiveSession, PlaneMode, Straighten};

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
        let Self { id_salt, session, labels } = self;
        let sm = ControlSize::Sm;
        let planes = session.planes().len();
        let mut out = PerspectiveBarOutput::default();
        ui.push_id(id_salt, |ui| {
            let mut mode = session.mode();
            let tabs = Segmented::new("mode", labels.mode.as_str(), &mut mode)
                .item(PlaneMode::Layout, labels.layout.as_str())
                .item(PlaneMode::Warp, labels.warp.as_str())
                .item_enabled(planes > 0)
                .size(sm)
                .show(ui);
            if tabs.changed() {
                out.edits.push(PerspectiveEdit::Mode(mode));
            }
            OptionsBar::divider(ui);
            match session.mode() {
                PlaneMode::Warp => {
                    for (label, tip, how) in [
                        (&labels.straighten, &labels.straighten_tip, Straighten::Vertical),
                        (&labels.level, &labels.level_tip, Straighten::Horizontal),
                        (&labels.both, &labels.both_tip, Straighten::Both),
                    ] {
                        if ui.add(Button::new(label.as_str()).size(sm)).tooltip(tip.as_str()).clicked() {
                            out.edits.push(PerspectiveEdit::Straighten(how));
                        }
                    }
                }
                PlaneMode::Layout => {
                    let words = Words::of(ui);
                    let text = words.get(PLANES.word);
                    let count = words.counted(ui.ctx(), PLANES, &text, planes as u64);
                    OptionsBar::hint(ui, &count);
                    if ui.add_enabled(planes > 0, Button::new(labels.remove_planes.as_str()).size(sm)).clicked() {
                        out.edits.push(PerspectiveEdit::Planes(EditEvent::set(Vec::new(), Modifiers::NONE)));
                    }
                }
            }
            OptionsBar::divider(ui);
            let pressed = OptionsBar::commit_cancel(ui, &labels.commit, &labels.cancel);
            out.discard = pressed.cancel;
            out.apply = pressed.commit;
        });
        out
    }
}
