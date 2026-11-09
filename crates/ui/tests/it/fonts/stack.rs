//! The font stack: chain order by UI language and scripts, app edits, families, CJK row fit and what an install does
//! to the context's providers.

use std::sync::Arc;

use craft_fonts_build::EmbeddedFace;
use craft_theme::{Contrast, ControlSize, Mode, Variant};
use craft_ui::fonts::{
    CjkRows, Emoji, FaceList, Family, FontRef, I18nScripts, RegularWeight, Stack, SystemFonts, UiFonts, set_weight, system_font_provider,
};
use craft_ui::list::{ListRow, ThumbSize};
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use egui::epaint::text::{Fonts, TextOptions};
use egui::{FallbackRequest, FontData, FontFamily, FontId, FontInsert, FontPriority, FontProvider, InsertFontFamily};

const ARABIC: &[u8] = include_bytes!("../../fonts/NotoSansArabic-test.ttf");
const CJK: &[u8] = include_bytes!("../../fonts/NotoSansCJKsc-test.otf");

const OFFERED: &[&str] = &["en", "ar", "ja", "zh-CN", "zh-TW", "zh-HK", "ko", "th", "ka", "ru"];

const fn face(family: &'static str, style: &'static str, scripts: &'static [&'static str]) -> EmbeddedFace {
    EmbeddedFace { family, style, scripts, bytes: &[], index: 0 }
}

/// craft-fonts' manifest today, plus the regional CJK faces and Georgian it is planned to have.
const CRAFT: &[EmbeddedFace] = &[
    face("Shippori Mincho", "Regular", &["Jpan", "Latn"]),
    face("BIZ UDPGothic", "Regular", &["Jpan", "Latn"]),
    face("BIZ UDPGothic", "Bold", &["Jpan", "Latn"]),
    face("Noto Sans CJK SC", "Regular", &["Hans", "Latn"]),
    face("Noto Sans CJK TC", "Regular", &["Hant", "Latn"]),
    face("Noto Sans CJK HK", "Regular", &["Hant-HK", "Latn"]),
    face("Noto Sans CJK KR", "Regular", &["Kore", "Latn"]),
    face("Noto Sans Arabic", "Regular", &["Arab"]),
    face("Noto Sans Thai", "Regular", &["Thai"]),
    face("Noto Sans Georgian", "Regular", &["Geor"]),
];

fn fonts(ui_language: &str) -> UiFonts<'static> {
    UiFonts::new(&I18nScripts)
        .app_face("ui", FontData::from_static(&[]), &[FontRef::proportional()])
        .app_face("mono", FontData::from_static(&[]), &[FontRef::monospace()])
        .craft_faces(CRAFT)
        .languages(OFFERED)
        .ui_language(ui_language)
        .system_fonts(SystemFonts::Off)
}

fn fonts_without_faces(ui_language: &str) -> UiFonts<'static> {
    UiFonts::new(&I18nScripts).craft_faces(CRAFT).languages(OFFERED).ui_language(ui_language).system_fonts(SystemFonts::Off)
}

fn head(stack: &Stack, family: &FontRef, n: usize) -> Vec<String> {
    stack.chain(family).iter().take(n).cloned().collect()
}

/// A font set over the stack's faces, for measuring text without a context.
fn measuring(stack: &Stack) -> Fonts {
    Fonts::new(TextOptions::default(), stack.flat_definitions())
}

fn egui_face(name: &str) -> FontData {
    (**craft_ui::fonts::deflated_default_definitions().font_data.get(name).expect("an egui default face")).clone()
}

