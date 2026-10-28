//! The menu bar drawn with egui, for Windows, Linux and the web (and macOS, for testing): an
//! already laid-out [`MenuBar`] as a row of menu titles whose menus open below them.
//!
//! [`MenuBarUi`] converts the menu model into [`craft_ui::menu`] items (commands, check and radio
//! items, shortcuts from [`craft_keymap::Chord`], submenus) and draws [`craft_ui::menu::MenuBar`],
//! which owns the look and the behaviour: press-drag-release, hover switching, submenus with an
//! aim triangle, scrolling, the keys, mnemonics (except on a Mac target), overflow into "⋯" or
//! one "Menu", and AccessKit.

use craft_keymap::{CommandRef, Platform};
use craft_ui::menu::{MenuBarLabels, MenuEntry, MenuItem};
use egui::{Id, Ui};

use crate::model::Builtin;
use crate::{Gate, Item, ItemKind, MenuBar, Node, Target};

pub use craft_ui::menu::bar::{is_focused, is_open, leave_for_shortcut};

/// A row or title, as the bar names it to the app's mnemonic hooks ([`MenuBarUi::english`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Labeled<'s> {
    /// An item, by its command.
    Command(&'s CommandRef),
    /// A menu or submenu, by its title as shown.
    Title(&'s str),
}

/// The app's English labels ([`MenuBarUi::english`]).
type EnglishHook<'h> = dyn Fn(Labeled<'_>) -> Option<String> + 'h;

/// How the bar looked before it took its look from the theme. Nothing reads it any more.
#[deprecated(note = "the bar's look comes from the theme; this is ignored")]
#[derive(Clone, Debug, PartialEq)]
pub struct BarStyle {
    /// Padding around a menu title.
    pub title_padding: egui::Vec2,
    /// The menu titles' text color.
    pub title_color: egui::Color32,
    /// The narrowest a menu is.
    pub menu_width: f32,
    /// Room a separator takes: the line and the space above and below it.
    pub separator_spacing: f32,
    /// Shown before the label of a checked check item.
    pub check_mark: String,
    /// Shown before the label of the selected radio item.
    pub radio_mark: String,
}

#[allow(deprecated)]
impl BarStyle {
    /// Title padding, title color and a menu width in step with the row height, from `style`.
    pub fn from_style(style: &egui::Style) -> BarStyle {
        BarStyle {
            title_padding: style.spacing.button_padding,
            title_color: style.visuals.widgets.inactive.text_color(),
            menu_width: 12.0 * style.spacing.interact_size.y,
            separator_spacing: 9.0,
            check_mark: "✓".to_owned(),
            radio_mark: "●".to_owned(),
        }
    }
}

#[allow(deprecated)]
impl Default for BarStyle {
    fn default() -> BarStyle {
        BarStyle::from_style(&egui::Style::default())
    }
}

/// The platform whose key names a target's shortcut column shows; for the web, the one the
/// browser runs on (`os`, [`egui::Context::os`]).
fn keymap_platform(target: Target, os: egui::os::OperatingSystem) -> Platform {
    use egui::os::OperatingSystem;
    match target {
        Target::Mac => Platform::Mac,
        Target::Windows => Platform::Windows,
        Target::Linux => Platform::Linux,
        Target::Web => match os {
            OperatingSystem::Mac | OperatingSystem::IOS => Platform::Mac,
            OperatingSystem::Windows => Platform::Windows,
            _ => Platform::Linux,
        },
    }
}

fn english(label: &'static str) -> String {
    label.to_owned()
}

/// The bar for one frame: `MenuBarUi::new(&bar).gate(gate).show(ui)` returns the commands
/// chosen. Draw it at the top of the window, in a top panel.
pub struct MenuBarUi<'a> {
    bar: &'a MenuBar,
    gate: Gate,
    target: Target,
    label: &'a dyn Fn(&'static str) -> String,
    english: Option<&'a EnglishHook<'a>>,
    preferred: &'a [(&'a str, char)],
    id: Id,
    fit: Option<f32>,
}

