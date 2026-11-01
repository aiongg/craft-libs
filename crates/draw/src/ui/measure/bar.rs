//! [`RulerBar`]: the Ruler tool's options bar body: the seven readouts, Use Measurement Scale,
//! Straighten Layer and Clear.

use std::sync::Arc;

use super::model::RulerLine;
use super::style::MeasureStyle;
use craft_theme::TypeRole;
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
        x: "rulerBar.x" = "X:";
        y: "rulerBar.y" = "Y:";
        w: "rulerBar.w" = "W:";
        h: "rulerBar.h" = "H:";
        angle: "rulerBar.angle" = "A:";
        l1: "rulerBar.l1" = "L1:";
        l2: "rulerBar.l2" = "L2:";
        use_scale: "rulerBar.useScale" = "Use Measurement Scale";
        straighten_layer: "rulerBar.straightenLayer" = "Straighten Layer";
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
///   all blank without a line), each a muted caption and a value with two decimals in the numeric
///   type role, `measure.readout_gap` apart; live while the line is dragged (the app passes the
///   line of this pass).
/// - Use Measurement Scale multiplies X, Y, W, H, L1 and L2 by the scale factor, not the angle
///   ([`RulerReading::scaled`](super::RulerReading::scaled)).
/// - A divider, the check box, then Straighten Layer and Clear, both disabled with no line.
/// - **AccessKit:** each readout is a labelled static value ("X", "1.00"); the check box and the
///   buttons as usual.
pub struct RulerBar<'a> {
    line: Option<RulerLine>,
    use_scale: &'a mut bool,
    factor: f64,
    labels: Option<RulerBarLabels>,
    output: Option<&'a mut RulerBarOutput>,
}

impl<'a> RulerBar<'a> {
    /// The bar for `line`, with the Use Measurement Scale check box editing `use_scale`.
    pub fn new(line: Option<RulerLine>, use_scale: &'a mut bool) -> Self {
        Self { line, use_scale, factor: 1.0, labels: None, output: None }
    }

    /// The measurement scale's factor (document units to the scale's units); default 1.
    pub fn scale(mut self, factor: f64) -> Self {
        self.factor = factor;
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
        let Self { line, use_scale, factor, labels, output } = self;
        slots.body(move |ui| {
            let labels = labels.map(Arc::new).unwrap_or_else(|| RulerBarLabels::shared(ui));
            let gap = MeasureStyle::of(&Tokens::of(ui)).readout_gap;
            let numbers = LocaleNumbers::of(ui);
            let reading = line.map(|line| {
                let reading = line.reading();
                if *use_scale { reading.scaled(factor) } else { reading }
            });
            let value = |v: Option<f64>| v.map_or_else(String::new, |v| numbers.format(v, 2..=2));
            let readouts = [
                (&labels.x, reading.map(|r| r.x)),
                (&labels.y, reading.map(|r| r.y)),
                (&labels.w, reading.map(|r| r.w)),
                (&labels.h, reading.map(|r| r.h)),
                (&labels.angle, reading.map(|r| r.angle)),
                (&labels.l1, reading.map(|r| r.l1)),
                (&labels.l2, reading.and_then(|r| r.l2)),
            ];
            for (caption, v) in readouts {
                readout(ui, caption, &value(v), gap);
            }
            OptionsBar::divider(ui);
            let _ = Checkbox::bool(labels.use_scale.clone(), use_scale).show(ui);
            OptionsBar::divider(ui);
            let enabled = line.is_some();
            let straighten = ui.add(Button::new(labels.straighten_layer.to_string()).enabled(enabled)).clicked();
            let clear = ui.add(Button::new(labels.clear.to_string()).enabled(enabled)).clicked();
            if let Some(output) = output {
                output.straighten_layer |= straighten;
                output.clear |= clear;
            }
        });
    }
}

/// One caption and value, a labelled group for AccessKit ("X", "1.00"), then the gap.
fn readout(ui: &mut Ui, caption: &str, value: &str, gap: f32) {
    let name = caption.trim_end_matches([':', '\u{ff1a}']).trim_end();
    let builder = UiBuilder::new().accessibility_role(Role::Group).accessibility_label(name).layout(*ui.layout());
    ui.scope_builder(builder, |ui| {
        ui.add(Text::new(caption).role(TypeRole::Caption).content(Content::Muted));
        ui.add(Text::new(value).role(TypeRole::Mono));
    });
    ui.add_space(gap);
}