#[trial]
fn ui_language_face_leads_when_the_app_face_lacks_its_script() {
    let proportional = FontRef::proportional();
    let position = |chain: &[String], name: &str| chain.iter().position(|n| n == name).unwrap_or_else(|| panic!("{name} missing in {chain:?}"));

    // A UI language the app face lacks leads with its own face.
    let stack = fonts("ar").stack();
    let chain = stack.chain(&proportional);
    assert_eq!(position(&chain, "Noto Sans Arabic Regular"), 0);
    assert!(position(&chain, "Noto Sans Arabic Regular") < position(&chain, "ui"));

    // CJK UIs keep the Latin face first: Han, kana and Hangul fit its rows. Their own region's face comes before the
    // other CJK faces.
    let cases = [
        ("ja", "BIZ UDPGothic Regular"),
        ("zh-TW", "Noto Sans CJK TC Regular"),
        ("zh-HK", "Noto Sans CJK HK Regular"),
        ("zh-CN", "Noto Sans CJK SC Regular"),
        ("ko", "Noto Sans CJK KR Regular"),
    ];
    for (language, region) in cases {
        let stack = fonts(language).stack();
        let chain = stack.chain(&proportional);
        assert_eq!(position(&chain, "ui"), 0, "{language}");
        for other in CRAFT.iter().filter(|f| f.scripts.contains(&"Latn") && f.name() != region) {
            assert!(position(&chain, region) < position(&chain, &other.name()), "{language}: {region} before {}", other.name());
        }
    }
    // Monospace keeps the app's face first, so code and numbers stay aligned.
    for language in ["ar", "ja"] {
        assert_eq!(fonts(language).stack().chain(&FontRef::monospace()).first().map(String::as_str), Some("mono"), "{language}");
    }
}

#[trial]
fn families_without_app_faces_start_with_egui_defaults() {
    let defaults = craft_ui::fonts::deflated_default_definitions();
    let first = |family: &FontFamily| defaults.families.get(family).and_then(|c| c.first()).cloned().expect("an egui default face");
    let (egui_ui, egui_mono) = (first(&FontFamily::Proportional), first(&FontFamily::Monospace));
    let only_ui = |language| fonts_without_faces(language).app_face("ui", FontData::from_static(&[]), &[FontRef::proportional()]).stack();
    let no_faces = |language| fonts_without_faces(language).stack();
    let (proportional, monospace) = (FontRef::proportional(), FontRef::monospace());

    let stack = only_ui("en");
    assert_eq!(head(&stack, &proportional, 2), ["ui", "Noto Sans Arabic Regular"]);
    assert_eq!(head(&stack, &monospace, 1), [egui_mono.as_str()], "egui's mono face, not a craft face");
    let stack = only_ui("ar");
    assert_eq!(head(&stack, &proportional, 2), ["Noto Sans Arabic Regular", "ui"]);
    assert_eq!(head(&stack, &monospace, 1), [egui_mono.as_str()]);

    // egui's defaults come right after the UI language's faces and before other scripts.
    for (language, lead) in [("en", None), ("ar", Some("Noto Sans Arabic Regular"))] {
        let stack = no_faces(language);
        let chain = stack.chain(&proportional);
        let egui_at = chain.iter().position(|n| *n == egui_ui).expect("egui's face");
        assert_eq!(chain.first().map(String::as_str), Some(lead.unwrap_or(&egui_ui)), "{language}");
        let other = if lead.is_some() { "BIZ UDPGothic Regular" } else { "Noto Sans Arabic Regular" };
        assert!(chain.iter().position(|n| n == other) > Some(egui_at), "{language}: {chain:?}");
        assert_eq!(head(&stack, &monospace, 1), [egui_mono.as_str()], "{language}");
    }
}

#[trial]
fn craft_preference_orders_the_best_region_only() {
    // With only Japanese faces, they are the best Han region a Taiwanese UI has.
    let stack = UiFonts::new(&I18nScripts).craft_faces(&CRAFT[..3]).ui_language("zh-TW").system_fonts(SystemFonts::Off).stack();
    let chain = stack.chain(&FontRef::proportional());
    let at = |name: &str| chain.iter().position(|n| n == name);
    assert!(at("BIZ UDPGothic Regular") < at("Shippori Mincho Regular"), "{chain:?}");
}

#[trial]
fn every_shipped_face_is_in_every_stack() {
    let stack = fonts("en").stack();
    let chain = stack.chain(&FontRef::proportional());
    for face in CRAFT.iter().filter(|f| f.style == "Regular") {
        assert!(chain.contains(&face.name()), "{} missing", face.name());
    }
    assert!(!chain.contains(&"BIZ UDPGothic Bold".to_owned()), "bold face in the regular chain");
}

