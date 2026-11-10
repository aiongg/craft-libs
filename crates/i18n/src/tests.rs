//! Tests for the i18n facade.

use super::*;

impl I18n {
    /// The embedded locales parsed so far, by code.
    fn parsed(&self) -> Vec<&str> {
        let mut codes: Vec<&str> = self.locales.iter().skip(2).filter(|l| l.parsed.get().is_some()).map(|l| l.code.as_str()).collect();
        codes.sort_unstable();
        codes
    }
}

static E: &Embedded = &[
    ("fr", "\tSave\tEnregistrer\n\tOpen\tOuvrir\nmenu\tOpen\tOuvrir le menu\n@id\tfile.save\tEnregistrer le fichier\n", "\tSave\tSauvegarder\n"),
    ("pt", "\tSave\tGuardar\n\tUndo\tDesfazer\n", ""),
    ("pt-br", "\tSave\tSalvar\n", ""),
    ("de", "\tBad\n@nope\tA\tB\n\tUndo\tRückgängig\n", ""),
];

/// `embedded`, offering every locale in it, with `en-US` numbers rather than the machine's.
fn all(embedded: &'static Embedded) -> (I18n, Vec<String>) {
    let codes: Vec<&str> = embedded.iter().map(|(c, _, _)| *c).collect();
    let (i, problems) = I18n::new(embedded, &codes);
    i.set_format(Format::from_tag("en-US"));
    (i, problems)
}

fn i18n() -> I18n {
    all(E).0
}

fn lang(i: &I18n, code: &str) -> Lang {
    i.from_code(code).unwrap()
}

#[test]
fn app_row_overrides_shared() {
    let i = i18n();
    assert_eq!(i.t(lang(&i, "fr"), "Save"), "Sauvegarder");
}

#[test]
fn shared_row_when_no_override() {
    let i = i18n();
    assert_eq!(i.t(lang(&i, "fr"), "Open"), "Ouvrir");
}

#[test]
fn region_falls_back_to_base() {
    let i = i18n();
    assert_eq!(i.t(lang(&i, "pt-br"), "Undo"), "Desfazer");
    assert_eq!(i.t(lang(&i, "pt-br"), "Save"), "Salvar");
}

#[test]
fn missing_is_english() {
    let i = i18n();
    assert_eq!(i.t(lang(&i, "fr"), "Close"), "Close");
    assert_eq!(i.t(Lang::EN, "Save"), "Save");
}

#[test]
fn context_then_plain() {
    let i = i18n();
    assert_eq!(i.tc(lang(&i, "fr"), "menu", "Open"), "Ouvrir le menu");
    assert_eq!(i.tc(lang(&i, "fr"), "menu", "Save"), "Sauvegarder");
}

#[test]
fn id_then_label() {
    let i = i18n();
    assert_eq!(i.tid(lang(&i, "fr"), "file.save", "Save"), "Enregistrer le fichier");
    assert_eq!(i.tid(lang(&i, "fr"), "file.nope", "Open"), "Ouvrir");
}

#[test]
fn bad_rows_skipped_and_reported() {
    let i = i18n();
    assert_eq!(i.t(lang(&i, "de"), "Undo"), "Rückgängig");
    assert_eq!(i.check_all().len(), 2);
}

#[test]
fn from_code_is_case_insensitive() {
    let i = i18n();
    assert_eq!(i.from_code("PT-BR"), i.from_code("pt-br"));
    assert_eq!(i.from_code("xx"), None);
    assert_eq!(i.from_code("en"), Some(Lang::EN));
    assert_eq!(i.code(Lang::EN), "en");
}

static P: &Embedded = &[
    ("fr", "@plural\t{n} page|{n} pages\t{n} page|{n} pages\n", ""),
    ("ru", "@plural\t{n} file|{n} files\t{n} файл|{n} файла|{n} файлов\n", ""),
    ("ja", "@plural\t{n} file|{n} files\t{n} 個\n", ""),
    ("ar", "@plural\t{n} file|{n} files\t{n} ملفات|{n} ملف واحد|{n} ملفان|{n} ملفات|{n} ملفًا|{n} ملف\n", ""),
    ("fa", "@plural\t{n} page|{n} pages\t{n} صفحه|{n} صفحه\n", ""),
];

