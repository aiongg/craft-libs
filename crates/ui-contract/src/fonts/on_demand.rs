//! On-demand font registration: a face for a script is read and handed to the engine the first
//! time a character of that script that text sets has no glyph, instead of every candidate face
//! being installed up front. It covers CJK and every other script the same way. The loader is a
//! source of the installed stack, so the engine asks it while it shapes the text, and the face
//! sets the character in the same layout.

#[cfg(not(target_arch = "wasm32"))]
mod files;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use craft_host::Wake;
#[cfg(not(target_arch = "wasm32"))]
use craft_host::target::Runtime;
use craft_i18n::Han;
use craft_ui_rt::{FaceSource, FallbackAsk, StackFace};
use skrifa::MetadataProvider as _;
use skrifa::raw::{FileRef, TableProvider as _};

use super::FontHost;
use super::faces::{baseline_fallback, has_char, set_weight};
use super::script::{EMOJI, MATH, SYMBOLS, script_of};
use super::stack::is_cjk;

/// Candidate font files per script, most preferred first. Each is read only when a character of
/// that script is missing, at most [`MAX_SCRIPT_READS`] files per script and one face per pass.
/// Apps and tests plug in their own source: the system's fonts ([`SystemCandidates`]), a fixed
/// list, or none ([`NoCandidates`]) so a test never depends on the machine.
///
/// Scripts are ISO 15924 codes: `Hani` (ideographs, CJK punctuation and full-width forms), `Kana`,
/// `Hang`, `Bopo`, and the other scripts of [`craft_i18n::Script`] that need a face beyond the
/// Latin, Cyrillic and Greek of the app's own (`Thai`, `Arab`, `Deva`, ...).
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
    /// Call `wake` once when the lookup that [`pending`](FontCandidates::pending) reports ends;
    /// at once when none is running. The loader asks once per lookup and requests a frame from
    /// it, so a pending lookup costs no timer.
    fn wake_when_ready(&self, wake: Wake) {
        wake.wake();
    }
}

/// The installed fonts as candidates: fontconfig on Linux, the font directories on macOS and
/// Windows (`craft_font_catalog::font_dirs`). Not on the web.
#[derive(Clone, Debug, Default)]
pub struct SystemCandidates;

