//! Served faces: ordered like embedded ones, fetched through a `FontFetcher` when the UI needs them.

use std::sync::{Arc, Mutex};

use craft_fonts_build::{EmbeddedFace, ServedFace};
use craft_ui::fonts::{Emoji, Family, FetchDone, FetchWake, FontFetcher, FontRef, I18nScripts, Stack, SystemFonts, UiFonts};
use craft_ui::testing::trial;
use egui::epaint::text::{Blob, Fonts, TextOptions};
use egui::{FontDefinitions, FontFamily, FontId};

const ARABIC: &[u8] = include_bytes!("../../fonts/NotoSansArabic-test.ttf");
/// Noto Sans with Vietnamese ổ but not ẫ.
const NOTO_SANS_LATIN: &[u8] = include_bytes!("../../fonts/NotoSans-test.ttf");
/// Noto Sans Symbols' ⌃ ⎋ and Noto Sans Symbols 2's ⌥ ⌫ ⏎: the keyboard keys are split between them.
const SYMBOLS: &[u8] = include_bytes!("../../fonts/NotoSansSymbols-test.ttf");
const SYMBOLS_2: &[u8] = include_bytes!("../../fonts/NotoSansSymbols2-test.ttf");
const CJK: &[u8] = include_bytes!("../../fonts/NotoSansCJKsc-test.otf");

const OFFERED: &[&str] = &["en", "ar", "ja", "zh-CN", "zh-TW", "zh-HK", "ko", "th", "ka", "ru"];

/// Coverage of every character, for faces whose tests are about scripts.
const ALL: &[u8] = &[0, 0xFF, 0xFF, 0x43];

const fn served(family: &'static str, style: &'static str, scripts: &'static [&'static str], url: &'static str, index: u32) -> ServedFace {
    ServedFace { family, style, scripts, index, url, size: 1, integrity: "", coverage: ALL }
}

fn without_bytes(face: &ServedFace) -> EmbeddedFace {
    EmbeddedFace { family: face.family, style: face.style, scripts: face.scripts, index: face.index, bytes: &[] }
}

fn coverage_of(font: &[u8]) -> &'static [u8] {
    Box::leak(craft_fonts_build::coverage::of_face(font, 0).expect("a font").into_boxed_slice())
}

fn coverage_of_chars(chars: &str) -> &'static [u8] {
    let mut ranges: Vec<(u32, u32)> = chars.chars().map(|c| (u32::from(c), u32::from(c))).collect();
    ranges.sort_unstable();
    Box::leak(craft_fonts_build::coverage::encode(&ranges).into_boxed_slice())
}

/// craft-fonts' faces as a web build serves them; the CJK collection's faces share one file.
const SERVED: &[ServedFace] = &[
    served("BIZ UDPGothic", "Regular", &["Jpan", "Latn"], "fonts/biz-ud-pgothic/BIZUDPGothic-Regular.ttf", 0),
    served("BIZ UDPGothic", "Bold", &["Jpan", "Latn"], "fonts/biz-ud-pgothic/BIZUDPGothic-Bold.ttf", 0),
    served("Noto Sans CJK SC", "Regular", &["Hans", "Latn"], "fonts/noto-sans-cjk/NotoSansCJK.ttc", 2),
    served("Noto Sans CJK TC", "Regular", &["Hant", "Latn"], "fonts/noto-sans-cjk/NotoSansCJK.ttc", 3),
    served("Noto Sans Arabic", "Regular", &["Arab"], "fonts/noto-sans-arabic/NotoSansArabic.ttf", 0),
    served("Noto Sans Thai", "Regular", &["Thai"], "fonts/noto-sans-thai/NotoSansThai.ttf", 0),
];

fn egui_face(name: &str) -> egui::FontData {
    (**FontDefinitions::default().font_data.get(name).expect("egui default_fonts in dev-dependencies")).clone()
}

