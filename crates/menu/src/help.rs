//! The Help menu kit: the rows every Craft app's Help menu has, in one order, with the app's own
//! links between them.
//!
//! Photoshop's Help menu: Photoshop Help…, Hands-on Tutorials…, What's New…, Updates…, System
//! Info…, About Photoshop (Windows). PhotoCraft's: its links (Discord, website, ArtCraft website,
//! GitHub, Report an Issue…), Check for Updates…, a separator, System Info…, About PhotoCraft
//! (`crates/engine/src/shell_commands.rs:90-98`, ordered in `crates/ui-egui/src/menus/items.rs:185`).
//! [`HelpMenu`] builds that menu as model rows; the app runs the commands it reports, opening the
//! link URLs with [`HelpMenu::url`].

#![expect(unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_keymap::CommandRef;

use crate::model::{Menu, Node};

/// A row of the Help menu kit. Each has a fixed command id (`help.<camelCase>`) so shortcuts,
/// automation and the keymap catalog name it the same in every app.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HelpCommand {
    /// "{app} Help": the app's online help (`help.helpCenter`), opened in the browser.
    HelpCenter,
    /// "Keyboard Shortcuts…": the app's shortcut summary or the Keyboard Shortcuts dialog
    /// (`help.keyboardShortcuts`).
    KeyboardShortcuts,
    /// "Check for Updates…" (`help.checkForUpdates`), run through craft-update. Left out on the
    /// web.
    CheckForUpdates,
    /// "Report an Issue…" (`help.reportIssue`), opened in the browser.
    ReportIssue,
    /// "System Info…" (`help.systemInfo`): the app's system information dialog.
    SystemInfo,
    /// "About {app}" (`help.about`), tagged [`crate::ItemRole::About`] so the Mac layout moves it
    /// to the app menu.
    About,
}

impl HelpCommand {
    /// The command id: `help.helpCenter`, `help.keyboardShortcuts`, `help.checkForUpdates`,
    /// `help.reportIssue`, `help.systemInfo`, `help.about`.
    // pc: crates/engine/src/shell_commands.rs:96-98 (`help.reportIssue`, `help.systemInfo`,
    // `help.about`); crates/engine/src/cmd/updates_cmds.rs:120 (`help.checkForUpdates`).
    pub fn id(self) -> &'static str {
        todo!()
    }

    /// The row for a command, `None` for any other command (an app link included).
    pub fn of(command: &CommandRef) -> Option<HelpCommand> {
        todo!()
    }
}

/// An app's own Help link: a row that opens `url` in the browser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelpLink {
    /// The command id the row runs (`help.discord`).
    pub id: String,
    /// The row's label, translated by the app ("Join the ArtCraft Discord…").
    pub label: String,
    /// An `https` URL.
    pub url: String,
}

/// The Help menu of one app.
///
/// ```no_run
/// use craft_menu::{HelpLabels, HelpLink, HelpMenu, MenuBar};
/// # let mut bar = MenuBar::default();
/// let help = HelpMenu::new("PhotoCraft")
///     .help_center("https://getartcraft.com/apps/photocraft/help")
///     .link(HelpLink { id: "help.discord".into(), label: "Join the ArtCraft Discord…".into(), url: "https://discord.gg/artcraft".into() })
///     .report_issue("https://github.com/storytold/photocraft/issues")
///     .check_for_updates(true);
/// bar.menus.push(help.menu(&HelpLabels::default()));
/// // When a click reports a command:
/// # let command: craft_menu::CommandRef = "help.discord".into();
/// if let Some(url) = help.url(&command) { /* ctx.open_url(egui::OpenUrl::new_tab(url)) */ }
/// ```
///
/// Order: Help Center, Keyboard Shortcuts…, the app's links in the order given, Report an Issue…,
/// Check for Updates…, a separator, System Info…, About. Rows the app didn't ask for are left out.
/// The Mac layout moves About to the app menu and adds the system's search field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelpMenu {
    app_name: String,
    help_center: Option<String>,
    keyboard_shortcuts: bool,
    links: Vec<HelpLink>,
    report_issue: Option<String>,
    check_for_updates: bool,
    system_info: bool,
}

impl HelpMenu {
    /// System Info… and About, nothing else.
    pub fn new(app_name: impl Into<String>) -> Self {
        todo!()
    }

    /// Adds "{app} Help", opening `url`.
    pub fn help_center(mut self, url: impl Into<String>) -> Self {
        todo!()
    }

    /// Adds "Keyboard Shortcuts…".
    pub fn keyboard_shortcuts(mut self, on: bool) -> Self {
        todo!()
    }

    /// Adds an app link after Keyboard Shortcuts… and the links before it.
    // pc: crates/ui-egui/src/links.rs:17 `COMMANDS`.
    pub fn link(mut self, link: HelpLink) -> Self {
        todo!()
    }

    /// Adds "Report an Issue…", opening `url`.
    pub fn report_issue(mut self, url: impl Into<String>) -> Self {
        todo!()
    }

    /// Adds "Check for Updates…" (never on the web build).
    pub fn check_for_updates(mut self, on: bool) -> Self {
        todo!()
    }

    /// Whether System Info… shows (on by default).
    pub fn system_info(mut self, on: bool) -> Self {
        todo!()
    }

    /// The rows in the kit's order, with About tagged [`crate::ItemRole::About`].
    // pc: crates/ui-egui/src/menus/items.rs:185-197 (update check before the separator, then
    // System Info and About).
    pub fn nodes(&self, labels: &HelpLabels) -> Vec<Node> {
        todo!()
    }

    /// The Help menu ([`crate::MenuRole::Help`]) titled `labels.help`, with [`HelpMenu::nodes`].
    pub fn menu(&self, labels: &HelpLabels) -> Menu {
        todo!()
    }

    /// The URL a Help row opens: Help Center, Report an Issue… or an app link. `None` for the
    /// rows the app runs itself (Keyboard Shortcuts…, Check for Updates…, System Info…, About)
    /// and for other commands.
    // pc: crates/ui-egui/src/links.rs:20 `url_for`.
    pub fn url(&self, command: &CommandRef) -> Option<&str> {
        todo!()
    }
}

craft_ui::words! {
    /// The Help menu kit's labels, English by default. `{app}` is the app's name.
    #[non_exhaustive]
    pub struct HelpLabels {
        help: "help.menu" = "Help";
        help_center: "help.helpCenter" = "{app} Help";
        keyboard_shortcuts: "help.keyboardShortcuts" = "Keyboard Shortcuts…";
        report_issue: "help.reportIssue" = "Report an Issue…";
        check_for_updates: "help.checkForUpdates" = "Check for Updates…";
        system_info: "help.systemInfo" = "System Info…";
        about: "help.about" = "About {app}";
    }
}
