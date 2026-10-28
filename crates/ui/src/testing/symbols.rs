//! craft-fonts' symbol faces for kittest harnesses.
//!
//! Apps draw the macOS key symbols of [`Kbd`](crate::controls::Kbd) (⌘⇧⌥⌃) from the symbol faces
//! they embed from craft-fonts; egui's default faces lack them, and kittest fails a frame that
//! draws a character no face has. [`install`] adds those faces from the craft-fonts checkout that
//! `CRAFT_FONTS_DIR` names, the same checkout the apps' builds embed from.

use std::path::Path;
use std::sync::OnceLock;

use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{Context, FontData, FontFamily};

/// The manifest's script tag for symbol faces.
const SYMBOLS: &str = "Zsym";

/// A face's name and file contents.
type Face = (String, Vec<u8>);

/// Whether `CRAFT_FONTS_DIR` is set, so [`install`] has faces to add.
pub fn available() -> bool {
    faces().is_some()
}

/// Adds craft-fonts' symbol faces after the proportional and monospace families' own faces.
/// Without `CRAFT_FONTS_DIR` it adds nothing.
///
/// # Panics
///
/// If `CRAFT_FONTS_DIR` is set but its manifest or a symbol face can't be read.
pub fn install(ctx: &Context) {
    for (name, bytes) in faces().unwrap_or_default() {
        let families =
            [FontFamily::Proportional, FontFamily::Monospace].map(|family| InsertFontFamily { family, priority: FontPriority::Lowest }).into();
        ctx.add_font(FontInsert::new(name, FontData::from_static(bytes), families));
    }
}

/// The symbol faces of the checkout, read once.
fn faces() -> Option<&'static [Face]> {
    static FACES: OnceLock<Option<Vec<Face>>> = OnceLock::new();
    FACES.get_or_init(|| std::env::var_os("CRAFT_FONTS_DIR").filter(|dir| !dir.is_empty()).map(|dir| read(Path::new(&dir)))).as_deref()
}

#[allow(clippy::panic, reason = "a misconfigured test environment")]
fn read(dir: &Path) -> Vec<Face> {
    let manifest = dir.join("fonts/manifest.txt");
    let text = std::fs::read_to_string(&manifest).unwrap_or_else(|e| panic!("CRAFT_FONTS_DIR: {}: {e}", manifest.display()));
    let entries = craft_fonts_build::parse_manifest(&text).unwrap_or_else(|e| panic!("CRAFT_FONTS_DIR: {}: {e}", manifest.display()));
    let faces: Vec<_> = entries
        .iter()
        .filter(|e| e.has_script(SYMBOLS))
        .map(|e| {
            let path = dir.join(&e.file);
            let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("CRAFT_FONTS_DIR: {}: {err}", path.display()));
            (format!("{} {}", e.family, e.style), bytes)
        })
        .collect();
    assert!(!faces.is_empty(), "CRAFT_FONTS_DIR: {} lists no {SYMBOLS} face", manifest.display());
    faces
}