impl<'a> MenuBarUi<'a> {
    /// The bar for `bar`, already laid out ([`crate::layout`]), on [`Target::current`].
    pub fn new(bar: &'a MenuBar) -> MenuBarUi<'a> {
        MenuBarUi {
            bar,
            gate: Gate::default(),
            target: Target::current(),
            label: &english,
            english: None,
            preferred: &[],
            id: Id::unique("craft-menu-bar"),
            fit: None,
        }
    }

    /// What may run this frame: the items `gate` forbids ([`Gate::allows`]) are drawn and
    /// behave as disabled, and while a dialog or the shortcut recorder has the keys the bar
    /// takes none (Alt, F10).
    pub fn gate(mut self, gate: Gate) -> MenuBarUi<'a> {
        self.gate = gate;
        self
    }

    /// The platform the bar is on, for the shortcut column ([`craft_keymap::Chord::display`]): ⌘
    /// glyphs on [`Target::Mac`] and in a browser on a Mac ([`egui::Context::os`]), Ctrl names
    /// everywhere else. Where the names are a Mac's, the bar has no mnemonics.
    pub fn target(mut self, target: Target) -> MenuBarUi<'a> {
        self.target = target;
        self
    }

    /// Ignored: the bar's look comes from the theme.
    #[deprecated(note = "the bar's look comes from the theme; this is ignored")]
    #[allow(deprecated)]
    pub fn style(self, _style: BarStyle) -> MenuBarUi<'a> {
        self
    }

    /// The app's translation of the labels the bar writes itself: `⋯` (the menu of titles that
    /// don't fit), `Menu` (the one menu when none fits), `Scroll menu up` and `Scroll menu down`
    /// (the scroll arrows' accessible names). The same hook as [`crate::LayoutOptions::label`].
    pub fn label(mut self, label: &'a dyn Fn(&'static str) -> String) -> MenuBarUi<'a> {
        self.label = label;
        self
    }

    /// The English label of a row or title, for labels the keyboard can't type a letter of
    /// (Chinese, Japanese, Korean…): such a label shows its English label's mnemonic after it,
    /// "文件(F)", before a trailing ellipsis ("開く(O)…"), as Windows menus do. Without this hook (or when it returns `None`) such a label
    /// has no mnemonic. Not asked on a Mac target, which has no mnemonics.
    pub fn english(mut self, english: &'a dyn Fn(Labeled<'_>) -> Option<String>) -> MenuBarUi<'a> {
        self.english = Some(english);
        self
    }

    /// Preferred mnemonics, each for a menu or submenu by its title as shown or for an item by
    /// its command id: the row takes that letter when its label has it and no row before it in
    /// its menu took it. Every other row takes the first unused letter of its label, word starts
    /// first.
    pub fn mnemonics(mut self, preferred: &'a [(&'a str, char)]) -> MenuBarUi<'a> {
        self.preferred = preferred;
        self
    }

    /// Take only the width the titles need, at most `max_width` (`f32::INFINITY` for no limit)
    /// and the room the `Ui` has, so that other controls can share the row; past that the menus
    /// overflow into "⋯". By default the bar fills the row.
    pub fn shrink_to_fit(mut self, max_width: f32) -> MenuBarUi<'a> {
        self.fit = Some(max_width);
        self
    }

    /// The bar's id, where its state lives in egui memory: tells two bars in one window apart.
    pub fn id(mut self, id: Id) -> MenuBarUi<'a> {
        self.id = id;
        self
    }

    /// Draw the bar and its open menu; returns the commands chosen this frame (at most one),
    /// clicked or run with ↩ or Space. Disabled items, and items the gate forbids, never run.
    /// [`Builtin::Quit`] (asks the window to close, see [`craft_quit::on_menu_event`]) and
    /// [`Builtin::Minimize`] run here and are left out.
    pub fn show(self, ui: &mut Ui) -> Vec<CommandRef> {
        let ctx = ui.ctx().clone();
        // Option is a typing key on a Mac, also in a browser there.
        let platform = keymap_platform(self.target, ctx.os());
        let mnemonics = platform != Platform::Mac;
        craft_ui::menu::set_mnemonics(&ctx, Some(mnemonics));
        let convert = Convert { gate: self.gate, platform, mnemonics, english: self.english, preferred: self.preferred };
        let menus: Vec<MenuItem<CommandRef>> = self.bar.menus.iter().map(|m| convert.sub(&m.title, &m.children)).collect();
        let hook = self.label;
        let labels = MenuBarLabels {
            more: hook("⋯"),
            all: hook("Menu"),
            scroll_up: hook("Scroll menu up"),
            scroll_down: hook("Scroll menu down"),
            ..MenuBarLabels::default()
        };
        let mut bar = craft_ui::menu::MenuBar::new(&menus)
            .id(self.id)
            .labels(&labels)
            .keyboard(!self.gate.modal && !self.gate.recording_shortcut)
            .text_editing(self.gate.text_editing);
        if let Some(max) = self.fit {
            bar = bar.shrink_to_fit(max);
        }
        bar.show(ui)
            .chosen
            .into_iter()
            .filter(|command| match Builtin::of(command) {
                Some(Builtin::Quit) => !craft_quit::on_menu_event(&ctx, Builtin::Quit.id()),
                Some(Builtin::Minimize) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    false
                }
                _ => true,
            })
            .collect()
    }
}

