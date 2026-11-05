//! [`FillBar`]: Content-Aware Fill's options bar.

use egui::{AsIdSalt, IdSalt, ModifierNames, Modifiers, Response, Ui};

use craft_ui::canvas::SelectionOperation;
use craft_ui::controls::{IconButton, NumberField};
use craft_ui::dialog::DialogLabels;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, UnitKind, Units};
use craft_ui::patterns::{CellWidth, OptionsBar};

use super::FillLabels;
use super::lasso::FillAreaEdit;
use super::tool::{FillTool, FillToolOptions};

/// What [`FillBar::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FillBarOutput {
    /// The bar's response.
    pub response: Response,
    /// Expand or Contract pressed.
    pub edit: Option<FillAreaEdit>,
    /// The Lasso's Reset pressed (`FillReset::FillArea`).
    pub reset_fill_area: bool,
}

/// The options bar, full width under the title (lead call 4; PhotoCraft's started beside the
/// strip): the tool's icon, then
///
/// - Sampling Brush: + and − (Add, Subtract), Size (1..=5000 px);
/// - Lasso: the four operations (New, Add with the Shift hint, Subtract with Alt, Intersect with
///   Shift+Alt, the hints in the platform's key names), Expand, Contract, the amount (1..=100 px)
///   and Reset (an icon button).
pub struct FillBar<'a> {
    id_salt: IdSalt,
    tool: FillTool,
    options: &'a mut FillToolOptions,
    labels: Option<FillLabels>,
}

impl<'a> FillBar<'a> {
    /// Starts the bar for `tool`, editing `options` in place; `id_salt` keeps its state apart from other bars.
    pub fn new(id_salt: impl AsIdSalt, tool: FillTool, options: &'a mut FillToolOptions) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, options, labels: None }
    }

    /// The bar's words; [`FillLabels::of`] when not set.
    pub fn labels(mut self, labels: FillLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the bar and returns its response and the edits it made.
    pub fn show(self, ui: &mut Ui) -> FillBarOutput {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:33 `caf_options_bar`.
        let Self { id_salt, tool, options, labels } = self;
        let labels = labels.unwrap_or_else(|| FillLabels::of(ui));
        let labels = &labels;
        let reset_label = DialogLabels::of(ui).reset;
        let mac = ui.ctx().os().is_mac();
        let mut edit = None;
        let mut reset_fill_area = false;
        let (edit_ref, reset_ref) = (&mut edit, &mut reset_fill_area);
        let response = OptionsBar::new(id_salt).show(ui, move |bar| {
            bar.leading(move |ui| {
                let (icon, label) = match tool {
                    FillTool::SamplingBrush => (Icon::Brush, &labels.sampling_brush),
                    FillTool::Lasso => (Icon::Lasso, &labels.lasso),
                    FillTool::Hand => (Icon::Hand, &labels.hand),
                    FillTool::Zoom => (Icon::ZoomIn, &labels.zoom),
                };
                OptionsBar::tool_icon(ui, icon, label.as_str());
            });
            bar.body(move |ui| {
                let sm = OptionsBar::CONTROL_SIZE;
                let numbers = LocaleNumbers::of(ui);
                let pixels = Units::of(ui).unit(UnitKind::Pixels);
                match tool {
                    FillTool::SamplingBrush => {
                        for (add, icon, name, salt) in [(true, Icon::Plus, &labels.add, "add"), (false, Icon::Minus, &labels.subtract, "subtract")] {
                            let button = IconButton::new(icon, name.as_str()).size(sm).selected(options.brush_add == add).id_salt(salt);
                            if ui.add(button).clicked() {
                                options.brush_add = add;
                            }
                        }
                        OptionsBar::divider(ui);
                        let field = NumberField::new("size", labels.size.as_str(), options.brush_size, &numbers)
                            .range(1.0..=FillToolOptions::SIZE_MAX)
                            .decimals(0..=0)
                            .unit(pixels);
                        if let Some(value) =
                            OptionsBar::number_cell(ui, &labels.size, CellWidth::Md, field).event.as_ref().and_then(|e| e.value()).copied()
                        {
                            options.brush_size = value.clamp(1.0, FillToolOptions::SIZE_MAX);
                        }
                    }
                    FillTool::Lasso => {
                        let hints = [
                            (SelectionOperation::New, Icon::Square, labels.new_selection.clone(), "new"),
                            (SelectionOperation::Add, Icon::Plus, hinted(&labels.add_to_selection, Modifiers::SHIFT, mac), "add"),
                            (SelectionOperation::Subtract, Icon::Minus, hinted(&labels.subtract_from_selection, Modifiers::ALT, mac), "subtract"),
                            (
                                SelectionOperation::Intersect,
                                Icon::SquaresSubtract,
                                hinted(&labels.intersect_with_selection, Modifiers::SHIFT | Modifiers::ALT, mac),
                                "intersect",
                            ),
                        ];
                        for (operation, icon, name, salt) in hints {
                            let button = IconButton::new(icon, name).size(sm).selected(options.lasso == operation).id_salt(salt);
                            if ui.add(button).clicked() {
                                options.lasso = operation;
                            }
                        }
                        OptionsBar::divider(ui);
                        let by = options.grow_by;
                        let (min, max) = FillAreaEdit::GROW_RANGE;
                        for (expand, name, salt) in [(true, &labels.expand, "expand"), (false, &labels.contract, "contract")] {
                            if ui.add(craft_ui::controls::Button::new(name.as_str()).size(sm).id_salt(salt)).clicked() {
                                *edit_ref = Some(FillAreaEdit::grow(expand, by));
                            }
                        }
                        let field = NumberField::new("grow-by", labels.grow_by.as_str(), by, &numbers).range(min..=max).decimals(0..=0).unit(pixels);
                        if let Some(value) =
                            OptionsBar::number_cell(ui, &labels.grow_by, CellWidth::Md, field).event.as_ref().and_then(|e| e.value()).copied()
                        {
                            options.grow_by = value.clamp(min, max);
                        }
                        let button = IconButton::new(Icon::Undo, reset_label.as_str()).size(sm).id_salt("reset");
                        if ui.add(button).clicked() {
                            *reset_ref = true;
                        }
                    }
                    FillTool::Hand | FillTool::Zoom => {}
                }
            });
        });
        FillBarOutput { response, edit, reset_fill_area }
    }
}

/// `template` with `{key}` filled by `modifiers` in the platform's notation.
fn hinted(template: &str, modifiers: Modifiers, mac: bool) -> String {
    let names = if mac { ModifierNames::SYMBOLS } else { ModifierNames::NAMES };
    craft_i18n::fmt(template, &[("key", &names.format(&modifiers, mac))])
}
