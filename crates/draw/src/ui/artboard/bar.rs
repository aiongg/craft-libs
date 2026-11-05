//! [`ArtboardBar`]: the Artboard tool's options bar cells.

use egui::{AsIdSalt, IdSalt, Modifiers, Ui};
use kurbo::Size;

use super::{ArtboardItem, ArtboardLabels, ArtboardPreset};
use craft_ui::controls::{Button, NumberField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, UnitKind, Units};
use craft_ui::patterns::{CellWidth, OptionsBar, OptionsBarSlots};

/// The smallest and largest side of a board, pixels (PhotoCraft's field range).
pub(super) const SIDE: std::ops::RangeInclusive<f64> = 1.0..=300_000.0;

/// What [`ArtboardBar`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app resizes the selected board or adds one"]
pub struct ArtboardBarOutput {
    /// The selected board's new size: a preset pick, Make Portrait or Make Landscape, or a typed W
    /// or H is `Set`; a caption scrub is a gesture. The app resizes the board (contents stay).
    pub size: Option<EditEvent<Size>>,
    /// "Add New Artboard": a board of the selected board's size, else the canvas's.
    pub add_new: bool,
}

/// The cells the Artboard tool adds to the options bar, in PhotoCraft's order: "Size" preset
/// dropdown (Custom, then the app's presets), W, H (1–300,000 px), Make Portrait (shown only when
/// the board is wider than tall), Make Landscape (only when taller than wide), "Add New Artboard".
/// W and H show their pixel unit. Without a selected board the size cells are disabled. Each cell
/// is a Tab stop in the bar's order. The bar has no ✓ or ⊘: nothing is pending.
pub struct ArtboardBar<'a> {
    id_salt: IdSalt,
    selected: Option<&'a ArtboardItem>,
    presets: &'a [ArtboardPreset],
    labels: Option<ArtboardLabels>,
    output: Option<&'a mut ArtboardBarOutput>,
}

impl<'a> ArtboardBar<'a> {
    /// The bar for the `selected` board with the app's `presets`.
    pub fn new(id_salt: impl AsIdSalt, selected: Option<&'a ArtboardItem>, presets: &'a [ArtboardPreset]) -> Self {
        ArtboardBar { id_salt: IdSalt::new(id_salt), selected, presets, labels: None, output: None }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ArtboardLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Where the bar reports its changes: a size replaces the last unread one, `add_new` is set
    /// by a click.
    pub fn output(mut self, output: &'a mut ArtboardBarOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Fills the bar's body slot.
    pub fn add_to(self, slots: &mut OptionsBarSlots<'a>) {
        slots.body(move |ui| self.cells(ui));
    }

    fn cells(self, ui: &mut Ui) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:293-338 `options_bar` (Size :303, Custom :305,
        // W :311, H :313, Make Portrait :318, Make Landscape :321, Add New Artboard :331).
        let labels = self.labels.clone().unwrap_or_else(|| ArtboardLabels::of(ui));
        let units = Units::of(ui);
        let numbers = units.numbers();
        let pixels = units.unit(UnitKind::Pixels);
        let bar_id = ui.make_persistent_id(self.id_salt);
        let mut out = ArtboardBarOutput::default();
        let size = self.selected.map(|b| Size::new(b.rect.width().abs(), b.rect.height().abs()));
        let shown = size.unwrap_or(Size::ZERO);
        OptionsBar::group(ui, size.is_some(), |ui| {
            OptionsBar::cell(ui, &labels.size, CellWidth::Content, |ui, _| {
                // The preset the board's size matches, else Custom.
                let mut choice = self.presets.iter().position(|p| p.size == shown);
                let mut select = Select::new((bar_id, "preset"), &labels.preset, &mut choice).option(None, labels.custom.as_str());
                for (i, preset) in self.presets.iter().enumerate() {
                    select = select.option(Some(i), preset.name.as_str());
                }
                let pick = select.size(OptionsBar::CONTROL_SIZE).show(ui);
                if let Some(EditEvent::Commit { value: Some(i), modifiers, .. } | EditEvent::Set { value: Some(i), modifiers, .. }) = pick.event
                    && let Some(preset) = self.presets.get(i)
                {
                    out.size = Some(EditEvent::set(preset.size, modifiers));
                }
            });
            let side = |ui: &mut Ui, salt: &str, caption: &str, name: &str, value: f64, other: f64, width_first: bool| {
                let field = NumberField::new((bar_id, salt), name, value, numbers).range(SIDE).decimals(0..=0).unit(pixels.clone());
                OptionsBar::number_cell(ui, caption, CellWidth::Md, field)
                    .event
                    .map(|e| e.map(|v| if width_first { Size::new(v, other) } else { Size::new(other, v) }))
            };
            let (width_event, height_event) = (
                side(ui, "w", &labels.width, &labels.width_name, shown.width, shown.height, true),
                side(ui, "h", &labels.height, &labels.height_name, shown.height, shown.width, false),
            );
            out.size = out.size.take().or(width_event).or(height_event);
            // Make Portrait is for a board wider than tall, Make Landscape for one taller than wide.
            let swapped = Size::new(shown.height, shown.width);
            let set = |size| EditEvent::set(size, Modifiers::NONE);
            if shown.width > shown.height
                && ui.add(Button::new(labels.make_portrait.as_str()).size(OptionsBar::CONTROL_SIZE).id_salt("portrait")).clicked()
            {
                out.size = Some(set(swapped));
            }
            if shown.height > shown.width
                && ui.add(Button::new(labels.make_landscape.as_str()).size(OptionsBar::CONTROL_SIZE).id_salt("landscape")).clicked()
            {
                out.size = Some(set(swapped));
            }
        });
        OptionsBar::divider(ui);
        out.add_new = ui.add(Button::new(labels.add_new.as_str()).size(OptionsBar::CONTROL_SIZE).id_salt("add-new")).clicked();
        if let Some(output) = self.output {
            output.size = out.size.or(output.size.take());
            output.add_new |= out.add_new;
        }
    }
}
