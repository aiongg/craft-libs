//! The options bar: the current tool's cells.

use craft_theme::ControlSize;
use craft_ui::controls::{Button, CheckState, Checkbox, IconButton, NumberField, Select};
use craft_ui::icons::Icon;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::{CellWidth, OptionsBar};
use egui::Ui;

use super::model::{Blend, MarqueeStyle, Mock, MoveTarget, SampleSize, Tool, View};
use super::tools::icon;

fn percent(ui: &mut Ui, numbers: &LocaleNumbers, id: &str, caption: &str, min: f64, value: &mut f64) {
    OptionsBar::cell(ui, &format!("{caption}:"), CellWidth::Md, |ui, label| {
        let edit =
            NumberField::new(id, caption, *value, numbers).range(min..=100.0).unit(Unit::new("{n}%")).size(ControlSize::Sm).scrub_by(label).show(ui);
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
    let numbers = LocaleNumbers::of(ui);
    let tools = &mut mock.tools;
    match tools.current {
        Tool::Move => {
            let options = &mut tools.move_;
            check(ui, "Auto-Select", &mut options.auto_select);
            let mut select = Select::new("move-target", "Auto-Select target", &mut options.target).size(ControlSize::Sm);
            for target in [MoveTarget::Layer, MoveTarget::Group] {
                select = select.option(target, if target == MoveTarget::Layer { "Layer" } else { "Group" });
            }
            let _ = select.show(ui);
            OptionsBar::divider(ui);
            check(ui, "Show Transform Controls", &mut options.transform);
            OptionsBar::divider(ui);
            for (glyph, name) in ALIGN {
                ui.add(IconButton::new(glyph, name).size(ControlSize::Sm));
            }
        }
        Tool::RectMarquee => {
            let options = &mut tools.marquee;
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
            choose(ui, "marquee-style", "Style", &mut options.style, &MarqueeStyle::ALL, MarqueeStyle::label);
        }
        Tool::Brush => {
            let options = &mut tools.brush;
            blend(ui, "brush-mode", &mut options.mode);
            percent(ui, &numbers, "brush-opacity", "Opacity", 1.0, &mut options.opacity);
            percent(ui, &numbers, "brush-flow", "Flow", 1.0, &mut options.flow);
            percent(ui, &numbers, "brush-smoothing", "Smoothing", 0.0, &mut options.smoothing);
        }
        Tool::Eraser => {
            let options = &mut tools.eraser;
            percent(ui, &numbers, "eraser-opacity", "Opacity", 1.0, &mut options.opacity);
            percent(ui, &numbers, "eraser-flow", "Flow", 1.0, &mut options.flow);
        }
        Tool::Gradient => {
            let options = &mut tools.gradient;
            blend(ui, "gradient-mode", &mut options.mode);
            percent(ui, &numbers, "gradient-opacity", "Opacity", 1.0, &mut options.opacity);
            check(ui, "Reverse", &mut options.reverse);
        }
        Tool::Eyedropper => {
            choose(ui, "eyedropper-sample", "Sample", &mut tools.eyedropper.sample, &SampleSize::ALL, SampleSize::label);
        }
        Tool::Hand | Tool::Zoom => {
            let fit = |ui: &mut Ui, view: &mut View| {
                if ui.add(Button::new("Fit Screen").size(ControlSize::Sm)).clicked() {
                    view.fit_pending = true;
                }
            };
            let actual = |ui: &mut Ui, view: &mut View| {
                if ui.add(Button::new("100%").size(ControlSize::Sm)).clicked() {
                    view.set_zoom(1.0);
                }
            };
            if tools.current == Tool::Hand {
                actual(ui, &mut mock.view);
                fit(ui, &mut mock.view);
            } else {
                fit(ui, &mut mock.view);
                actual(ui, &mut mock.view);
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
