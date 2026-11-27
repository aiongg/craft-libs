//! Clipboard text and images. Reads answer through the event queue: `WindowEvent::Paste` or
//! `PasteImage` in the window that asked, plus a step request, as the browser's promise requires.

use craft_ui_rt::{Clipboard, WindowId};
#[cfg(not(target_arch = "wasm32"))]
use craft_ui_rt::{RgbaImage, SizeU, WindowEvent};

use crate::shared::Cell;
#[cfg(not(target_arch = "wasm32"))]
use crate::shared::push;

pub struct WinitClipboard {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    shared: Cell,
    #[cfg(not(target_arch = "wasm32"))]
    system: Option<arboard::Clipboard>,
}

impl WinitClipboard {
    pub(crate) fn new(shared: Cell) -> Self {
        WinitClipboard {
            shared,
            #[cfg(not(target_arch = "wasm32"))]
            system: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn system(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.system.is_none() {
            self.system = arboard::Clipboard::new().inspect_err(|e| log::warn!("no system clipboard: {e}")).ok();
        }
        self.system.as_mut()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Clipboard for WinitClipboard {
    fn set_text(&mut self, text: &str) {
        if let Some(c) = self.system()
            && let Err(e) = c.set_text(text)
        {
            log::warn!("clipboard write failed: {e}");
        }
    }

    fn request_text(&mut self, window: WindowId) {
        let text = self.system().and_then(|c| c.get_text().ok());
        if let Some(text) = text {
            push(&self.shared, window, WindowEvent::Paste(text));
        }
    }

    fn set_image(&mut self, image: &RgbaImage) {
        let data = arboard::ImageData { width: image.size.width as usize, height: image.size.height as usize, bytes: image.pixels.as_slice().into() };
        if let Some(c) = self.system()
            && let Err(e) = c.set_image(data)
        {
            log::warn!("clipboard image write failed: {e}");
        }
    }

    fn request_image(&mut self, window: WindowId) {
        let Some(data) = self.system().and_then(|c| c.get_image().ok()) else { return };
        let (Ok(width), Ok(height)) = (u32::try_from(data.width), u32::try_from(data.height)) else { return };
        let image = RgbaImage { size: SizeU::new(width, height), pixels: data.bytes.into_owned() };
        push(&self.shared, window, WindowEvent::PasteImage(std::sync::Arc::new(image)));
    }
}

/// `navigator.clipboard`, looked up by name: the typed bindings need an unstable web-sys cfg.
/// Images are not read or written on the web yet (they need PNG blobs and `ClipboardItem`).
#[cfg(target_arch = "wasm32")]
mod web {
    use js_sys::{Function, Promise, Reflect};
    use wasm_bindgen::{JsCast, JsValue};

    pub fn clipboard() -> Option<JsValue> {
        let navigator = web_sys::window()?.navigator();
        Reflect::get(&navigator, &"clipboard".into()).ok().filter(|c| !c.is_undefined())
    }

    pub fn method(clipboard: &JsValue, name: &str) -> Option<Function> {
        Reflect::get(clipboard, &name.into()).ok()?.dyn_into().ok()
    }

    pub fn promise(value: JsValue) -> Option<Promise> {
        value.dyn_into().ok()
    }
}

#[cfg(target_arch = "wasm32")]
impl Clipboard for WinitClipboard {
    fn set_text(&mut self, text: &str) {
        let Some(c) = web::clipboard() else { return };
        if let Some(write) = web::method(&c, "writeText") {
            let _ = write.call1(&c, &text.into());
        }
    }

    fn request_text(&mut self, window: WindowId) {
        let Some(c) = web::clipboard() else { return };
        let Some(read) = web::method(&c, "readText") else { return };
        let Some(promise) = read.call0(&c).ok().and_then(web::promise) else { return };
        let shared = self.shared.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(text) = wasm_bindgen_futures::JsFuture::from(promise).await.ok().and_then(|v| v.as_string()) {
                crate::shared::push(&shared, window, craft_ui_rt::WindowEvent::Paste(text));
            }
        });
    }
}
