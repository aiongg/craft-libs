//! The integration binary of craft-timeline: every test is a registered trial.

mod features;
mod graph;
mod lanes;
mod panel;
mod ruler;
mod transport;

use craft_ui::testing::{TRIALS, suite};
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args = Arguments::from_args();
    let own = module_path!();
    let mut trials: Vec<Trial> = TRIALS
        .iter()
        .filter(|trial| trial.module == own || trial.module.starts_with(&format!("{own}::")))
        .map(|trial| {
            let run = trial.run;
            Trial::test(trial.full_name(), move || {
                run();
                Ok(())
            })
        })
        .collect();
    trials.extend(suite::trials(env!("CARGO_PKG_NAME"), &craft_timeline::testing::registered()));
    libtest_mimic::run(&args, trials).exit();
}
