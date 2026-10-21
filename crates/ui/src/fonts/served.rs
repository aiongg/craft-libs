//! Faces a web build serves beside its wasm instead of embedding them, fetched when the UI needs
//! them.

use std::sync::Arc;

use egui::FontFamily;
use egui::epaint::text::Blob;

use super::{CraftFace, Stack, baseline_fallback, set_weight};

#[cfg(target_arch = "wasm32")]
pub(super) mod web;

/// The script tag served emoji faces get: ISO 15924 `Zsye`, emoji.
pub(super) const EMOJI: &str = "Zsye";

/// A craft-fonts face served beside the app, one per line of `fonts/manifest.txt`. The tables that
/// `craft-fonts-build`'s `Embed::serve` writes are slices of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServedFace {
    /// Family name, e.g. `"Noto Sans Arabic"`.
    pub family: &'static str,
    /// `"Regular"` or `"Bold"`; faces with other styles are not used for UI text.
    pub style: &'static str,
    /// The manifest's script tags, as in [`CraftFace::scripts`].
    pub scripts: &'static [&'static str],
    /// Face index in a font collection (`.ttc`); faces of one collection share a URL.
    pub index: u32,
    /// Where the file is fetched from, relative to the page or absolute.
    pub url: &'static str,
    /// File size in bytes.
    pub size: u64,
    /// Subresource Integrity value (`sha256-…`) the browser checks the file against; empty for
    /// none.
    pub integrity: &'static str,
}

impl ServedFace {
    /// The egui font name, e.g. `"Noto Sans Arabic Regular"`.
    pub fn name(&self) -> String {
        format!("{} {}", self.family, self.style)
    }

    /// The face as the stack orders it, without bytes.
    pub(super) fn craft_face(&self) -> CraftFace {
        CraftFace { family: self.family, style: self.style, scripts: self.scripts, bytes: &[], index: self.index }
    }
}

impl Stack {
    /// The served faces in the stack's chains (see [`super::UiFonts::served_faces`]).
    pub fn served(&self) -> &[ServedFace] {
        &self.served
    }

    /// The served faces the UI language needs, which [`Self::install`] fetches at once on the web:
    /// for each family, the best face for each of the language's scripts that the family's own
    /// faces lack. Empty for a language the app's faces cover.
    pub fn ui_downloads(&self) -> Vec<&ServedFace> {
        self.ui_downloads.iter().filter_map(|name| self.served_face(name)).collect()
    }

    /// The served faces that could draw `c` in `family`, in the order the family asks them: those
    /// tagged with `c`'s script. Empty for scripts the app's own faces are expected to cover
    /// (Latin, Greek, Cyrillic) and for characters of no particular script; for emoji and symbols,
    /// the served emoji face.
    pub fn downloads_for(&self, family: &FontFamily, c: char) -> Vec<&ServedFace> {
        let Some(script) = script_of(c) else { return Vec::new() };
        let configured = self.definitions.families.get(family).or_else(|| self.definitions.families.get(&FontFamily::Proportional));
        let mut faces: Vec<&ServedFace> = Vec::new();
        for name in configured.into_iter().flatten().chain(self.after_system.chain(family)) {
            if let Some(face) = self.served_face(name)
                && face.craft_face().covers(script)
                && !faces.contains(&face)
            {
                faces.push(face);
            }
        }
        faces
    }

    /// Add the file served at `url` to every served face it holds, for [`Self::install`] to
    /// install. A face in a family of another weight is set to that weight, and renamed as
    /// [`super::UiFonts::stack`] would have. The web build does this for each file it fetches; an
    /// app that gets the files some other way can call it itself.
    pub fn provide(&mut self, url: &str, file: &Blob) {
        let faces: Vec<ServedFace> = self.served.iter().filter(|face| face.url == url).copied().collect();
        for face in faces {
            let name = face.name();
            let data = baseline_fallback(egui::FontData::from_blob(Arc::clone(file), face.index));
            let weight = |family: &FontFamily| self.weights.iter().find(|(f, _)| f == family).map_or(400, |(_, w)| *w);
            let weighted = |family: &FontFamily| {
                let mut data = data.clone();
                let weight = weight(family);
                let name = if set_weight(&mut data, weight) && weight != 400 { format!("{name} @{weight}") } else { name.clone() };
                (name, data)
            };
            let mut configured = Vec::new();
            for (family, chain) in &mut self.definitions.families {
                for entry in chain.iter_mut().filter(|entry| **entry == name) {
                    let (new_name, data) = weighted(family);
                    if *family == FontFamily::Proportional && self.script_face.as_ref() == Some(entry) {
                        self.script_face = Some(new_name.clone());
                    }
                    entry.clone_from(&new_name);
                    configured.push((new_name, data));
                }
            }
            for (name, data) in configured {
                self.definitions.font_data.insert(name, Arc::new(data));
            }
            let mut after = Vec::new();
            for (family, chain) in self.after_system.chains_mut() {
                for entry in chain.iter_mut().filter(|entry| **entry == name) {
                    let (new_name, data) = weighted(family);
                    entry.clone_from(&new_name);
                    after.push((new_name, data));
                }
            }
            for (name, data) in after {
                self.after_system.insert(&name, data);
            }
        }
    }

