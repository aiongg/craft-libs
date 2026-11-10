//! The options bar: the theme and mode, the rarer appearance options in More, the workspace's own controls, and Reset
//! layout and the workspace switch at the end.

use crate::viewer::{
    CONTRASTS, DENSITIES, DIRECTIONS, GalleryCommands, MODES, MOTIONS, Preview, SCALES, TEXT_SIZES, VariantAxis, Viewer, Workspace, theme_entries,
};
use craft_behavior::{ColumnWidth, IconId, Value};
use craft_ui::bars::{Bar, BarKind};
use craft_ui::buttons::{IconButton, ToggleButton};
use craft_ui::choice::Segmented;
use craft_ui::contract::{Bind, Entry, Show as _, ShowContainer as _, Trigger};
use craft_ui::display::{Divider, Shade, Text};
use craft_ui::layout_components::Grid;
use craft_ui::menu::Menu;
use craft_ui::pickers::Select;
use craft_ui::popover::Popover;
use egui::Ui;

/// The More popover's rows: a caption and its control.
static FORM: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::Content];

/// A caption before the control it names.
fn caption(ui: &mut Ui, text: &'static str) {
    Text::new(format!("{text}:")).shade(Shade::Muted).show(ui);
}

/// A select over `entries` after its caption.
fn pick<T: Value>(ui: &mut Ui, caption_text: &'static str, value: &mut T, entries: &[Entry<T>]) {
    caption(ui, caption_text);
    Select::new(caption_text, Bind::bound(value), entries).show(ui);
}

/// A segmented row over `entries` after its caption.
fn choose<T: Value>(ui: &mut Ui, caption_text: &'static str, value: &mut T, entries: &[Entry<T>]) {
    caption(ui, caption_text);
    Segmented::new(caption_text, Bind::bound(value), entries).show(ui);
}

/// The More popover's form: the root values the bar's own controls leave out.
fn more_form(ui: &mut Ui, viewer: &mut Viewer) {
    let env = &mut viewer.env;
    let (contrasts, densities, motions) = (CONTRASTS.entries(), DENSITIES.entries(), MOTIONS.entries());
    let (directions, text_sizes, scales) = (DIRECTIONS.entries(), TEXT_SIZES.entries(), SCALES.entries());
    Grid::new(&FORM).show(ui, |ui, _| {
        choose(ui, "Contrast", &mut env.contrast, &contrasts);
        choose(ui, "Size", &mut env.density, &densities);
        choose(ui, "Reduced motion", &mut env.motion, &motions);
        choose(ui, "Direction", &mut env.direction, &directions);
        choose(ui, "Text", &mut env.text_size, &text_sizes);
        pick(ui, "Scale", &mut env.scale, &scales);
    });
}

/// The Variants toggle and its menu of axes: the toggle shows whether a Variants view is open, and a press opens the
/// menu. Returns the axis chosen.
fn variants_menu(ui: &mut Ui, shown: bool, has_target: bool) -> Option<VariantAxis> {
    let mut on = shown;
    let button = ToggleButton::new("Variants", Bind::bound(&mut on)).trailing_icon(IconId::new("chevron-down")).show(ui);
    let entries: Vec<Entry<VariantAxis>> = VariantAxis::ALL.into_iter().map(|axis| Entry::new(axis, axis.menu_label()).enabled(has_target)).collect();
    Menu::new("Variants").values().opened_by(&button.response, Trigger::Press).pick(ui, &entries).picked
}

/// The theme and mode, the More popover, the workspace's own controls, then Reset layout and the workspace switch.
pub fn options_bar(ui: &mut Ui, viewer: &mut Viewer) {
    let (modes, themes) = (MODES.entries(), theme_entries());
    let (previews, workspaces) = (
        [Entry::new(Preview::App, "App"), Entry::new(Preview::Components, "Components")],
        [Entry::new(Workspace::Components, "Components"), Entry::new(Workspace::Theme, "Theme")],
    );
    let has_target = viewer.browser.target_story().is_some();
    let workspace = viewer.workspace;
    Bar::new("Appearance", BarKind::Options).show(ui, |ui, _| {
        pick(ui, "Theme", &mut viewer.env.theme, &themes);
        choose(ui, "Mode", &mut viewer.env.mode, &modes);
        let more = ToggleButton::new("More appearance settings", Bind::bound(&mut viewer.more_open)).icon(IconId::new("more")).icon_only().show(ui);
        let mut open = viewer.more_open;
        Popover::new("More appearance settings", Bind::bound(&mut open)).opened_by(&more.response).show(ui, |ui, _| more_form(ui, viewer));
        viewer.more_open = open;
        Divider::vertical().show(ui);
        match workspace {
            Workspace::Components => {
                if let Some(axis) = variants_menu(ui, viewer.variants.is_some(), has_target) {
                    viewer.variants = Some(axis);
                }
            }
            Workspace::Theme => choose(ui, "Preview", &mut viewer.preview, &previews),
        }
        IconButton::command(IconId::new("undo"), GalleryCommands::ResetLayout).show(ui);
        choose(ui, "Workspace", &mut viewer.workspace, &workspaces);
    });
}
