//! On-demand font registration (gap 33): a face for a script is read and added to the stack the
//! first time a visible character of that script has no glyph, instead of every candidate face
//! being installed up front. PhotoCraft's lazy loader (`crates/ui-egui/src/text/cjk_fonts.rs`) is
//! the model; it covers CJK and every other script the same way.

use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use craft_i18n::Han;
use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{FontData, FontDefinitions, FontFamily, FontId, FontTweak, Shape};
use skrifa::MetadataProvider as _;
use skrifa::raw::{FileRef, TableProvider as _};

use super::{baseline_fallback, set_weight};

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
}

/// The installed fonts as candidates: fontconfig on Linux, the font directories on macOS and
/// Windows (`craft_font_catalog::font_dirs`). Not on the web.
#[derive(Clone, Debug, Default)]
pub struct SystemCandidates;

impl FontCandidates for SystemCandidates {
    fn files(&self, script: &str, han: Option<Han>) -> Vec<PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            system::files(script, han)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (script, han);
            Vec::new()
        }
    }

    fn covers(&self, data: &[u8], c: char) -> Option<u32> {
        covering_face(data, c)
    }

    fn pending(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            system::pending()
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
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

/// The egui plugin that registers faces on demand. [`UiFonts::on_demand`](super::UiFonts::on_demand)
/// installs one; a host that builds its stack by hand installs it with [`OnDemandFonts::install`].
///
/// Each pass it looks at the shapes drawn for a missing glyph, picks the script of the first such
/// character, reads that script's candidates until one covers it, and registers that face after
/// the craft faces of every family (lowest priority, so it follows the stack's own chain). A face
/// is read once; a script whose candidates are exhausted is not tried again until the fonts are
/// reinstalled.
pub struct OnDemandFonts {
    candidates: Arc<dyn FontCandidates>,
    han: Option<Han>,
    /// Scripts tried so far and how many files each has read. A script that ran out of
    /// candidates counts [`MAX_SCRIPT_READS`].
    reads: Vec<(String, usize)>,
    /// Faces registered so far: (font name, file, face index).
    registered: Vec<(String, PathBuf, u32)>,
    /// The pass after an install registers nothing: egui applies `set_fonts` at the start of the
    /// next pass, so this pass's shapes were laid out with the old stack.
    skip_pass: bool,
    /// A face was registered by the last pass; egui adds it at the start of the next one.
    adding: bool,
}

impl OnDemandFonts {
    /// A loader over `candidates`, with the Han preference of the effective locale.
    pub fn new(candidates: Arc<dyn FontCandidates>, han: Option<Han>) -> Self {
        OnDemandFonts { candidates, han, reads: Vec::new(), registered: Vec::new(), skip_pass: false, adding: false }
    }

    /// Install the loader as an egui plugin on `ctx`. Replaces an earlier one.
    pub fn install(mut self, ctx: &egui::Context) {
        // egui keeps the first plugin of a type, and `set_fonts` drops the registered faces, so
        // an earlier loader's state is replaced rather than kept.
        self.skip_pass = true;
        let mut new = Some(self);
        ctx.with_plugin::<OnDemandFonts, _>(|plugin| {
            if let Some(new) = new.take() {
                *plugin = new;
            }
        });
        if let Some(new) = new {
            ctx.add_plugin(new);
        }
    }

    /// The faces registered so far, for the font list in Preferences and the tests.
    pub fn registered(&self) -> &[(String, PathBuf, u32)] {
        &self.registered
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

    /// The first character with no glyph among `shapes`, if any: what the next pass loads for.
    /// Only characters of scripts that still have candidates to read count.
    fn missing_char(&self, ctx: &egui::Context, shapes: &[egui::epaint::ClippedShape]) -> Option<char> {
        fn collect(shape: &Shape, out: &mut Vec<char>, wanted: &dyn Fn(char) -> bool) {
            match shape {
                Shape::Text(t) => {
                    let text = &t.galley.job.text;
                    if !text.is_ascii() {
                        for c in text.chars().filter(|c| wanted(*c)) {
                            if out.len() >= 256 {
                                return;
                            }
                            if !out.contains(&c) {
                                out.push(c);
                            }
                        }
                    }
                }
                Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out, wanted)),
                _ => {}
            }
        }
        let wanted = |c: char| script_of(c).is_some_and(|script| self.wanted(script));
        let mut chars = Vec::new();
        for shape in shapes {
            collect(&shape.shape, &mut chars, &wanted);
        }
        let font = FontId::proportional(12.0);
        ctx.fonts_mut(|fonts| chars.into_iter().find(|c| !fonts.has_glyph(&font, *c)))
    }

    /// Whether the stack was reinstalled since the last face was registered: egui's definitions
    /// no longer hold it.
    fn faces_dropped(&self, ctx: &egui::Context) -> bool {
        let Some((name, ..)) = self.registered.first() else { return false };
        !self.adding && !ctx.fonts(|f| f.definitions().font_data.contains_key(name))
    }

    /// The next face to register for `c`: the first unread candidate of its script whose
    /// character map has it. `None` when nothing is left, or while a background lookup may still
    /// add candidates for the preferred script.
    fn next_face(&mut self, c: char, script: &'static str) -> Option<(String, FontData)> {
        let cjk = is_cjk(script);
        let pending = cjk && self.candidates.pending();
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
            let mut data = FontData::from_owned(bytes);
            data.index = index;
            return Some((name, data));
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

impl egui::Plugin for OnDemandFonts {
    fn debug_name(&self) -> &'static str {
        "craft-ui-on-demand-fonts"
    }

    fn output_hook(&mut self, ctx: &egui::Context, output: &mut egui::FullOutput) {
        if std::mem::take(&mut self.skip_pass) {
            ctx.request_repaint();
            return;
        }
        if self.faces_dropped(ctx) {
            *self = OnDemandFonts::new(self.candidates.clone(), self.han);
            ctx.request_repaint();
            return;
        }
        self.adding = false;
        let Some(c) = self.missing_char(ctx, &output.shapes) else { return };
        let Some(script) = script_of(c) else { return };
        let pending = is_cjk(script) && self.candidates.pending();
        match self.next_face(c, script) {
            Some((name, data)) => {
                register(ctx, name, data);
                self.adding = true;
                ctx.request_repaint();
            }
            None if pending => ctx.request_repaint_after(std::time::Duration::from_millis(50)),
            // The script ran out of candidates for this character; another script may still load.
            None => ctx.request_repaint(),
        }
    }
}

/// Add `data` to every font family at the lowest priority, on the primary face's baseline and at
/// regular weight (variable system faces often default to Thin).
fn register(ctx: &egui::Context, name: String, data: FontData) {
    let mut data = baseline_fallback(data);
    set_weight(&mut data, 400);
    let mut families: Vec<FontFamily> = ctx.fonts(|f| f.definitions().families.keys().cloned().collect());
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        if !families.contains(&family) {
            families.push(family);
        }
    }
    let families = families.into_iter().map(|family| InsertFontFamily { family, priority: FontPriority::Lowest }).collect();
    ctx.add_font(FontInsert { name, data, families });
}

