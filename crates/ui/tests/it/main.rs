//! The integration binary of craft-ui: every test is a registered trial.

mod access;
pub(crate) mod body;
mod collection;
mod callsites;
mod commands;
mod contract;
mod controls;
mod drag;
mod egui_surface;
mod focus;
mod glossary;
mod label;
mod layout;
mod motion;
mod overlay;
mod paint;
mod parity;
mod runtime;
mod sections;
mod style;
mod text;
mod values;
mod windows_kit;

use craft_ui::testing::{TRIALS, registered, suite};
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
