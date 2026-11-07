//! The integration binary of craft-ui: every test is a registered trial.

mod contract;
mod egui_surface;
mod focus;
mod layout;
mod paint;
mod runtime;
mod style;

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
