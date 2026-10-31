//! On-demand font registration (gap 33): a face for a script is read and added to the stack the
//! first time a visible character of that script has no glyph, instead of every candidate face
//! being installed up front. PhotoCraft's lazy loader (`crates/ui-egui/src/text/cjk_fonts.rs`) is
//! the model; it covers CJK and every other script the same way.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::path::PathBuf;
use std::sync::Arc;

use craft_i18n::Han;
use egui::FontDefinitions;

/// Candidate font files per script, most preferred first. Each is read only when a character of
/// that script is missing, at most [`MAX_SCRIPT_READS`] files per script and one face per pass.
/// Apps and tests plug in their own source: the system's fonts ([`SystemCandidates`]), a fixed
/// list, or none ([`NoCandidates`]) so a test never depends on the machine.
pub trait FontCandidates: Send + Sync {
    /// Files that may cover `script` (ISO 15924), most preferred first. For Han the `han`
    /// preference orders the regional faces (Japanese before Simplified for a `ja` locale).
    fn files(&self, script: &str, han: Option<Han>) -> Vec<PathBuf>;
    /// The face index in `data` whose character map has `c`; `None` when none has it.
    fn covers(&self, data: &[u8], c: char) -> Option<u32>;
    /// A background lookup (fontconfig's CJK scan) is still running: the loader waits for the
    /// preferred script instead of falling through to another one.
    fn pending(&self) -> bool {
        false
    }
}

/// The installed fonts as candidates: fontconfig on Linux, the font directories on macOS and
/// Windows (`craft_font_catalog::font_dirs`). Not on the web.
#[derive(Clone, Debug, Default)]
pub struct SystemCandidates;

impl FontCandidates for SystemCandidates {
    // pc: crates/ui-egui/src/text/cjk_fonts.rs:148 `SystemCache` and `:482 script_iso`; the
    // fontconfig lookup through `crates/ui-egui/src/text/cjk_fonts/` and craft-font-catalog's
    // `fontconfig_dirs`.
    fn files(&self, script: &str, han: Option<Han>) -> Vec<PathBuf> {
        todo!()
    }

    // pc: crates/ui-egui/src/text/cjk_fonts.rs:446 `craft_face_installs_coverage` (cmap lookup
    // through skrifa's charmap, every face of a collection).
    fn covers(&self, data: &[u8], c: char) -> Option<u32> {
        todo!()
    }

    // pc: crates/ui-egui/src/text/cjk_fonts.rs:610 `wait_for_fontconfig_cjk_scan`.
    fn pending(&self) -> bool {
        todo!()
    }
}

/// No candidates: text draws with the installed stack only (tests, the web build).
#[derive(Clone, Debug, Default)]
pub struct NoCandidates;

impl FontCandidates for NoCandidates {
    fn files(&self, _script: &str, _han: Option<Han>) -> Vec<PathBuf> {
        Vec::new()
    }

    fn covers(&self, _data: &[u8], _c: char) -> Option<u32> {
        None
    }
}

/// How many candidate files a script may read before the loader gives up on it (PhotoCraft's cap).
pub const MAX_SCRIPT_READS: usize = 6;

/// The egui plugin that registers faces on demand. [`UiFonts::on_demand`](super::UiFonts::on_demand)
/// installs one; a host that builds its stack by hand installs it with [`OnDemandFonts::install`].
///
/// Each pass it looks at the shapes drawn for a missing glyph, picks the script of the first such
/// character, reads that script's candidates until one covers it, and inserts that face after the
/// craft faces of every family (`Stack::insert`). A face is read once; a script whose
/// candidates are exhausted is not tried again until the fonts are reinstalled.
pub struct OnDemandFonts {
    candidates: Arc<dyn FontCandidates>,
    han: Option<Han>,
    /// Scripts tried so far and how many files each has read.
    reads: Vec<(String, usize)>,
    /// Faces registered so far: (font name, file, face index).
    registered: Vec<(String, PathBuf, u32)>,
}

impl OnDemandFonts {
    /// A loader over `candidates`, with the Han preference of the effective locale.
    pub fn new(candidates: Arc<dyn FontCandidates>, han: Option<Han>) -> Self {
        OnDemandFonts { candidates, han, reads: Vec::new(), registered: Vec::new() }
    }

    /// Install the loader as an egui plugin on `ctx`. Replaces an earlier one.
    // pc: crates/ui-egui/src/text/cjk_fonts.rs:943 `CjkFontPlugin` and `:976 install`.
    pub fn install(self, ctx: &egui::Context) {
        todo!()
    }

    /// The faces registered so far, for the font list in Preferences and the tests.
    pub fn registered(&self) -> &[(String, PathBuf, u32)] {
        &self.registered
    }

    /// The first character with no glyph among `shapes`, if any: what the next pass loads for.
    // pc: crates/ui-egui/src/text/cjk_fonts.rs:826 `missing_char` (egui's `missing_glyph` hook on
    // the pass's shapes).
    fn missing_char(ctx: &egui::Context, shapes: &[egui::epaint::ClippedShape]) -> Option<char> {
        todo!()
    }
}

/// The Han preference for the interface: the UI language's when it is Japanese, Korean or Chinese,
/// else the OS locale's, else none (`Hani` falls to the manifest order). PhotoCraft's rule for
/// which regional CJK face wins a shared code point.
// pc: crates/ui-egui/src/text/cjk_fonts.rs:564 `effective_locale`.
pub fn han_for_locale(ui_language: &str, os_locale: Option<&str>) -> Option<Han> {
    todo!()
}

/// egui's default font definitions built from deflated copies of its faces, for builds without
/// `default_fonts` (the web size gate): the same faces, tweaks and family order. The deflated
/// copies come from craft-fonts-build.
// pc: crates/ui-egui/src/text/egui_fonts.rs:42 `definitions` and `:48 from_deflated`.
pub fn deflated_default_definitions() -> FontDefinitions {
    todo!()
}