#[trial]
fn app_can_prefer_add_replace_and_reorder_faces() {
    let proportional = FontRef::proportional();
    let stack = fonts("ja").prefer("Jpan", "Shippori Mincho").stack();
    assert_eq!(head(&stack, &proportional, 3), ["ui", "Shippori Mincho Regular", "BIZ UDPGothic Regular"]);

    // An app whose UI face covers Arabic keeps it first.
    let stack = fonts("ar").app_scripts(&["Latn", "Arab"]).stack();
    assert_eq!(head(&stack, &proportional, 1), ["ui"]);
    assert!(stack.chain(&proportional).contains(&"Noto Sans Arabic Regular".to_owned()));

    let mut stack = fonts("en").stack();
    stack.insert(&proportional, 1, "brand", FontData::from_static(&[]));
    stack.remove("Noto Sans Arabic Regular");
    assert_eq!(head(&stack, &proportional, 3), ["ui", "brand", "BIZ UDPGothic Regular"]);
    stack.insert(&proportional, 0, "BIZ UDPGothic Regular", FontData::from_static(&[]));
    assert_eq!(head(&stack, &proportional, 3), ["BIZ UDPGothic Regular", "ui", "brand"]);
    let definitions = stack.flat_definitions();
    assert!(!definitions.families.values().flatten().any(|n| n == "Noto Sans Arabic Regular"));
}

#[trial]
fn named_families_take_their_weight_script_and_monospace_face() {
    let position = |chain: &[String], name: &str| chain.iter().position(|n| n == name).unwrap_or_else(|| panic!("{name} missing in {chain:?}"));
    let (semibold, script, mono_named) = (FontRef::named("semibold"), FontRef::named("arabic"), FontRef::named("console"));
    let stack = fonts("ja")
        .app_face("ui-semibold", FontData::from_static(&[]), std::slice::from_ref(&semibold))
        .family(Family::named("semibold").weight(600))
        .family(Family::named("arabic").script("Arab"))
        .family(Family::monospace().weight(500))
        .stack();
    assert!(stack.weights().iter().any(|(family, _)| *family == FontRef::monospace()));
    // A weighted family leads with its own face and takes the bold cut before the regular one.
    let chain = stack.chain(&semibold);
    assert_eq!(chain[0], "ui-semibold");
    assert!(position(&chain, "BIZ UDPGothic Bold") < position(&chain, "BIZ UDPGothic Regular"));
    // A family with a lead script puts that script's face before the proportional faces.
    let chain = stack.chain(&script);
    assert!(position(&chain, "Noto Sans Arabic Regular") < position(&chain, "ui"));

    // A monospaced family keeps the monospace app face ahead of the UI language's face; a text family does not.
    let (code, plain) = (FontRef::named("code"), FontRef::named("plain"));
    let stack = fonts("ka")
        .app_face("code-face", FontData::from_static(&[]), std::slice::from_ref(&code))
        .family(Family::named("code").monospaced())
        .family(Family::named("console").monospaced())
        .family(Family::named("plain"))
        .stack();
    for (family, lead) in [(&code, "code-face"), (&mono_named, "mono")] {
        let chain = stack.chain(family);
        assert!(position(&chain, lead) < position(&chain, "Noto Sans Georgian Regular"), "{family:?}");
    }
    let chain = stack.chain(&plain);
    assert!(position(&chain, "Noto Sans Georgian Regular") < position(&chain, "ui"));
}

#[trial]
fn system_fonts_before_craft_moves_other_scripts_after_them() {
    let mut emoji = Emoji::default();
    emoji.color = Some(FontData::from_static(&[]));
    emoji.monochrome = Some(FontData::from_static(&[]));
    let stack = fonts("ja").system_fonts(SystemFonts::BeforeCraft).emoji(emoji).stack();
    let configured = stack.chain(&FontRef::proportional());
    assert!(configured.contains(&"BIZ UDPGothic Regular".to_owned()));
    assert!(!configured.contains(&"Noto Sans Arabic Regular".to_owned()));
    // The flat chain ends with the faces asked after the system fonts.
    let flat = stack.flat_definitions().families.remove(&FontFamily::Proportional).unwrap_or_default();
    let position = |name: &str| flat.iter().position(|n| n == name);
    let (arabic, color, mono) = (position("Noto Sans Arabic Regular"), position("emoji-color"), position("emoji-monochrome"));
    assert!(arabic.is_some_and(|arabic| arabic >= configured.len()) && arabic < color && color < mono, "{flat:?}");
}

