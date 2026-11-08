//! The options bar and the status bar.

use crate::viewer::{
    CONTRASTS, CellFilter, DENSITIES, DIRECTIONS, GalleryCommands, MODES, MOTIONS, SCALES, SIZES, TEXT_SIZES, Workspace, theme_entries,
};
use craft_behavior::Value;
use craft_ui::bars::{Bar, BarKind};
use craft_ui::buttons::{Button, ToggleButton};
use craft_ui::choice::Segmented;
use craft_ui::contract::{Bind, Entry, Show as _, ShowContainer as _};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::Stack;
use craft_ui::pickers::Select;
use craft_ui::popover::Popover;
use craft_ui::runtime::env::RootEnv;
use egui::Ui;

/// A select over `entries` after its caption.
fn pick<T: Value>(ui: &mut Ui, caption: &'static str, value: &mut T, entries: &[Entry<T>]) {
    Text::new(caption).shade(Shade::Muted).show(ui);
    Select::new(caption, Bind::bound(value), entries).show(ui);
}

/// The workspace switch, the root values the stories and panels are shown under (the theme first), the cells a story
/// shows, the More popover for the rest, and Reset Layout last.
pub fn options_bar(ui: &mut Ui, env: &mut RootEnv, cells: &mut CellFilter, workspace: &mut Workspace, more_open: &mut bool) {
    let (modes, contrasts, text_sizes) = (MODES.entries(), CONTRASTS.entries(), TEXT_SIZES.entries());
    let (directions, sizes) = (DIRECTIONS.entries(), SIZES.entries());
    let (densities, motions, scales) = (DENSITIES.entries(), MOTIONS.entries(), SCALES.entries());
    let (themes, workspaces) = (theme_entries(), [Entry::new(Workspace::Stories, "Stories"), Entry::new(Workspace::Theme, "Theme")]);
    Bar::new("Appearance", BarKind::Options).show(ui, |ui, _| {
        Segmented::new("Workspace", Bind::bound(&mut *workspace), &workspaces).show(ui);
        pick(ui, "Theme", &mut env.theme, &themes);
        pick(ui, "Mode", &mut env.mode, &modes);
        pick(ui, "Contrast", &mut env.contrast, &contrasts);
        pick(ui, "Text size", &mut env.text_size, &text_sizes);
        Segmented::new("Direction", Bind::bound(&mut cells.direction), &directions).show(ui);
        Segmented::new("Control size", Bind::bound(&mut cells.size), &sizes).show(ui);
        let more = ToggleButton::new("More", Bind::bound(&mut *more_open)).show(ui);
        Popover::new("More appearance options", Bind::bound(&mut *more_open)).opened_by(&more.response).show(ui, |ui, _| {
            Stack::new().show(ui, |ui, _| {
                pick(ui, "Density", &mut env.density, &densities);
                pick(ui, "Motion", &mut env.motion, &motions);
                pick(ui, "Scale", &mut env.scale, &scales);
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
