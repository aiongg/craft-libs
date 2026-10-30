//! The options bar: the current tool's cells.

use craft_theme::ControlSize;
use craft_ui::controls::{Button, CheckState, Checkbox, IconButton, NumberField, Select};
use craft_ui::icons::Icon;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::{CellWidth, OptionsBar};
use egui::Ui;

use super::model::{Blend, MarqueeStyle, Mock, MoveTarget, SampleSize, Tool};
use super::tools::icon;

fn percent(ui: &mut Ui, numbers: &LocaleNumbers, id: &str, caption: &str, value: &mut f64) {
    OptionsBar::cell(ui, &format!("{caption}:"), CellWidth::Md, |ui, label| {
        let edit =
            NumberField::new(id, caption, *value, numbers).range(0.0..=100.0).unit(Unit::new("{n}%")).size(ControlSize::Sm).scrub_by(label).show(ui);
        if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
            *value = *v;
        }
    });
}

fn choose<T: Copy + PartialEq + Send + Sync + 'static>(ui: &mut Ui, id: &str, caption: &str, value: &mut T, all: &[T], label: fn(T) -> &'static str) {
    OptionsBar::cell(ui, &format!("{caption}:"), CellWidth::Content, |ui, _| {
        let mut select = Select::new(id, caption, value).size(ControlSize::Sm);
        for option in all {
            select = select.option(*option, label(*option));
        }
        let _ = select.show(ui);
    });
}

fn blend(ui: &mut Ui, id: &str, value: &mut Blend) {
    choose(ui, id, "Mode", value, &Blend::ALL, Blend::label);
}

fn check(ui: &mut Ui, label: &str, value: &mut bool) {
    let mut state = CheckState::from(*value);
    let _ = Checkbox::new(label, &mut state).size(ControlSize::Sm).show(ui);
    *value = state == CheckState::Checked;
}

const ALIGN: [(Icon, &str); 6] = [
    (Icon::AlignEdgeLeft, "Align left edges"),
    (Icon::AlignEdgeHcenter, "Align horizontal centres"),
    (Icon::AlignEdgeRight, "Align right edges"),
    (Icon::AlignEdgeTop, "Align top edges"),
    (Icon::AlignEdgeVcenter, "Align vertical centres"),
    (Icon::AlignEdgeBottom, "Align bottom edges"),
];

/// The cells of the current tool; its values live in `mock.tools`.
fn cells(ui: &mut Ui, mock: &mut Mock) {
    let numbers = LocaleNumbers::from_tag("en-US");
    let tools = &mut mock.tools;
    match tools.current {
        Tool::Move => {
            let options = &mut tools.move_;
            check(ui, "Auto-Select", &mut options.auto_select);
            choose(ui, "move-target", "Select", &mut options.target, &[MoveTarget::Group, MoveTarget::Layer], |t| match t {
                MoveTarget::Group => "Group",
                MoveTarget::Layer => "Layer",
            });
            check(ui, "Show Transform Controls", &mut options.transform);
            OptionsBar::divider(ui);
            for (glyph, name) in ALIGN {
                ui.add(IconButton::new(glyph, name).size(ControlSize::Sm));
            }
        }
        Tool::RectMarquee => {
            let options = &mut tools.marquee;
            choose(ui, "marquee-style", "Style", &mut options.style, &MarqueeStyle::ALL, MarqueeStyle::label);
            OptionsBar::cell(ui, "Feather:", CellWidth::Md, |ui, label| {
                let edit = NumberField::new("marquee-feather", "Feather", options.feather, &numbers)
                    .range(0.0..=1000.0)
                    .unit(Unit::new("{n} px"))
                    .size(ControlSize::Sm)
                    .scrub_by(label)
                    .show(ui);
                if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                    options.feather = *v;
                }
            });
        }
        Tool::Brush => {
            let options = &mut tools.brush;
            blend(ui, "brush-mode", &mut options.mode);
            percent(ui, &numbers, "brush-opacity", "Opacity", &mut options.opacity);
            percent(ui, &numbers, "brush-flow", "Flow", &mut options.flow);
            percent(ui, &numbers, "brush-smoothing", "Smoothing", &mut options.smoothing);
        }
        Tool::Eraser => {
            let options = &mut tools.eraser;
            percent(ui, &numbers, "eraser-opacity", "Opacity", &mut options.opacity);
            percent(ui, &numbers, "eraser-flow", "Flow", &mut options.flow);
        }
        Tool::Gradient => {
            let options = &mut tools.gradient;
            blend(ui, "gradient-mode", &mut options.mode);
            percent(ui, &numbers, "gradient-opacity", "Opacity", &mut options.opacity);
            check(ui, "Reverse", &mut options.reverse);
        }
        Tool::Eyedropper => {
            choose(ui, "eyedropper-sample", "Sample", &mut tools.eyedropper.sample, &SampleSize::ALL, SampleSize::label);
        }
        Tool::Hand | Tool::Zoom => {
            if ui.add(Button::new("Fit Screen").size(ControlSize::Sm)).clicked() {
                mock.view.fit_pending = true;
            }
            if ui.add(Button::new("100%").size(ControlSize::Sm)).clicked() {
                mock.view.set_zoom(1.0);
            }
        }
        _ => {}
    }
}

/// The bar across the available width.
pub fn show(ui: &mut Ui, mock: &mut Mock, home: &mut bool) {
    let current = mock.tools.current;
    OptionsBar::new("options").show(ui, |bar| {
        bar.leading(|ui| {
            OptionsBar::home(ui, home, "Home");
            OptionsBar::tool_icon(ui, icon(current), current.label());
        });
        bar.body(|ui| cells(ui, mock));
    });
}
