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

    identity_holds::<crate::docking::DockConfig>();
    identity_holds::<crate::presets::PresetLabels>();
    identity_holds::<crate::list::TreeText>();
    identity_holds::<crate::list::ThumbSizeLabels>();
    identity_holds::<crate::interface_scale::InterfaceScaleLabels>();
    #[cfg(feature = "window-frame")]
    identity_holds::<crate::title_bar::CaptionLabels>();
    identity_holds::<crate::controls::ThumbRowLabels>();
    identity_holds::<crate::canvas::RefPointLabels>();
}

/// A table that drifted from the struct's `Default` shows here.
fn identity_holds<L: Labels + Default + std::fmt::Debug>() {
    let shown = |labels: L| format!("{labels:?}");
    assert_eq!(shown(L::english().localized_in(&Words::new(|_| None))), shown(L::default()));
    assert_eq!(shown(L::english().localized_in(&Words::english())), shown(L::default()));
}

#[test]
fn localized_hooks_keep_their_keys() {
    let shown = |key: &str| Some(format!("<{key}>"));
    let dock = crate::docking::DockConfig::default().localized(shown);
    assert_eq!(
        (dock.float_label.as_str(), dock.panel_menu_label.as_str(), dock.float_hint_label.as_str()),
        ("<dock.float>", "<dock.panelMenu>", "<dock.floatHint>")
    );
    let presets = crate::presets::PresetLabels::default().localized(shown);
    assert_eq!(
        (presets.grid_view.as_str(), presets.group_count.as_str(), presets.tree.collapse_tip.as_str()),
        ("<preset.gridView>", "<preset.groupCount>", "<preset.collapseTip>")
    );
    #[cfg(feature = "window-frame")]
    {
        let captions = crate::title_bar::CaptionLabels::default().localized(shown);
        assert_eq!((captions.minimize.as_str(), captions.restore.as_str()), ("<window.minimize>", "<window.restore>"));
    }
    let halves = crate::controls::ThumbRowLabels::default().localized(shown);
    assert_eq!((halves.low.as_str(), halves.high.as_str()), ("<thumbRow.low>", "<thumbRow.high>"));
    // Each table word reaches its own field, in the order the table lists them.
    // Counts and the float hint need their placeholders to be accepted.
    let by_word = Words::new(|word| Some(format!("<{}>{}", word.key(), word.english.find('{').map_or("", |at| &word.english[at..]))));
    for (labels, words) in [
        (format!("{:?}", crate::docking::DockConfig::english().localized_in(&by_word)), <crate::docking::DockConfig as Labels>::WORDS),
        (format!("{:?}", crate::presets::PresetLabels::english().localized_in(&by_word)), <crate::presets::PresetLabels as Labels>::WORDS),
    ] {
        for word in words {
            assert_eq!(labels.matches(&format!("\"<{}>", word.key())).count(), 1, "{}", word.key());
        }
    }
}

#[test]
fn key_caps_thumb_sizes_and_reference_points_go_through_words() {
    use crate::list::ThumbSize;
    let fake = Words::new(|word| Some(format!("<{}>", word.english)));
    let right = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::PageDown);
    assert_eq!(crate::controls::Kbd::parts_in(&right, egui::os::OperatingSystem::Nix, &fake), ["<Page Down>"]);
    assert_eq!(crate::controls::Kbd::parts(&right, egui::os::OperatingSystem::Nix), ["Page Down"]);
    assert_eq!(ThumbSize::Px(30).label(&fake), "<Custom>");
    assert_eq!(ThumbSize::Large.name(), "Large");
    let names = crate::canvas::RefPointLabels::english().localized_in(&fake).names();
    assert_eq!((names[0].as_str(), names[8].as_str()), ("<Top left>", "<Bottom right>"));
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

#[test]
fn canvas_and_print_labels_localized_with_identity_equal_their_english() {
    use crate::canvas::{
        CropLabels, CropNames, GradientLineNames, GuideNames, PerspectiveCropNames, RulerNames, ScrollbarNames, TransformFieldLabels,
        TransformLabels, TransformNames,
    };
    use crate::printing::PrintTexts;
    use crate::shell::CanvasLabels;

    identity_holds::<CropNames>();
    identity_holds::<CropLabels>();
    identity_holds::<PerspectiveCropNames>();
    identity_holds::<TransformNames>();
    identity_holds::<TransformLabels>();
    identity_holds::<TransformFieldLabels>();
    identity_holds::<RulerNames>();
    identity_holds::<GuideNames>();
    identity_holds::<ScrollbarNames>();
    identity_holds::<GradientLineNames>();
    identity_holds::<PrintTexts>();
    identity_holds::<CanvasLabels>();
}