impl FontCandidates for SystemCandidates {
    fn files(&self, script: &str, han: Option<Han>) -> Vec<PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        if Runtime::current() != Runtime::Web {
            return files::files(script, han);
        }
        let _ = (script, han);
        Vec::new()
    }

    fn covers(&self, data: &[u8], c: char) -> Option<u32> {
        covering_face(data, c)
    }

    fn pending(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return Runtime::current() == Runtime::Native && files::pending();
        #[cfg(target_arch = "wasm32")]
        false
    }

    fn wake_when_ready(&self, wake: Wake) {
        #[cfg(not(target_arch = "wasm32"))]
        if Runtime::current() != Runtime::Web {
            return files::wake_when_ready(wake);
        }
        wake.wake();
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

/// Skip absurdly large files: a corrupt or non-font path must not eat memory.
const MAX_FONT_BYTES: u64 = 128 << 20;

/// The loader that registers faces on demand: the first source an installed stack asks
/// ([`super::UiFonts::install`] with [`super::UiFonts::on_demand`]).
///
/// When the engine finds no face for a character, the loader picks the character's script, reads that script's
/// candidates until one covers it, and hands that face to the engine, which appends it to the family (after the
/// stack's own chain). A face is read once and answers every family; a script whose candidates are exhausted is not
/// tried again until the stack is installed again.
///
/// The loader keeps each face it registered (one face per file), to answer the next character it has. Memory is bounded by `MAX_FONT_BYTES` per file and [`MAX_SCRIPT_READS`] files per script,
/// so there is no blob budget to enforce. In a collection the face is chosen by character-map coverage, then by
/// the Han region's family, and a face whose tables are malformed is skipped.
pub struct OnDemandFonts {
    candidates: Arc<dyn FontCandidates>,
    han: Option<Han>,
    /// Scripts tried so far and how many files each has read. A script that ran out of
    /// candidates counts [`MAX_SCRIPT_READS`].
    reads: Vec<(String, usize)>,
    /// Faces registered so far: (font name, file, face index).
    registered: Vec<(String, PathBuf, u32)>,
    /// The registered faces, in the order of `registered`.
    faces: Vec<StackFace>,
    /// The candidates were asked to wake the UI when their running lookup ends.
    waiting: bool,
}

impl OnDemandFonts {
    /// A loader over `candidates`, with the Han preference of the effective locale.
    pub fn new(candidates: Arc<dyn FontCandidates>, han: Option<Han>) -> Self {
        OnDemandFonts { candidates, han, reads: Vec::new(), registered: Vec::new(), faces: Vec::new(), waiting: false }
    }

    /// The loader as the source a stack offered through `host` asks, replacing an earlier one: a new stack starts
    /// with no registered faces.
    pub fn install(self, host: &FontHost) -> Arc<dyn FaceSource> {
        let source = Arc::new(OnDemandSource { fonts: Mutex::new(self), host: host.clone() });
        host.set_on_demand(Arc::clone(&source));
        source
    }

    fn reads_of(&self, script: &str) -> usize {
        self.reads.iter().find(|(s, _)| s == script).map_or(0, |(_, n)| *n)
    }

    fn wanted(&self, script: &str) -> bool {
        self.reads_of(script) < MAX_SCRIPT_READS
    }

    fn count_read(&mut self, script: &str) {
        match self.reads.iter_mut().find(|(s, _)| s == script) {
            Some((_, n)) => *n += 1,
            None => self.reads.push((script.to_owned(), 1)),
        }
    }

    fn set_reads(&mut self, script: &str, reads: usize) {
        match self.reads.iter_mut().find(|(s, _)| s == script) {
            Some((_, n)) => *n = reads,
            None => self.reads.push((script.to_owned(), reads)),
        }
    }

    fn exhaust(&mut self, script: &str) {
        self.set_reads(script, MAX_SCRIPT_READS);
    }

    /// The next face to register for `c`: the first unread candidate of its script whose
    /// character map has it. `None` when nothing is left, or while a background lookup may still
    /// add candidates for the preferred script.
    fn next_face(&mut self, host: &FontHost, c: char, script: &'static str) -> Option<StackFace> {
        let cjk = is_cjk(script);
        let pending = cjk && self.candidates.pending();
        if pending && !self.waiting {
            self.waiting = true;
            // The lookup may add the candidate: the engine forgets this miss and asks again.
            let host = host.clone();
            self.candidates.wake_when_ready(Wake::new(move || host.refresh()));
        } else if !pending {
            self.waiting = false;
        }
        let reads_before = self.reads_of(script);
        let region = match script {
            "Kana" => Some(Han::Japanese),
            "Hang" => Some(Han::Korean),
            "Bopo" => Some(Han::Traditional),
            _ => self.han,
        };
        for path in self.candidates.files(script, self.han) {
            if self.reads_of(script) >= MAX_SCRIPT_READS {
                break;
            }
            if self.registered.iter().any(|(_, p, _)| *p == path) {
                continue;
            }
            let Some(bytes) = read_font(&path) else { continue };
            self.count_read(script);
            if !file_is_readable(&bytes) {
                continue;
            }
            let Some(mut index) = self.candidates.covers(&bytes, c) else { continue };
            if let Some(region) = region.filter(|_| cjk) {
                index = regional_face(&bytes, c, region).unwrap_or(index);
            }
            if !face_is_readable(&bytes, index) {
                continue;
            }
            let name = format!("on-demand-{}", self.registered.len());
            log::info!("UI font fallback: registered {} (face {index}) for {script} as {name}", path.display());
            self.registered.push((name.clone(), path, index));
            // On the primary face's baseline and at regular weight (variable system faces often default to Thin).
            let mut face = baseline_fallback(StackFace::new(name, Arc::new(bytes), index));
            set_weight(&mut face, 400);
            self.faces.push(face.clone());
            return Some(face);
        }
        if pending {
            // The lookup may add a better candidate: try again, reading the same files.
            self.set_reads(script, reads_before);
            return None;
        }
        log::info!("UI font fallback: no installed font covers {script}");
        self.exhaust(script);
        None
    }
}

/// An [`OnDemandFonts`] loader as a stack's source, with the host it has offer the stack again when a running lookup
/// ends.
pub(crate) struct OnDemandSource {
    fonts: Mutex<OnDemandFonts>,
    host: FontHost,
}

impl OnDemandSource {
    pub(crate) fn registered(&self) -> Vec<(String, PathBuf, u32)> {
        self.fonts.lock().unwrap_or_else(PoisonError::into_inner).registered.clone()
    }
}

impl FaceSource for OnDemandSource {
    /// A registered face that has the character, else the next face read for its script; nothing for characters of
    /// scripts the stack's own faces are expected to draw, or whose candidates ran out.
    fn face_for(&self, ask: &FallbackAsk<'_>) -> Option<StackFace> {
        let c = ask.cluster.chars().next()?;
        let script = face_script(c)?;
        let mut fonts = self.fonts.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(face) = fonts.faces.iter().find(|face| has_char(face, c)) {
            return Some(face.clone());
        }
        if !fonts.wanted(script) {
            return None;
        }
        fonts.next_face(&self.host, c, script)
    }
}

/// The ISO 15924 script of a character that may need an on-demand face, or `None` for Latin,
/// Cyrillic, Greek, symbols and everything the stack's own faces are expected to draw. Hiragana
/// is looked up as `Kana`.
fn face_script(c: char) -> Option<&'static str> {
    match script_of(c)? {
        "Hira" => Some("Kana"),
        "Latn" | "Cyrl" | "Grek" | "Syrc" | "Thaa" | "Mong" | EMOJI | SYMBOLS | MATH => None,
        script => Some(script),
    }
}

