//! The integration binary of craft-ui: every test is a registered trial.

mod api_map;
mod canvas;
mod components_index;
mod form_dialogs;

mod shell;

use craft_ui::testing::{registered, suite, trials};
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args = Arguments::from_args();
    // This binary's test modules and its own library's trials; the crates below run theirs in their own binaries.
    let own = [concat!(module_path!(), "::"), "craft_ui::"];
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