/// What converting the model to menu items needs.
struct Convert<'s> {
    gate: Gate,
    platform: Platform,
    /// The target has mnemonics (not a Mac).
    mnemonics: bool,
    english: Option<&'s EnglishHook<'s>>,
    preferred: &'s [(&'s str, char)],
}

impl Convert<'_> {
    /// A menu or submenu: a submenu row with the app's preferred letter and English label.
    fn sub(&self, title: &str, children: &[Node]) -> MenuItem<CommandRef> {
        let mut row = MenuItem::submenu(title, self.entries(children));
        if let Some(letter) = self.preferred_letter(title) {
            row = row.mnemonic(letter);
        }
        if let Some(english) = self.english_of(Labeled::Title(title)) {
            row = row.english(english);
        }
        row
    }

    /// The rows of `nodes`. AppKit's own items ([`Node::Standard`]) have nothing to run here and
    /// are left out; a desktop or web layout has none.
    fn entries(&self, nodes: &[Node]) -> Vec<MenuEntry<CommandRef>> {
        nodes
            .iter()
            .filter_map(|n| match n {
                Node::Item(item) => Some(self.item(item).into()),
                Node::Separator => Some(MenuEntry::Separator),
                Node::Submenu { label, children, .. } => Some(self.sub(label, children).into()),
                Node::Standard { .. } => None,
            })
            .collect()
    }

    fn item(&self, item: &Item) -> MenuItem<CommandRef> {
        let mut row = MenuItem::new(item.label.as_str(), item.command.clone()).enabled(item.enabled && self.gate.allows(item));
        row = match item.kind {
            ItemKind::Plain => row,
            ItemKind::Check(on) => row.check(on),
            ItemKind::Radio(on) => row.radio(on),
        };
        if let Some(chord) = &item.shortcut {
            row = row.shortcut(craft_keymap::ui::kbd(chord, self.platform));
        }
        if let Some(letter) = self.preferred_letter(&item.command.id) {
            row = row.mnemonic(letter);
        }
        if let Some(english) = self.english_of(Labeled::Command(&item.command)) {
            row = row.english(english);
        }
        row
    }

    /// The app's preferred letter for a title as shown or a command id.
    fn preferred_letter(&self, key: &str) -> Option<char> {
        self.preferred.iter().find(|(k, _)| *k == key).map(|(_, c)| *c)
    }

    fn english_of(&self, labeled: Labeled<'_>) -> Option<String> {
        self.english.filter(|_| self.mnemonics).and_then(|f| f(labeled))
    }
}
