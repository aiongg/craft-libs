use std::sync::Arc;

use craft_i18n::{Embedded, I18n, Lang};
use egui::Context;

use super::words::problems;
use super::{Labels, Word, Words};

crate::words! {
    struct Probe {
        grid: "probe.gridView" = "Grid View";
        name: "probe.name" = "Name {name}";
        /// Looked up by its English text.
        opacity: "" = "Opacity";
    }
}

static CATALOGS: &Embedded = &[
    ("de", "\tOpacity\tDeckkraft\n@id\tprobe.gridView\tRasteransicht\n\tGrid View\tRaster\n", ""),
    ("ru", "@plural\t{n} file|{n} files\t{n} файл|{n} файла|{n} файлов\n", ""),
];

fn catalog() -> (Arc<I18n>, Lang, Lang) {
    let (i18n, problems) = I18n::new(CATALOGS, &["de", "ru"]);
    assert!(problems.is_empty(), "{problems:?}");
    let de = i18n.from_code("de").unwrap();
    let ru = i18n.from_code("ru").unwrap();
    (Arc::new(i18n), de, ru)
}

fn with_ui<R>(words: Option<Words>, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let ctx = Context::default();
    if let Some(words) = words {
        words.install(&ctx);
    }
    let mut out = None;
    let mut f = Some(f);
    let mut output = ctx.run_ui(Default::default(), |ui| {
        if let Some(f) = f.take() {
            out = Some(f(ui));
        }
    });
    output.textures_delta.clear();
    out.unwrap()
}

#[test]
fn without_an_install_words_are_english() {
    let probe = with_ui(None, |ui| Probe::of(ui));
    assert_eq!(probe, Probe::default());
    assert_eq!(probe.grid, "Grid View");
    let word = Word::keyed("probe.gridView", "Grid View");
    assert!(matches!(Words::english().get(word), std::borrow::Cow::Borrowed("Grid View")));
}

#[test]
fn an_id_row_beats_the_english_row_and_the_english_row_beats_the_default() {
    let (i18n, de, _) = catalog();
    let probe = with_ui(Some(Words::from_i18n(i18n, de)), |ui| Probe::of(ui));
    assert_eq!(probe.grid, "Rasteransicht");
    assert_eq!(probe.opacity, "Deckkraft");
    assert_eq!(probe.name, "Name {name}");
    let words = Words::from_i18n(catalog().0, de);
    assert_eq!(words.get(Word::keyed("probe.other", "Grid View")), "Raster");
}

#[test]
fn a_field_the_host_set_is_not_replaced_by_the_catalog() {
    let (i18n, de, _) = catalog();
    let probe = with_ui(Some(Words::from_i18n(i18n, de)), |ui| Probe { grid: "Mine".into(), ..Probe::of(ui) });
    assert_eq!(probe.grid, "Mine");
    assert_eq!(probe.opacity, "Deckkraft");
}

#[test]
fn a_translation_missing_a_placeholder_is_ignored() {
    let words = Words::new(|word| Some(if word.id == "probe.name" { "Nom".into() } else { String::new() }));
    assert_eq!(words.get(Word::keyed("probe.name", "Name {name}")), "Name {name}");
    assert_eq!(words.get(Word::plain("Opacity")), "Opacity");
    let words = Words::new(|_| Some("Nom {name}".into()));
    assert_eq!(words.format(Word::keyed("probe.name", "Name {name}"), &[("name", "x")]), "Nom x");
}

#[test]
fn counts_use_plural_forms() {
    let (i18n, _, ru) = catalog();
    let english = Words::english();
    assert_eq!(english.count("probe.files", 1, "{n} file", "{n} files"), "1 file");
    assert_eq!(english.count("probe.files", 2, "{n} file", "{n} files"), "2 files");
    let russian = Words::from_i18n(i18n, ru);
    let count = |n| russian.count("probe.files", n, "{n} file", "{n} files");
    assert_eq!((count(1), count(2), count(5)), ("1 файл".to_owned(), "2 файла".to_owned(), "5 файлов".to_owned()));
}

#[test]
fn every_registered_word_is_unique_and_ids_are_module_dot_camel_case() {
    let all: Vec<Word> = super::words::all().collect();
    assert_eq!(problems(all.iter().copied()), Vec::<String>::new());
    let mut texts = std::collections::HashSet::new();
    assert!(all.iter().all(|word| texts.insert(*word)));
    assert_eq!(problems(Probe::WORDS.iter().copied()), Vec::<String>::new());
    let bad = [Word::keyed("grid", "A"), Word::keyed("Probe.grid", "A"), Word::keyed("probe.a", "A"), Word::keyed("probe.a", "B")];
    assert_eq!(problems(bad).len(), 3);
}

#[test]
fn a_labels_struct_localized_with_identity_equals_its_default() {
    let identity = Words::new(|_| None);
    assert_eq!(Probe::english().localized_in(&identity), Probe::default());
    assert_eq!(Probe::english().localized(|_| None), Probe::default());
    let keys: Vec<&str> = Probe::WORDS.iter().map(Word::key).collect();
    assert_eq!(keys, ["probe.gridView", "probe.name", "Opacity"]);
    let seen = std::cell::RefCell::new(Vec::new());
    let probe = Probe::english().localized(|key| {
        seen.borrow_mut().push(key.to_owned());
        (key == "Opacity").then(|| "Deckkraft".to_owned())
    });
    assert_eq!(*seen.borrow(), keys);
    assert_eq!(probe.opacity, "Deckkraft");
}

#[test]
fn chrome_labels_localized_with_identity_equal_their_english() {
    use crate::behavior::EditMenuLabels;
    use crate::dialog::DialogLabels;
    use crate::menu::MenuBarLabels;
    use crate::panes::PaneLabels;
    use crate::patterns::PaletteLabels;
    use crate::preferences::PreferencesTexts;
    use crate::tab_strip::TabStripText;

    let identity = Words::new(|_| None);
    macro_rules! check {
        ($($labels:ty),*) => {$(
            assert_eq!(<$labels>::english().localized_in(&identity), <$labels>::default());
            assert_eq!(<$labels>::english().localized(|_| None), <$labels>::default());
        )*};
    }
    check!(DialogLabels, MenuBarLabels, EditMenuLabels, PaletteLabels, TabStripText, PaneLabels);
    let texts = <PreferencesTexts as Labels>::english().localized_in(&identity);
    assert_eq!(format!("{texts:?}"), format!("{:?}", PreferencesTexts::english()));
    assert_eq!(PreferencesTexts::WORDS.len(), 13);
}
