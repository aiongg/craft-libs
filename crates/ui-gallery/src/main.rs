//! The craft-ui story viewer: the registered stories of the linked crates, shown under a chosen appearance.

use craft_behavior::placement::Defaults;
use craft_ui::app::AppSpec;
use craft_ui_gallery::WINDOW;
use craft_ui_gallery::chrome::Gallery;

const TITLE: &str = "craft-ui stories";
const WINDOW_MIN: [f32; 2] = [640.0, 480.0];

fn spec() -> AppSpec {
    AppSpec::new("craft-ui-gallery", TITLE, Defaults::new(WINDOW, WINDOW_MIN)).stories()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if !craft_ui_gallery::registry().install_story_sets() {
        eprintln!("the selected story commands are invalid");
        std::process::exit(1);
    }
    if let Err(error) = craft_ui::app::run(spec(), |_| Gallery::open()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

/// The web build: the gallery on the page's `gallery_canvas`, started by trunk's bundle. The number of stories goes
/// to the browser console, to compare with the native count.
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsValue;
    use web_sys::console;

    wasm_bindgen_futures::spawn_local(async {
        if !craft_ui_gallery::registry().install_story_sets() {
            console::error_1(&JsValue::from_str("the selected story commands are invalid"));
            return;
        }
        let (counts, stories) = craft_ui_gallery::linked_counts();
        let per_crate: Vec<String> = counts.iter().map(|(name, count)| format!("{name}={count}")).collect();
        console::log_1(&JsValue::from_str(&format!("craft-ui-gallery: {stories} stories; registrations by crate: {}", per_crate.join(" "))));
        if let Err(error) = craft_ui::app::start_web("gallery_canvas", spec(), |_| Gallery::open()).await {
            console::error_1(&JsValue::from_str(&format!("craft-ui-gallery: the app did not start: {error}")));
        }
    });
}
