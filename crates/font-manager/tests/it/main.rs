//! The integration binary of craft-font-manager: every test is a registered trial.

mod axes;
mod glyphs;
mod missing;
mod panel;
mod panel_parts;
mod picker;

use craft_font_manager::testing::registered;
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
    trials.extend(suite::trials(env!("CARGO_PKG_NAME"), &registered()));
    libtest_mimic::run(&args, trials).exit();
}