#[test]
fn fmt_fills_once() {
    assert_eq!(fmt("{a} to {b}", &[("a", "x"), ("b", "{a}")]), "x to {a}");
    assert_eq!(fmt("{z} {", &[]), "{z} {");
}

#[test]
fn left_to_right_text_gets_no_marks() {
    assert_eq!(fmt("Saved {name} in {dir}.", &[("name", "Report 2.pdf"), ("dir", &ltr("/home/sara"))]), "Saved Report 2.pdf in /home/sara.");
    assert_eq!(fmt("{a}", &[("a", "x"), ("b", "שלום")]), "x", "an unused value doesn't count");
    assert_eq!(fmt_plain("{a}", &[("a", &isolate("x"))]), "x");
    assert_eq!(fmt_plain("\u{2066}{a} {b}", &[("a", "x\u{2069}"), ("b", &isolate(&ltr("y")))]), "x y");
    let i = all(P).0;
    assert_eq!(i.trn(Lang::EN, 2, "{n} page", "{n} pages"), "2 pages");
}

#[test]
fn right_to_left_text_isolates_values() {
    let saved = fmt("تم حفظ {name} في {dir}.", &[("name", "Report 2.pdf"), ("dir", &ltr("/home/سارة/docs"))]);
    assert_eq!(saved, "تم حفظ \u{2068}Report 2.pdf\u{2069} في \u{2066}/home/سارة/docs\u{2069}.");
    assert_eq!(strip_isolates(&saved), "تم حفظ Report 2.pdf في /home/سارة/docs.");
    assert_eq!(fmt("Opened {name}.", &[("name", "שלום.txt")]), "Opened \u{2068}שלום.txt\u{2069}.");
    assert_eq!(fmt("[{a}] {b}", &[("a", ""), ("b", "ש")]), "[] \u{2068}ש\u{2069}");
    let i = all(P).0;
    assert_eq!(i.trn(lang(&i, "fa"), 3, "{n} page", "{n} pages"), "\u{2066}3\u{2069} صفحه");
}

#[test]
fn a_value_cannot_escape_its_isolate() {
    assert_eq!(isolate("a\u{2069}b"), "\u{2068}ab\u{2069}");
    assert_eq!(isolate("a\u{2067}b"), "\u{2068}a\u{2067}b\u{2069}\u{2069}");
    let one = ltr("12");
    assert_eq!(fmt("ש {n}", &[("n", &one)]), format!("ש {one}"));
    // Two isolates side by side are not one, so they get wrapped.
    assert_eq!(fmt("ש{v}", &[("v", "\u{2066}a\u{2069}\u{2066}b\u{2069}")]), "ש\u{2068}\u{2066}a\u{2069}\u{2066}b\u{2069}\u{2069}");
}

/// `text` in display order, as one line in a left-to-right paragraph, without bidi controls.
fn visual(text: &str) -> String {
    let info = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
    let para = &info.paragraphs[0];
    let shown = info.reorder_line(para, para.range.clone());
    shown
        .chars()
        .filter(|c| bidi_class(*c) != BidiClass::BN && !"\u{202A}\u{202B}\u{202C}\u{202D}\u{202E}\u{2066}\u{2067}\u{2068}\u{2069}".contains(*c))
        .collect()
}

