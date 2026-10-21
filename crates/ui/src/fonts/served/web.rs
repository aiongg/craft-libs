//! Fetching served faces in the browser: the UI language's at install, others when egui asks the
//! context's providers for a character no installed face has.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use egui::epaint::text::Blob;
use egui::{FallbackRequest, FontFamily, FontInsert, FontProvider};
use wasm_bindgen::{JsCast as _, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::script_of;
use crate::fonts::Stack;

/// Install `stack`, add the files fetched so far, and fetch the UI language's faces.
pub(in crate::fonts) fn install(ctx: &egui::Context, mut stack: Stack) {
    let loader = Loader::of(ctx);
    let fetch = {
        let mut state = loader.lock();
        for (url, file) in &state.files {
            stack.provide(url, file);
        }
        state.ui_pending.clear();
        let mut fetch: Vec<(&'static str, &'static str)> = Vec::new();
        for face in stack.ui_downloads() {
            if state.files.contains_key(face.url) || state.failed.contains(face.url) {
                continue;
            }
            state.ui_pending.insert(face.url);
            if !fetch.iter().any(|(url, _)| *url == face.url) {
                fetch.push((face.url, face.integrity));
            }
        }
        state.stack = Some(stack.clone());
        fetch
    };
    stack.install_faces(ctx, Some(Arc::clone(&loader) as Arc<dyn FontProvider>));
    for (url, integrity) in fetch {
        loader.fetch(url, integrity);
    }
}

pub(in crate::fonts) fn loading(ctx: &egui::Context) -> bool {
    let loader: Option<Arc<Loader>> = ctx.data(|d| d.get_temp(id()));
    loader.is_some_and(|loader| !loader.lock().ui_pending.is_empty())
}

fn id() -> egui::Id {
    egui::Id::unique("craft_ui::fonts::served")
}

/// The context's fetched files and the stack they go into. Also the provider asked for characters
/// no face has, which starts fetching the face that draws them.
struct Loader {
    ctx: egui::Context,
    me: Weak<Loader>,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// The installed stack, without the files' data.
    stack: Option<Stack>,
    /// Fetched files by URL; the browser's HTTP cache keeps them across page loads.
    files: BTreeMap<&'static str, Blob>,
    in_flight: BTreeSet<&'static str>,
    failed: BTreeSet<&'static str>,
    /// The UI language's files not fetched yet.
    ui_pending: BTreeSet<&'static str>,
    /// The (family, script) pairs a missing character has started a fetch for. Each starts at
    /// most one, so a character no served face has doesn't fetch them all.
    asked: Vec<(FontFamily, &'static str)>,
}

impl Loader {
    fn of(ctx: &egui::Context) -> Arc<Self> {
        let existing: Option<Arc<Self>> = ctx.data(|d| d.get_temp(id()));
        existing.unwrap_or_else(|| {
            let loader = Arc::new_cyclic(|me| Self { ctx: ctx.clone(), me: me.clone(), state: Mutex::default() });
            ctx.data_mut(|d| d.insert_temp(id(), Arc::clone(&loader)));
            loader
        })
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn fetch(&self, url: &'static str, integrity: &'static str) {
        if !self.lock().in_flight.insert(url) {
            return;
        }
        let Some(me) = self.me.upgrade() else { return };
        wasm_bindgen_futures::spawn_local(async move {
            let result = fetch_file(url, integrity).await;
            me.arrived(url, result);
        });
    }

    fn arrived(&self, url: &'static str, result: Result<Vec<u8>, String>) {
        let stack = {
            let mut state = self.lock();
            state.in_flight.remove(url);
            state.ui_pending.remove(url);
            match result {
                Ok(bytes) => {
                    let file: Blob = Arc::new(bytes);
                    state.files.insert(url, Arc::clone(&file));
                    state.stack.as_mut().map(|stack| {
                        stack.provide(url, &file);
                        stack.clone()
                    })
                }
                Err(error) => {
                    state.failed.insert(url);
                    web_sys::console::warn_1(&format!("craft-ui: cannot fetch the font {url}: {error}").into());
                    None
                }
            }
        };
        if let Some(stack) = stack
            && let Some(me) = self.me.upgrade()
        {
            stack.install_faces(&self.ctx, Some(me as Arc<dyn FontProvider>));
        }
        self.ctx.request_repaint();
    }
}

impl FontProvider for Loader {
    /// Start fetching the face that draws the character, and find nothing for now: the face is
    /// installed when it arrives, which asks again.
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        let c = request.cluster.chars().next()?;
        let script = script_of(c)?;
        // egui asks during layout; a fetch that is finishing holds the lock only briefly.
        let mut state = self.state.try_lock().ok()?;
        let key = (request.family.clone(), script);
        if state.asked.contains(&key) {
            return None;
        }
        let stack = state.stack.as_ref()?;
        let mut next = None;
        for face in stack.downloads_for(request.family, c) {
            if state.in_flight.contains(face.url) {
                return None;
            }
            if !state.files.contains_key(face.url) && !state.failed.contains(face.url) {
                next = Some((face.url, face.integrity));
                break;
            }
        }
        let (url, integrity) = next?;
        state.asked.push(key);
        drop(state);
        self.fetch(url, integrity);
        None
    }
}

async fn fetch_file(url: &str, integrity: &str) -> Result<Vec<u8>, String> {
    let js = |e: JsValue| e.as_string().unwrap_or_else(|| format!("{e:?}"));
    let window = web_sys::window().ok_or("no window")?;
    let init = web_sys::RequestInit::new();
    if !integrity.is_empty() {
        init.set_integrity(integrity);
    }
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str_and_init(url, &init)).await.map_err(js)?.dyn_into().map_err(js)?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let buffer = JsFuture::from(response.array_buffer().map_err(js)?).await.map_err(js)?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}