fn test_fonts(ui_language: &str) -> UiFonts<'static> {
    UiFonts::new(&I18nScripts)
        .app_face("ui", egui_face("Ubuntu-Light"), &[FontRef::proportional()])
        .craft_faces(&[
            EmbeddedFace { family: "Noto Sans Arabic", style: "Regular", scripts: &["Arab"], bytes: ARABIC, index: 0 },
            EmbeddedFace { family: "Noto Sans CJK SC", style: "Regular", scripts: &["Hans", "Latn"], bytes: CJK, index: 0 },
        ])
        .languages(&["en", "ar", "ja", "ka"])
        .ui_language(ui_language)
        .system_fonts(SystemFonts::Off)
}

fn test_stack(ui_language: &str) -> Stack {
    test_fonts(ui_language).stack()
}

#[trial]
fn flat_definitions_end_each_chain_with_the_faces_after_the_system_fonts() {
    let mut emoji = Emoji::default();
    emoji.monochrome = Some(FontData::from_static(CJK));
    let stack = test_fonts("en").emoji(emoji).stack();
    let flat = stack.flat_definitions();
    let configured = stack.chain(&FontRef::proportional());
    let chain = &flat.families[&FontFamily::Proportional];
    assert_eq!(chain[..configured.len()], *configured);
    let after = &chain[configured.len()..];
    assert!(after.iter().any(|n| n == "egui-icons") && after.iter().any(|n| n == "emoji-monochrome"), "{after:?}");
    assert_eq!(flat.font_data.get("emoji-monochrome").map(|d| d.bytes().len()), Some(CJK.len()));
    assert!(chain.iter().all(|n| flat.font_data.contains_key(n)));
    assert!(flat.families[&FontFamily::Monospace].len() > stack.chain(&FontRef::monospace()).len());
}

#[trial]
fn face_list_prefers_a_face_with_the_whole_cluster() {
    let mut faces = FaceList::default();
    for name in ["Ubuntu-Light", "Hack"] {
        faces.insert(name, egui_face(name));
    }
    faces.set_chain(FontRef::proportional(), vec!["Ubuntu-Light".into(), "Hack".into()]);
    let face = |cluster| faces.font_for(&FallbackRequest { cluster, family: &FontFamily::Proportional }).map(|insert| insert.name);
    assert_eq!(face("a").as_deref(), Some("Ubuntu-Light"));
    // Only Hack has U+0301 COMBINING ACUTE ACCENT, and neither has U+0302.
    assert_eq!(face("a\u{0301}").as_deref(), Some("Hack"));
    assert_eq!(face("a\u{0302}").as_deref(), Some("Ubuntu-Light"));
}

#[trial]
fn variable_faces_draw_at_regular_weight() {
    // The Arabic test face's default instance is Thin (100), like many variable system fonts.
    let thin = || FontData::from_static(ARABIC);
    let wght = |data: &FontData| data.tweak.coords.as_ref().iter().find(|(t, _)| *t == egui::epaint::text::Tag::new(b"wght")).map(|(_, v)| *v);
    assert_eq!(wght(&thin()), None);

    let provider = move |request: &FallbackRequest<'_>| {
        Some(FontInsert::new("system:Arabic", thin(), vec![InsertFontFamily { family: request.family.clone(), priority: FontPriority::Lowest }]))
    };
    let wrapped = RegularWeight::new(Arc::new(provider)).with_weight(FontRef::named("bold"), 700);
    let regular = wrapped.font_for(&FallbackRequest { cluster: "ع", family: &FontFamily::Proportional }).expect("found");
    assert_eq!((regular.name.as_str(), wght(&regular.data)), ("system:Arabic", Some(400.0)));
    let bold = wrapped.font_for(&FallbackRequest { cluster: "ع", family: &FontFamily::Name("bold".into()) }).expect("found");
    assert_eq!((bold.name.as_str(), wght(&bold.data)), ("system:Arabic @700", Some(700.0)));

    // Craft faces get the same treatment, and a weight the app set stays.
    let flat = test_stack("ar").flat_definitions();
    assert_eq!(flat.font_data.get("Noto Sans Arabic Regular").map(|d| wght(d)), Some(Some(400.0)));
    let mut set = thin();
    set.tweak.coords.push(*b"wght", 300.0);
    assert!(!set_weight(&mut set, 400));
}