fn is_cjk(script: &str) -> bool {
    matches!(script, "Hani" | "Kana" | "Hang" | "Bopo")
}

/// The ISO 15924 script of a character that may need an on-demand face, or `None` for Latin,
/// Cyrillic, Greek, symbols and everything the stack's own faces are expected to draw.
fn script_of(c: char) -> Option<&'static str> {
    Some(match u32::from(c) {
        0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FF | 0xFFA0..=0xFFDC => "Hang",
        0x3040..=0x30FF | 0x31F0..=0x31FF | 0xFF65..=0xFF9F | 0x1B000..=0x1B16F => "Kana",
        0x3100..=0x312F | 0x31A0..=0x31BF => "Bopo",
        0x2E80..=0x2FDF
        | 0x3000..=0x303F
        | 0x3190..=0x319F
        | 0x31C0..=0x31EF
        | 0x3200..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF64
        | 0xFFE0..=0xFFEF
        | 0x20000..=0x3FFFF => "Hani",
        0x0E00..=0x0E7F => "Thai",
        0x0E80..=0x0EFF => "Laoo",
        0x1780..=0x17FF | 0x19E0..=0x19FF => "Khmr",
        0x1000..=0x109F | 0xA9E0..=0xA9FF | 0xAA60..=0xAA7F => "Mymr",
        0x0F00..=0x0FFF => "Tibt",
        0x0600..=0x06FF | 0x0750..=0x077F | 0x0870..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => "Arab",
        0x0590..=0x05FF | 0xFB1D..=0xFB4F => "Hebr",
        0x0530..=0x058F | 0xFB13..=0xFB17 => "Armn",
        0x10A0..=0x10FF | 0x1C90..=0x1CBF | 0x2D00..=0x2D2F => "Geor",
        0x1200..=0x139F | 0x2D80..=0x2DDF | 0xAB00..=0xAB2F => "Ethi",
        0x0900..=0x097F | 0xA8E0..=0xA8FF => "Deva",
        0x0980..=0x09FF => "Beng",
        0x0A00..=0x0A7F => "Guru",
        0x0A80..=0x0AFF => "Gujr",
        0x0B00..=0x0B7F => "Orya",
        0x0B80..=0x0BFF => "Taml",
        0x0C00..=0x0C7F => "Telu",
        0x0C80..=0x0CFF => "Knda",
        0x0D00..=0x0D7F => "Mlym",
        0x0D80..=0x0DFF => "Sinh",
        _ => return None,
    })
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

