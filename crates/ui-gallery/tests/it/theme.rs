//! The theme workspace: its words, the file it edits, what it caches and what its browser does with a pick.

use craft_theme::{AppSizes, Appearance, ColorToken, Contrast, Mode, Size, SizeValue, TextSize, TokenRef, builtins, explain};
use craft_ui::contract::Bind;
use craft_ui::testing::{Fixture, trial};
use craft_ui_gallery::theme::{Pick, ThemeSubject, ThemeWorkspace, TokenBrowser, parse_sizes};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

fn appearance() -> Appearance {
    Appearance { mode: Mode::Light, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false }
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
    workspace.sizes = layer;
    workspace.refresh(&subject);
    assert!(!Arc::ptr_eq(&first, workspace.explained().expect("explained")), "a new app layer explains again");
    assert_eq!(&workspace.sizes, &parse_sizes("sizes.json", br#"{ "all": { "size.control.md": 30 } }"#).expect("loads"));
    assert_ne!(workspace.sizes, AppSizes::default());
}

/// A browser over the first built-in theme whose reveal request the test sets, keeping its own pick.
fn browser(reveal: &Rc<Cell<bool>>, pick: &Pick) -> Fixture {
    let explained = Arc::new(explain(&builtins()[0], appearance(), None));
    let (reveal, pick) = (Rc::clone(reveal), pick.clone());
    let mut chosen = None;
    Fixture::new(move |ui| {
        let mut browser = TokenBrowser::new(&explained, Bind::bound(&mut chosen));
        if reveal.get() {
            browser = browser.reveal(&pick);
        }
        browser.show(ui);
    })
}

#[trial]
fn a_reveal_opens_the_picked_group_each_time_it_is_asked() {
    let reveal = Rc::new(Cell::new(false));
    let pick = Pick::Token(TokenRef::Color(ColorToken::TextFaint));
    let mut fixture = browser(&reveal, &pick);
    fixture.settle();
    assert!(fixture.by_label("text.faint").is_none(), "the groups start closed");
    for round in 0..2 {
        reveal.set(true);
        fixture.frame(Vec::new());
        reveal.set(false);
        fixture.settle();
        assert!(fixture.by_label("text.faint").is_some(), "round {round}: the revealed token is listed under its open group");
        let group = fixture.by_label("text").expect("the group heading is listed");
        fixture.click(&group).key(Key::ArrowLeft, Modifiers::NONE).settle();
        assert!(fixture.by_label("text.faint").is_none(), "round {round}: the group closes");
    }
}
