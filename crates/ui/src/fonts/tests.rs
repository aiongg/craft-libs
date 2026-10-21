use super::*;
use egui::{FallbackRequest, FontInsert, FontPriority, InsertFontFamily};

const ARABIC: &[u8] = include_bytes!("../../tests/fonts/NotoSansArabic-test.ttf");
const CJK: &[u8] = include_bytes!("../../tests/fonts/NotoSansCJKsc-test.otf");

/// The table craft-i18n's `scripts` module gives for these languages.
struct TestScripts;

impl LanguageScripts for TestScripts {
    fn scripts(&self, language: &str) -> Vec<&'static str> {
        match language {
            "ar" => vec!["Arab"],
            "ja" => vec!["Hani", "Hira", "Kana"],
            "ko" => vec!["Hang", "Hani"],
            "zh-CN" | "zh-TW" | "zh-HK" => vec!["Hani"],
            "th" => vec!["Thai"],
            "ka" => vec!["Geor"],
            "ru" => vec!["Cyrl"],
            _ => vec!["Latn"],
        }
    }

    fn han(&self, language: &str) -> Option<Han> {
        match language {
            "ja" => Some(Han::Japanese),
            "ko" => Some(Han::Korean),
            "zh-CN" => Some(Han::Simplified),
            "zh-TW" => Some(Han::Traditional),
            "zh-HK" => Some(Han::HongKong),
            _ => None,
        }
    }
}

const OFFERED: &[&str] = &["en", "ar", "ja", "zh-CN", "zh-TW", "zh-HK", "ko", "th", "ka", "ru"];

const fn face(family: &'static str, style: &'static str, scripts: &'static [&'static str]) -> CraftFace {
    CraftFace { family, style, scripts, bytes: &[], index: 0 }
}

/// craft-fonts' manifest today, plus the regional CJK faces and Georgian it is planned to have.
const CRAFT: &[CraftFace] = &[
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
    UiFonts::new(&TestScripts)
        .app_face("ui", FontData::from_static(&[]), &[FontFamily::Proportional])
        .app_face("mono", FontData::from_static(&[]), &[FontFamily::Monospace])
        .craft_faces(CRAFT)
        .languages(OFFERED)
        .ui_language(ui_language)
        .system_fonts(SystemFonts::Off)
}

fn head(stack: &Stack, family: &FontFamily, n: usize) -> Vec<String> {
    stack.chain(family).iter().take(n).cloned().collect()
}

#[test]
fn ui_language_face_leads_when_the_app_face_lacks_its_script() {
    let cases: &[(&str, &[&str])] = &[
        ("ar", &["Noto Sans Arabic Regular", "ui"]),
        ("ja", &["BIZ UDPGothic Regular", "Shippori Mincho Regular", "Noto Sans CJK SC Regular"]),
        ("zh-TW", &["Noto Sans CJK TC Regular", "Noto Sans CJK HK Regular", "BIZ UDPGothic Regular"]),
        ("zh-HK", &["Noto Sans CJK HK Regular", "Noto Sans CJK TC Regular", "BIZ UDPGothic Regular"]),
        ("zh-CN", &["Noto Sans CJK SC Regular", "BIZ UDPGothic Regular"]),
        ("ko", &["Noto Sans CJK KR Regular", "BIZ UDPGothic Regular"]),
        ("th", &["Noto Sans Thai Regular", "ui"]),
        ("ka", &["Noto Sans Georgian Regular", "ui"]),
        ("ru", &["ui", "Noto Sans Arabic Regular", "BIZ UDPGothic Regular"]),
        ("en", &["ui", "Noto Sans Arabic Regular", "BIZ UDPGothic Regular"]),
    ];
    for (language, want) in cases {
        let stack = fonts(language).stack();
        assert_eq!(head(&stack, &FontFamily::Proportional, want.len()), *want, "{language}");
        // Monospace keeps the app's face first, so code and numbers stay aligned.
        assert_eq!(stack.chain(&FontFamily::Monospace).first().map(String::as_str), Some("mono"), "{language}");
    }
}