/// A font deflated by build.rs.
macro_rules! deflated {
    ($name:literal) => {
        include_bytes!(concat!(env!("OUT_DIR"), "/egui_fonts/", $name, ".ttf.deflate"))
    };
}

static HACK: LazyLock<Vec<u8>> = LazyLock::new(|| inflate(deflated!("Hack-Regular")));
static EGUI_ICONS: LazyLock<Vec<u8>> = LazyLock::new(|| inflate(deflated!("egui-icons")));
static UBUNTU_LIGHT: LazyLock<Vec<u8>> = LazyLock::new(|| inflate(deflated!("Ubuntu-Light")));

/// Inflated fonts are bounded (the largest is 0.42 MB), so corrupt data can't exhaust memory.
const MAX_INFLATED_BYTES: u64 = 16 << 20;

/// Inflate a font. Data that doesn't inflate gives empty bytes (the font is left out), never a panic.
fn inflate(deflated: &[u8]) -> Vec<u8> {
    use std::io::Read as _;
    let mut out = Vec::new();
    match flate2::read::DeflateDecoder::new(deflated).take(MAX_INFLATED_BYTES + 1).read_to_end(&mut out) {
        Ok(_) if out.len() as u64 <= MAX_INFLATED_BYTES => out,
        _ => Vec::new(),
    }
}

