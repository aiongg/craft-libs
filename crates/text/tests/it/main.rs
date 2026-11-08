//! The integration binary of craft-text: every test is a registered trial.

mod character;
mod columns;
mod diag;
mod editor;
mod find;
mod hyperlink;
mod notes;
mod paragraph_dialog;
mod ruler;
mod spelling;
mod style_panel;
mod tabs_dialog;

use craft_ui::testing::{TRIALS, suite};
use libtest_mimic::{Arguments, Trial};

/// `value` after `edit`: a test's variation of a value whose type is non-exhaustive.
fn set<T>(mut value: T, edit: impl FnOnce(&mut T)) -> T {
    edit(&mut value);
    value
}

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
    trials.extend(suite::trials(env!("CARGO_PKG_NAME"), &craft_text::testing::registered()));
    libtest_mimic::run(&args, trials).exit();
}
