//! The options bar and the status bar.

use crate::viewer::{Appearance, CONTRASTS, DENSITIES, DIRECTIONS, GalleryCommands, MODES, MOTIONS, SCALES, SIZES, TEXT_SIZES};
use craft_behavior::Value;
use craft_ui::bars::{Bar, BarKind};
use craft_ui::buttons::{Button, ToggleButton};
use craft_ui::choice::Segmented;
use craft_ui::contract::{Bind, Entry, Show as _, ShowContainer as _};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::Stack;
use craft_ui::pickers::Select;
use craft_ui::popover::Popover;
use egui::Ui;

/// A select over `entries` after its caption.
fn pick<T: Value>(ui: &mut Ui, caption: &'static str, value: &mut T, entries: &[Entry<T>]) {
    Text::new(caption).shade(Shade::Muted).show(ui);
    Select::new(caption, Bind::bound(value), entries).show(ui);
}

/// The appearance the stories are shown under: the choices of the bar, the More popover for the rest, and Reset
/// Layout last.
pub fn options_bar(ui: &mut Ui, appearance: &mut Appearance, more_open: &mut bool) {
    let (modes, contrasts, text_sizes) = (MODES.entries(), CONTRASTS.entries(), TEXT_SIZES.entries());
    let (directions, sizes) = (DIRECTIONS.entries(), SIZES.entries());
    let (densities, motions, scales) = (DENSITIES.entries(), MOTIONS.entries(), SCALES.entries());
    Bar::new("Appearance", BarKind::Options).show(ui, |ui, _| {
        pick(ui, "Theme", &mut appearance.mode, &modes);
        pick(ui, "Contrast", &mut appearance.contrast, &contrasts);
        pick(ui, "Text size", &mut appearance.text_size, &text_sizes);
        Segmented::new("Direction", Bind::bound(&mut appearance.direction), &directions).show(ui);
        Segmented::new("Control size", Bind::bound(&mut appearance.size), &sizes).show(ui);
        let more = ToggleButton::new("More", Bind::bound(&mut *more_open)).show(ui);
        Popover::new("More appearance options", Bind::bound(&mut *more_open)).opened_by(&more.response).show(ui, |ui, _| {
            Stack::new().show(ui, |ui, _| {
                pick(ui, "Density", &mut appearance.density, &densities);
                pick(ui, "Motion", &mut appearance.motion, &motions);
                pick(ui, "Scale", &mut appearance.scale, &scales);
            });
        });
        Button::command(GalleryCommands::ResetLayout).show(ui);
    });
}

/// The status bar: `text` and, when saving the settings failed, why.
pub fn status_bar(ui: &mut Ui, text: &str) {
    Bar::new("Status", BarKind::Status).show(ui, |ui, _| {
        Text::new(text.to_owned()).show(ui);
    });
}