#[test]
fn a_value_with_bidi_controls_cannot_reorder_the_sentence() {
    let file_names = [
        "\u{202E}fdp.exe",
        "\u{202B}abc def",
        "\u{202D}abc def",
        "\u{2067}abc def",
        "\u{2068}abc\u{2069}\u{202E}def",
        "\u{202E}abc\u{2069}",
        "x\u{200F}",
        "x\u{061C}",
    ];
    for name in file_names {
        let text = fmt("Open {name} -> {name} now?", &[("name", name)]);
        let shown = visual(&text);
        assert!(shown.starts_with("Open ") && shown.contains(" -> ") && shown.ends_with(" now?"), "{name:?}: {shown:?}");
        let i = all(P).0;
        let wording = i.trn(Lang::EN, 2, "{n} page in {name}, done", "{n} pages in {name}, done");
        let shown = visual(&fmt(&wording, &[("name", name)]));
        assert!(shown.starts_with("2 pages in ") && shown.ends_with(", done"), "{name:?}: {shown:?}");
    }
    // An override the value opens is closed inside its isolate, and a stray PDF dropped.
    assert_eq!(fmt("Open {name}.", &[("name", "\u{202E}abc")]), "Open \u{2068}\u{202E}abc\u{202C}\u{2069}.");
    assert_eq!(fmt("Open {name}.", &[("name", "a\u{202C}b")]), "Open \u{2068}ab\u{2069}.");
    assert_eq!(fmt("Open {n}.", &[("n", &ltr("\u{202B}1"))]), "Open \u{2066}\u{202B}1\u{202C}\u{2069}.");
    assert_eq!(isolate("\u{2067}a\u{202E}b\u{2069}c"), "\u{2068}\u{2067}a\u{202E}b\u{202C}\u{2069}c\u{2069}");
}

#[test]
fn one_other() {
    let (i, problems) = all(P);
    assert!(problems.is_empty(), "{problems:?}");
    let fr = lang(&i, "fr");
    assert_eq!(i.trn(fr, 0, "{n} page", "{n} pages"), "0 page");
    assert_eq!(i.trn(fr, 2, "{n} page", "{n} pages"), "2 pages");
}

#[test]
fn three_forms_ru() {
    let i = all(P).0;
    let ru = lang(&i, "ru");
    for (n, want) in [(1, "1 файл"), (3, "3 файла"), (5, "5 файлов"), (11, "11 файлов"), (21, "21 файл")] {
        assert_eq!(i.trn(ru, n, "{n} file", "{n} files"), want);
    }
}

#[test]
fn one_form_ja() {
    let i = all(P).0;
    assert_eq!(i.trn(lang(&i, "ja"), 7, "{n} file", "{n} files"), "7 個");
}

#[test]
fn english_fallback() {
    let i = all(P).0;
    assert_eq!(i.trn(Lang::EN, 1, "{n} file", "{n} files"), "1 file");
    assert_eq!(i.trn(lang(&i, "fr"), 2, "{n} x", "{n} xs"), "2 xs");
}

// One region call sets the numbers and the dates: de-DE writes `1.234` and `Oktober 2026`.
#[test]
fn set_region_sets_numbers_and_dates() {
    let i = all(P).0;
    i.set_region(&Region::from_tag("de-DE"), Digits::Locale);
    assert_eq!(i.format().integer(1234), "1.234");
    assert_eq!(i.dates().year_month(2026, 10), "Oktober 2026");
    i.set_region(&Region::from_tag("ar-EG"), Digits::Locale);
    assert_eq!(i.format().integer(12), "\u{661}\u{662}");
    assert!(i.dates().year_month(2026, 10).contains('\u{662}'), "the locale's digits in dates");
    i.set_region(&Region::from_tag("ar-EG"), Digits::Latin);
    assert_eq!(i.format().integer(12), "12");
    let at = DateTime { date: CalendarDate { year: 2026, month: 10, day: 31 }, hour: 14, minute: 5, second: 0 };
    let text = i.dates().format(at, DateStyle::DATE_TIME);
    assert!(text.contains("2026") && !text.chars().any(|c| ('\u{660}'..='\u{669}').contains(&c)), "{text}");
    assert!(i.dates().year_month(2026, 10).contains("2026"));
}

#[test]
fn beyond_sweep() {
    let i = all(P).0;
    i.set_format(Format::from_tag("fr"));
    assert_eq!(i.trn(lang(&i, "fr"), 1_000_000, "{n} page", "{n} pages"), "1\u{202f}000\u{202f}000 pages");
}

