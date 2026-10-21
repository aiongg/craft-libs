use super::*;
use egui::{FallbackRequest, FontInsert, FontPriority, InsertFontFamily};

const ARABIC: &[u8] = include_bytes!("../../tests/fonts/NotoSansArabic-test.ttf");
const CJK: &[u8] = include_bytes!("../../tests/fonts/NotoSansCJKsc-test.otf");

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
    UiFonts::new(&I18nScripts)
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
    // CJK UIs keep the Latin face first: Han, kana and Hangul fit its rows.
    let cases: &[(&str, &[&str])] = &[
        ("ar", &["Noto Sans Arabic Regular", "ui"]),
        ("ja", &["ui", "BIZ UDPGothic Regular", "Shippori Mincho Regular", "Noto Sans CJK SC Regular"]),
        ("zh-TW", &["ui", "Noto Sans CJK TC Regular", "Noto Sans CJK HK Regular", "Noto Sans CJK SC Regular"]),
        ("zh-HK", &["ui", "Noto Sans CJK HK Regular", "Noto Sans CJK TC Regular", "Noto Sans CJK SC Regular"]),
        ("zh-CN", &["ui", "Noto Sans CJK SC Regular", "Noto Sans CJK TC Regular"]),
        // craft-fonts' preference for BIZ UDPGothic orders Japanese faces only where they are
        // the best region: for Korean they keep manifest order.
        ("ko", &["ui", "Noto Sans CJK KR Regular", "Shippori Mincho Regular", "BIZ UDPGothic Regular"]),
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
fn families_without_app_faces_start_with_egui_defaults() {
    let defaults = FontDefinitions::default();
    let first = |family: &FontFamily| defaults.families.get(family).and_then(|c| c.first()).cloned().expect("egui default_fonts in dev-dependencies");
    let (egui_ui, egui_mono) = (first(&FontFamily::Proportional), first(&FontFamily::Monospace));
    let only_ui = |language| fonts_without_faces(language).app_face("ui", FontData::from_static(&[]), &[FontFamily::Proportional]).stack();
    let no_faces = |language| fonts_without_faces(language).stack();

    let stack = only_ui("en");
    assert_eq!(head(&stack, &FontFamily::Proportional, 2), ["ui", "Noto Sans Arabic Regular"]);
    assert_eq!(head(&stack, &FontFamily::Monospace, 1), [egui_mono.as_str()], "egui's mono face, not a craft face");
    let stack = only_ui("ar");
    assert_eq!(head(&stack, &FontFamily::Proportional, 2), ["Noto Sans Arabic Regular", "ui"]);
    assert_eq!(head(&stack, &FontFamily::Monospace, 1), [egui_mono.as_str()]);

    // egui's defaults come right after the UI language's faces and before other scripts.
    for (language, lead) in [("en", None), ("ar", Some("Noto Sans Arabic Regular"))] {
        let stack = no_faces(language);
        let chain = stack.chain(&FontFamily::Proportional);
        let egui_at = chain.iter().position(|n| *n == egui_ui).expect("egui's face");
        assert_eq!(chain.first().map(String::as_str), Some(lead.unwrap_or(&egui_ui)), "{language}");
        let other = if lead.is_some() { "BIZ UDPGothic Regular" } else { "Noto Sans Arabic Regular" };
        assert!(chain.iter().position(|n| n == other) > Some(egui_at), "{language}: {chain:?}");
        assert_eq!(head(&stack, &FontFamily::Monospace, 1), [egui_mono.as_str()], "{language}");
    }
}

fn fonts_without_faces(ui_language: &str) -> UiFonts<'static> {
    UiFonts::new(&I18nScripts).craft_faces(CRAFT).languages(OFFERED).ui_language(ui_language).system_fonts(SystemFonts::Off)
}