fn web_fonts(ui_language: &str, served: &[ServedFace]) -> UiFonts<'static> {
    UiFonts::new(&I18nScripts)
        .app_face("ui", egui_face("Ubuntu-Light"), &[FontRef::proportional()])
        .app_face("mono", egui_face("Hack"), &[FontRef::monospace()])
        .family(Family::named("semibold").weight(600))
        .served_faces(served)
        .languages(OFFERED)
        .ui_language(ui_language)
        .system_fonts(SystemFonts::Off)
}

fn names(faces: Vec<&ServedFace>) -> Vec<String> {
    faces.into_iter().map(ServedFace::name).collect()
}

/// A font set over the stack's installed faces, for asking which characters it draws.
fn measuring(stack: &Stack) -> Fonts {
    Fonts::new(TextOptions::default(), stack.flat_definitions())
}

#[trial]
fn served_faces_take_the_places_of_embedded_ones_without_data() {
    let embedded: Vec<EmbeddedFace> = SERVED.iter().map(without_bytes).collect();
    for language in ["en", "ar", "ja", "zh-TW", "th"] {
        let web = web_fonts(language, SERVED).stack();
        let desktop = web_fonts(language, &[]).craft_faces(&embedded).stack();
        for family in [FontRef::proportional(), FontRef::monospace(), FontRef::named("semibold")] {
            assert_eq!(web.chain(&family), desktop.chain(&family), "{language} {family:?}");
        }
        let installed = web.flat_definitions().font_data;
        assert!(SERVED.iter().all(|face| !installed.contains_key(&face.name())), "{language}");
        assert_eq!(web.served().len(), SERVED.len());
    }
    // Served faces count before they are fetched: they are fetched when text needs them.
    let stack = web_fonts("en", SERVED).stack();
    assert!(stack.covers_language("ar", false) && stack.covers_language("ja", false));
    assert!(!stack.covers_language("km", false));
}

#[trial]
fn the_ui_language_downloads_its_own_faces_first() {
    let cases: &[(&str, &[&str])] = &[
        ("en", &[]),
        ("ru", &[]),
        ("ar", &["Noto Sans Arabic Regular"]),
        // The semibold family takes the bold face.
        ("ja", &["BIZ UDPGothic Regular", "BIZ UDPGothic Bold"]),
        ("zh-TW", &["Noto Sans CJK TC Regular"]),
        ("zh-CN", &["Noto Sans CJK SC Regular"]),
        ("th", &["Noto Sans Thai Regular"]),
    ];
    for (language, want) in cases {
        assert_eq!(names(web_fonts(language, SERVED).stack().ui_downloads()), *want, "{language}");
    }
}

#[trial]
fn a_missing_character_downloads_the_face_for_its_script() {
    let stack = web_fonts("en", SERVED).stack();
    let regular = FontRef::proportional();
    assert_eq!(names(stack.downloads_for(&regular, 'ع')), ["Noto Sans Arabic Regular"]);
    // Han follows the offered languages: Japanese before Chinese.
    assert_eq!(names(stack.downloads_for(&regular, '日'))[..2], ["BIZ UDPGothic Regular", "Noto Sans CJK SC Regular"]);
    assert_eq!(names(stack.downloads_for(&FontRef::named("semibold"), 'あ'))[0], "BIZ UDPGothic Bold");
    // The CJK faces' Latin doesn't stand in for a Latin face.
    for c in ['a', 'ж', 'λ', '1', 'ổ', '🔍', '⌘'] {
        assert!(stack.downloads_for(&regular, c).is_empty(), "{c}");
    }
    let twemoji = served("Twemoji", "Regular", &[], "fonts/Twemoji.ttf", 0);
    let stack = web_fonts("en", SERVED).emoji(Emoji { served: Some(twemoji), ..Emoji::default() }).stack();
    for c in ['🔍', '⌚', '✅', '⭐'] {
        assert_eq!(names(stack.downloads_for(&regular, c)), ["Twemoji Regular"], "{c}");
    }
    assert!(stack.downloads_for(&regular, '⌘').is_empty(), "a symbol, not an emoji");
    assert!(stack.ui_downloads().is_empty());
}

