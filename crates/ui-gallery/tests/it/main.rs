//! The viewer's integration tests: one binary, run by `libtest-mimic` over the registered trials.

mod theme;
mod viewer;

use craft_ui::testing::TRIALS;
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args = Arguments::from_args();
    let trials: Vec<Trial> = TRIALS
        .iter()
        .map(|trial| {
            let run = trial.run;
            Trial::test(trial.full_name(), move || {
                run();
                Ok(())
            })
        })
        .collect();
    libtest_mimic::run(&args, trials).exit();
}
