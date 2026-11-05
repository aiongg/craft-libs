//! [`RulerBar`]: the Ruler tool's options bar body: the seven readouts, Use Measurement Scale,
//! Straighten Layer and Clear.

use std::sync::Arc;

use super::model::RulerLine;
use super::style::MeasureStyle;
use craft_theme::TypeRole;
use craft_ui::canvas::RulerUnits;
use craft_ui::controls::{Button, Checkbox};
use craft_ui::localized::{LocaleNumbers, SharedLabels as _};
use craft_ui::patterns::{OptionsBar, OptionsBarSlots};
use craft_ui::primitives::{Content, Text};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{Ui, UiBuilder};

craft_ui::words! {
    /// The Ruler options bar's words, English by default.
    #[non_exhaustive]
    pub struct RulerBarLabels {
        #[allow(missing_docs)]
        x: "rulerBar.x" = "X:";
        #[allow(missing_docs)]
        y: "rulerBar.y" = "Y:";
        #[allow(missing_docs)]
        w: "rulerBar.w" = "W:";
        #[allow(missing_docs)]
        h: "rulerBar.h" = "H:";
        #[allow(missing_docs)]
        angle: "rulerBar.angle" = "A:";
        #[allow(missing_docs)]
        l1: "rulerBar.l1" = "L1:";
        #[allow(missing_docs)]
        l2: "rulerBar.l2" = "L2:";
        /// The readouts' AccessKit names, without the caption's colon.
        x_name: "rulerBar.xName" = "X";
        #[allow(missing_docs)]
        y_name: "rulerBar.yName" = "Y";
        #[allow(missing_docs)]
        w_name: "rulerBar.wName" = "W";
        #[allow(missing_docs)]
        h_name: "rulerBar.hName" = "H";
        #[allow(missing_docs)]
        angle_name: "rulerBar.angleName" = "A";
        #[allow(missing_docs)]
        l1_name: "rulerBar.l1Name" = "L1";
        #[allow(missing_docs)]
        l2_name: "rulerBar.l2Name" = "L2";
        #[allow(missing_docs)]
        use_scale: "rulerBar.useScale" = "Use Measurement Scale";
        #[allow(missing_docs)]
        straighten_layer: "rulerBar.straightenLayer" = "Straighten Layer";
        #[allow(missing_docs)]
        clear: "rulerBar.clear" = "Clear";
    }
}

/// What the Ruler bar reported this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RulerBarOutput {
    /// Straighten Layer was clicked: the app runs its command (the engine's).
    pub straighten_layer: bool,
    /// Clear was clicked: the app clears the line.
    pub clear: bool,
}

/// The Ruler tool's options bar body.
///
/// ```no_run
/// use craft_draw::ui::measure::{RulerBar, RulerBarOutput, RulerLine};
/// use craft_ui::patterns::OptionsBar;
/// # fn demo(ui: &mut egui::Ui, line: Option<RulerLine>, use_scale: &mut bool, factor: f64) {
/// let mut out = RulerBarOutput::default();
/// OptionsBar::new("options").show(ui, |bar| RulerBar::new(line, use_scale).scale(factor).output(&mut out).add_to(bar));
/// # }
/// ```
///
/// - Readouts X, Y (the start), W, H (end minus start), A, L1 and L2 (blank without a protractor;
///   all blank without a line), each a muted caption and a value in the numeric type role,
///   `measure.readout_gap` apart; live while the line is dragged (the app passes the line of this
///   pass). Values have two decimals; with [`units`](Self::units) and no measurement scale the
///   lengths and positions are written in the ruler units (X and W in the first, Y and H in the
///   second, L1 and L2 in the first), the angle stays a plain number.
/// - Use Measurement Scale multiplies X, Y, W, H, L1 and L2 by the scale factor, not the angle
///   ([`RulerReading::scaled`](super::RulerReading::scaled)).
/// - One divider after the readouts, then the check box, Straighten Layer and Clear (both disabled
///   with no line), at [`OptionsBar::CONTROL_SIZE`].
/// - **AccessKit:** each readout is a labelled static value ("X", "1.00"), named by the
///   `*_name` words; the check box and the buttons as usual.
pub struct RulerBar<'a> {
    line: Option<RulerLine>,
    use_scale: &'a mut bool,
    factor: f64,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    labels: Option<RulerBarLabels>,
    output: Option<&'a mut RulerBarOutput>,
}

