//! The integration binary of craft-ui-contract: every test is a registered trial.

// The test files name the library as `craft_ui`, as the crates that use it do.
extern crate craft_ui_contract as craft_ui;

mod access;
pub(crate) mod body;
mod canvas;
mod collection;
mod contract;
mod drag;
mod egui_surface;
mod fonts;
mod icon_ids;
mod keys_consumed;
mod layout;
mod motion;
mod paint;

mod runtime;

mod style;
mod text;

use craft_ui::testing::{registered, suite, trials};
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args = Arguments::from_args();
    // This binary's test modules and its own library's trials; the crates below run theirs in their own binaries.
    let own = [concat!(module_path!(), "::"), "craft_ui_contract::"];
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
