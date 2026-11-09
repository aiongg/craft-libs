//! The integration binary of craft-table: every test is a registered trial.

mod chart;
mod conditions;
mod fields;
mod grid;
mod outline;
mod refpick;
mod stats;

use craft_table::ui::testing::registered;
use craft_ui::testing::{suite, trials};
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