/// egui's default font definitions built from deflated copies of its faces, for builds without
/// `default_fonts` (the web size gate): the same faces (without egui's optional emoji fonts), tweaks and family order. The deflated
/// copies are made by this crate's build script from `epaint_default_fonts`.
pub fn deflated_default_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    let faces: [(&str, &'static [u8], f32); 3] = [
        ("Hack", HACK.as_slice(), 1.0),
        ("Ubuntu-Light", UBUNTU_LIGHT.as_slice(), 1.0),
        // The handful of icons in `egui::special_emojis`, which no platform font has.
        ("egui-icons", EGUI_ICONS.as_slice(), 0.90),
    ];
    for (name, bytes, scale) in faces {
        if bytes.is_empty() {
            log::error!("egui default font {name} is unavailable");
            continue;
        }
        let data = FontData::from_static(bytes).tweak(FontTweak { scale, ..Default::default() });
        fonts.font_data.insert(name.to_owned(), Arc::new(data));
    }
    let stack = |names: &[&str]| -> Vec<String> { names.iter().filter(|n| fonts.font_data.contains_key(**n)).map(|n| (*n).to_owned()).collect() };
    // egui 0.37's family order: egui-icons last, as a fallback.
    let monospace = stack(&["Hack", "Ubuntu-Light", "egui-icons"]);
    let proportional = stack(&["Ubuntu-Light", "egui-icons"]);
    fonts.families.insert(FontFamily::Monospace, monospace);
    fonts.families.insert(FontFamily::Proportional, proportional);
    fonts
}

/// The installed fonts: the font folders' files by name, the CJK lists per platform, and
/// Linux's fontconfig lookup for the CJK scripts.
#[cfg(not(target_arch = "wasm32"))]
mod system {
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use craft_font_catalog::{FontEnv, font_dirs, is_font_file};
    use craft_i18n::Han;

    use super::{covering_face, is_cjk};

    /// At most this many name matches per script.
    const MAX_CANDIDATES: usize = 16;
    /// Folders nested deeper than this are not listed.
    const MAX_DEPTH: usize = 8;