#[trial]
fn a_fetched_file_reaches_every_face_and_family_that_uses_it() {
    let mut stack = web_fonts("en", SERVED).stack();
    let font_id = |family: FontFamily| FontId::new(14.0, family);
    let semibold = FontRef::named("semibold");
    assert!(!measuring(&stack).has_glyph(&font_id(FontFamily::Proportional), 'ع'));

    // The Arabic test face is variable, so the semibold family gets its own instance at 600.
    let file: Blob = Arc::new(ARABIC.to_vec());
    stack.provide("fonts/noto-sans-arabic/NotoSansArabic.ttf", &file);
    let mut fonts = measuring(&stack);
    assert!(fonts.has_glyph(&font_id(FontFamily::Proportional), 'ع'));
    assert!(fonts.has_glyph(&font_id(FontFamily::Name("semibold".into())), 'ع'));
    assert!(stack.chain(&FontRef::proportional()).contains(&"Noto Sans Arabic Regular".to_owned()));
    assert!(stack.chain(&semibold).contains(&"Noto Sans Arabic Regular @600".to_owned()));
    assert!(!stack.flat_definitions().font_data.contains_key("Noto Sans Thai Regular"));

    // Both faces of a collection come from one file.
    let file: Blob = Arc::new(CJK.to_vec());
    stack.provide("fonts/noto-sans-cjk/NotoSansCJK.ttc", &file);
    let installed = stack.flat_definitions().font_data;
    for name in ["Noto Sans CJK SC Regular", "Noto Sans CJK TC Regular"] {
        assert!(installed.contains_key(name), "{name}");
    }
}

#[trial]
fn fetched_cjk_faces_fit_the_latin_rows_as_embedded_ones_do() {
    const SC: &[ServedFace] = &[served("Noto Sans CJK SC", "Regular", &["Hans", "Latn"], "fonts/sc.otf", 0)];
    let mut web = web_fonts("zh-CN", SC).stack();
    web.provide("fonts/sc.otf", &(Arc::new(CJK.to_vec()) as Blob));
    let embedded = EmbeddedFace { bytes: CJK, ..without_bytes(&SC[0]) };
    let desktop = web_fonts("zh-CN", &[]).craft_faces(&[embedded]).stack();
    let tweak = |stack: &Stack| {
        let tweak = &stack.flat_definitions().font_data["Noto Sans CJK SC Regular"].tweak;
        (tweak.scale, tweak.y_offset_factor)
    };
    assert_ne!(tweak(&desktop).1, 0.0, "the CJK face moves onto the Latin baseline");
    assert_eq!(tweak(&web), tweak(&desktop));
}

/// The CJK interface faces as `Embed::subset_to` embeds them, with Noto Sans CJK SC as the
/// Simplified one.
fn ui_faces(others: &'static [u8]) -> [EmbeddedFace; 5] {
    [
        EmbeddedFace { family: "Noto Sans SC UI", style: "Regular", scripts: &["Hans", "Latn"], bytes: CJK, index: 0 },
        EmbeddedFace { family: "Noto Sans TC UI", style: "Regular", scripts: &["Hant", "Latn"], bytes: others, index: 0 },
        EmbeddedFace { family: "Noto Sans HK UI", style: "Regular", scripts: &["Hant-HK", "Latn"], bytes: others, index: 0 },
        EmbeddedFace { family: "Noto Sans KR UI", style: "Regular", scripts: &["Kore", "Latn"], bytes: others, index: 0 },
        EmbeddedFace { family: "BIZ UDPGothic", style: "Regular", scripts: &["Jpan", "Latn"], bytes: others, index: 0 },
    ]
}