impl<'a> RulerBar<'a> {
    /// The bar for `line`, with the Use Measurement Scale check box editing `use_scale`.
    pub fn new(line: Option<RulerLine>, use_scale: &'a mut bool) -> Self {
        Self { line, use_scale, factor: 1.0, units: None, labels: None, output: None }
    }

    /// The measurement scale's factor (document units to the scale's units); default 1.
    pub fn scale(mut self, factor: f64) -> Self {
        self.factor = factor;
        self
    }

    /// The ruler units of the readouts' values ("x, y"), as [`RulerInput::units`](super::RulerInput::units);
    /// default: plain numbers. They apply while Use Measurement Scale is off: a scaled value is in
    /// the scale's own unit, which the bar does not know.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The words; default: [`RulerBarLabels`] of the installed words.
    pub fn labels(mut self, labels: RulerBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Where the bar reports its buttons.
    pub fn output(mut self, output: &'a mut RulerBarOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Fills the bar's body slot.
    pub fn add_to(self, slots: &mut OptionsBarSlots<'a>) {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:497 `options_bar` (the Ruler branch), :489
        // `readout` (11 pt dim caption, monospace value, `{v:.2}`, 6 pt gap).
        let Self { line, use_scale, factor, units, labels, output } = self;
        slots.body(move |ui| {
            let labels = labels.map(Arc::new).unwrap_or_else(|| RulerBarLabels::shared(ui));
            let gap = MeasureStyle::of(&Tokens::of(ui)).readout_gap;
            let numbers = LocaleNumbers::of(ui);
            let reading = line.map(|line| {
                let reading = line.reading();
                if *use_scale { reading.scaled(factor) } else { reading }
            });
            let scaled = *use_scale;
            let (ux, uy) = if scaled { (None, None) } else { (units.map(|u| u.0), units.map(|u| u.1)) };
            let length = |unit: Option<&RulerUnits>, v: Option<f64>| {
                v.map_or_else(String::new, |v| unit.map_or_else(|| numbers.format(v, 2..=2), |unit| unit.format_value(v, 0.0)))
            };
            let angle = reading.map_or_else(String::new, |r| numbers.format(r.angle, 2..=2));
            let readouts = [
                (&labels.x, &labels.x_name, length(ux, reading.map(|r| r.x))),
                (&labels.y, &labels.y_name, length(uy, reading.map(|r| r.y))),
                (&labels.w, &labels.w_name, length(ux, reading.map(|r| r.w))),
                (&labels.h, &labels.h_name, length(uy, reading.map(|r| r.h))),
                (&labels.angle, &labels.angle_name, angle),
                (&labels.l1, &labels.l1_name, length(ux, reading.map(|r| r.l1))),
                (&labels.l2, &labels.l2_name, length(ux, reading.and_then(|r| r.l2))),
            ];
            for (caption, name, value) in &readouts {
                readout(ui, caption, name, value, gap);
            }
            OptionsBar::divider(ui);
            let size = OptionsBar::CONTROL_SIZE;
            let _ = Checkbox::bool(labels.use_scale.clone(), use_scale).size(size).show(ui);
            let enabled = line.is_some();
            let straighten = ui.add(Button::new(labels.straighten_layer.to_string()).size(size).enabled(enabled)).clicked();
            let clear = ui.add(Button::new(labels.clear.to_string()).size(size).enabled(enabled)).clicked();
            if let Some(output) = output {
                output.straighten_layer |= straighten;
                output.clear |= clear;
            }
        });
    }
}

/// One caption and value, a group named `name` for AccessKit ("X", "1.00"), then the gap.
fn readout(ui: &mut Ui, caption: &str, name: &str, value: &str, gap: f32) {
    let builder = UiBuilder::new().accessibility_role(Role::Group).accessibility_label(name).layout(*ui.layout());
    ui.scope_builder(builder, |ui| {
        ui.add(Text::new(caption).role(TypeRole::Caption).content(Content::Muted));
        ui.add(Text::new(value).role(TypeRole::Mono));
    });
    ui.add_space(gap);
}