#[test]
fn craft_preference_orders_the_best_region_only() {
    // With only Japanese faces, they are the best Han region a Taiwanese UI has.
    let stack = UiFonts::new(&I18nScripts).craft_faces(&CRAFT[..3]).ui_language("zh-TW").system_fonts(SystemFonts::Off).stack();
    let chain = stack.chain(&FontFamily::Proportional);
    let at = |name: &str| chain.iter().position(|n| n == name);
    assert!(at("BIZ UDPGothic Regular") < at("Shippori Mincho Regular"), "{chain:?}");
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
    assert_eq!(head(&stack, &FontFamily::Proportional, 3), ["ui", "Shippori Mincho Regular", "BIZ UDPGothic Regular"]);

    // An app whose UI face covers Arabic keeps it first.
    let stack = fonts("ar").app_scripts(&["Latn", "Arab"]).stack();
    assert!(stack.script_face.is_none(), "{:?}", stack.script_face);
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
        .family(Family::monospace().weight(500))
        .stack();
    assert!(stack.weights().contains(&(FontFamily::Monospace, 500)));
    let semibold = FontFamily::Name("semibold".into());
    assert_eq!(head(&stack, &semibold, 3), ["ui-semibold", "BIZ UDPGothic Bold", "BIZ UDPGothic Regular"]);
    // A family without an app face of its own falls back to the proportional faces.
    let arabic = FontFamily::Name("arabic".into());
    assert_eq!(head(&stack, &arabic, 3), ["Noto Sans Arabic Regular", "ui", "BIZ UDPGothic Regular"]);
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
    test_fonts(ui_language).stack()
}

fn test_fonts(ui_language: &str) -> UiFonts<'static> {
    let defaults = FontDefinitions::default();
    let ubuntu = defaults.font_data.get("Ubuntu-Light").map(|d| (**d).clone()).expect("egui default_fonts in dev-dependencies");
    UiFonts::new(&I18nScripts)
        .app_face("ui", ubuntu, &[FontFamily::Proportional])
        .craft_faces(&[
            CraftFace { family: "Noto Sans Arabic", style: "Regular", scripts: &["Arab"], bytes: ARABIC, index: 0 },
            CraftFace { family: "Noto Sans CJK SC", style: "Regular", scripts: &["Hans", "Latn"], bytes: CJK, index: 0 },
        ])
        .languages(&["en", "ar", "ja", "ka"])
        .ui_language(ui_language)
        .system_fonts(SystemFonts::Off)
}