    fn served_face(&self, name: &str) -> Option<&ServedFace> {
        self.served.iter().find(|face| face.name() == name)
    }
}

/// Whether the UI language's served faces are still being fetched, for an app that waits for them
/// before drawing its interface. Always false outside the web, and once every fetch has finished,
/// failed ones included.
#[cfg(target_arch = "wasm32")]
pub fn loading(ctx: &egui::Context) -> bool {
    web::loading(ctx)
}

/// Whether the UI language's served faces are still being fetched: never, outside the web.
#[cfg(not(target_arch = "wasm32"))]
pub fn loading(_ctx: &egui::Context) -> bool {
    false
}

/// The ISO 15924 script a served face must have to draw `c`, by Unicode block. `None` for Latin,
/// Greek, Cyrillic and characters common to every script, which no served face is fetched for.
/// Emoji and symbols map to [`EMOJI`].
pub(super) fn script_of(c: char) -> Option<&'static str> {
    // Sorted by start; ranges don't overlap.
    const BLOCKS: &[(u32, u32, &str)] = &[
        (0x0530, 0x058F, "Armn"),
        (0x0590, 0x05FF, "Hebr"),
        (0x0600, 0x06FF, "Arab"),
        (0x0700, 0x074F, "Syrc"),
        (0x0750, 0x077F, "Arab"),
        (0x0780, 0x07BF, "Thaa"),
        (0x0860, 0x086F, "Syrc"),
        (0x0870, 0x08FF, "Arab"),
        (0x0900, 0x097F, "Deva"),
        (0x0980, 0x09FF, "Beng"),
        (0x0A00, 0x0A7F, "Guru"),
        (0x0A80, 0x0AFF, "Gujr"),
        (0x0B00, 0x0B7F, "Orya"),
        (0x0B80, 0x0BFF, "Taml"),
        (0x0C00, 0x0C7F, "Telu"),
        (0x0C80, 0x0CFF, "Knda"),
        (0x0D00, 0x0D7F, "Mlym"),
        (0x0D80, 0x0DFF, "Sinh"),
        (0x0E00, 0x0E7F, "Thai"),
        (0x0E80, 0x0EFF, "Laoo"),
        (0x0F00, 0x0FFF, "Tibt"),
        (0x1000, 0x109F, "Mymr"),
        (0x10A0, 0x10FF, "Geor"),
        (0x1100, 0x11FF, "Hang"),
        (0x1200, 0x139F, "Ethi"),
        (0x1780, 0x17FF, "Khmr"),
        (0x1800, 0x18AF, "Mong"),
        (0x19E0, 0x19FF, "Khmr"),
        (0x1C90, 0x1CBF, "Geor"),
        (0x2190, 0x2BFF, EMOJI),
        (0x2D00, 0x2D2F, "Geor"),
        (0x2D80, 0x2DDF, "Ethi"),
        (0x2E80, 0x303F, "Hani"),
        (0x3040, 0x309F, "Hira"),
        (0x30A0, 0x30FF, "Kana"),
        (0x3100, 0x312F, "Bopo"),
        (0x3130, 0x318F, "Hang"),
        (0x31A0, 0x31BF, "Bopo"),
        (0x31C0, 0x31EF, "Hani"),
        (0x31F0, 0x31FF, "Kana"),
        (0x3200, 0x9FFF, "Hani"),
        (0xA8E0, 0xA8FF, "Deva"),
        (0xA960, 0xA97F, "Hang"),
        (0xAB00, 0xAB2F, "Ethi"),
        (0xAC00, 0xD7FF, "Hang"),
        (0xF900, 0xFAFF, "Hani"),
        (0xFB13, 0xFB17, "Armn"),
        (0xFB1D, 0xFB4F, "Hebr"),
        (0xFB50, 0xFDFF, "Arab"),
        (0xFE30, 0xFE4F, "Hani"),
        (0xFE70, 0xFEFF, "Arab"),
        (0xFF00, 0xFF65, "Hani"),
        (0xFF66, 0xFF9F, "Kana"),
        (0xFFA0, 0xFFDC, "Hang"),
        (0x1F000, 0x1FAFF, EMOJI),
        (0x20000, 0x3FFFF, "Hani"),
    ];
    let c = u32::from(c);
    let i = BLOCKS.partition_point(|(start, ..)| *start <= c);
    let (_, end, script) = BLOCKS.get(i.checked_sub(1)?)?;
    (c <= *end).then_some(*script)
}
