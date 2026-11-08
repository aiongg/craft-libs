//! The integration binary of craft-draw: every test is a registered trial.

// draw-shape
mod add_beside;
mod pattern;
mod shape;

// draw-gradient-a
mod gradient_bar;

// draw-gradient-b
mod gradient_overlay;
mod gradient_presets;

// draw-connector
mod connector;
mod shape_drag;

// draw-pen-a
mod bezier;

// draw-pen-b
mod corner;
mod pen;

// draw-crop-b
mod crop_tool;
mod perspective_crop;

// draw-bars
mod crop_bar;
mod measure;

// draw-transform-a
mod transform_box;
mod transform_session;

// draw-transform-b
mod symmetry;
mod transform_fields;

// draw-stroke-fill
mod picture_fill;
mod stroke;

// draw-slice-artboard
mod artboard;
mod slice;

// draw-brush
mod brush_presets;
mod brush_settings;

use craft_draw::testing::registered;
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