#[test]
fn every_shipped_face_is_in_every_stack() {
    for language in OFFERED {
        let stack = fonts(language).stack();
        let chain = stack.chain(&FontFamily::Proportional);
        for face in CRAFT.iter().filter(|f| f.style == "Regular") {
            assert!(chain.contains(&face.name()), "{language}: {} missing", face.name());
        }
        assert!(!chain.contains(&"BIZ UDPGothic Bold".to_owned()), "{language}: bold face in the regular chain");
    }
}

#[test]
fn app_can_prefer_add_replace_and_reorder_faces() {
    let stack = fonts("ja").prefer("Jpan", "Shippori Mincho").stack();
    assert_eq!(head(&stack, &FontFamily::Proportional, 2), ["Shippori Mincho Regular", "BIZ UDPGothic Regular"]);

    // An app whose UI face covers Arabic keeps it first.
    let stack = fonts("ar").app_scripts(&["Latn", "Arab"]).stack();
    assert_eq!(head(&stack, &FontFamily::Proportional, 1), ["ui"]);
    assert!(stack.chain(&FontFamily::Proportional).contains(&"Noto Sans Arabic Regular".to_owned()));

    let mut stack = fonts("en").stack();
    stack.insert(&FontFamily::Proportional, 1, "brand", FontData::from_static(&[]));
    stack.remove("Noto Sans Arabic Regular");
    assert_eq!(head(&stack, &FontFamily::Proportional, 3), ["ui", "brand", "BIZ UDPGothic Regular"]);
    stack.insert(&FontFamily::Proportional, 0, "BIZ UDPGothic Regular", FontData::from_static(&[]));
    assert_eq!(head(&stack, &FontFamily::Proportional, 3), ["BIZ UDPGothic Regular", "ui", "brand"]);
    let definitions = stack.checked_definitions();
    assert!(!definitions.families.values().flatten().any(|n| n == "Noto Sans Arabic Regular"));
}

#[test]
fn named_families_take_their_weight_and_lead_script() {
    let stack = fonts("ja")
        .app_face("ui-semibold", FontData::from_static(&[]), &[FontFamily::Name("semibold".into())])
        .family(Family::named("semibold").weight(600))
        .family(Family::named("arabic").script("Arab"))
        .stack();
    let semibold = FontFamily::Name("semibold".into());
    assert_eq!(head(&stack, &semibold, 2), ["BIZ UDPGothic Bold", "BIZ UDPGothic Regular"]);
    assert!(stack.chain(&semibold).contains(&"ui-semibold".to_owned()));
    // A family without an app face of its own falls back to the proportional faces.
    let arabic = FontFamily::Name("arabic".into());
    assert_eq!(head(&stack, &arabic, 2), ["Noto Sans Arabic Regular", "BIZ UDPGothic Regular"]);
    assert!(stack.chain(&arabic).contains(&"ui".to_owned()));
}

#[test]
fn system_fonts_before_craft_moves_other_scripts_after_them() {
    let emoji = Emoji { color: Some(FontData::from_static(&[])), web: None, monochrome: Some(FontData::from_static(&[])) };
    let stack = fonts("ja").system_fonts(SystemFonts::BeforeCraft).emoji(emoji).stack();
    let configured = stack.chain(&FontFamily::Proportional);
    assert!(configured.contains(&"BIZ UDPGothic Regular".to_owned()));
    assert!(!configured.contains(&"Noto Sans Arabic Regular".to_owned()));
    let after = stack.after_system.chain(&FontFamily::Proportional);
    let position = |name: &str| after.iter().position(|n| n == name);
    let (arabic, color, mono) = (position("Noto Sans Arabic Regular"), position("emoji-color"), position("emoji-monochrome"));
    assert!(arabic.is_some() && arabic < color && color < mono, "{after:?}");
}