#[test]
fn coverage_names_language_string_and_character() {
    let stack = test_stack("en");
    let samples = [
        // With U+061C ARABIC LETTER MARK and U+034F COMBINING GRAPHEME JOINER, which draw nothing.
        Sample { language: "ar", key: "menu.file", text: "ملف\u{061C} عر\u{034F}بي" },
        Sample { language: "ja", key: "menu.file", text: "日本語のファイル" },
        Sample { language: "ka", key: "menu.file", text: "ფაილი" },
    ];
    let missing = missing_glyphs(&stack, &FontFamily::Proportional, samples, false);
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
fn remove_and_replace_reach_the_faces_after_the_system_fonts() {
    let arabic = [Sample { language: "ar", key: "menu.file", text: "ملف" }];
    let stack = || test_fonts("en").system_fonts(SystemFonts::BeforeCraft).stack();
    let draws_arabic = |stack: &Stack| missing_glyphs(stack, &FontFamily::Proportional, arabic, false).is_empty();
    let mut removed = stack();
    assert!(removed.after_system.contains("Noto Sans Arabic Regular"));
    assert!(draws_arabic(&removed));
    removed.remove("Noto Sans Arabic Regular");
    assert!(!draws_arabic(&removed));
    let mut replaced = stack();
    replaced.replace("Noto Sans Arabic Regular", FontData::from_static(CJK));
    assert!(!draws_arabic(&replaced));
}

#[test]
fn face_list_prefers_a_face_with_the_whole_cluster() {
    let defaults = FontDefinitions::default();
    let mut faces = FaceList::default();
    for name in ["Ubuntu-Light", "Hack"] {
        faces.insert(name, (**defaults.font_data.get(name).expect("egui default_fonts in dev-dependencies")).clone());
    }
    faces.set_chain(FontFamily::Proportional, vec!["Ubuntu-Light".into(), "Hack".into()]);
    let face = |cluster| faces.font_for(&FallbackRequest { cluster, family: &FontFamily::Proportional }).map(|insert| insert.name);
    assert_eq!(face("a").as_deref(), Some("Ubuntu-Light"));
    // Only Hack has U+0301 COMBINING ACUTE ACCENT, and neither has U+0302.
    assert_eq!(face("a\u{0301}").as_deref(), Some("Hack"));
    assert_eq!(face("a\u{0302}").as_deref(), Some("Ubuntu-Light"));
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

#[test]
fn widgets_fit_the_cjk_face_that_follows_the_latin_one() {
    let stack = test_stack("ja");
    assert_eq!(stack.chain(&FontFamily::Proportional).first().map(String::as_str), Some("ui"));
    let mut style = Style::default();
    let before = style.spacing.interact_size.y;
    stack.fit_interact_size(&mut style);
    assert!(style.spacing.interact_size.y > before, "CJK rows are taller than the Latin face's");
}

#[test]
fn installing_another_language_refits_widgets() {
    let ctx = egui::Context::default();
    let before = ctx.style_of(egui::Theme::Dark).spacing.interact_size.y;
    test_stack("ar").install(&ctx);
    let arabic = ctx.style_of(egui::Theme::Dark).spacing.interact_size.y;
    assert!(arabic > before);
    test_stack("en").install(&ctx);
    assert_eq!(ctx.style_of(egui::Theme::Dark).spacing.interact_size.y, before, "back to the style's own height");
    assert_eq!(ctx.style_of(egui::Theme::Light).spacing.interact_size.y, before);
    // A height the app set after installing stays.
    ctx.all_styles_mut(|s| s.spacing.interact_size.y = 40.0);
    test_stack("ar").install(&ctx);
    test_stack("en").install(&ctx);
    assert_eq!(ctx.style_of(egui::Theme::Dark).spacing.interact_size.y, 40.0);
}

#[test]
fn install_keeps_the_context_providers_unless_it_brings_a_system_lookup() {
    let arabic = |_: &FallbackRequest<'_>| {
        Some(FontInsert::new(
            "host:Arabic",
            FontData::from_static(ARABIC),
            vec![InsertFontFamily { family: FontFamily::Proportional, priority: FontPriority::Lowest }],
        ))
    };
    let draws_arabic = |system: SystemFonts| {
        let ctx = egui::Context::default();
        ctx.add_font_provider(Arc::new(arabic));
        let defaults = FontDefinitions::default();
        let ubuntu = defaults.font_data.get("Ubuntu-Light").map(|d| (**d).clone()).expect("egui default_fonts");
        let stack = UiFonts::new(&I18nScripts).app_face("ui", ubuntu, &[FontFamily::Proportional]).system_fonts(system).stack();
        stack.install(&ctx);
        stack.install(&ctx);
        ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        ctx.fonts_mut(|f| f.has_glyph(&FontId::proportional(14.0), 'ع'))
    };
    assert!(!draws_arabic(SystemFonts::Off), "Off asks no system fonts");
    if cfg!(not(all(feature = "system_fonts", not(target_arch = "wasm32")))) {
        assert!(draws_arabic(SystemFonts::AfterCraft), "the host's provider stays");
    }
}

#[cfg(all(feature = "system_fonts", not(target_arch = "wasm32")))]
#[test]
fn the_system_provider_is_shared_per_locale() {
    let (a, b) = (system_font_provider("ja"), system_font_provider("ja"));
    assert!(a.zip(b).is_some_and(|(a, b)| Arc::ptr_eq(&a, &b)));
}

#[test]
fn egui_faces_join_the_families_the_app_names() {
    let semibold = FontFamily::Name("semibold".into());
    let stack = fonts("en")
        .family(Family::named("semibold").weight(600))
        .egui_face("Hack", &[FontFamily::Proportional, semibold.clone()])
        .egui_face("no such face", &[FontFamily::Proportional])
        .stack();
    let proportional = stack.chain(&FontFamily::Proportional);
    assert_eq!(proportional.last().map(String::as_str), Some("Hack"), "{proportional:?}");
    assert!(stack.chain(&semibold).iter().any(|n| n == "Hack"));
    assert!(!stack.chain(&FontFamily::Monospace).iter().any(|n| n == "Hack"), "only the families named");
    assert!(stack.definitions.font_data.contains_key("Hack"));
    assert!(!proportional.iter().any(|n| n == "no such face"));
}
