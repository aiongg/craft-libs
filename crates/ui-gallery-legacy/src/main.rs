//! Desktop and web entry of the gallery.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Craft UI gallery").with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native("craft-ui-gallery", options, Box::new(|cc| Ok(Box::new(craft_ui_gallery::Gallery::new(cc.storage)))))
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast;
    wasm_bindgen_futures::spawn_local(async {
        let canvas = eframe::web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("gallery_canvas"))
            .and_then(|e| e.dyn_into::<eframe::web_sys::HtmlCanvasElement>().ok());
        let Some(canvas) = canvas else {
            return;
        };
        let started = eframe::WebRunner::new()
            .start(canvas, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(craft_ui_gallery::Gallery::new(cc.storage)))))
            .await;
        if let Err(error) = started {
            eframe::web_sys::console::error_1(&error);
        }
    });
}