#[trial]
fn preferences_name_the_interface_faces_too() {
    let order = |faces: &[EmbeddedFace], prefer: Option<&str>| {
        let mut fonts = UiFonts::new(&I18nScripts).craft_faces(faces).ui_language("ja").system_fonts(SystemFonts::Off);
        if let Some(family) = prefer {
            fonts = fonts.prefer("Jpan", family);
        }
        let stack = fonts.stack();
        stack.chain(&FontRef::proportional()).iter().filter(|n| faces.iter().any(|f| f.name() == **n)).cloned().collect::<Vec<_>>()
    };
    let face = |family, style, scripts| EmbeddedFace { family, style, scripts, bytes: &[], index: 0 };
    let mincho = face("Shippori Mincho", "Regular", &["Jpan", "Latn"]);
    let gothic = ui_faces(&[])[4];
    // craft-fonts' preference for BIZ UDPGothic.
    assert_eq!(order(&[mincho, gothic], None), ["BIZ UDPGothic Regular", "Shippori Mincho Regular"]);
    // The app's.
    let mincho_ui = face("Shippori Mincho UI", "Regular", &["Jpan", "Latn"]);
    assert_eq!(order(&[gothic, mincho_ui], Some("Shippori Mincho")), ["Shippori Mincho UI Regular", "BIZ UDPGothic Regular"]);
}

#[trial]
fn a_taiwanese_ui_takes_characters_no_interface_face_has_from_the_system_fonts() {
    use craft_ui::fonts::{Sample, missing_glyphs};
    use egui::{FallbackRequest, FontData, FontInsert, FontPriority, InsertFontFamily};
    // Noto Sans stands in for faces cut without 中 and 文; the system fonts have them.
    let asked = Arc::new(Mutex::new(Vec::<String>::new()));
    let stack = |simplified: &'static [u8]| {
        let mut faces = ui_faces(NOTO_SANS_LATIN);
        faces[0].bytes = simplified;
        let recorded = Arc::clone(&asked);
        let system = move |request: &FallbackRequest<'_>| {
            recorded.lock().map(|mut asked| asked.push(request.cluster.to_owned())).ok()?;
            Some(FontInsert::new(
                "system:Noto Sans CJK SC",
                FontData::from_static(CJK),
                vec![InsertFontFamily { family: request.family.clone(), priority: FontPriority::Lowest }],
            ))
        };
        UiFonts::new(&I18nScripts)
            .app_face("ui", FontData::from_static(NOTO_SANS_LATIN), &[FontRef::proportional()])
            .craft_faces(&faces)
            .languages(&["en", "zh-TW", "zh-CN", "ja", "ko"])
            .ui_language("zh-TW")
            .system_provider(Arc::new(system))
            .stack()
    };
    let sample = [Sample::new("zh-TW", "menu.language", "中文")];
    let missing = |stack: &Stack, with_system| -> Vec<char> {
        missing_glyphs(stack, &FontRef::proportional(), sample, with_system).iter().map(|m| m.character).collect()
    };

    let without = stack(NOTO_SANS_LATIN);
    // Other regions' interface faces come before the system fonts, so they draw what they have.
    let head: Vec<String> = without.chain(&FontRef::proportional()).iter().take(4).cloned().collect();
    assert_eq!(head, ["ui", "Noto Sans TC UI Regular", "Noto Sans HK UI Regular", "Noto Sans SC UI Regular"]);
    assert_eq!(missing(&without, false), ['中', '文']);
    assert_eq!(missing(&without, true), Vec::<char>::new());
    // The face the system provider returns for 中 draws 文 too.
    assert_eq!(*asked.lock().expect("lock"), ["中"]);

    asked.lock().expect("lock").clear();
    let with_simplified = stack(CJK);
    assert_eq!(missing(&with_simplified, true), Vec::<char>::new());
    assert!(asked.lock().expect("lock").is_empty(), "the Simplified interface face has them");
}

#[trial]
fn a_web_build_downloads_the_interface_face_for_its_language() {
    let faces = [
        served("Noto Sans SC UI", "Regular", &["Hans", "Latn"], "fonts/ui/NotoSansSCUI-Regular.0123456789abcdef.ttf", 0),
        served("Noto Sans TC UI", "Regular", &["Hant", "Latn"], "fonts/ui/NotoSansTCUI-Regular.0123456789abcdef.ttf", 0),
        served("BIZ UDPGothic", "Regular", &["Jpan", "Latn"], "fonts/biz-ud-pgothic/BIZUDPGothic-Regular.0123456789abcdef.ttf", 0),
        served("Noto Sans Arabic", "Regular", &["Arab"], "fonts/noto-sans-arabic/NotoSansArabic.ttf", 0),
    ];
    // The interface faces are Regular only: the semibold family takes that face too.
    for (language, want) in [("zh-TW", "Noto Sans TC UI Regular"), ("zh-CN", "Noto Sans SC UI Regular"), ("ja", "BIZ UDPGothic Regular")] {
        assert_eq!(names(web_fonts(language, &faces).stack().ui_downloads()), [want], "{language}");
    }
}

