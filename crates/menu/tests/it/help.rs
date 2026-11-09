//! The Help menu kit.

use craft_menu::{CommandRef, HelpCommand, HelpLabels, HelpLink, HelpMenu, ItemRole, Node};
use craft_ui::testing::trial;

fn full() -> HelpMenu {
    HelpMenu::new("PhotoCraft")
        .help_center("https://example.com/help")
        .keyboard_shortcuts(true)
        .link(HelpLink::new("help.discord", "Join the Discord…", "https://discord.gg/x"))
        .report_issue("https://example.com/issues")
        .check_for_updates(true)
}

// Links come after Keyboard Shortcuts, Check for Updates before the separator, System Info then About; About is tagged ItemRole::About.

// url() returns the link's URL for its id and the help center's for help.helpCenter, None for help.about.
