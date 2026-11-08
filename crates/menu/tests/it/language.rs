//! The Language menu.

use serde_json::json;

use craft_menu::{CommandRef, ItemKind, ItemRole, LANGUAGE_COMMAND, LanguageChoice, LanguageLabels, LanguageMenu, LanguageRow, Node};
use craft_ui::testing::trial;

fn row(code: &str, name: &str) -> LanguageRow {
    LanguageRow::new(code, name)
}

// Automatic, a separator, then English and the offered languages in their own names; the current one selected.
#[trial]
fn language_rows_are_radio_items_with_automatic_first() {
    let menu = LanguageMenu::new(vec![row("en", "English"), row("fr", "Français")], LanguageChoice::Code("fr".into()));
    let nodes = menu.items(&LanguageLabels::default());
    assert_eq!(nodes.len(), 4);
    let item = |n: &Node| match n {
        Node::Item(i) => (i.label.clone(), i.kind, i.command.clone()),
        _ => panic!("an item"),
    };
    assert_eq!(item(&nodes[0]), ("Automatic".into(), ItemKind::Radio(false), CommandRef::with_params(LANGUAGE_COMMAND, json!({"lang": "auto"}))));
    assert!(matches!(nodes[1], Node::Separator));
    assert_eq!(item(&nodes[2]).1, ItemKind::Radio(false));
    assert_eq!(item(&nodes[3]), ("Français".into(), ItemKind::Radio(true), CommandRef::with_params(LANGUAGE_COMMAND, json!({"lang": "fr"}))));

    let automatic = LanguageMenu::new(vec![row("en", "English")], LanguageChoice::Automatic);
    assert_eq!(item(&automatic.items(&LanguageLabels::default())[0]).1, ItemKind::Radio(true));
    assert_eq!(automatic.clone().automatic(false).items(&LanguageLabels::default()).len(), 1);
    assert!(matches!(automatic.submenu(&LanguageLabels::default()), Node::Submenu { role: Some(ItemRole::Language), .. }));
}

// app.language {"lang":"fr"} reads as Code("fr"), {"lang":"auto"} as Automatic, other commands as None.
#[trial]
fn from_command_reads_the_language_parameter() {
    let lang = |v| CommandRef::with_params(LANGUAGE_COMMAND, v);
    assert_eq!(LanguageChoice::from_command(&lang(json!({"lang": "fr"}))), Some(LanguageChoice::Code("fr".into())));
    assert_eq!(LanguageChoice::from_command(&lang(json!({"lang": "auto"}))), Some(LanguageChoice::Automatic));
    assert_eq!(LanguageChoice::from_command(&lang(json!({"lang": 3}))), None);
    assert_eq!(LanguageChoice::from_command(&CommandRef::new(LANGUAGE_COMMAND)), None);
    assert_eq!(LanguageChoice::from_command(&CommandRef::new("file.open")), None);
}