/// craft-fonts' faces for Latin, Greek and Cyrillic, symbols and emoji, as a web build serves them.
const SERVED_SYMBOLS: &[ServedFace] = &[
    served("Noto Sans", "Regular", &["Latn", "Cyrl", "Grek", "Deva"], "fonts/noto-sans/NotoSans.ttf", 0),
    served("Noto Color Emoji", "Regular", &["Zsye"], "fonts/noto-color-emoji/NotoColorEmoji.ttf", 0),
    served("Noto Sans Symbols", "Regular", &["Zsym"], "fonts/noto-sans-symbols/NotoSansSymbols.ttf", 0),
    served("Noto Sans Symbols 2", "Regular", &["Zsym"], "fonts/noto-sans-symbols-2/NotoSansSymbols2-Regular.ttf", 0),
    served("Noto Sans Math", "Regular", &["Zmth", "Zsym"], "fonts/noto-sans-math/NotoSansMath-Regular.ttf", 0),
];

#[trial]
fn letters_symbols_and_emoji_download_their_own_kind_of_face() {
    let stack = web_fonts("en", &[SERVED, SERVED_SYMBOLS].concat()).stack();
    let regular = FontRef::proportional();
    let first = |c: char| names(stack.downloads_for(&regular, c)).first().cloned().unwrap_or_default();
    for c in ['ổ', 'ẫ', 'ạ', 'ư', 'λ', 'ῶ', 'ж', 'ӂ', 'ǅ', '₫'] {
        assert_eq!(names(stack.downloads_for(&regular, c)), ["Noto Sans Regular"], "{c}");
    }
    let symbols = ["Noto Sans Symbols Regular", "Noto Sans Symbols 2 Regular", "Noto Sans Math Regular"];
    for c in ['⌘', '⌥', '⇧', '⌃', '⎋', '⏎', '⌫', '⇥', '↩', '⌦', '⇪', '⏏', '☰', '✓', '♠', '🁣', '♔'] {
        assert_eq!(names(stack.downloads_for(&regular, c)), symbols, "{c}");
    }
    for c in ['⊞', '∀', '⟨', '𝔸'] {
        assert_eq!(first(c), "Noto Sans Math Regular", "{c}");
    }
    for c in ['🔍', '😀', '⌚', '☕', '✅', '⭐', '🃏'] {
        assert_eq!(names(stack.downloads_for(&regular, c)), ["Noto Color Emoji Regular"], "{c}");
    }
}

/// Fetches nothing until the test answers, on a clock the test moves.
#[derive(Default)]
struct FakeFetch {
    pending: Mutex<Vec<(&'static str, &'static str, FetchDone)>>,
    requested: Mutex<Vec<&'static str>>,
    now: Mutex<f64>,
    timers: Mutex<Vec<(f64, FetchWake)>>,
}

impl FontFetcher for FakeFetch {
    fn fetch(&self, url: &'static str, integrity: &'static str, done: FetchDone) {
        self.requested.lock().expect("lock").push(url);
        self.pending.lock().expect("lock").push((url, integrity, done));
    }

    fn now(&self) -> f64 {
        *self.now.lock().expect("lock")
    }

    fn after(&self, seconds: f64, wake: FetchWake) {
        let at = self.now() + seconds;
        self.timers.lock().expect("lock").push((at, wake));
    }
}

