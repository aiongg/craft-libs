//! The integration binary of craft-ui-widgets: every test is a registered trial.

// The test files name the library as `craft_ui`, as the crates that use it do.
extern crate craft_ui_widgets as craft_ui;

mod bars;
pub(crate) mod body;
mod commands;
mod contract;
mod field_text_centred;
mod keys;
mod label;
mod layer;
mod layout;
mod menu;
mod overlay;

mod pickers;
mod popover;
mod runtime;

mod suite_rtl;
mod suite_rules;
mod suite_title;
mod text;
mod tile_grid;
mod value_drag;
mod values;

use craft_ui::testing::{registered, suite, trials};
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args = Arguments::from_args();
    // This binary's test modules and its own library's trials; the crates below run theirs in their own binaries.
    let own = [concat!(module_path!(), "::"), "craft_ui_widgets::"];
    let mut trials: Vec<Trial> = trials()
        .into_iter()
        .filter(|trial| own.iter().any(|prefix| format!("{}::", trial.module).starts_with(prefix)))
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
