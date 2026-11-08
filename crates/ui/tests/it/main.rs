//! The integration binary of craft-ui: every test is a registered trial.

mod access;
mod access_selected;
mod api_map;
mod bars;
pub(crate) mod body;
mod button_clicks;
mod buttons;
mod callsites;
mod canvas;
mod canvas_controls;
mod choice;
mod collection;
mod commands;
mod components_index;
mod contract;
mod controls;
mod dialog;
mod dialogs;
mod display;
mod docking;
mod drag;
// outer edit
mod edit;
// end of outer edit
mod egui_surface;
mod examples;
mod focus;
mod fonts;
mod form;
mod form_dialogs;
// full-window parts
mod full_window_parts;
// end of full-window parts
mod glossary;
mod internals;
mod keys;
mod keys_consumed;
mod label;
mod layer;
mod layout;
mod list;
mod lists;
mod menu;
mod menu_mac;
mod motion;
mod notices;
mod overlay;
mod paint;

// Panels family.
mod panels;

mod parity;
mod pickers;
mod popover;
mod preferences;
mod presets;
mod prompts;
mod rails;
mod runtime;
mod shell;
mod sections;

mod style;
mod suite_rtl;
mod suite_rules;
mod suite_title;
mod swatch;
mod swatch_mixed;
mod table;
mod tabs;
mod text;
mod tile_grid;
mod tool_fixture;
mod value_drag;
mod values;
mod windows_kit;

use craft_ui::testing::{TRIALS, registered, suite};
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args = Arguments::from_args();
    let mut trials: Vec<Trial> = TRIALS
        .iter()
        .map(|trial| {
            let run = trial.run;
            Trial::test(trial.full_name(), move || {
                run();
                Ok(())
            })
        })
        .collect();
    trials.extend(suite::trials(env!("CARGO_PKG_NAME"), &registered()));
    libtest_mimic::run(&args, trials).exit();
}
