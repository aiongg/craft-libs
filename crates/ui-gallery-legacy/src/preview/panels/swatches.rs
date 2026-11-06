//! The Swatches panel: named colours that set the foreground, or the background with Alt.

use craft::color::ui::SwatchPalette;
use craft_theme::{Rgba, Srgba};
use craft_ui::edit::EditEvent;
use craft_ui::patterns::Panel;
use egui::Ui;

use super::super::model::Mock;

/// Photoshop's default swatch row, by name.
const SWATCHES: [(&str, [u8; 3]); 24] = [
    ("Black", [0, 0, 0]),
    ("White", [255, 255, 255]),
    ("Gray", [128, 128, 128]),
    ("Red", [237, 28, 36]),
    ("Orange", [247, 148, 29]),
    ("Yellow", [255, 242, 0]),
    ("Lime", [141, 198, 63]),
    ("Green", [0, 166, 81]),
    ("Teal", [0, 169, 157]),
    ("Cyan", [0, 174, 239]),
    ("Blue", [0, 114, 188]),
    ("Navy", [46, 49, 146]),
    ("Purple", [146, 39, 143]),
    ("Magenta", [236, 0, 140]),
    ("Pink", [244, 154, 193]),
    ("Brown", [141, 85, 36]),
    ("Light red", [247, 150, 152]),
    ("Light orange", [253, 198, 137]),
    ("Light yellow", [255, 247, 153]),
    ("Light green", [196, 223, 155]),
    ("Light cyan", [125, 207, 247]),
    ("Light blue", [131, 147, 202]),
    ("Light purple", [199, 164, 207]),
    ("Dark gray", [64, 64, 64]),
];

fn srgba(c: [u8; 3]) -> Srgba {
    Rgba::rgb(c[0], c[1], c[2]).into()
}

pub fn show(ui: &mut Ui, mock: &mut Mock) {
    Panel::new("swatches").title("Swatches").show(ui, |ui| {
        let swatches = SWATCHES.iter().map(|(name, c)| (*name, srgba(*c)));
        let picked = SwatchPalette::new("swatches.palette", "Swatches", srgba(mock.colors.fg), swatches).size(craft_theme::ControlSize::Sm).show(ui);
        if let Some(EditEvent::Set { value, modifiers, .. }) = picked.event {
            let Rgba([r, g, b, _]) = value.into();
            if modifiers.alt {
                mock.colors.bg = [r, g, b];
            } else {
                mock.colors.fg = [r, g, b];
            }
        }
    });
}
