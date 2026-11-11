//! The integration binary of craft-imaging: every test is a registered trial.

mod adjust_editors;
mod adjust_tone;
mod blur_pins;
mod curve;
mod fill;
mod filter_gallery;
mod histogram;
mod layer_style;
mod liquify;
mod live_preview;
mod perspective_warp;
mod pin_warp;
mod scopes;
mod select;
mod select_mask;
mod selection;
mod transform_preview;
mod vanishing_point;
mod warp;
mod wide_angle;

use craft_ui::testing::{suite, trials};
use libtest_mimic::{Arguments, Trial};

fn main() {
    let args = Arguments::from_args();
    let own = module_path!();
    let mut trials: Vec<Trial> = trials()
        .into_iter()
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
    libtest_mimic::run(&args, trials).exit();
}