impl FakeFetch {
    /// The URLs requested since the last call.
    fn requested(&self) -> Vec<&'static str> {
        std::mem::take(&mut *self.requested.lock().expect("lock"))
    }

    fn answer(&self, url: &str, result: Result<&[u8], &str>) {
        self.answer_checked(url, None, result);
    }

    /// Answers the request for `url` made with `integrity` (any, for `None`).
    fn answer_checked(&self, url: &str, integrity: Option<&str>, result: Result<&[u8], &str>) {
        let mut pending = self.pending.lock().expect("lock");
        let i = pending
            .iter()
            .position(|(u, checked, _)| *u == url && integrity.is_none_or(|wanted| wanted == *checked))
            .unwrap_or_else(|| panic!("{url} is not being fetched"));
        let (.., done) = pending.remove(i);
        drop(pending);
        done(result.map(<[u8]>::to_vec).map_err(str::to_owned));
    }

    fn wait(&self, seconds: f64) {
        let now = {
            let mut now = self.now.lock().expect("lock");
            *now += seconds;
            *now
        };
        let due: Vec<_> = {
            let mut timers = self.timers.lock().expect("lock");
            let (due, later) = std::mem::take(&mut *timers).into_iter().partition(|(at, _)| *at <= now);
            *timers = later;
            due
        };
        for (_, wake) in due {
            wake();
        }
    }
}

const NOTO_SANS_URL: &str = "fonts/noto-sans/NotoSans.ttf";
const SYMBOLS_2_URL: &str = "fonts/noto-sans-symbols-2/NotoSansSymbols2-Regular.ttf";
const MATH_URL: &str = "fonts/noto-sans-math/NotoSansMath-Regular.ttf";

/// An English UI on the web whose faces lack Vietnamese ổ and the keyboard keys, with `fetch`
/// fetching its served faces; and whether the context draws a character, as a pass would ask.
fn web_context(fetch: &Arc<FakeFetch>) -> (egui::Context, impl Fn(char) -> bool) {
    let mut faces = SERVED_SYMBOLS.to_vec();
    faces[0].coverage = coverage_of(NOTO_SANS_LATIN);
    faces[2].coverage = coverage_of(SYMBOLS);
    faces[3].coverage = coverage_of(SYMBOLS_2);
    // Arrows and technical symbols, ⌥ and ⇥ among them; never served.
    faces[4].coverage = craft_fonts_build::coverage::encode(&[(0x2190, 0x21FF), (0x2300, 0x23FF)]).leak();
    web_context_with("en", &faces, fetch)
}

/// A web context in `ui_language` with these served faces, fetched by `fetch`; and whether the
/// context draws a character, as a pass would ask.
fn web_context_with(ui_language: &str, faces: &[ServedFace], fetch: &Arc<FakeFetch>) -> (egui::Context, impl Fn(char) -> bool + use<>) {
    let ctx = egui::Context::default();
    web_fonts(ui_language, faces).fetcher(Arc::clone(fetch) as Arc<dyn FontFetcher>).install(&ctx);
    let draws = {
        let ctx = ctx.clone();
        move |c: char| {
            ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
            ctx.fonts_mut(|f| f.has_glyph(&FontId::proportional(14.0), c))
        }
    };
    (ctx, draws)
}

#[trial]
fn a_missing_character_fetches_only_the_face_that_draws_it() {
    let fetch = Arc::new(FakeFetch::default());
    let (_ctx, draws) = web_context(&fetch);
    assert!(fetch.requested().is_empty(), "English needs no served face up front");

    assert!(!draws('ổ'));
    assert_eq!(fetch.requested(), [NOTO_SANS_URL]);
    // While a face is on its way, characters it draws wait for it.
    assert!(!draws('ạ'));
    assert!(fetch.requested().is_empty());
    fetch.answer(NOTO_SANS_URL, Ok(NOTO_SANS_LATIN));
    assert!(draws('ổ') && draws('ạ'));

    // ⌥ is in the second symbol face only: the first is never fetched for it.
    assert!(!draws('⌥'));
    assert_eq!(fetch.requested(), [SYMBOLS_2_URL]);
    fetch.answer(SYMBOLS_2_URL, Ok(SYMBOLS_2));
    assert!(draws('⌥') && draws('⌫'));
    assert!(fetch.requested().is_empty());

    // A letter no served face has fetches nothing.
    assert!(!draws('ẫ'));
    assert!(fetch.requested().is_empty(), "the emoji face is never a candidate for letters or keys");
}