#[test]
fn counts_follow_the_region() {
    let i = all(P).0;
    i.set_format(Format::from_tag("ar-EG"));
    assert_eq!(i.trn(lang(&i, "ar"), 1234, "{n} file", "{n} files"), "\u{2066}١٬٢٣٤\u{2069} ملفًا");
    i.set_format(Format::new(&Region::from_tag("ar-EG"), Digits::Latin));
    assert_eq!(i.trn(lang(&i, "ar"), 1234, "{n} file", "{n} files"), "\u{2066}1,234\u{2069} ملفًا");
    // An English interface with a German region.
    i.set_format(Format::from_tag("de-DE"));
    assert_eq!(i.trn(Lang::EN, 12_345, "{n} file", "{n} files"), "12.345 files");
}

#[test]
fn plain_counts_and_the_system_format() {
    let i = all(P).0;
    i.set_format(Format::from_tag("ar-EG"));
    assert_eq!(i.trn_plain(lang(&i, "ar"), 1234, "{n} file", "{n} files"), "1234 ملفًا");
    i.set_format(Format::from_tag("de-DE"));
    assert_eq!(i.trn_plain(Lang::EN, 12_345, "copy {n}", "copy {n}"), "copy 12345");
    i.reset_format();
    assert_eq!(i.trn(Lang::EN, 5, "{n} file", "{n} files"), fmt("{n} files", &[("n", &Format::new(&Region::system(), Digits::Locale).integer(5))]));
}

#[test]
fn decimal_counts() {
    let i = all(P).0;
    i.set_format(Format::from_tag("en"));
    assert_eq!(i.trn_decimal(Lang::EN, 1.5, 1, "{n} file", "{n} files"), "1.5 files");
    assert_eq!(i.trn_decimal(Lang::EN, 1.0, 1, "{n} file", "{n} files"), "1.0 files", "visible decimals are plural");
    assert_eq!(i.trn_decimal(Lang::EN, 1.0, 0, "{n} file", "{n} files"), "1 file");
    // Russian decimals are `other`, which integers never reach: the `few` form.
    i.set_format(Format::from_tag("ru"));
    let ru = lang(&i, "ru");
    assert_eq!(i.trn_decimal(ru, 1.5, 1, "{n} file", "{n} files"), "1,5 файла");
    assert_eq!(i.trn_decimal(ru, 21.0, 0, "{n} file", "{n} files"), "21 файл");
    // Arabic decimals are `other`.
    i.set_format(Format::from_tag("ar-EG"));
    let ar = lang(&i, "ar");
    assert_eq!(i.trn_decimal(ar, 2.5, 1, "{n} file", "{n} files"), "\u{2066}٢٫٥\u{2069} ملف");
    assert_eq!(i.trn_decimal(ar, 3.0, 0, "{n} file", "{n} files"), "\u{2066}٣\u{2069} ملفات");
    // Not a number: the translated `other` form.
    assert_eq!(i.trn_decimal(ru, f64::NAN, 1, "{n} file", "{n} files"), "NaN файла");
    assert_eq!(i.trn_decimal(Lang::EN, f64::INFINITY, 1, "{n} file", "{n} files"), "∞ files");
}

static L: &Embedded = &[("fr", "@id\tlanguage.name\tFrançais\n", ""), ("pt-br", "", ""), ("zh-hans", "", ""), ("zh-hant", "", ""), ("ar", "", "")];

#[test]
fn languages_order() {
    let i = all(L).0;
    let codes: Vec<&str> = i.languages().into_iter().map(|l| i.code(l)).collect();
    assert_eq!(codes, ["en", "ar", "fr", "pt-br", "zh-hans", "zh-hant"]);
}

#[test]
fn names() {
    let i = all(L).0;
    assert_eq!(i.name(Lang::EN), "English");
    assert_eq!(i.name(lang(&i, "fr")), "Français");
    assert_eq!(i.name(lang(&i, "ar")), "ar");
}

#[test]
fn rtl() {
    let i = all(L).0;
    assert!(i.rtl(lang(&i, "ar")));
    assert!(!i.rtl(lang(&i, "fr")));
    assert!(!i.rtl(Lang::EN));
}

