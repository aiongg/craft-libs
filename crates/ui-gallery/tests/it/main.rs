//! The viewer's integration tests: one binary, run by `libtest-mimic` over the registered trials.

mod mock;
mod overview;
mod shell;
mod theme;
mod viewer;

use craft_ui::testing::{suite, trials};
use craft_ui_gallery::testing::registered;
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args = Arguments::from_args();
    let mut trials: Vec<Trial> = trials()
        .into_iter()
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
