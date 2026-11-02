//! [`VanishingPointBar`]: Vanishing Point's options bar (task WS-VP2).

use egui::{AsIdSalt, IdSalt, Modifiers, Response, Ui};

use craft_theme::ControlSize;
use craft_ui::controls::{Checkbox, NumberField};
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::{CellWidth, OptionsBar};

use super::VanishingPointLabels;
use super::editor::{PasteImage, StampSettings, VanishingPointSettings, VanishingTool};

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
    /// The setting the user changed this pass, with every setting as it is now (written to the
    /// settings): a number field's gesture (`Begin`, `Update`, `Commit`) or a checkbox's `Set`.
    pub edit: Option<EditEvent<VanishingPointSettings>>,
    /// The paste's Width changed (a session change: one undo step when the field settles).
    pub paste: Option<PasteImage>,
}

/// The options bar, full width under the title: for Stamp Diameter (1..=5000 px), Hardness,
/// Opacity (0..=100 %), Aligned; for every tool Grid Size (px); with an image on the clipboard
/// Paste and Width (percent of the plane); New Layer. PhotoCraft's tool pills and its ✓ and ⊘
/// buttons go: the strip holds the tools and the side column's footer OK and Cancel.
///
/// Width shows while Paste is on. New Layer shows with an image on the clipboard, with strokes
/// ([`has_strokes`](Self::has_strokes)) or with the Stamp tool, as PhotoCraft's bar does. Every
/// field is an ordinary focusable control of the library's `OptionsBar` cells; the bar changes the
/// settings and reports the paste.
pub struct VanishingPointBar<'a> {
    id_salt: IdSalt,
    tool: VanishingTool,
    settings: &'a mut VanishingPointSettings,
    paste: PasteImage,
    has_clip: bool,
    has_strokes: bool,
    labels: Option<VanishingPointLabels>,
}

impl<'a> VanishingPointBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, tool: VanishingTool, settings: &'a mut VanishingPointSettings, paste: PasteImage, has_clip: bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, settings, paste, has_clip, has_strokes: false, labels: None }
    }

    /// Whether the session has clone strokes (default no): they show New Layer.
    pub fn has_strokes(mut self, has_strokes: bool) -> Self {
        self.has_strokes = has_strokes;
        self
    }

    /// The words (default: the installed words).
    pub fn labels(mut self, labels: VanishingPointLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> VanishingPointBarOutput {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1230 `options_bar`.
        let Self { id_salt, tool, settings, paste, has_clip, has_strokes, labels } = self;
        let labels = labels.unwrap_or_else(|| VanishingPointLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let mut edit = None;
        let mut pasted = None;
        // A number field's event with the settings it wrote, or a checkbox's change.
        let report = |edit: &mut Option<EditEvent<VanishingPointSettings>>, settings: &VanishingPointSettings, event: EditEvent<f64>| {
            *edit = Some(event.map(|_| *settings));
        };
        let toggled = |edit: &mut Option<EditEvent<VanishingPointSettings>>, settings: &VanishingPointSettings, changed: bool| {
            if changed {
                *edit = Some(EditEvent::set(*settings, Modifiers::NONE));
            }
        };
        let response = OptionsBar::new(id_salt).show(ui, |bar| {
            bar.row(|ui| {
                if tool == VanishingTool::Stamp {
                    let stamp = &mut settings.stamp;
                    let cell = OptionsBar::number_cell(
                        ui,
                        &labels.diameter,
                        CellWidth::Md,
                        NumberField::new("diameter", labels.diameter.as_str(), stamp.diameter, &numbers)
                            .range(DIAMETER)
                            .decimals(0..=0)
                            .unit(Unit::new("{n} px")),
                    );
                    if let Some(event) = cell.event {
                        if let Some(v) = event.value() {
                            stamp.diameter = *v;
                        }
                        report(&mut edit, settings, event);
                    }
                    type Percent<'a> = (&'a str, &'a String, fn(&mut StampSettings) -> &mut f64);
                    let percents: [Percent; 2] =
                        [("hardness", &labels.hardness, |s| &mut s.hardness), ("opacity", &labels.opacity, |s| &mut s.opacity)];
                    for (salt, caption, field) in percents {
                        let value = field(&mut settings.stamp);
                        let cell = OptionsBar::percent_cell(ui, salt, caption, *value, &numbers);
                        if let Some(event) = cell.event {
                            if let Some(v) = event.value() {
                                *value = *v;
                            }
                            report(&mut edit, settings, event);
                        }
                    }
                    let changed =
                        Checkbox::bool(labels.aligned.as_str(), &mut settings.stamp.aligned).size(ControlSize::Sm).show(ui).response.changed();
                    toggled(&mut edit, settings, changed);
                    OptionsBar::divider(ui);
                }
                let cell = OptionsBar::number_cell(
                    ui,
                    &labels.grid_size,
                    CellWidth::Md,
                    NumberField::new("grid-size", labels.grid_size.as_str(), settings.grid_size, &numbers)
                        .range(GRID_SIZE)
                        .decimals(0..=0)
                        .unit(Unit::new("{n} px")),
                );
                if let Some(event) = cell.event {
                    if let Some(v) = event.value() {
                        settings.grid_size = *v;
                    }
                    report(&mut edit, settings, event);
                }
                if has_clip {
                    OptionsBar::divider(ui);
                    let changed = Checkbox::bool(labels.paste.as_str(), &mut settings.paste).size(ControlSize::Sm).show(ui).response.changed();
                    toggled(&mut edit, settings, changed);
                    if settings.paste {
                        let cell = OptionsBar::number_cell(
                            ui,
                            &labels.width,
                            CellWidth::Md,
                            NumberField::new("paste-width", labels.width.as_str(), paste.width * 100.0, &numbers)
                                .range(PASTE_WIDTH)
                                .decimals(0..=0)
                                .unit(Unit::new("{n}%")),
                        );
                        if let Some(v) = cell.event.as_ref().and_then(|e| e.value()) {
                            pasted = Some(PasteImage { width: *v / 100.0, ..paste });
                        }
                    }
                }
                if has_clip || has_strokes || tool == VanishingTool::Stamp {
                    OptionsBar::divider(ui);
                    let changed =
                        Checkbox::bool(labels.new_layer.as_str(), &mut settings.new_layer).size(ControlSize::Sm).show(ui).response.changed();
                    toggled(&mut edit, settings, changed);
                }
            });
        });
        VanishingPointBarOutput { response, edit, paste: pasted }
    }
}
