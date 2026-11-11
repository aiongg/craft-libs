//! The integration binary of craft-ui: every test is a registered trial.

mod access;
mod api_map;
mod bars;
pub(crate) mod body;
mod canvas;
mod canvas_controls;
mod collection;
mod commands;
mod components_index;
mod contract;
mod dialog;
mod dialogs;
mod docking;
mod drag;
mod field_text_centred;
// outer edit
mod edit;
// end of outer edit
mod egui_surface;
mod focus;
mod fonts;
mod form_dialogs;
// full-window parts
mod full_window_parts;
// end of full-window parts
mod icon_ids;
mod internals;
mod keys;
mod keys_consumed;
mod label;
mod layer;
mod layout;
mod list;
mod measure;
mod menu;
mod motion;
mod notices;
mod overlay;
mod paint;

// Panels family.
mod panels;

mod pickers;
mod popover;
mod preferences;
mod presets;
mod rails;
mod runtime;
mod sections;
mod shell;

mod style;
mod suite_rtl;
mod suite_rules;
mod suite_title;
mod text;
mod tile_grid;
mod value_drag;
mod values;
mod windows_kit;

use craft_ui::testing::registered;

fn main() {
    craft_ui::testing::run(module_path!(), env!("CARGO_PKG_NAME"), &registered());
}
