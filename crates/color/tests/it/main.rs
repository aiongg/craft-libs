//! The integration binary of craft-color: every test is a registered trial.

mod color_parts;
mod paint;
mod panels;
mod picker;
mod sampling;

use craft_color::ui::testing::registered;
use craft_ui::testing::{TRIALS, suite};
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
