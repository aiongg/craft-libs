//! [`OptionsBar::cell`]: a caption and a compact control, with a width class.

use egui::text::LayoutJob;
use egui::{Label, Response, Ui, vec2};

use super::options_bar::OptionsBar;
use crate::controls::{CheckState, Checkbox, NumberField, Select};
use crate::direction;
use crate::localized::{LocaleNumbers, Unit};
use crate::primitives::{Content, Text};
use crate::story::StoryArgs;
use crate::theme::{Tokens, TokensOf};
use craft_theme::{ControlSize, TypeRole};

/// How wide a cell's control is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellWidth {
    /// `options_bar.field_sm`: a short number.
    Sm,
    /// `options_bar.field_md`: a number with a unit.
    Md,
    /// `options_bar.field_lg`: a long number.
    Lg,
    /// The control's own width, for one that sizes itself (a select, a checkbox).
    Content,
}

impl OptionsBar {
    /// A caption and the control it names. `caption` is the host's translated text including its
    /// colon ("Size:"). The control is laid out in a box of the class's width, `options_bar.gap`
    /// after the caption, and `add` receives the caption's response so a number field can
    /// [`scrub_by`](NumberField::scrub_by) it. The caption is not selectable and is not shortened
    /// for a narrow bar: the bar scrolls instead.
    pub fn cell<R>(ui: &mut Ui, caption: &str, width: CellWidth, add: impl FnOnce(&mut Ui, &Response) -> R) -> R {
        let tokens = Tokens::of(ui);
        let style = OptionsBar::style(&tokens, false);
        ui.spacing_mut().item_spacing.x = style.gap;
        let mut format = Text::format(&tokens, TypeRole::Body, Content::Muted);
        format.valign = ui.text_valign();
        let label = ui.add(Label::new(LayoutJob::single_section(caption.to_owned(), format)).selectable(false));
        match width {
            CellWidth::Content => add(ui, &label),
            fixed => {
                let side = match fixed {
                    CellWidth::Sm => style.fields[0],
                    CellWidth::Md => style.fields[1],
                    _ => style.fields[2],
                };
                let size = vec2(side, ui.available_height());
                ui.allocate_ui_with_layout(size, direction::row(ui).with_main_justify(true), |ui| {
                    ui.set_width(side);
                    add(ui, &label)
                })
                .inner
            }
        }
    }

    /// Hint text in the body: `text.faint`.
    pub fn hint(ui: &mut Ui, text: &str) -> Response {
        ui.add(Text::new(text).content(Content::Faint))
    }
}

#[derive(Clone, Debug)]
struct Demo {
    size: f64,
    opacity: f64,
    mode: String,
    sample_all: CheckState,
    home: bool,
    last: String,
}

impl Default for Demo {
    fn default() -> Self {
        Self { size: 30.0, opacity: 100.0, mode: "Normal".into(), sample_all: CheckState::Unchecked, home: false, last: String::new() }
    }
}

fn brush_cells(ui: &mut Ui, numbers: &LocaleNumbers, size: &mut f64, opacity: &mut f64, mode: &mut String) {
    OptionsBar::cell(ui, "Size:", super::CellWidth::Sm, |ui, caption| {
        let edit = NumberField::new("demo-size", "Size", *size, numbers)
            .range(1.0..=5000.0)
            .unit(Unit::new("{n} px"))
            .size(ControlSize::Sm)
            .scrub_by(caption)
            .show(ui);
        if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
            *size = *v;
        }
    });
    OptionsBar::cell(ui, "Opacity:", super::CellWidth::Md, |ui, caption| {
        let edit = NumberField::new("demo-opacity", "Opacity", *opacity, numbers)
            .range(1.0..=100.0)
            .unit(Unit::new("{n}%"))
            .size(ControlSize::Sm)
            .scrub_by(caption)
            .show(ui);
        if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
            *opacity = *v;
        }
    });
    OptionsBar::cell(ui, "Mode:", super::CellWidth::Content, |ui, _| {
        let _ = Select::new("demo-mode", "Mode", mode)
            .option("Normal".to_owned(), "Normal")
            .option("Multiply".to_owned(), "Multiply")
            .option("Screen".to_owned(), "Screen")
            .size(ControlSize::Sm)
            .show(ui);
    });
}

fn sample_group(ui: &mut Ui, state: &mut CheckState, disabled: bool) {
    OptionsBar::group(ui, !disabled, |ui| {
        let _ = Checkbox::new("Sample all layers", state).size(ControlSize::Sm).show(ui);
        OptionsBar::hint(ui, "Samples the visible image");
    });
}

/// The args of the options bar story.
#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct BarArgs {
    pub(super) narrow: bool,
    pub(super) session: bool,
    pub(super) disabled: bool,
    pub(super) two_rows: bool,
    #[arg(range = 160.0..=1200.0)]
    pub(super) width: f32,
}

/// The bar of PhotoCraft's brush tool, with a disabled group and, while a session is on, commit and cancel.
pub(super) fn show_story(ui: &mut Ui, args: &mut BarArgs) {
    let (narrow, session, disabled, two_rows) = (args.narrow, args.session, args.disabled, args.two_rows);
    let width = args.width;
    let state_id = ui.scope_id().with("options-bar-demo");
    let mut demo = ui.data(|d| d.get_temp::<Demo>(state_id)).unwrap_or_default();
    let numbers = LocaleNumbers::from_tag("en-US");
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let mut finished = None;
        OptionsBar::new("options-bar-demo").narrow(narrow).show(ui, |bar| {
            bar.leading(|ui| {
                OptionsBar::home(ui, &mut demo.home, "Home");
                OptionsBar::tool_icon(ui, crate::icons::Icon::Brush, "Brush");
            });
            if two_rows {
                bar.row(|ui| brush_cells(ui, &numbers, &mut demo.size, &mut demo.opacity, &mut demo.mode));
                bar.row(|ui| sample_group(ui, &mut demo.sample_all, disabled));
            } else {
                bar.row(|ui| {
                    brush_cells(ui, &numbers, &mut demo.size, &mut demo.opacity, &mut demo.mode);
                    OptionsBar::divider(ui);
                    sample_group(ui, &mut demo.sample_all, disabled);
                });
            }
            if session {
                bar.trailing(|ui| finished = Some(OptionsBar::commit_cancel(ui, "Commit transform", "Cancel transform")));
            }
        });
        if let Some(done) = finished {
            if done.commit {
                demo.last = "Committed".into();
            } else if done.cancel {
                demo.last = "Cancelled".into();
            }
        }
        if !demo.last.is_empty() {
            ui.add(Text::new(demo.last.clone()).content(Content::Muted));
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, demo));
}