#[test]
fn tags() {
    let i = all(L).0;
    for (tag, want) in [
        ("fr_CA.UTF-8", Some("fr")),
        ("pt-PT", Some("pt-br")),
        ("pt_BR", Some("pt-br")),
        ("zh_TW.UTF-8", Some("zh-hant")),
        ("zh-HK", Some("zh-hant")),
        ("zh", Some("zh-hans")),
        ("zh-Hant-MO", Some("zh-hant")),
        ("zh_CN", Some("zh-hans")),
        ("zh-US", Some("zh-hant")),
        ("zh_SG", Some("zh-hans")),
        ("C", Some("en")),
        ("POSIX", Some("en")),
        ("en_US", Some("en")),
        ("ja_JP", None),
        ("", None),
        ("@@", None),
    ] {
        assert_eq!(i.match_tag(tag).map(|l| i.code(l)), want, "{tag}");
    }
}

#[test]
fn region_implies_script() {
    assert_eq!(candidates("pa-PK"), ["pa-pk", "pa-arab-pk", "pa-arab"]);
    assert_eq!(candidates("pa_PK.UTF-8"), ["pa-pk", "pa-arab-pk", "pa-arab"]);
    assert_eq!(candidates("pa-IN"), ["pa-in", "pa"]);
    assert_eq!(candidates("sd-IN"), ["sd-in", "sd-deva-in", "sd-deva"]);
    assert_eq!(candidates("pa-Guru-PK"), ["pa-guru-pk", "pa-guru", "pa"]);
    assert_eq!(candidates("zh-US"), ["zh-us", "zh-hant", "zh"]);
    assert_eq!(candidates("zh-SG"), ["zh-sg", "zh-hans", "zh"]);
    assert_eq!(candidates("kk-CN"), ["kk-cn", "kk-arab-cn", "kk-arab"]);
    assert_eq!(candidates("kk-KZ"), ["kk-kz", "kk"]);
    assert_eq!(candidates("ug-KZ"), ["ug-kz", "ug-cyrl-kz", "ug-cyrl"]);
}

#[test]
fn match_tags_takes_first_match() {
    let i = all(L).0;
    assert_eq!(i.match_tags(["ja_JP", "fr_FR"]), lang(&i, "fr"));
    assert_eq!(i.match_tags(["ja"]), Lang::EN);
    assert_eq!(i.match_tags([]), Lang::EN);
}

#[test]
fn pseudo_marks_every_lookup() {
    let i = i18n();
    assert_eq!(i.t(Lang::PSEUDO, "Save"), "⟦Save⟧");
    assert_eq!(i.t(Lang::PSEUDO, "A\nB"), "⟦A⟧\n⟦B⟧");
    assert_eq!(i.tc(Lang::PSEUDO, "menu", "Open"), "⟦Open⟧");
    assert_eq!(i.tid(Lang::PSEUDO, "file.save", "Save"), "⟦Save⟧");
    assert_eq!(i.trn(Lang::PSEUDO, 2, "{n} x", "{n} xs"), "⟦2 xs⟧");
    assert!(!i.languages().contains(&Lang::PSEUDO));
    assert_eq!(i.from_code("pseudo"), cfg!(debug_assertions).then_some(Lang::PSEUDO));
    assert_eq!(i.get(Lang::PSEUDO, "key", "Esc"), Some("⟦Esc⟧"));
    assert_eq!(i.looked_up().into_iter().collect::<Vec<_>>(), ["A\nB", "Esc", "Open", "Save"]);
}

static F: &Embedded =
    &[("pt", "", ""), ("pt-br", "", ""), ("sr", "\tSave\tСачувај\n", ""), ("sr-latn", "", ""), ("fr", "\tDelete\tSupprimer\nkey\tEsc\tÉchap\n", "")];

#[test]
fn european_portuguese_prefers_pt() {
    let i = all(F).0;
    for tag in ["pt", "pt-PT", "pt_PT.UTF-8"] {
        assert_eq!(i.match_tag(tag).map(|l| i.code(l)), Some("pt"), "{tag}");
    }
    assert_eq!(i.match_tag("pt_BR").map(|l| i.code(l)), Some("pt-br"));
}