#[trial]
fn the_ui_language_faces_are_fetched_at_install_and_reported_as_loading() {
    let fetch = Arc::new(FakeFetch::default());
    let ctx = egui::Context::default();
    assert!(!craft_ui::fonts::loading(&ctx));
    web_fonts("ar", SERVED).fetcher(Arc::clone(&fetch) as Arc<dyn FontFetcher>).install(&ctx);
    assert_eq!(fetch.requested(), ["fonts/noto-sans-arabic/NotoSansArabic.ttf"]);
    assert!(craft_ui::fonts::loading(&ctx));
    fetch.answer("fonts/noto-sans-arabic/NotoSansArabic.ttf", Ok(ARABIC));
    assert!(!craft_ui::fonts::loading(&ctx));
    ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
    assert!(ctx.fonts_mut(|f| f.has_glyph(&FontId::proportional(14.0), 'ع')));
}

#[trial]
fn served_faces_are_never_fetched_without_a_fetcher() {
    let ctx = egui::Context::default();
    let stack = web_fonts("ar", SERVED).install(&ctx);
    assert_eq!(names(stack.ui_downloads()), ["Noto Sans Arabic Regular"]);
    assert!(!craft_ui::fonts::loading(&ctx));
}

#[trial]
fn bytes_fetched_under_one_integrity_never_reach_a_face_requiring_another() {
    const URL: &str = "fonts/shared.ttf";
    const STRICT: &str = "sha256-strict";
    // Two faces of one file: one needs no integrity check, the other a strict one.
    let faces = [
        served("Noto Sans Arabic", "Regular", &["Arab"], URL, 0),
        ServedFace { integrity: STRICT, ..served("Noto Sans Thai", "Regular", &["Thai"], URL, 0) },
    ];
    let installed = |ctx: &egui::Context, name: &str| ctx.fonts(|f| f.definitions().font_data.contains_key(name));

    let fetch = Arc::new(FakeFetch::default());
    let (ctx, draws) = web_context_with("en", &faces, &fetch);
    assert!(!draws('ก'));
    assert_eq!(fetch.requested(), [URL]);
    fetch.answer_checked(URL, Some(STRICT), Ok(ARABIC));
    draws('ก');
    assert!(installed(&ctx, "Noto Sans Thai Regular"));
    assert!(!installed(&ctx, "Noto Sans Arabic Regular"), "fetched under a strict check: the unchecked face does not take it");

    let fetch = Arc::new(FakeFetch::default());
    let (ctx, draws) = web_context_with("en", &faces, &fetch);
    assert!(!draws('ع'));
    fetch.answer_checked(URL, Some(""), Ok(ARABIC));
    assert!(draws('ع'));
    assert!(!installed(&ctx, "Noto Sans Thai Regular"), "fetched without a check: the strict face does not take it");
}

