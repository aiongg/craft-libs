//! The installed system fonts: a provider over one index of them, built once per process, and the OS's own interface
//! face.

#[cfg(feature = "system_fonts")]
mod fontique;
#[cfg(feature = "system_fonts")]
mod index;
pub(crate) mod ui_face;

use std::sync::Arc;

use egui::{Context, FontProvider};

/// A system font provider that prefers `locale`'s fonts, shared by every stack and check that asks
/// for the same locale. Every locale's provider asks one index of the installed fonts, built once
/// on a background thread; a lookup before it is built waits for it. `None` without the
/// `system_fonts` feature and on the web.
#[cfg(feature = "system_fonts")]
pub fn system_font_provider(locale: &str) -> Option<Arc<dyn FontProvider>> {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock, PoisonError};
    static PROVIDERS: OnceLock<Mutex<BTreeMap<String, Arc<dyn FontProvider>>>> = OnceLock::new();
    let mut providers = PROVIDERS.get_or_init(Mutex::default).lock().unwrap_or_else(PoisonError::into_inner);
    Some(Arc::clone(providers.entry(locale.to_owned()).or_insert_with(|| fontique::provider(locale, true))))
}

/// A system font provider that prefers `locale`'s fonts, shared by every stack and check that asks
/// for the same locale. `None` without the `system_fonts` feature and on the web.
#[cfg(not(feature = "system_fonts"))]
pub fn system_font_provider(_locale: &str) -> Option<Arc<dyn FontProvider>> {
    None
}

/// The system font provider an install gives a context when the app sets none, and the function that starts the
/// index for that context. A lookup before the index is built finds nothing, so the UI thread doesn't wait; the
/// context reloads its fonts and repaints once it is.
#[cfg(feature = "system_fonts")]
pub(crate) fn installed_system_font_provider(locale: &str) -> Option<(Arc<dyn FontProvider>, fn(&Context))> {
    Some((fontique::provider(locale, false), fontique::start))
}

#[cfg(not(feature = "system_fonts"))]
pub(crate) fn installed_system_font_provider(_locale: &str) -> Option<(Arc<dyn FontProvider>, fn(&Context))> {
    None
}