/// A font file's bytes, or `None` for a missing, empty, oversized or unreadable file.
fn read_font(path: &Path) -> Option<Vec<u8>> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_FONT_BYTES {
        return None;
    }
    std::fs::read(path).ok()
}

/// Whether the text engine can parse the file: it is a font or a collection with a face.
fn file_is_readable(bytes: &[u8]) -> bool {
    match FileRef::new(bytes) {
        Ok(FileRef::Font(_)) => true,
        Ok(FileRef::Collection(collection)) => !collection.is_empty(),
        Err(_) => false,
    }
}

/// Whether face `index` parses and has the tables egui reads; egui panics on a face whose table
/// directory points outside the file, so a face is checked before it is registered.
fn face_is_readable(bytes: &[u8], index: u32) -> bool {
    let Ok(font) = skrifa::FontRef::from_index(bytes, index) else { return false };
    font.head().is_ok() && font.hhea().is_ok() && font.maxp().is_ok() && font.cmap().is_ok()
}

/// The index of the first face in `bytes` (a font or a collection) whose character map has `c`.
/// `None` for fonts without it and for data that doesn't parse.
fn covering_face(bytes: &[u8], c: char) -> Option<u32> {
    match FileRef::new(bytes).ok()? {
        FileRef::Font(font) => font.charmap().map(c).map(|_| 0),
        FileRef::Collection(collection) => {
            collection.iter().position(|font| font.is_ok_and(|font| font.charmap().map(c).is_some())).and_then(|i| u32::try_from(i).ok())
        }
    }
}

/// In a collection of regional faces (Noto Sans CJK JP, SC, TC, HK, KR), the first face that has
/// `c` and is the `region`'s: its family name ends with the region's code. `None` for a single
/// font or a collection without such a face, which leaves the first covering face.
fn regional_face(bytes: &[u8], c: char, region: Han) -> Option<u32> {
    let FileRef::Collection(collection) = FileRef::new(bytes).ok()? else { return None };
    let suffix = match region {
        Han::Simplified => "SC",
        Han::Traditional => "TC",
        Han::HongKong => "HK",
        Han::Japanese => "JP",
        Han::Korean => "KR",
        _ => return None,
    };
    let index = collection.iter().position(|font| {
        font.is_ok_and(|font| {
            font.charmap().map(c).is_some()
                && font
                    .localized_strings(skrifa::string::StringId::FAMILY_NAME)
                    .english_or_first()
                    .is_some_and(|name| name.to_string().ends_with(suffix))
        })
    })?;
    u32::try_from(index).ok()
}

/// The Han preference for the interface: the UI language's when it is Japanese, Korean or Chinese,
/// else the OS locale's, else none (`Hani` falls to the manifest order). PhotoCraft's rule for
/// which regional CJK face wins a shared code point.
pub fn han_for_locale(ui_language: &str, os_locale: Option<&str>) -> Option<Han> {
    craft_i18n::han(ui_language).or_else(|| os_locale.and_then(craft_i18n::han))
}