#[trial]
fn cjk_uis_keep_the_rows_and_baseline_of_a_latin_ui() {
    let latin =
        UiFonts::new(&I18nScripts).app_face("ui", egui_face("Ubuntu-Light"), &[FontRef::proportional()]).system_fonts(SystemFonts::Off).stack();
    for language in ["ja", "ko", "zh-CN", "en"] {
        let stack = test_stack(language);
        assert_eq!(stack.chain(&FontRef::proportional()).first().map(String::as_str), Some("ui"), "{language}");
        for (size, ppp) in [(14.0, 1.0), (13.0, 1.5), (40.0, 2.0)] {
            let font = FontId::proportional(size);
            let layout = |stack: &Stack, text: &str| {
                measuring(stack).with_pixels_per_point(ppp).layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::default())
            };
            let label = layout(&stack, "Ab漢한中");
            let row = egui::Rect::from_x_y_ranges(label.rect.x_range(), layout(&latin, "Ab").rect.y_range());
            assert_eq!(label.rect.height(), row.height(), "{language} at {size} pt");
            // Rasterised glyphs reach at most a pixel past their outlines.
            assert!(row.expand(1.0 / ppp).contains_rect(label.mesh_bounds), "{language} at {size} pt: {:?} in {row:?}", label.mesh_bounds);
        }
    }

    // The app can ask for the CJK face's own metrics instead of the fit.
    let tweak = |rows: CjkRows| test_fonts("ja").cjk_rows(rows).stack().flat_definitions().font_data["Noto Sans CJK SC Regular"].tweak.clone();
    let (fit, grow) = (tweak(CjkRows::Fit), tweak(CjkRows::Grow));
    assert!(fit.y_offset_factor != grow.y_offset_factor, "the fit moves the CJK face onto the Latin baseline");
}

#[trial]
fn install_keeps_the_context_providers_unless_it_brings_a_system_lookup() {
    let arabic = |_: &FallbackRequest<'_>| {
        Some(FontInsert::new(
            "host:Arabic",
            FontData::from_static(ARABIC),
            vec![InsertFontFamily { family: FontFamily::Proportional, priority: FontPriority::Lowest }],
        ))
    };
    let draws_arabic = |system: SystemFonts| {
        let mut frames = Frames::new();
        let ctx = frames.ctx.clone();
        ctx.add_font_provider(Arc::new(arabic));
        let stack = UiFonts::new(&I18nScripts).app_face("ui", egui_face("Ubuntu-Light"), &[FontRef::proportional()]).system_fonts(system).stack();
        stack.install(&ctx);
        stack.install(&ctx);
        frames.frame(vec![], |_| {});
        ctx.fonts_mut(|f| f.has_glyph(&FontId::proportional(14.0), 'ع'))
    };
    assert!(!draws_arabic(SystemFonts::Off), "Off asks no system fonts");
    if system_font_provider("en").is_none() {
        assert!(draws_arabic(SystemFonts::AfterCraft), "the host's provider stays");
    }
}

#[trial]
fn egui_faces_join_the_families_the_app_names() {
    let semibold = FontRef::named("semibold");
    let stack = fonts("en")
        .family(Family::named("semibold").weight(600))
        .egui_face("Hack", &[FontRef::proportional(), semibold.clone()])
        .egui_face("no such face", &[FontRef::proportional()])
        .stack();
    let proportional = stack.chain(&FontRef::proportional());
    assert_eq!(proportional.last().map(String::as_str), Some("Hack"), "{proportional:?}");
    assert!(stack.chain(&semibold).iter().any(|n| n == "Hack"));
    assert!(!stack.chain(&FontRef::monospace()).iter().any(|n| n == "Hack"), "only the families named");
    assert!(stack.flat_definitions().font_data.contains_key("Hack"));
    assert!(!proportional.iter().any(|n| n == "no such face"));
}

#[trial]
fn control_heights_follow_the_leading_face_of_the_stack() {
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let heights = |language: &str| {
        let frames = Frames::new();
        test_stack(language).install(&frames.ctx);
        with_cx(&frames.ctx, &tokens, Variant::SECONDARY, ControlSize::Md, |cx| {
            (cx.control().height, ListRow::row_height(cx, ThumbSize::None, false))
        })
    };
    let (latin_control, latin_row) = heights("en");
    assert_eq!(latin_control, tokens.size.control.md, "a Latin face keeps the token height");
    let (arabic_control, arabic_row) = heights("ar");
    assert!(arabic_control > latin_control, "an Arabic-led stack grows the control: {arabic_control} against {latin_control}");
    assert!(arabic_row > latin_row, "and the list row on it: {arabic_row} against {latin_row}");
}