#[test]
fn script_locale_does_not_fall_back_across_scripts() {
    let i = all(F).0;
    assert_eq!(i.t(lang(&i, "sr-latn"), "Save"), "Save");
}

#[test]
fn glibc_script_modifier_picks_the_script() {
    let i = all(F).0;
    for (tag, want) in [("sr_RS.UTF-8@latin", Some("sr-latn")), ("sr_RS@latin", Some("sr-latn")), ("sr_RS.UTF-8", Some("sr"))] {
        assert_eq!(i.match_tag(tag).map(|l| i.code(l)), want, "{tag}");
    }
    let cyrillic_only = all(&[("sr", "", "")]).0;
    assert_eq!(cyrillic_only.match_tag("sr_RS@latin"), None);
}

#[cfg(all(unix, not(any(target_vendor = "apple", target_os = "android"))))]
#[test]
fn posix_env_keeps_the_modifier() {
    let env = |vars: &'static [(&'static str, &'static str)]| move |k: &str| vars.iter().find(|(n, _)| *n == k).map(|(_, v)| (*v).to_owned());
    assert_eq!(posix_env_tags(env(&[("LANG", "sr_RS.UTF-8@latin")])), ["sr_RS.UTF-8@latin"]);
    assert_eq!(posix_env_tags(env(&[("LANGUAGE", "fr:de"), ("LC_ALL", ""), ("LANG", "sr_RS@latin")])), ["fr", "de", "sr_RS@latin"]);
    let i = all(F).0;
    assert_eq!(i.code(i.match_tags(posix_env_tags(env(&[("LANG", "sr_RS.UTF-8@latin")])).iter().map(String::as_str))), "sr-latn");
}

#[test]
fn get_has_no_plain_fallback() {
    let i = all(F).0;
    let fr = lang(&i, "fr");
    assert_eq!(i.get(fr, "key", "Esc"), Some("Échap"));
    assert_eq!(i.get(fr, "key", "Delete"), None);
    assert_eq!(i.get(fr, "", "Delete"), Some("Supprimer"));
    assert_eq!(i.get(Lang::EN, "", "Delete"), None);
}

static C: &Embedded = &[("pt", "menu\tOpen\tAbrir menu\n@id\tfile.open\tAbrir ficheiro\n", ""), ("pt-br", "\tOpen\tAbrir\n", "")];

#[test]
fn locale_before_context() {
    let i = all(C).0;
    let br = lang(&i, "pt-br");
    assert_eq!(i.tc(br, "menu", "Open"), "Abrir");
    assert_eq!(i.tid(br, "file.open", "Open"), "Abrir");
    assert_eq!(i.tc(lang(&i, "pt"), "menu", "Open"), "Abrir menu");
}

static PT: &Embedded = &[
    ("pt", "@plural\t{n} file|{n} files\t{n} ficheiro|{n} ficheiros\n", ""),
    ("pt-br", "@plural\t{n} file|{n} files\t{n} arquivo|{n} arquivos\n", ""),
];

#[test]
fn european_portuguese_zero_is_plural() {
    let i = all(PT).0;
    assert_eq!(i.trn(lang(&i, "pt"), 0, "{n} file", "{n} files"), "0 ficheiros");
    assert_eq!(i.trn(lang(&i, "pt"), 1, "{n} file", "{n} files"), "1 ficheiro");
    assert_eq!(i.trn(lang(&i, "pt-br"), 0, "{n} file", "{n} files"), "0 arquivo");
}

