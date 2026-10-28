//! A menu's data: its entries ([`MenuEntry`]), each a row ([`MenuItem`]) or a separator.

use crate::controls::Kbd;
use crate::icons::Icon;

/// What a row is besides a command: a check item or one of a group of radio items, with its state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemKind {
    /// A command.
    #[default]
    Plain,
    /// A setting that is on or off; a check mark shows while on.
    Check(bool),
    /// One of a group of choices; a dot shows at the chosen one.
    Radio(bool),
}

impl ItemKind {
    /// The checked state of a check or radio item; `None` for a plain item.
    pub fn checked(self) -> Option<bool> {
        match self {
            ItemKind::Plain => None,
            ItemKind::Check(on) | ItemKind::Radio(on) => Some(on),
        }
    }
}

/// One entry of a menu: a row, or a separator between groups of rows.
///
/// Separators at the start or end of a menu, and a separator after another, are not drawn.
#[derive(Clone, Debug)]
pub enum MenuEntry<T> {
    Item(MenuItem<T>),
    Separator,
}

impl<T> MenuEntry<T> {
    /// The row, or `None` for a separator.
    pub fn item(&self) -> Option<&MenuItem<T>> {
        match self {
            MenuEntry::Item(item) => Some(item),
            MenuEntry::Separator => None,
        }
    }
}

impl<T> From<MenuItem<T>> for MenuEntry<T> {
    fn from(item: MenuItem<T>) -> Self {
        MenuEntry::Item(item)
    }
}

/// What a row does.
#[derive(Clone, Debug)]
enum Action<T> {
    /// Runs the command: the menu returns it.
    Run(T),
    /// Opens a submenu of these entries.
    Submenu(Vec<MenuEntry<T>>),
}

/// One row of a menu: a command (`T`, returned when it runs) or a submenu, with its label, an
/// optional icon, a check or radio mark, a shortcut and a mnemonic.
///
/// ```
/// use craft_ui::controls::Kbd;
/// use craft_ui::menu::{MenuEntry, MenuItem};
/// use egui::{Key, KeyboardShortcut, Modifiers};
///
/// let entries: Vec<MenuEntry<&str>> = vec![
///     MenuItem::new("Save", "save").shortcut(Kbd::new(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S))).into(),
///     MenuEntry::Separator,
///     MenuItem::new("Rulers", "rulers").check(true).into(),
///     MenuItem::submenu("Zoom", vec![MenuItem::new("Fit", "fit").into(), MenuItem::new("100%", "actual").into()]).into(),
/// ];
/// ```
#[derive(Clone, Debug)]
pub struct MenuItem<T> {
    label: String,
    english: Option<String>,
    icon: Option<Icon>,
    shortcut: Option<Kbd>,
    kind: ItemKind,
    enabled: bool,
    mnemonic: Option<char>,
    /// Takes part in its level's mnemonics.
    keyed: bool,
    action: Action<T>,
}

impl<T> MenuItem<T> {
    /// A command row labelled `label` (as shown, translated); the menu returns `command` when the
    /// row runs.
    pub fn new(label: impl Into<String>, command: T) -> Self {
        Self::with_action(label.into(), Action::Run(command))
    }

    /// A row that opens a submenu of `children`.
    pub fn submenu(label: impl Into<String>, children: Vec<MenuEntry<T>>) -> Self {
        Self::with_action(label.into(), Action::Submenu(children))
    }

    fn with_action(label: String, action: Action<T>) -> Self {
        Self { label, english: None, icon: None, shortcut: None, kind: ItemKind::Plain, enabled: true, mnemonic: None, keyed: true, action }
    }

    /// An icon before the label. Every row of a menu that has an icon keeps room for one.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The shortcut that runs the command, shown at the row's end in [`Kbd`]'s text form and
    /// reported to AccessKit.
    pub fn shortcut(mut self, shortcut: Kbd) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// A check item, checked or not.
    pub fn check(mut self, checked: bool) -> Self {
        self.kind = ItemKind::Check(checked);
        self
    }

    /// A radio item, chosen or not.
    pub fn radio(mut self, chosen: bool) -> Self {
        self.kind = ItemKind::Radio(chosen);
        self
    }

    /// A plain, check or radio item.
    pub fn kind(mut self, kind: ItemKind) -> Self {
        self.kind = kind;
        self
    }

    /// Whether the row can run (or open its submenu). A disabled row is dimmed, the keys skip it,
    /// and a click on it leaves the menu open. A submenu is also disabled while none of the
    /// commands under it can run.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The preferred mnemonic letter: the row takes it when its label has it and no row before
    /// it in its menu took it. Every other row takes the first unused letter of its label, word
    /// starts first.
    pub fn mnemonic(mut self, letter: char) -> Self {
        self.mnemonic = Some(letter);
        self
    }

    /// The English label, for a label the keyboard can't type a letter of (Chinese, Japanese,
    /// Korean…): the row then shows its English label's mnemonic after it, "文件(F)", as Windows
    /// menus do. Without it such a row has no mnemonic.
    pub fn english(mut self, english: impl Into<String>) -> Self {
        self.english = Some(english.into());
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn item_kind(&self) -> ItemKind {
        self.kind
    }

    /// The command, or `None` for a submenu.
    pub fn command(&self) -> Option<&T> {
        match &self.action {
            Action::Run(command) => Some(command),
            Action::Submenu(_) => None,
        }
    }

    /// The submenu's entries, or `None` for a command.
    pub fn children(&self) -> Option<&[MenuEntry<T>]> {
        match &self.action {
            Action::Run(_) => None,
            Action::Submenu(children) => Some(children),
        }
    }

    pub(crate) fn icon_of(&self) -> Option<Icon> {
        self.icon
    }

    pub(crate) fn shortcut_of(&self) -> Option<&Kbd> {
        self.shortcut.as_ref()
    }

    pub(crate) fn preferred(&self) -> Option<char> {
        self.mnemonic
    }

    /// A row without a mnemonic that takes no letter from the other rows of its level (a menu
    /// bar's search matches).
    pub(crate) fn without_mnemonic(mut self) -> Self {
        self.keyed = false;
        self
    }

    pub(crate) fn takes_mnemonic(&self) -> bool {
        self.keyed
    }

    pub(crate) fn english_of(&self) -> Option<&str> {
        self.english.as_deref()
    }

    /// Whether the row can run: enabled, and for a submenu, some command under it can run.
    pub fn is_enabled(&self) -> bool {
        self.enabled
            && match &self.action {
                Action::Run(_) => true,
                Action::Submenu(children) => children.iter().filter_map(MenuEntry::item).any(MenuItem::is_enabled),
            }
    }
}

/// The entry at `path` (an index into each level's entries in turn) under `entries`.
pub(crate) fn at_path<'e, T>(entries: &'e [MenuEntry<T>], path: &[usize]) -> Option<&'e MenuItem<T>> {
    let (&last, parents) = path.split_last()?;
    let mut level = entries;
    for &i in parents {
        level = level.get(i)?.item()?.children()?;
    }
    level.get(last)?.item()
}
