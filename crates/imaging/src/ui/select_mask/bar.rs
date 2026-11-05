//! [`SelectMaskBar`]: Select and Mask's options bar (task WS-SM2).

use egui::{AsIdSalt, IdSalt, Modifiers, Response, Ui};

use craft_theme::ControlSize;
use craft_ui::controls::{Button, Checkbox, IconButton, NumberField, TooltipExt as _};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::patterns::{CellWidth, OptionsBar};

use super::SelectMaskLabels;
use super::tools::{BrushSizes, SelectMaskTool, SelectMaskToolOptions};

/// What [`SelectMaskBar::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectMaskBarOutput {
    pub response: Response,
    /// The edit lifecycle of the option the user changed this pass, with the options after it
    /// (already written to the options passed in).
    pub edit: Option<EditEvent<SelectMaskToolOptions>>,
    /// Sample All Layers changed: the preview is asked for again.
    pub sample_all_layers: bool,
    /// Select Subject pressed: the workspace records `SelectMaskEdit::Subject`.
    pub select_subject: bool,
}

/// The options bar, full width under the title (`OptionsBar`): the current tool's icon; for the
/// editing tools + and − (Add, Subtract); for the brush tools Size (1..=5000 px, a
/// `NumberField`); for the Brush Hardness (0..=100 %); for Quick Selection, Refine Edge Brush and
/// Object Selection Sample All Layers; for the editing tools Select Subject and Refine Hair, drawn
/// disabled with the tooltip "Refine Hair needs hair detection, which isn't available yet" (lead
/// call 28).
pub struct SelectMaskBar<'a> {
    id_salt: IdSalt,
    tool: SelectMaskTool,
    options: &'a mut SelectMaskToolOptions,
    labels: Option<SelectMaskLabels>,
}

impl<'a> SelectMaskBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, tool: SelectMaskTool, options: &'a mut SelectMaskToolOptions) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, options, labels: None }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: SelectMaskLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> SelectMaskBarOutput {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1497 `options_bar`.
        let Self { id_salt, tool, options, labels } = self;
        let labels = labels.unwrap_or_else(|| SelectMaskLabels::of(ui));
        let labels = &labels;
        let mut edit = None;
        let mut sample_all_layers = false;
        let mut select_subject = false;
        let (edit_ref, sample_ref, subject_ref) = (&mut edit, &mut sample_all_layers, &mut select_subject);
        let response = OptionsBar::new(id_salt).show(ui, move |bar| {
            bar.leading(move |ui| {
                OptionsBar::tool_icon(ui, tool.icon(), tool.label(labels));
            });
            bar.body(move |ui| {
                let sm = ControlSize::Sm;
                let numbers = LocaleNumbers::of(ui);
                if tool.edits() {
                    for (add, icon, name, salt) in [(true, Icon::Plus, &labels.add, "add"), (false, Icon::Minus, &labels.subtract, "subtract")] {
                        let button = IconButton::new(icon, name.as_str()).size(sm).selected(options.add == add).id_salt(salt);
                        if ui.add(button).clicked() && options.add != add {
                            options.add = add;
                            *edit_ref = Some(EditEvent::set(*options, Modifiers::NONE));
                        }
                    }
                    OptionsBar::divider(ui);
                }
                if tool.brushes() {
                    let size = options.sizes.get(tool);
                    let field = NumberField::new("size", labels.size.as_str(), size, &numbers)
                        .range(BrushSizes::RANGE)
                        .decimals(0..=0)
                        .unit(Unit::new("{n} px"));
                    if let Some(event) = OptionsBar::number_cell(ui, &labels.size, CellWidth::Md, field).event {
                        if let Some(&value) = event.value()
                            && value != size
                        {
                            options.sizes.set(tool, value);
                        }
                        *edit_ref = Some(event.map(|_| *options));
                    }
                }
                if tool == SelectMaskTool::Brush
                    && let Some(event) = OptionsBar::percent_cell(ui, "hardness", &labels.hardness, options.hardness, &numbers).event
                {
                    if let Some(&value) = event.value() {
                        options.hardness = value.clamp(0.0, 100.0);
                    }
                    *edit_ref = Some(event.map(|_| *options));
                }
                if matches!(tool, SelectMaskTool::Quick | SelectMaskTool::RefineEdge | SelectMaskTool::Object)
                    && Checkbox::bool(labels.sample_all_layers.as_str(), &mut options.sample_all_layers).size(sm).show(ui).response.changed()
                {
                    *sample_ref = true;
                    *edit_ref = Some(EditEvent::set(*options, Modifiers::NONE));
                }
                if tool.edits() {
                    OptionsBar::divider(ui);
                    if ui.add(Button::new(labels.select_subject.as_str()).size(sm)).clicked() {
                        *subject_ref = true;
                    }
                    // Hair detection is not available: the button says so.
                    ui.add_enabled(false, Button::new(labels.refine_hair.as_str()).size(sm)).tooltip(labels.refine_hair_missing.as_str());
                }
            });
        });
        SelectMaskBarOutput { response, edit, sample_all_layers, select_subject }
    }
}
