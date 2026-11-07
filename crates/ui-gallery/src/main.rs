//! The craft-ui story viewer: the registered stories of the linked crates, shown under a chosen appearance.

use craft_ui::shell::app::{AppSpec, run};
use craft_ui_gallery::chrome::Gallery;
use craft_window::restore::Defaults;

const WINDOW: [f32; 2] = [1100.0, 800.0];
const WINDOW_MIN: [f32; 2] = [640.0, 480.0];

fn main() -> eframe::Result {
    let spec = AppSpec::new("craft-ui-gallery", "craft-ui stories", Defaults::new(WINDOW, WINDOW_MIN));
    run(spec, |_| Gallery::open())
}
