//! The theme workspace: its words, the file it edits, what it caches and what its browser does with a pick.

use craft_theme::{AppSizes, Appearance, ColorToken, Contrast, Mode, Size, SizeValue, TextSize, TokenRef, builtins, explain};
use craft_ui::testing::{Fixture, trial};
use craft_ui_gallery::theme::{Pick, ThemeSubject, ThemeWorkspace, TokenBrowser, parse_sizes, summary};
use std::sync::Arc;

fn appearance() -> Appearance {
    Appearance { mode: Mode::Light, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false }
}

#[trial]
fn standard_contrast_reports_pairs_below_minimum_and_high_contrast_fails_them() {
    assert_eq!(summary(Mode::Dark, Contrast::Standard, 3), "Dark: 3 pairs below minimum");
    assert_eq!(summary(Mode::Light, Contrast::Standard, 1), "Light: 1 pair below minimum");
    assert_eq!(summary(Mode::Dark, Contrast::High, 3), "Dark: 3 pairs fail");
    assert_eq!(summary(Mode::Dark, Contrast::High, 1), "Dark: 1 pair fails");
    assert_eq!(summary(Mode::Light, Contrast::High, 0), "Light: every pair meets its minimum");
}

#[trial]
fn a_loaded_file_is_exported_unchanged_and_a_bad_one_names_its_path() {
    let layer = parse_sizes("sizes.json", br#"{ "comfortable": { "size.control.md": 28 } }"#).expect("a valid file loads");
    assert_eq!(layer.comfortable.get("size.control.md"), Some(&SizeValue::Number(28.0)));
    let again = parse_sizes("sizes.json", layer.to_json().as_bytes()).expect("an exported file loads");
    assert_eq!(again, layer);

    let message = parse_sizes("bad.json", br#"{ "all": { "size.control.md": 99 } }"#).expect_err("a value out of range is refused");
    assert!(message.contains("bad.json") && message.contains("all.size.control.md"), "{message}");
}

#[trial]
fn the_explanation_is_computed_again_only_when_the_subject_or_the_layer_changes() {
    let theme = &builtins()[0];
    let subject = ThemeSubject::new(theme, appearance());
    let mut workspace = ThemeWorkspace::new();
    workspace.refresh(&subject);
    let first = Arc::clone(workspace.explained().expect("refresh explains"));
    workspace.refresh(&subject);
    assert!(Arc::ptr_eq(&first, workspace.explained().expect("explained")), "an unchanged subject keeps its explanation");

    let layer = parse_sizes("sizes.json", br#"{ "all": { "size.control.md": 30 } }"#).expect("a valid file loads");
    workspace.apply_sizes(Some(layer));
    workspace.refresh(&subject);
    assert!(!Arc::ptr_eq(&first, workspace.explained().expect("explained")), "a new app layer explains again");
    assert_eq!(workspace.sizes(), &parse_sizes("sizes.json", br#"{ "all": { "size.control.md": 30 } }"#).expect("loads"));
    assert_ne!(workspace.sizes(), &AppSizes::default());
}

#[trial]
fn a_pick_from_the_host_opens_its_group_in_the_browser() {
    let explained = Arc::new(explain(&builtins()[0], appearance(), None));
    let pick = Pick::Token(TokenRef::Color(ColorToken::TextFaint));
    let mut fixture = Fixture::new(move |ui| {
        TokenBrowser::new(&explained).pick(&pick).show(ui);
    });
    fixture.settle();
    assert!(fixture.by_label("text.faint").is_some(), "the picked token is listed under its open group");
}