#[trial]
fn a_han_character_fetches_the_one_interface_face_that_has_it() {
    let cut = |family: &'static str, scripts: &'static [&'static str], url: &'static str, chars: &str| ServedFace {
        coverage: coverage_of_chars(chars),
        ..served(family, "Regular", scripts, url, 0)
    };
    let faces = [
        cut("Noto Sans SC UI", &["Hans", "Latn"], "fonts/ui/SC.ttf", "中文日本语ổ"),
        cut("Noto Sans TC UI", &["Hant", "Latn"], "fonts/ui/TC.ttf", "中文日本語體ổ"),
        cut("Noto Sans HK UI", &["Hant-HK", "Latn"], "fonts/ui/HK.ttf", "中文日本語ổ"),
        cut("Noto Sans KR UI", &["Kore", "Latn"], "fonts/ui/KR.ttf", "中文日本語한ổ"),
        cut("BIZ UDPGothic", &["Jpan", "Latn"], "fonts/ui/JP.ttf", "中文日本語のổ"),
        ServedFace { coverage: coverage_of(NOTO_SANS_LATIN), ..SERVED_SYMBOLS[0] },
    ];
    let fetch = Arc::new(FakeFetch::default());
    let (_ctx, draws) = web_context_with("zh-Hans", &faces, &fetch);
    assert_eq!(fetch.requested(), ["fonts/ui/SC.ttf"], "the UI language's face, at once");
    fetch.answer("fonts/ui/SC.ttf", Ok(CJK));

    // 體 is in the Traditional face only.
    assert!(!draws('體'));
    assert_eq!(fetch.requested(), ["fonts/ui/TC.ttf"]);
    fetch.answer("fonts/ui/TC.ttf", Ok(CJK));
    assert!(!draws('體'), "the test file lacks it");
    assert!(fetch.requested().is_empty(), "a fetched face is not fetched again, and no other face has it");
    // A rare character no face has fetches nothing: tofu or the browser draws it.
    assert!(!draws('龘'));
    assert!(fetch.requested().is_empty());
    // Every cut has ổ, but Latin letters come from Noto Sans.
    assert!(!draws('ổ'));
    assert_eq!(fetch.requested(), [NOTO_SANS_URL]);

    // Among faces that have a character, the UI language's region comes first.
    let both =
        [cut("Noto Sans TC UI", &["Hant", "Latn"], "fonts/ui/TC.ttf", "體"), cut("Noto Sans HK UI", &["Hant-HK", "Latn"], "fonts/ui/HK.ttf", "體")];
    for (language, want) in [("zh-Hant", "Noto Sans TC UI Regular"), ("zh-HK", "Noto Sans HK UI Regular"), ("zh-Hans", "Noto Sans TC UI Regular")] {
        assert_eq!(names(web_fonts(language, &both).stack().downloads_for(&FontRef::proportional(), '體'))[0], want, "{language}");
    }
    let regular = FontRef::proportional();
    for (language, want) in [
        ("zh-Hans", "Noto Sans SC UI Regular"),
        ("zh-Hant", "Noto Sans TC UI Regular"),
        ("zh-HK", "Noto Sans HK UI Regular"),
        ("ja", "BIZ UDPGothic Regular"),
        ("ko", "Noto Sans KR UI Regular"),
    ] {
        assert_eq!(names(web_fonts(language, &faces).stack().downloads_for(&regular, '中'))[0], want, "{language}");
    }
}

#[trial]
fn a_failed_download_moves_on_and_is_retried_later() {
    let fetch = Arc::new(FakeFetch::default());
    let (_ctx, draws) = web_context(&fetch);

    assert!(!draws('⌥'));
    assert_eq!(fetch.requested(), [SYMBOLS_2_URL]);
    fetch.answer(SYMBOLS_2_URL, Err("HTTP 503"));
    // The next face that draws it is fetched at once.
    assert!(!draws('⌥'));
    assert_eq!(fetch.requested(), [MATH_URL]);
    fetch.answer(MATH_URL, Err("HTTP 503"));
    assert!(!draws('⌥'));
    assert!(fetch.requested().is_empty(), "failed faces wait for their retry time");

    // Failures of one script don't stop another's face.
    assert!(!draws('ổ'));
    assert_eq!(fetch.requested(), [NOTO_SANS_URL]);
    fetch.answer(NOTO_SANS_URL, Ok(NOTO_SANS_LATIN));
    assert!(draws('ổ'));

    // At their retry time the failed faces are fetched again, in order.
    fetch.wait(1.0);
    assert!(!draws('⌥'));
    assert!(fetch.requested().is_empty());
    fetch.wait(1.0);
    assert!(!draws('⌥'));
    assert_eq!(fetch.requested(), [SYMBOLS_2_URL]);
    fetch.answer(SYMBOLS_2_URL, Ok(SYMBOLS_2));
    assert!(draws('⌥'));

    // A face that fails again waits twice as long.
    assert!(!draws('⇥'));
    assert_eq!(fetch.requested(), [MATH_URL]);
    fetch.answer(MATH_URL, Err("HTTP 503"));
    fetch.wait(3.0);
    assert!(!draws('⇥'));
    assert!(fetch.requested().is_empty());
    fetch.wait(1.0);
    assert!(!draws('⇥'));
    assert_eq!(fetch.requested(), [MATH_URL]);
}