fn test_stack(ui_language: &str) -> Stack {
    let defaults = FontDefinitions::default();
    let ubuntu = defaults.font_data.get("Ubuntu-Light").map(|d| (**d).clone()).expect("egui default_fonts in dev-dependencies");
    UiFonts::new(&TestScripts)
        .app_face("ui", ubuntu, &[FontFamily::Proportional])
        .craft_faces(&[
            CraftFace { family: "Noto Sans Arabic", style: "Regular", scripts: &["Arab"], bytes: ARABIC, index: 0 },
            CraftFace { family: "Noto Sans CJK SC", style: "Regular", scripts: &["Hans", "Latn"], bytes: CJK, index: 0 },
        ])
        .languages(&["en", "ar", "ja", "ka"])
        .ui_language(ui_language)
        .system_fonts(SystemFonts::Off)
        .stack()
}

#[test]
fn coverage_names_language_string_and_character() {
    let stack = test_stack("en");
    let samples = [
        Sample { language: "ar", key: "menu.file", text: "ملف عربي" },
        Sample { language: "ja", key: "menu.file", text: "日本語のファイル" },
        Sample { language: "ka", key: "menu.file", text: "ფაილი" },
    ];
    let missing = missing_glyphs(&stack, &FontFamily::Proportional, samples);
    let chars: Vec<char> = missing.iter().map(|m| m.character).collect();
    assert_eq!(chars, ['ფ', 'ა', 'ი', 'ლ']);
    assert!(missing.iter().all(|m| m.language == "ka" && m.key == "menu.file"));
    assert_eq!(missing[0].to_string(), "ka menu.file: U+10E4 'ფ' in \"ფაილი\"");

    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_covers(&stack, samples))).expect_err("Georgian has no face");
    let message = failure.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.contains("4 characters") && message.contains("ka menu.file: U+10E4"), "{message}");

    assert_covers(&stack, samples.into_iter().take(2));
}

#[test]
fn variable_faces_draw_at_regular_weight() {
    // The Arabic test face's default instance is Thin (100), like many variable system fonts.
    let thin = || FontData::from_static(ARABIC);
    let wght = |data: &FontData| data.tweak.coords.as_ref().iter().find(|(t, _)| *t == egui::epaint::text::Tag::new(b"wght")).map(|(_, v)| *v);
    assert_eq!(wght(&thin()), None);

    let provider = move |request: &FallbackRequest<'_>| {
        Some(FontInsert::new("system:Arabic", thin(), vec![InsertFontFamily { family: request.family.clone(), priority: FontPriority::Lowest }]))
    };
    let wrapped = RegularWeight::new(Arc::new(provider)).with_weight(FontFamily::Name("bold".into()), 700);
    let regular = wrapped.font_for(&FallbackRequest { cluster: "ع", family: &FontFamily::Proportional }).expect("found");
    assert_eq!((regular.name.as_str(), wght(&regular.data)), ("system:Arabic", Some(400.0)));
    let bold = wrapped.font_for(&FallbackRequest { cluster: "ع", family: &FontFamily::Name("bold".into()) }).expect("found");
    assert_eq!((bold.name.as_str(), wght(&bold.data)), ("system:Arabic @700", Some(700.0)));

    // Craft faces get the same treatment, and a weight the app set stays.
    let stack = test_stack("ar");
    assert_eq!(stack.definitions.font_data.get("Noto Sans Arabic Regular").map(|d| wght(d)), Some(Some(400.0)));
    let mut set = thin();
    set.tweak.coords.push(*b"wght", 300.0);
    assert!(!set_weight(&mut set, 400));
}

#[test]
fn widgets_grow_to_the_ui_language_row_height() {
    let mut style = Style::default();
    let before = style.spacing.interact_size.y;
    test_stack("en").fit_interact_size(&mut style);
    assert_eq!(style.spacing.interact_size.y, before, "Latin rows fit egui's default");

    let stack = test_stack("ar");
    stack.fit_interact_size(&mut style);
    let mut fonts = stack.fonts(false);
    let body = style.text_styles.get(&TextStyle::Body).cloned().expect("body style");
    let row = fonts.with_pixels_per_point(1.0).row_height(&body);
    assert!(row + 2.0 * style.spacing.button_padding.y > before, "Arabic rows are taller");
    assert!(style.spacing.interact_size.y >= row + 2.0 * style.spacing.button_padding.y);
}
