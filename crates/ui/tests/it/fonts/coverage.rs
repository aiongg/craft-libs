//! Coverage checks over a stack: which characters its faces draw and which languages it covers.

use craft_fonts_build::EmbeddedFace;
use craft_ui::fonts::{FontRef, I18nScripts, Sample, Stack, SystemFonts, UiFonts, assert_covers, missing_glyphs};
use craft_ui::testing::trial;

const ARABIC: &[u8] = include_bytes!("../../fonts/NotoSansArabic-test.ttf");
/// Noto Sans with Vietnamese ổ but not ẫ.
const NOTO_SANS: &[u8] = include_bytes!("../../fonts/NotoSans-test.ttf");
const CJK: &[u8] = include_bytes!("../../fonts/NotoSansCJKsc-test.otf");

fn test_stack(ui_language: &str) -> Stack {
    let ubuntu = (**craft_ui::fonts::deflated_default_definitions().font_data.get("Ubuntu-Light").expect("egui default_fonts")).clone();
    UiFonts::new(&I18nScripts)
        .app_face("ui", ubuntu, &[FontRef::proportional()])
        .craft_faces(&[
            EmbeddedFace { family: "Noto Sans Arabic", style: "Regular", scripts: &["Arab"], bytes: ARABIC, index: 0 },
            EmbeddedFace { family: "Noto Sans CJK SC", style: "Regular", scripts: &["Hans", "Latn"], bytes: CJK, index: 0 },
        ])
        .languages(&["en", "ar", "ja", "ka"])
        .ui_language(ui_language)
        .system_fonts(SystemFonts::Off)
        .stack()
}

#[trial]
fn coverage_names_language_string_and_character() {
    let stack = test_stack("en");
    let samples = [
        // With U+061C ARABIC LETTER MARK and U+034F COMBINING GRAPHEME JOINER, which draw nothing.
        Sample::new("ar", "menu.file", "ملف\u{061C} عر\u{034F}بي"),
        Sample::new("ja", "menu.file", "日本語のファイル"),
        Sample::new("ka", "menu.file", "ფაილი"),
    ];
    let missing = missing_glyphs(&stack, &FontRef::proportional(), samples, false);
    let chars: Vec<char> = missing.iter().map(|m| m.character).collect();
    assert_eq!(chars, ['ფ', 'ა', 'ი', 'ლ']);
    assert!(missing.iter().all(|m| m.language == "ka" && m.key == "menu.file"));
    assert_eq!(missing[0].to_string(), "ka menu.file: U+10E4 'ფ' in \"ფაილი\"");

    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_covers(&stack, samples))).expect_err("Georgian has no face");
    let message = failure.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.contains("4 characters") && message.contains("ka menu.file: U+10E4"), "{message}");

    assert_covers(&stack, samples.into_iter().take(2));
}

#[trial]
fn a_latin_ui_takes_letters_its_face_lacks_from_a_latin_craft_face() {
    let cjk = EmbeddedFace { family: "Noto Sans CJK SC", style: "Regular", scripts: &["Hans", "Latn"], bytes: CJK, index: 0 };
    let noto_sans = EmbeddedFace { family: "Noto Sans", style: "Regular", scripts: &["Latn", "Cyrl", "Grek", "Deva"], bytes: NOTO_SANS, index: 0 };
    let vietnamese = [Sample::new("vi", "menu.file", "Tiếng Việt ổ ư ạ")];
    let stack = |faces: &[EmbeddedFace]| {
        UiFonts::new(&I18nScripts).craft_faces(faces).languages(&["en", "vi", "zh-CN"]).ui_language("vi").system_fonts(SystemFonts::Off).stack()
    };

    let without = stack(&[cjk]);
    assert!(missing_glyphs(&without, &FontRef::proportional(), vietnamese, false).iter().any(|m| m.character == 'ổ'));
    let with = stack(&[cjk, noto_sans]);
    assert_covers(&with, vietnamese);
    // Noto Sans CJK has Vietnamese letters too, at CJK proportions: Noto Sans comes first.
    let chain = with.chain(&FontRef::proportional());
    let at = |name: &str| chain.iter().position(|n| n == name).unwrap_or(usize::MAX);
    assert!(at("Noto Sans Regular") < at("Noto Sans CJK SC Regular"), "{chain:?}");
}

#[trial]
fn covers_language_checks_every_script_of_the_language() {
    let stack = test_stack("en");
    for language in ["en", "ar", "ja"] {
        assert!(stack.covers_language(language, false), "{language}");
    }
    assert!(!stack.covers_language("ka", false), "no Georgian face");
    // Not offered: the scripts come from craft-i18n.
    assert!(stack.covers_language("zh-CN", false));
    assert!(!stack.covers_language("th", false));
}
