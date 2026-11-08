//! The integration binary of craft-imaging: every test is a registered trial.

mod adjust_editors;
mod adjust_tone;
mod blend_keys;
mod blur_pins;
mod channels;
mod curve;
mod fill;
mod histogram;
mod layer_style;
mod layer_style_parts;
mod perspective_warp;
mod pin_warp;
mod scopes;
mod select;
mod select_mask;
mod selection;
mod transform_preview;
mod warp;

mod fill;
use craft_imaging::testing::registered;
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
    trials.extend(suite::trials(env!("CARGO_PKG_NAME"), &craft_imaging::testing::registered()));
    trials.extend(suite::trials(env!("CARGO_PKG_NAME"), &registered()));
    libtest_mimic::run(&args, trials).exit();
}