#[test]
fn canvas_names_reach_their_fields_and_closures_fill_placeholders() {
    use crate::canvas::{BezierNames, CropNames, PenNames, TransformFieldLabels, WarpNames, WarpSide};

    let by_word = Words::new(|word| Some(format!("<{}>{}", word.key(), word.english.find('{').map_or("", |at| &word.english[at..]))));
    let crop = format!("{:?}", CropNames::english().localized_in(&by_word));
    for word in CropNames::WORDS {
        assert_eq!(crop.matches(&format!("\"<{}>", word.key())).count(), 1, "{}", word.key());
    }
    let fields = TransformFieldLabels::english().localized_in(&by_word);
    assert_eq!((fields.skew_h.glyph.as_str(), fields.skew_v.name.as_str()), ("<transform.skewHGlyph>", "<transform.skewVName>"));

    assert_eq!(PenNames::english().anchor.as_ref()(3, 5), "Anchor 3 of 5");
    assert_eq!(BezierNames::english().anchor.as_ref()(1, 2), "Anchor 1 of 2");
    let warp = WarpNames::english();
    assert_eq!((warp.anchor)(2, 3), "Row 2, column 3");
    assert_eq!((warp.handle)(2, 3, WarpSide::Right), "Row 2, column 3, right handle");
    let german = Words::new(|word| (word.key() == "warp.handleUp").then(|| "Zeile {row}, Spalte {column}, oben".to_owned()));
    assert_eq!((WarpNames::english().localized_in(&german).handle)(1, 2, WarpSide::Up), "Zeile 1, Spalte 2, oben");
}

#[test]
fn editor_labels_localized_with_identity_equal_their_english() {
    use crate::composites::histogram::HistogramLabels;
    use crate::composites::{CurveLabels, GradientLabels, LevelsLabels};

    let identity = Words::new(|_| None);
    macro_rules! check {
        ($($labels:ty),*) => {$(
            assert_eq!(<$labels>::english().localized_in(&identity), <$labels>::default());
            assert_eq!(<$labels>::english().localized(|_| None), <$labels>::default());
        )*};
    }
    check!(GradientLabels, CurveLabels, LevelsLabels, HistogramLabels);
}

#[test]
fn number_errors_follow_the_words_unless_the_host_translates_them() {
    use craft_i18n::NumberError;
    let shown = Words::new(|word| Some(format!("<{}>", word.key())));
    let numbers = crate::localized::LocaleNumbers::from_tag("en");
    assert_eq!(numbers.describe_in(NumberError::Empty, &shown), "<Enter a number>");
    let host = crate::localized::LocaleNumbers::from_tag("en").with_messages(|english| format!("host {english}"));
    assert_eq!(host.describe_in(NumberError::Empty, &shown), "host Enter a number");
}

#[test]
fn labels_are_built_once_per_installed_words() {
    use super::SharedLabels as _;
    let ctx = Context::default();
    let first = Probe::shared_ctx(&ctx);
    assert!(Arc::ptr_eq(&first, &Probe::shared_ctx(&ctx)), "English labels are built once");
    assert_eq!(*first, Probe::default());
    let (i18n, de, _) = catalog();
    Words::from_i18n(i18n, de).install(&ctx);
    let german = Probe::shared_ctx(&ctx);
    assert!(Arc::ptr_eq(&german, &Probe::shared_ctx(&ctx)));
    assert_eq!(german.grid, "Rasteransicht");
    assert_eq!(Probe::of_ctx(&ctx), *german);
}

#[test]
fn counted_words_are_registered_and_read_one_and_other_in_english() {
    use super::words::counts;
    let all: Vec<Word> = super::words::all().collect();
    for count in counts::ALL {
        assert!(all.contains(&count.word), "{:?} is not a registered word", count.word);
    }
    let english = Words::english();
    let labels = crate::presets::PresetLabels::default();
    let text = |n| english.counted(counts::PRESETS, &labels.count, n);
    assert_eq!((text(1), text(2)), ("1 preset".to_owned(), "2 presets".to_owned()));
    assert_eq!(english.counted(counts::PRESETS, "{n} items", 1), "1 items", "a host's text is used as given");
}