    /// A CJK writing system with its own installed fonts.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Cjk {
        Japanese,
        Simplified,
        Traditional,
        Korean,
    }

    /// The order in which the regional faces are tried: the preferred region first.
    fn order(han: Option<Han>) -> [Cjk; 4] {
        use Cjk::*;
        match han {
            Some(Han::Japanese) => [Japanese, Simplified, Traditional, Korean],
            Some(Han::Korean) => [Korean, Traditional, Simplified, Japanese],
            Some(Han::Traditional | Han::HongKong) => [Traditional, Simplified, Japanese, Korean],
            _ => [Simplified, Traditional, Japanese, Korean],
        }
    }

    pub(super) fn files(script: &str, han: Option<Han>) -> Vec<PathBuf> {
        if !is_cjk(script) {
            return candidates(script, listing());
        }
        let order = order(han);
        let first = match script {
            "Kana" => Cjk::Japanese,
            "Hang" => Cjk::Korean,
            "Bopo" => Cjk::Traditional,
            _ => order[0],
        };
        let mut files: Vec<PathBuf> = Vec::new();
        for region in std::iter::once(first).chain(order) {
            for path in cjk_files(region) {
                if path.is_file() && !files.contains(&path) {
                    files.push(path);
                }
            }
        }
        for path in last_resort_files() {
            if path.is_file() && !files.contains(&path) {
                files.push(path);
            }
        }
        files
    }

    pub(super) fn pending() -> bool {
        #[cfg(target_os = "linux")]
        {
            fontconfig::start();
            fontconfig::pending()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    /// System CJK font files for `region`, most preferred first. Paths that don't exist are
    /// included; callers skip them.
    fn cjk_files(region: Cjk) -> Vec<PathBuf> {
        use Cjk::*;
        let p = |path: &str| PathBuf::from(path);
        if cfg!(target_os = "macos") {
            let pingfang = mac_pingfang();
            match region {
                Japanese => vec![p("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"), p("/System/Library/Fonts/ヒラギノ角ゴシック W4.ttc")],
                Simplified => pingfang
                    .into_iter()
                    .chain([p("/System/Library/Fonts/Hiragino Sans GB.ttc"), p("/System/Library/Fonts/STHeiti Light.ttc")])
                    .collect(),
                Traditional => pingfang.into_iter().chain([p("/System/Library/Fonts/STHeiti Light.ttc")]).collect(),
                Korean => vec![p("/System/Library/Fonts/AppleSDGothicNeo.ttc")],
            }
        } else if cfg!(target_os = "windows") {
            let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
            let w = |file: &str| Path::new(&dir).join("Fonts").join(file);
            match region {
                Japanese => vec![w("YuGothR.ttc"), w("YuGothM.ttc"), w("meiryo.ttc"), w("msgothic.ttc")],
                Simplified => vec![w("msyh.ttc"), w("msyh.ttf"), w("simsun.ttc")],
                Traditional => vec![w("msjh.ttc"), w("msjh.ttf"), w("mingliu.ttc")],
                Korean => vec![w("malgun.ttf"), w("gulim.ttc")],
            }
        } else {
            // One Noto Sans CJK collection covers all four regions; the loader picks the face.
            let mut files: Vec<PathBuf> = [
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/local/share/fonts/noto/NotoSansCJK-Regular.ttc",
                "/usr/local/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                "/usr/local/share/fonts/noto/NotoSansCJK-VF.ttc",
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/google-noto-sans-cjk-fonts/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/google-noto-sans-cjk-vf-fonts/NotoSansCJK-VF.ttc",
                "/usr/share/fonts/noto-cjk/NotoSansCJK-VF.ttc",
            ]
            .map(p)
            .to_vec();
            files.extend(
                match region {
                    Japanese => &[
                        "/usr/share/fonts/opentype/ipafont-gothic/ipag.ttf",
                        "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf",
                        "/usr/share/fonts/truetype/takao-gothic/TakaoPGothic.ttf",
                    ][..],
                    Simplified | Traditional => &[
                        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
                        "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
                        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
                        "/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc",
                    ][..],
                    Korean => &[
                        "/usr/share/fonts/truetype/nanum/NanumGothic.ttf",
                        "/usr/share/fonts/nanum/NanumGothic.ttf",
                        "/usr/share/fonts/truetype/unfonts-core/UnDotum.ttf",
                        "/usr/share/fonts/un-core/UnDotum.ttf",
                    ][..],
                }
                .iter()
                .map(|f| p(f)),
            );
            // Flatpak mounts the host's font folder outside the runtime's.
            let mut with_host = Vec::new();
            for file in files {
                let host = file.strip_prefix("/usr/share/fonts").ok().map(|relative| Path::new("/run/host/fonts").join(relative));
                with_host.push(file);
                with_host.extend(host);
            }
            #[cfg(target_os = "linux")]
            if let Some(font) = fontconfig::result(region)
                && !with_host.contains(&font)
            {
                with_host.push(font);
            }
            with_host
        }
    }

    /// Broad-coverage fonts tried after every region's own: they cover Han, kana and Hangul but
    /// look worse than the regional fonts.
    fn last_resort_files() -> Vec<PathBuf> {
        let p = |path: &str| PathBuf::from(path);
        if cfg!(target_os = "macos") {
            vec![p("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"), p("/Library/Fonts/Arial Unicode.ttf")]
        } else if cfg!(target_os = "windows") {
            let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
            vec![Path::new(&dir).join("Fonts").join("ARIALUNI.TTF")]
        } else {
            vec![
                p("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf"),
                p("/usr/share/fonts/google-droid-sans-fonts/DroidSansFallbackFull.ttf"),
                p("/run/host/fonts/truetype/droid/DroidSansFallbackFull.ttf"),
                p("/run/host/fonts/google-droid-sans-fonts/DroidSansFallbackFull.ttf"),
            ]
        }
    }

    /// macOS ships PingFang as a pre-installed asset outside `/System/Library/Fonts`
    /// (`/System/Library/AssetsV2/com_apple_MobileAsset_Font*/<hash>.asset/AssetData/PingFang.ttc`).
    fn mac_pingfang() -> Option<PathBuf> {
        let old = PathBuf::from("/System/Library/Fonts/PingFang.ttc");
        if old.is_file() {
            return Some(old);
        }
        let roots = [Path::new("/System/Library/AssetsV2"), Path::new("/System/Library/AssetsV2/PreinstalledAssetsV2/InstallWithOs")];
        for root in roots {
            let Ok(entries) = std::fs::read_dir(root) else { continue };
            for entry in entries.flatten().take(512) {
                if !entry.file_name().to_string_lossy().starts_with("com_apple_MobileAsset_Font") {
                    continue;
                }
                let Ok(assets) = std::fs::read_dir(entry.path()) else { continue };
                for asset in assets.flatten().take(64) {
                    let path = asset.path().join("AssetData/PingFang.ttc");
                    if path.is_file() {
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    /// Every font file in the platform's font folders, listed on first use and kept for the
    /// process (names only; nothing is opened).
    fn listing() -> &'static [PathBuf] {
        static FILES: OnceLock<Vec<PathBuf>> = OnceLock::new();
        FILES.get_or_init(|| {
            let mut files = Vec::new();
            for dir in font_dirs(&FontEnv::from_process()) {
                collect(&dir, 0, &mut files);
            }
            files.sort();
            files.dedup();
            files
        })
    }

    fn collect(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth > MAX_DEPTH {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(&path, depth + 1, out);
            } else if is_font_file(&path) {
                out.push(path);
            }
        }
    }

    /// Font file names for a non-CJK script, most preferred first: lowercase, without extension,
    /// spaces, underscores or style suffix (`NotoSansThai-Bold.ttf` and `NotoSansThai[wght].ttf`
    /// are both `notosansthai`). Noto first (the usual Linux font), then Windows, macOS and other
    /// Linux families.
    fn file_names(script: &str) -> &'static [&'static str] {
        match script {
            "Thai" => &[
                "notosansthai",
                "notosansthailooped",
                "notosansthaiui",
                "notoserifthai",
                "leelawui",
                "leelawad",
                "tahoma",
                "thonburi",
                "sukhumvitset",
                "ayuthaya",
                "loma",
                "garuda",
                "waree",
                "kinnari",
                "norasi",
            ],
            "Laoo" => &["notosanslao", "notosanslaoui", "notoseriflao", "leelawui", "laosangammn", "laomn", "phetsarathot", "saysetthaot"],
            "Khmr" => {
                &["notosanskhmer", "notosanskhmerui", "notoserifkhmer", "leelawui", "daunpenh", "khmersangammn", "khmermn", "khmeros", "khmerossys"]
            }
            "Mymr" => &["notosansmyanmar", "notosansmyanmarui", "notoserifmyanmar", "mmrtext", "myanmarsangammn", "myanmarmn", "padauk"],
            "Tibt" => &["notoseriftibetan", "himalaya", "kailasa", "jomolhari", "tibetanmachineuni"],
            "Arab" => &[
                "notosansarabic",
                "notosansarabicui",
                "notonaskharabic",
                "notonaskharabicui",
                "notokufiarabic",
                "segoeui",
                "tahoma",
                "geezapro",
                "sfarabic",
                "dejavusans",
                "kacstone",
                "kacstbook",
                "amiri",
                "arial",
            ],
            "Hebr" => &["notosanshebrew", "notoserifhebrew", "segoeui", "arialhb", "sfhebrew", "dejavusans", "arial", "freesans", "culmus"],
            "Armn" => &["notosansarmenian", "notoserifarmenian", "segoeui", "sylfaen", "mshtakan", "dejavusans", "freesans"],
            "Geor" => &["notosansgeorgian", "notoserifgeorgian", "segoeui", "sylfaen", "dejavusans", "freesans"],
            "Ethi" => &["notosansethiopic", "notoserifethiopic", "ebrima", "kefa", "abyssinicasil"],
            "Deva" => &[
                "notosansdevanagari",
                "notosansdevanagariui",
                "notoserifdevanagari",
                "nirmala",
                "mangal",
                "kohinoor",
                "kohinoordevanagari",
                "itfdevanagari",
                "devanagarimt",
                "devanagarisangammn",
                "lohitdevanagari",
                "lohit-devanagari",
                "gargi",
                "freesans",
            ],
            "Beng" => &[
                "notosansbengali",
                "notosansbengaliui",
                "notoserifbengali",
                "nirmala",
                "vrinda",
                "kohinoorbangla",
                "banglasangammn",
                "banglamn",
                "lohitbengali",
                "lohit-bengali",
                "lohit-assamese",
            ],
            "Guru" => &[
                "notosansgurmukhi",
                "notosansgurmukhiui",
                "notoserifgurmukhi",
                "nirmala",
                "raavi",
                "gurmukhisangammn",
                "gurmukhimn",
                "lohit-gurmukhi",
                "lohit-punjabi",
            ],
            "Gujr" => &[
                "notosansgujarati",
                "notosansgujaratiui",
                "notoserifgujarati",
                "nirmala",
                "shruti",
                "kohinoorgujarati",
                "gujaratisangammn",
                "gujaratimt",
                "lohit-gujarati",
            ],
            "Orya" => {
                &["notosansoriya", "notosansoriyaui", "notoseriforiya", "nirmala", "kalinga", "oriyasangammn", "oriyamn", "lohit-odia", "lohit-oriya"]
            }
            "Taml" => &["notosanstamil", "notosanstamilui", "notoseriftamil", "nirmala", "latha", "tamilsangammn", "tamilmn", "lohit-tamil"],
            "Telu" => &[
                "notosanstelugu",
                "notosansteluguui",
                "notoseriftelugu",
                "nirmala",
                "gautami",
                "kohinoortelugu",
                "telugusangammn",
                "telugumn",
                "lohit-telugu",
            ],
            "Knda" => {
                &["notosanskannada", "notosanskannadaui", "notoserifkannada", "nirmala", "tunga", "kannadasangammn", "kannadamn", "lohit-kannada"]
            }
            "Mlym" => &[
                "notosansmalayalam",
                "notosansmalayalamui",
                "notoserifmalayalam",
                "nirmala",
                "kartika",
                "malayalamsangammn",
                "malayalammn",
                "lohit-malayalam",
            ],
            "Sinh" => &["notosanssinhala", "notosanssinhalaui", "notoserifsinhala", "nirmala", "iskpota", "sinhalasangammn", "sinhalamn", "lklug"],
            _ => &[],
        }
    }

    /// `(name, stem, style rank)` of a font file: [`file_names`] form, the file stem, and 0 for the
    /// regular face, 1 for a variable font, 2 for other weights, 3 for bold, italic and condensed
    /// faces. Non-font files give `None`.
    fn file_key(path: &Path) -> Option<(String, &str, u8)> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        if !matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc") {
            return None;
        }
        let stem = path.file_stem()?.to_str()?;
        let squash = |s: &str| s.chars().filter(|c| !matches!(c, ' ' | '_')).collect::<String>().to_lowercase();
        let style = |s: &str| {
            let s = s.to_ascii_lowercase();
            if s.is_empty() || s == "-regular" {
                0
            } else if s.starts_with('[') {
                1
            } else if ["bold", "italic", "oblique", "condensed", "narrow", "black", "heavy", "thin", "light"].iter().any(|w| s.contains(w)) {
                3
            } else {
                2
            }
        };
        let cut = stem.find(['-', '[']).unwrap_or(stem.len());
        let (base, rest) = stem.split_at(cut);
        Some((squash(base), stem, style(rest)))
    }

    /// Font files among `files` that may cover `script`, most preferred first. Pure, so tests
    /// pass their own listing.
    pub(super) fn candidates(script: &str, files: &[PathBuf]) -> Vec<PathBuf> {
        let names = file_names(script);
        let mut ranked: Vec<(usize, u8, &PathBuf)> = files
            .iter()
            .filter_map(|path| {
                let (base, stem, style) = file_key(path)?;
                // A name with a hyphen ("lohit-tamil") matches the whole stem.
                let full = stem.to_lowercase().replace([' ', '_'], "");
                let rank = names.iter().position(|n| *n == base || *n == full)?;
                Some((rank, if names.get(rank).is_some_and(|n| *n == full) { 0 } else { style }, path))
            })
            .collect();
        ranked.sort();
        ranked.into_iter().take(MAX_CANDIDATES).map(|(.., path)| path.clone()).collect()
    }

    /// Linux's fontconfig choice per CJK region, found by `fc-match` on a background thread.
    #[cfg(target_os = "linux")]
    mod fontconfig {
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Mutex, OnceLock, PoisonError};

        use super::{Cjk, MAX_FONT_BYTES_FC, covering_face};

        struct Scan {
            started: AtomicBool,
            result: Mutex<Option<[Option<PathBuf>; 4]>>,
        }

        fn scan() -> &'static Scan {
            static SCAN: OnceLock<Scan> = OnceLock::new();
            SCAN.get_or_init(|| Scan { started: AtomicBool::new(false), result: Mutex::new(None) })
        }

        const REGIONS: [Cjk; 4] = [Cjk::Japanese, Cjk::Simplified, Cjk::Traditional, Cjk::Korean];

        /// Start the lookup; later calls do nothing. Without `fc-match`, the built-in paths stand.
        pub(super) fn start() {
            let scan = scan();
            if scan.started.swap(true, Ordering::Relaxed) {
                return;
            }
            let spawned = std::thread::Builder::new().name("craft-ui-fontconfig-cjk".into()).spawn(move || {
                let result = REGIONS.map(fc_match);
                *scan.result.lock().unwrap_or_else(PoisonError::into_inner) = Some(result);
            });
            if spawned.is_err() {
                *scan.result.lock().unwrap_or_else(PoisonError::into_inner) = Some([None, None, None, None]);
            }
        }

        /// Whether the lookup has started and is still running.
        pub(super) fn pending() -> bool {
            let scan = scan();
            scan.started.load(Ordering::Relaxed) && scan.result.lock().unwrap_or_else(PoisonError::into_inner).is_none()
        }

        pub(super) fn result(region: Cjk) -> Option<PathBuf> {
            let scan = scan();
            if !scan.started.load(Ordering::Relaxed) {
                return None;
            }
            let index = REGIONS.iter().position(|r| *r == region)?;
            scan.result.lock().unwrap_or_else(PoisonError::into_inner).as_ref()?.get(index)?.clone()
        }

        /// The font `fc-match :lang=<region>` names, if it is a font file with every character of
        /// the region's sample.
        fn fc_match(region: Cjk) -> Option<PathBuf> {
            let (language, sample) = match region {
                Cjk::Japanese => ("ja", "レイヤー図"),
                Cjk::Simplified => ("zh-cn", "图层"),
                Cjk::Traditional => ("zh-tw", "圖層"),
                Cjk::Korean => ("ko", "카드 배경"),
            };
            let output = std::process::Command::new("fc-match").args(["--format=%{file}\n", &format!(":lang={language}")]).output().ok()?;
            if !output.status.success() {
                return None;
            }
            let line = String::from_utf8(output.stdout).ok()?;
            let path = PathBuf::from(line.lines().find(|l| !l.is_empty())?);
            let ext = path.extension()?.to_str()?.to_ascii_lowercase();
            if !path.is_absolute() || !matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc") {
                return None;
            }
            let meta = std::fs::metadata(&path).ok()?;
            if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_FONT_BYTES_FC {
                return None;
            }
            let bytes = std::fs::read(&path).ok()?;
            sample.chars().all(|c| covering_face(&bytes, c).is_some()).then(|| path.canonicalize().unwrap_or(path))
        }
    }

    #[cfg(target_os = "linux")]
    const MAX_FONT_BYTES_FC: u64 = super::MAX_FONT_BYTES;
}
