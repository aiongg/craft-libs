//! The Help menu kit.

use crate::{CommandRef, HelpCommand, HelpLabels, HelpLink, HelpMenu, ItemRole, Node};

fn full() -> HelpMenu {
    HelpMenu::new("PhotoCraft")
        .help_center("https://example.com/help")
        .keyboard_shortcuts(true)
        .link(HelpLink { id: "help.discord".into(), label: "Join the Discord…".into(), url: "https://discord.gg/x".into() })
        .report_issue("https://example.com/issues")
        .check_for_updates(true)
}

#[test]
// Links come after Keyboard Shortcuts, Check for Updates before the separator, System Info then About; About is tagged ItemRole::About.
fn help_rows_follow_the_kit_order() {
    let nodes = full().items(&HelpLabels::default());
    let shape: Vec<String> = nodes
        .iter()
        .map(|n| match n {
            Node::Item(i) => format!("{}={}", i.command.id, i.label),
            Node::Separator => "-".to_owned(),
            _ => "?".to_owned(),
        })
        .collect();
    assert_eq!(
        shape,
        [
            "help.helpCenter=PhotoCraft Help",
            "help.keyboardShortcuts=Keyboard Shortcuts…",
            "help.discord=Join the Discord…",
            "help.reportIssue=Report an Issue…",
            "help.checkForUpdates=Check for Updates…",
            "-",
            "help.systemInfo=System Info…",
            "help.about=About PhotoCraft",
        ]
    );
    assert!(matches!(nodes.last(), Some(Node::Item(i)) if i.role == Some(ItemRole::About)));
    let bare = HelpMenu::new("PhotoCraft").items(&HelpLabels::default());
    assert_eq!(bare.len(), 2, "System Info and About, no separator before them");
}

#[test]
// url() returns the link's URL for its id and the help center's for help.helpCenter, None for help.about.
fn help_link_commands_give_their_urls() {
    let help = full();
    let url = |id: &str| help.url(&CommandRef::new(id));
    assert_eq!(url("help.discord"), Some("https://discord.gg/x"));
    assert_eq!(url("help.helpCenter"), Some("https://example.com/help"));
    assert_eq!(url("help.reportIssue"), Some("https://example.com/issues"));
    assert_eq!(url("help.about"), None);
    assert_eq!(url("file.open"), None);
    assert_eq!(HelpCommand::from_command(&CommandRef::new("help.systemInfo")), Some(HelpCommand::SystemInfo));
    assert_eq!(HelpCommand::from_command(&CommandRef::new("help.discord")), None);
}
