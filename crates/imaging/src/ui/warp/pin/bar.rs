//! [`PinWarpBar`]: Puppet Warp's options bar.

use egui::{AsIdSalt, IdSalt, Modifiers, RichText};

use craft_ui::controls::{Button, Checkbox, IconButton, NumberField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::patterns::{CellWidth, OptionsBar, OptionsBarSlots};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::words;

use super::{Pin, PinDensity, PinEdit, PinMode, PinWarpOptions, PinWarpSession};

words! {
    /// The words of [`PinWarpBar`] (PhotoCraft's Puppet Warp bar). `Default` is English.
    pub struct PinWarpBarLabels {
        mode: "pinWarpBar.mode" = "Mode:";
        mode_name: "pinWarpBar.modeName" = "Mode";
        rigid: "pinWarpBar.rigid" = "Rigid";
        normal: "pinWarpBar.normal" = "Normal";
        distort: "pinWarpBar.distort" = "Distort";
        density: "pinWarpBar.density" = "Density:";
        density_name: "pinWarpBar.densityName" = "Density";
        fewer_points: "pinWarpBar.fewerPoints" = "Fewer Points";
        more_points: "pinWarpBar.morePoints" = "More Points";
        expansion: "pinWarpBar.expansion" = "Expansion:";
        expansion_name: "pinWarpBar.expansionName" = "Expansion";
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
///
/// ```no_run
/// use craft_imaging::ui::warp::pin::{PinWarpBar, PinWarpBarOutput, PinWarpSession};
/// use craft_ui::patterns::OptionsBar;
/// # fn demo(ui: &mut egui::Ui, session: &PinWarpSession) {
/// let mut out = PinWarpBarOutput::default();
/// OptionsBar::new("options").show(ui, |bar| PinWarpBar::new("pins", session).output(&mut out).add_to(bar));
/// # }
/// ```
pub struct PinWarpBar<'a> {
    id_salt: IdSalt,
    session: &'a PinWarpSession,
    labels: Option<PinWarpBarLabels>,
    notice: Option<&'a str>,
    output: Option<&'a mut PinWarpBarOutput>,
}

/// What the bar reported; each pass adds to it.
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
        Self { id_salt: IdSalt::new(id_salt), session, labels: None, notice: None, output: None }
    }

    /// The words; default: [`PinWarpBarLabels`] of the installed words.
    pub fn labels(mut self, labels: PinWarpBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// A refused remesh's text from [`PinWarpSession::apply`], shown after the options until the
    /// app clears it.
    pub fn notice(mut self, text: Option<&'a str>) -> Self {
        self.notice = text;
        self
    }

    /// Where the bar reports its edits and its commit and cancel buttons.
    pub fn output(mut self, output: &'a mut PinWarpBarOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Fills the bar's body slot with the options and its trailing slot with cancel and commit.
    pub fn add_to(self, slots: &mut OptionsBarSlots<'a>) {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:395 (`options_bar`). The two slots get disjoint
        // halves of the output.
        let Self { id_salt, session, labels, notice, output } = self;
        let (edits, ends) = match output {
            Some(PinWarpBarOutput { edits, apply, discard }) => (Some(edits), Some((apply, discard))),
            None => (None, None),
        };
        let trailing_labels = labels.clone();
        slots.body(move |ui| {
            let labels = labels.unwrap_or_else(|| PinWarpBarLabels::of(ui));
            let mut scratch = Vec::new();
            let edits = edits.unwrap_or(&mut scratch);
            ui.push_id(id_salt, |ui| body(ui, session, &labels, notice, edits));
        });
        slots.trailing(move |ui| {
            let labels = trailing_labels.unwrap_or_else(|| PinWarpBarLabels::of(ui));
            let pressed = OptionsBar::commit_cancel(ui, &labels.commit, &labels.cancel);
            if let Some((apply, discard)) = ends {
                *apply |= pressed.commit;
                *discard |= pressed.cancel;
            }
        });
    }
}

/// The options: Mode, Density, Expansion, Show Mesh, the notice and the selected pin's controls,
/// then Remove All Pins.
fn body(ui: &mut egui::Ui, session: &PinWarpSession, labels: &PinWarpBarLabels, notice: Option<&str>, edits: &mut Vec<PinEdit>) {
    let sm = OptionsBar::CONTROL_SIZE;
    let numbers = LocaleNumbers::of(ui);
    let options = *session.options();
    let pins = session.pins();
    let picked = session.selected().first().copied().filter(|&i| i < pins.len());
    OptionsBar::cell(ui, &labels.mode, CellWidth::Content, |ui, _| {
        let mut mode = options.mode;
        let pick = Select::new("mode", labels.mode_name.as_str(), &mut mode)
            .option(PinMode::Rigid, labels.rigid.as_str())
            .option(PinMode::Normal, labels.normal.as_str())
            .option(PinMode::Distort, labels.distort.as_str())
            .size(sm)
            .fit(true)
            .show(ui);
        if let Some(&mode) = pick.event.as_ref().and_then(EditEvent::value).filter(|&&m| m != options.mode) {
            edits.push(PinEdit::Options(PinWarpOptions { mode, ..options }));
        }
    });
    OptionsBar::cell(ui, &labels.density, CellWidth::Content, |ui, _| {
        let mut density = options.density;
        let pick = Select::new("density", labels.density_name.as_str(), &mut density)
            .option(PinDensity::Fewer, labels.fewer_points.as_str())
            .option(PinDensity::Normal, labels.normal.as_str())
            .option(PinDensity::More, labels.more_points.as_str())
            .size(sm)
            .fit(true)
            .show(ui);
        if let Some(&density) = pick.event.as_ref().and_then(EditEvent::value).filter(|&&d| d != options.density) {
            edits.push(PinEdit::Options(PinWarpOptions { density, ..options }));
        }
    });
    let field = NumberField::new("expansion", labels.expansion_name.as_str(), options.expansion, &numbers)
        .range(-50.0..=100.0)
        .decimals(0..=0)
        .unit(Unit::new("{n} px"));
    // A remesh per update would rebuild the mesh while the value is still being typed or
    // scrubbed: only the value that ends the edit goes out.
    if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) =
        OptionsBar::number_cell(ui, &labels.expansion, CellWidth::Sm, field).event
        && value != options.expansion
    {
        edits.push(PinEdit::Options(PinWarpOptions { expansion: value, ..options }));
    }
    let mut show = session.show_mesh();
    if Checkbox::bool(labels.show_mesh.as_str(), &mut show).size(sm).show(ui).response.changed() {
        edits.push(PinEdit::ShowMesh(show));
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
                    edits.push(PinEdit::Pins(set(with_pin(pins, i, Pin { depth: pin.depth + by, ..pin }))));
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
                edits.push(PinEdit::Pins(set(with_pin(pins, i, Pin { angle, ..pin }))));
            }
        });
        if let Some(angle) = pin.angle {
            let field =
                NumberField::new("rotation", labels.rotation.as_str(), angle, &numbers).range(-360.0..=360.0).decimals(0..=1).unit(Unit::new("{n}°"));
            if let Some(event) = OptionsBar::number_cell(ui, &labels.rotation, CellWidth::Sm, field).event {
                edits.push(PinEdit::Pins(event.map(|angle| with_pin(pins, i, Pin { angle: Some(angle), ..pin }))));
            }
        }
        OptionsBar::divider(ui);
    }
    if ui.add(Button::new(labels.remove_all.as_str()).size(sm)).clicked() {
        edits.push(PinEdit::Pins(set(Vec::new())));
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