/// The counts craft-i18n's checker expects, which rejects a catalog whose `@plural` rows list
/// any other: every directory in craft-i18n's `locales/`, then languages it may add.
#[test]
fn plural_form_counts() {
    #[rustfmt::skip]
    let catalogs = [
        ("ar", 6), ("bg", 2), ("cs", 3), ("da", 2), ("de", 2), ("el", 2), ("es", 2), ("et", 2),
        ("fi", 2), ("fr", 2), ("hr", 3), ("hu", 2), ("id", 1), ("it", 2), ("ja", 1), ("ka", 2),
        ("kk", 2), ("ko", 1), ("lt", 3), ("nl", 2), ("pl", 3), ("pt-br", 2), ("ru", 3), ("sl", 4),
        ("sr", 3), ("sr-latn", 3), ("sv", 2), ("te", 2), ("tr", 2), ("uk", 3), ("vi", 1),
        ("zh-hans", 1), ("zh-hant", 1),
    ];
    let more = [("en", 2), ("pt", 2), ("he", 3), ("ro", 3), ("nb", 2), ("ca", 2), ("sk", 3), ("th", 1), ("ms", 1), ("fa", 2), ("hi", 2)];
    for (locale, n) in catalogs.into_iter().chain(more) {
        assert_eq!(plural_rules(locale).map(|(_, c)| c.len()), Some(n), "{locale}");
    }
    assert!(plural_rules("tlh").is_none(), "no CLDR rules");
}

#[test]
fn serbian_latin_plurals() {
    let i = all(&[("sr-latn", "@plural\t{n} file|{n} files\t{n} fajl|{n} fajla|{n} fajlova\n", "")]).0;
    let sr = lang(&i, "sr-latn");
    let files = |n| i.trn(sr, n, "{n} file", "{n} files");
    assert_eq!((files(1), files(3), files(5)), ("1 fajl".into(), "3 fajla".into(), "5 fajlova".into()));
}

#[test]
fn script_subtag_never_picks_another_script() {
    let i = all(&[("sr", "", ""), ("zh-hant", "", ""), ("fr", "", ""), ("be", "", "")]).0;
    for (tag, want) in [
        ("sr-Latn-RS", None),
        ("sr-Cyrl-RS", Some("sr")),
        ("zh-Hant-TW", Some("zh-hant")),
        ("zh-Hans", None),
        ("fr-Latn-FR", Some("fr")),
        ("be_BY@latin", None),
        ("be_BY", Some("be")),
    ] {
        assert_eq!(i.match_tag(tag).map(|l| i.code(l)), want, "{tag}");
    }
}

#[test]
fn duplicate_row_keeps_the_first() {
    let i = all(&[("fr", "\tSave\tEnregistrer\n\tSave\tSauver\n", "")]).0;
    assert_eq!(i.check_all(), ["fr/common: line 2: defined twice (same context and source text); the first is kept"]);
    assert_eq!(i.t(lang(&i, "fr"), "Save"), "Enregistrer");
}

#[test]
fn reserved_and_repeated_codes_are_skipped() {
    let (i, problems) = all(&[("en", "\tSave\tX\n", ""), ("pseudo", "", ""), ("fr", "\tSave\tEnregistrer\n", ""), ("FR", "\tSave\tY\n", "")]);
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert_eq!(i.t(Lang::EN, "Save"), "Save");
    assert_eq!(i.t(lang(&i, "fr"), "Save"), "Enregistrer");
    assert_eq!(i.languages().len(), 2);
}

#[test]
fn only_offered_locales_are_offered() {
    let (i, _) = I18n::new(&[("ar", "\tSave\tحفظ\n", ""), ("pt", "\tUndo\tDesfazer\n", ""), ("pt-br", "", "")], &["PT-BR"]);
    let codes: Vec<&str> = i.languages().into_iter().map(|l| i.code(l)).collect();
    assert_eq!(codes, ["en", "pt-br"]);
    assert_eq!(i.from_code("ar"), None);
    assert_eq!(i.match_tag("ar_EG"), None);
    assert_eq!(i.match_tag("pt_PT"), i.from_code("pt-br"));
    assert_eq!(i.t(lang(&i, "pt-br"), "Undo"), "Desfazer");
}

#[test]
fn owned_catalogs_serve_like_embedded_ones() {
    let unpacked = vec![
        ("PT".to_owned(), "\tUndo\tDesfazer\n".to_owned(), "@id\tlanguage.name\tPortuguês\n".to_owned()),
        ("pt-br".to_owned(), String::new(), "\tUndo\tDesfazer!\n".to_owned()),
    ];
    let (i, problems) = I18n::from_owned(unpacked, &["pt", "pt-br"]);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(i.t(lang(&i, "pt"), "Undo"), "Desfazer");
    assert_eq!(i.t(lang(&i, "pt-br"), "Undo"), "Desfazer!");
    assert_eq!(i.name(lang(&i, "pt")), "Português");
}

