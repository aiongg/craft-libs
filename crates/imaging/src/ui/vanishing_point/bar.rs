//! [`VanishingPointBar`]: Vanishing Point's options bar (task WS-VP2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use craft_theme::ControlSize;
use craft_ui::controls::{Checkbox, NumberField};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::{CellWidth, OptionsBar};

use super::VanishingPointLabels;
use super::editor::{PasteImage, VanishingPointOptions, VanishingTool};

/// Diameter, in pixels.
const DIAMETER: std::ops::RangeInclusive<f64> = 1.0..=5000.0;
/// Grid Size, in pixels (PhotoCraft's range).
const GRID_SIZE: std::ops::RangeInclusive<f64> = 2.0..=2000.0;
/// The paste's Width, in percent of the plane's width (PhotoCraft's range).
const PASTE_WIDTH: std::ops::RangeInclusive<f64> = 1.0..=1000.0;

/// What [`VanishingPointBar::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VanishingPointBarOutput {
    pub response: Response,
    /// An option changed (written to the options).
    pub changed: bool,
    /// The paste's Width changed (a session change: one undo step when the field settles).
    pub paste: Option<PasteImage>,
}

/// The options bar, full width under the title: for Stamp Diameter (1..=5000 px), Hardness,
/// Opacity (0..=100 %), Aligned; for every tool Grid Size (px); with an image on the clipboard
/// Paste and Width (percent of the plane); New Layer. PhotoCraft's tool pills and its ✓ and ⊘
/// buttons go: the strip holds the tools and the side column's footer OK and Cancel.
///
/// Width shows while Paste is on, and New Layer always (PhotoCraft hid it until there was a paste,
/// a stroke or the Stamp tool; the bar does not know the strokes). Every field is an ordinary
/// focusable control of the library's `OptionsBar` cells; the bar changes the options and reports
/// the paste, nothing else.
pub struct VanishingPointBar<'a> {
    id_salt: IdSalt,
    tool: VanishingTool,
    options: &'a mut VanishingPointOptions,
    paste: PasteImage,
    has_clip: bool,
    labels: &'a VanishingPointLabels,
}

impl<'a> VanishingPointBar<'a> {
    pub fn new(
        id_salt: impl AsIdSalt,
        tool: VanishingTool,
        options: &'a mut VanishingPointOptions,
        paste: PasteImage,
        has_clip: bool,
        labels: &'a VanishingPointLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, options, paste, has_clip, labels }
    }

    pub fn show(self, ui: &mut Ui) -> VanishingPointBarOutput {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1230 `options_bar`.
        let Self { id_salt, tool, options, paste, has_clip, labels } = self;
        let numbers = LocaleNumbers::of(ui);
        let mut changed = false;
        let mut pasted = None;
        let response = OptionsBar::new(id_salt).show(ui, |bar| {
            bar.row(|ui| {
                if tool == VanishingTool::Stamp {
                    let stamp = &mut options.stamp;
                    let edit = OptionsBar::number_cell(
                        ui,
                        &labels.diameter,
                        CellWidth::Md,
                        NumberField::new("diameter", labels.diameter.as_str(), stamp.diameter, &numbers)
                            .range(DIAMETER)
                            .decimals(0..=0)
                            .unit(Unit::new("{n} px")),
                    );
                    if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                        stamp.diameter = *v;
                        changed = true;
                    }
                    for (salt, caption, value) in
                        [("hardness", &labels.hardness, &mut stamp.hardness), ("opacity", &labels.opacity, &mut stamp.opacity)]
                    {
                        let edit = OptionsBar::percent_cell(ui, salt, caption, *value, &numbers);
                        if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                            *value = *v;
                            changed = true;
                        }
                    }
                    changed |= Checkbox::bool(labels.aligned.as_str(), &mut stamp.aligned).size(ControlSize::Sm).show(ui).response.changed();
                    OptionsBar::divider(ui);
                }
                let edit = OptionsBar::number_cell(
                    ui,
                    &labels.grid_size,
                    CellWidth::Md,
                    NumberField::new("grid-size", labels.grid_size.as_str(), options.grid_size, &numbers)
                        .range(GRID_SIZE)
                        .decimals(0..=0)
                        .unit(Unit::new("{n} px")),
                );
                if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                    options.grid_size = *v;
                    changed = true;
                }
                if has_clip {
                    OptionsBar::divider(ui);
                    changed |= Checkbox::bool(labels.paste.as_str(), &mut options.paste).size(ControlSize::Sm).show(ui).response.changed();
                    if options.paste {
                        let edit = OptionsBar::number_cell(
                            ui,
                            &labels.width,
                            CellWidth::Md,
                            NumberField::new("paste-width", labels.width.as_str(), paste.width * 100.0, &numbers)
                                .range(PASTE_WIDTH)
                                .decimals(0..=0)
                                .unit(Unit::new("{n}%")),
                        );
                        if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                            pasted = Some(PasteImage { width: *v / 100.0, ..paste });
                        }
                    }
                }
                OptionsBar::divider(ui);
                changed |= Checkbox::bool(labels.new_layer.as_str(), &mut options.new_layer).size(ControlSize::Sm).show(ui).response.changed();
            });
        });
        VanishingPointBarOutput { response, changed, paste: pasted }
    }
}
