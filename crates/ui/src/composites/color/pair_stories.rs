//! Stories of [`SwatchPair`]: foreground over background, and fill over stroke.

use craft_theme::Srgba;
use egui::Ui;

use super::{PairWell, SwatchPair, Well};
use crate::behavior::FieldTab;
use crate::controls::{TextField, slider};
use crate::edit;
use crate::localized::LocaleNumbers;
use crate::primitives::{Content, Gap, Stack, Text};
use crate::story::{Args, Level, Story};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "composites/color/pair-foreground",
            level: Level::Composites,
            component: "SwatchPair",
            name: "Foreground and background",
            args: || Args::new().with_bool("buttons", true).with_bool("disabled", false),
            show: |ui, args| show(ui, args, "fg-bg", ["Foreground colour", "Background colour"], false),
        },
        Story {
            id: "composites/color/pair-fill",
            level: Level::Composites,
            component: "SwatchPair",
            name: "Fill and stroke",
            args: || Args::new().with_bool("buttons", true).with_bool("disabled", false),
            show: |ui, args| show(ui, args, "fill-stroke", ["Fill colour", "Stroke colour"], true),
        },
        Story {
            id: "composites/color/pair-dialog",
            level: Level::Composites,
            component: "SwatchPair",
            name: "Dialog (field scope)",
            args: Args::new,
            show: show_dialog,
        },
    ]
}

/// A pair editing two colours: swap exchanges them and the default button sets black and white.
/// With `stroke_ring` the secondary well is a ring, as DesignCraft's stroke.
fn show(ui: &mut Ui, args: &mut Args, key: &str, names: [&str; 2], stroke_ring: bool) {
    let numbers = LocaleNumbers::from_tag("en");
    let black = Srgba::rgb(0.0, 0.0, 0.0);
    let white = Srgba::rgb(1.0, 1.0, 1.0);
    let start = if stroke_ring { (Srgba::rgb(0.2, 0.55, 0.8), Srgba::rgb(0.9, 0.35, 0.2)) } else { (black, white) };
    slider::with_values(ui, key, [(Well::Primary, start.0, start.1, String::new())], |ui, state| {
        let (active, first, second, last) = &mut state[0];
        let mut pair =
            SwatchPair::new(key, "Colours", [PairWell::new(names[0], *first), PairWell::new(names[1], *second).ring(stroke_ring)], &numbers)
                .active(*active)
                .enabled(!args.bool("disabled"))
                .picker(|p| p.alpha(false));
        if args.bool("buttons") {
            pair = pair.swap_button("Swap colours").default_button("Default colours");
        }
        let shown = pair.show(ui);
        *active = shown.active;
        if shown.swap {
            std::mem::swap(first, second);
        }
        if shown.defaults {
            (*first, *second) = (black, white);
        }
        if let Some(event) = &shown.edit.event {
            if let Some(p) = event.value() {
                *if shown.well == Well::Primary { &mut *first } else { &mut *second } = p.color;
            }
            *last = edit::describe(&event.clone().map(|p| p.color.to_rgba().to_hex()));
        }
        Stack::new(Gap::Stack).show(ui, |ui| {
            ui.add(Text::new(format!("Active: {}", names[*active as usize])).content(Content::Muted));
            ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
        });
    });
}

/// A pair between two text fields in a dialog's [`FieldTab`] scope: Tab goes from field to field
/// and passes the pair; Ctrl+Tab reaches it.
fn show_dialog(ui: &mut Ui, _: &mut Args) {
    let numbers = LocaleNumbers::from_tag("en");
    let start = (String::from("Spring"), String::from("Warm tones"), Srgba::rgb(0.2, 0.55, 0.8), Srgba::rgb(0.9, 0.35, 0.2));
    slider::with_values(ui, "pair-dialog", [(start, Well::Primary)], |ui, state| {
        let ((name, notes, first, second), active) = &mut state[0];
        FieldTab::new("pair-dialog").show(ui, |ui| {
            Stack::new(Gap::Stack).show(ui, |ui| {
                if let Some(text) = TextField::new("pair-dialog-name", "Palette name", name).show(ui).event.and_then(|e| e.value().cloned()) {
                    *name = text;
                }
                let shown = SwatchPair::new(
                    "pair-dialog-pair",
                    "Colours",
                    [PairWell::new("Fill colour", *first), PairWell::new("Stroke colour", *second)],
                    &numbers,
                )
                .active(*active)
                .swap_button("Swap colours")
                .picker(|p| p.alpha(false))
                .show(ui);
                *active = shown.active;
                if shown.swap {
                    std::mem::swap(first, second);
                }
                if let Some(p) = shown.edit.event.as_ref().and_then(|e| e.value()) {
                    *if shown.well == Well::Primary { &mut *first } else { &mut *second } = p.color;
                }
                if let Some(text) = TextField::new("pair-dialog-notes", "Notes", notes).show(ui).event.and_then(|e| e.value().cloned()) {
                    *notes = text;
                }
            });
        });
    });
}