#[test]
fn region_after_script_falls_back() {
    let i = all(&[("zh-hant", "\tSave\t儲存\n", ""), ("zh-hant-hk", "", "")]).0;
    assert_eq!(i.t(lang(&i, "zh-hant-hk"), "Save"), "儲存");
}

#[test]
fn more_right_to_left() {
    let i = all(&[("ps", "", ""), ("ckb", "", ""), ("yi", "", ""), ("ug", "", "")]).0;
    assert!(i.languages().into_iter().skip(1).all(|l| i.rtl(l)));
}

#[test]
fn check_all_reports_every_bad_row() {
    let (i, problems) = all(E);
    assert!(i.parsed().is_empty(), "new parses nothing");
    assert!(problems.iter().all(|p| !p.contains("line")), "{problems:?}");
    let found = i.check_all();
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with("de/common: line 1:"), "{found:?}");
    assert!(found[1].starts_with("de/common: line 2:"), "{found:?}");
    assert_eq!(i.check_all(), found);
    assert_eq!(i.parsed(), ["de", "fr", "pt", "pt-br"]);
}

#[test]
fn lookup_parses_only_what_it_reaches() {
    static LAZY: &Embedded =
        &[("pt", "\tUndo\tDesfazer\n", ""), ("pt-br", "\tSave\tSalvar\n", ""), ("de", "\tUndo\tRückgängig\n", ""), ("fr", "\tUndo\tAnnuler\n", "")];
    let i = all(LAZY).0;
    assert!(i.parsed().is_empty());
    assert_eq!(i.t(lang(&i, "pt-br"), "Save"), "Salvar");
    assert_eq!(i.parsed(), ["pt-br"]);
    assert_eq!(i.t(lang(&i, "pt-br"), "Undo"), "Desfazer");
    assert_eq!(i.parsed(), ["pt", "pt-br"]);
    assert_eq!(i.t(lang(&i, "pt-br"), "Undo"), "Desfazer");
    assert_eq!(i.parsed(), ["pt", "pt-br"]);
    assert_eq!(i.t(Lang::EN, "Undo"), "Undo");
    assert_eq!(i.t(Lang::PSEUDO, "Undo"), "⟦Undo⟧");
    assert_eq!(i.parsed(), ["pt", "pt-br"]);
}

#[test]
fn name_reads_escapes_without_parsing() {
    let i = all(&[("fr", "@id\tlanguage.name\tFran\\\\ais\\tx\n", ""), ("de", "\tA\tB\n@id\tlanguage.name\tDeutsch\n", "")]).0;
    assert_eq!(i.name(lang(&i, "fr")), "Fran\\ais\tx");
    assert_eq!(i.name(lang(&i, "de")), "Deutsch");
    assert!(i.parsed().is_empty());
}

#[test]
fn app_name_overrides_shared() {
    let i = all(&[
        ("fr", "@id\tlanguage.name\tFrançais\n", "@id\tlanguage.name\tFrançais (app)\n"),
        ("de", "@id\tlanguage.name\tDeutsch\n", "\tA\tB\n"),
        ("es", "", ""),
    ])
    .0;
    assert_eq!(i.name(lang(&i, "fr")), "Français (app)");
    assert_eq!(i.name(lang(&i, "de")), "Deutsch");
    assert_eq!(i.name(lang(&i, "es")), "es");
}

#[test]
fn i18n_is_send_and_sync() {
    fn check<T: Send + Sync>() {}
    check::<I18n>();
}

/// Prints what this machine reports, so CI logs show `system_lang` on each platform.
#[test]
fn system_lang_reads_the_system() {
    let i = all(&[("fr", "", ""), ("de", "", ""), ("pt-br", "", ""), ("sr-latn", "", "")]).0;
    let first = i.system_lang();
    println!("sys-locale: {:?}; system_lang: {}", sys_locale::get_locales().collect::<Vec<_>>(), i.code(first));
    assert_eq!(i.system_lang(), first);
}
