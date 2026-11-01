//! [`PinWarpBar`]: Puppet Warp's options bar (task TR-T12).

use egui::{AsIdSalt, IdSalt, Modifiers, RichText, Ui};

use craft_theme::ControlSize;
use craft_ui::controls::{Button, Checkbox, IconButton, NumberField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::{CellWidth, OptionsBar};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::words;

use super::{Pin, PinDensity, PinEdit, PinMode, PinWarpOptions, PinWarpSession};

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
        let Self { id_salt, session, labels, notice } = self;
        let sm = ControlSize::Sm;
        let numbers = LocaleNumbers::of(ui);
        let options = *session.options();
        let pins = session.pins();
        let picked = session.selected().first().copied().filter(|&i| i < pins.len());
        let mut out = PinWarpBarOutput::default();
        ui.push_id(id_salt, |ui| {
            OptionsBar::cell(ui, &labels.mode, CellWidth::Content, |ui, _| {
                let mut mode = options.mode;
                let pick = Select::new("mode", labels.mode.trim_end_matches(':'), &mut mode)
                    .option(PinMode::Rigid, labels.rigid.as_str())
                    .option(PinMode::Normal, labels.normal.as_str())
                    .option(PinMode::Distort, labels.distort.as_str())
                    .size(sm)
                    .show(ui);
                if let Some(&mode) = pick.event.as_ref().and_then(EditEvent::value).filter(|&&m| m != options.mode) {
                    out.edits.push(PinEdit::Options(PinWarpOptions { mode, ..options }));
                }
            });
            OptionsBar::cell(ui, &labels.density, CellWidth::Content, |ui, _| {
                let mut density = options.density;
                let pick = Select::new("density", labels.density.trim_end_matches(':'), &mut density)
                    .option(PinDensity::Fewer, labels.fewer_points.as_str())
                    .option(PinDensity::Normal, labels.normal.as_str())
                    .option(PinDensity::More, labels.more_points.as_str())
                    .size(sm)
                    .show(ui);
                if let Some(&density) = pick.event.as_ref().and_then(EditEvent::value).filter(|&&d| d != options.density) {
                    out.edits.push(PinEdit::Options(PinWarpOptions { density, ..options }));
                }
            });
            let field = NumberField::new("expansion", labels.expansion.trim_end_matches(':'), options.expansion, &numbers)
                .range(-50.0..=100.0)
                .decimals(0..=0)
                .unit(Unit::new("{n} px"));
            // A remesh per update would rebuild the mesh while the value is still being typed or
            // scrubbed: only the value that ends the edit goes out.
            if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) =
                OptionsBar::number_cell(ui, &labels.expansion, CellWidth::Sm, field).event
                && value != options.expansion
            {
                out.edits.push(PinEdit::Options(PinWarpOptions { expansion: value, ..options }));
            }
            let mut show = session.show_mesh();
            if Checkbox::bool(labels.show_mesh.as_str(), &mut show).size(sm).show(ui).response.changed() {
                out.edits.push(PinEdit::ShowMesh(show));
            }
            if let Some(text) = notice {
                let bad = Tokens::of(ui).color.status.bad.fg.to_egui();
                ui.label(RichText::new(text).color(bad));
            }
            OptionsBar::divider(ui);
            if let Some(i) = picked {
                let pin = pins[i];
                OptionsBar::cell(ui, &labels.pin_depth, CellWidth::Content, |ui, _| {
                    for (icon, name, by, salt) in [(Icon::Plus, &labels.forward, 1, "forward"), (Icon::Minus, &labels.backward, -1, "backward")] {
                        if ui.add(IconButton::new(icon, name.as_str()).size(sm).id_salt(salt)).clicked() {
                            out.edits.push(PinEdit::Pins(set(with_pin(pins, i, Pin { depth: pin.depth + by, ..pin }))));
                        }
                    }
                });
                OptionsBar::cell(ui, &labels.rotate, CellWidth::Content, |ui, _| {
                    let mut fixed = pin.angle.is_some();
                    let pick = Select::new("rotate", labels.rotation.as_str(), &mut fixed)
                        .option(false, labels.auto.as_str())
                        .option(true, labels.fixed.as_str())
                        .size(sm)
                        .show(ui);
                    if let Some(&fixed) = pick.event.as_ref().and_then(EditEvent::value).filter(|&&f| f != pin.angle.is_some()) {
                        let angle = fixed.then_some(0.0);
                        out.edits.push(PinEdit::Pins(set(with_pin(pins, i, Pin { angle, ..pin }))));
                    }
                });
                if let Some(angle) = pin.angle {
                    let field = NumberField::new("rotation", labels.rotation.as_str(), angle, &numbers)
                        .range(-360.0..=360.0)
                        .decimals(0..=1)
                        .unit(Unit::new("{n}°"));
                    if let Some(event) = OptionsBar::number_cell(ui, &labels.rotation, CellWidth::Sm, field).event {
                        out.edits.push(PinEdit::Pins(event.map(|angle| with_pin(pins, i, Pin { angle: Some(angle), ..pin }))));
                    }
                }
                OptionsBar::divider(ui);
            }
            if ui.add(Button::new(labels.remove_all.as_str()).size(sm)).clicked() {
                out.edits.push(PinEdit::Pins(set(Vec::new())));
            }
            OptionsBar::divider(ui);
            let pressed = OptionsBar::commit_cancel(ui, &labels.commit, &labels.cancel);
            out.discard = pressed.cancel;
            out.apply = pressed.commit;
        });
        out
    }
}

/// `pins` with pin `i` replaced.
fn with_pin(pins: &[Pin], i: usize, pin: Pin) -> Vec<Pin> {
    let mut all = pins.to_vec();
    all[i] = pin;
    all
}

/// A one-shot change of the pins.
fn set(pins: Vec<Pin>) -> EditEvent<Vec<Pin>> {
    EditEvent::set(pins, Modifiers::NONE)
}
