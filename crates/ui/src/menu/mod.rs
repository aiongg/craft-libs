//! Menus: [`Menu`], a popup of [`MenuItem`] rows, separators and submenus, driven by the pointer
//! and the keyboard as PhotoCraft's menus are.
//!
//! - [`MenuItem`] and [`MenuEntry`]: a menu's data. A row has a label, an optional icon, a check or
//!   radio mark ([`ItemKind`]), a shortcut ([`Kbd`](crate::controls::Kbd)), an enabled state, a
//!   preferred mnemonic, and a command (`T`) or a submenu of entries.
//! - [`Menu`]: the popup. Its owner (a menu bar's title, a button, a secondary click) makes the
//!   egui [`Popup`](egui::Popup) with its anchor and open state; the menu draws the rows in it,
//!   tracks the highlight, the pointer heading for a submenu, and the keys, and returns the
//!   command run ([`MenuResponse`]).
//! - [`MenuBar`]: a row of titles, each opening a [`Menu`], with PhotoCraft's press-drag-release,
//!   hover switching, Alt / F10 / Alt+letter, overflow into "…" and the Help menu's search field;
//!   [`bar::is_open`], [`bar::is_focused`] and [`bar::leave_for_shortcut`] for the app's keys.
//! - [`mnemonics`]: whether menus have mnemonics here (off on macOS); [`set_mnemonics`] overrides.
//! - [`any_open`]: whether any menu is open, for the app's single-key shortcuts, which wait
//!   meanwhile.
//!
//! Menus draw with the list box rows' tokens (`listbox.*`) and their own `menu.*` (least width,
//! shortcut gap, scroll strip, the accent-filled active row); [`Menu::style`] is the pure style
//! function, dumped under `crates/ui/tests/dumps/components/menu.txt`.

pub mod bar;
mod context;
mod item;
mod mnemonic;
mod nav;
mod popup;
mod press;
pub(crate) mod row;
#[cfg(test)]
mod tests;

pub use bar::{MenuBar, MenuBarLabels, MenuBarResponse};
pub use context::{ContextMenu, OpenAt};
pub use item::{ItemKind, MenuEntry, MenuItem};
pub use mnemonic::{mnemonics, set_mnemonics};
pub use nav::MenuStep;
pub use popup::{Menu, MenuResponse, any_open};
pub use press::PressMenu;
pub use row::MenuStyle;

use egui::{Id, Key, KeyboardShortcut, Modifiers, Ui};

use crate::controls::{Button, Kbd};
use crate::icons::Icon;
use crate::primitives::{Content, Gap, Row, Text};
use crate::story::{Level, SizeArg, Story, StoryArgs};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Menu", "ContextMenu", "PressMenu"];

#[derive(StoryArgs, Debug, PartialEq)]
struct MenusArgs {
    size: SizeArg,
    icons: bool,
    #[arg(range = 5.0..=200.0)]
    rows: u32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct IconsArgs {
    size: SizeArg,
    icons: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/menu/menus", Level::Composites, "Menu", "Menus", MenusArgs { size: SizeArg::Md, icons: true, rows: 40 }, show_menus),
        typed_story!(
            "composites/menu/context-menu",
            Level::Composites,
            "ContextMenu",
            "Context menu",
            IconsArgs { size: SizeArg::Md, icons: true },
            show_context_menu
        ),
        typed_story!(
            "composites/menu/press-menu",
            Level::Composites,
            "PressMenu",
            "Press menu",
            IconsArgs { size: SizeArg::Md, icons: true },
            show_press_menu
        ),
    ]
}

/// The story's commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    Rulers,
    Grid,
    Snap,
    Zoom(u16),
    Fit,
    Mode(u8),
    Row(usize),
}

fn shortcut(modifiers: Modifiers, key: Key) -> Kbd {
    Kbd::new(&KeyboardShortcut::new(modifiers, key))
}

/// An Edit menu with icons, shortcuts, check and radio items, a disabled row and submenus two
/// deep.
pub(crate) fn edit_menu(icons: bool) -> Vec<MenuEntry<Command>> {
    let icon = |item: MenuItem<Command>, i: Icon| if icons { item.icon(i) } else { item };
    let shift = Modifiers::COMMAND | Modifiers::SHIFT;
    vec![
        icon(MenuItem::new("Undo", Command::Undo).shortcut(shortcut(Modifiers::COMMAND, Key::Z)), Icon::Undo).into(),
        icon(MenuItem::new("Redo", Command::Redo).shortcut(shortcut(shift, Key::Z)), Icon::Redo).enabled(false).into(),
        MenuEntry::Separator,
        MenuItem::new("Cut", Command::Cut).shortcut(shortcut(Modifiers::COMMAND, Key::X)).into(),
        icon(MenuItem::new("Copy", Command::Copy).shortcut(shortcut(Modifiers::COMMAND, Key::C)), Icon::Copy).into(),
        MenuItem::new("Paste", Command::Paste).shortcut(shortcut(Modifiers::COMMAND, Key::V)).into(),
        icon(MenuItem::new("Delete", Command::Delete).shortcut(shortcut(Modifiers::NONE, Key::Delete)), Icon::Trash).into(),
        MenuEntry::Separator,
        MenuItem::submenu(
            "View",
            vec![
                MenuItem::new("Rulers", Command::Rulers).check(true).shortcut(shortcut(Modifiers::COMMAND, Key::R)).into(),
                MenuItem::new("Grid", Command::Grid).check(false).into(),
                MenuItem::new("Snap", Command::Snap).check(true).enabled(false).into(),
                MenuEntry::Separator,
                MenuItem::submenu(
                    "Zoom",
                    vec![
                        MenuItem::new("Fit on Screen", Command::Fit).shortcut(shortcut(Modifiers::COMMAND, Key::Num0)).into(),
                        MenuItem::new("50%", Command::Zoom(50)).into(),
                        MenuItem::new("100%", Command::Zoom(100)).shortcut(shortcut(Modifiers::COMMAND, Key::Num1)).into(),
                        MenuItem::new("200%", Command::Zoom(200)).into(),
                    ],
                )
                .into(),
            ],
        )
        .into(),
        MenuItem::submenu(
            "Mode",
            vec![
                MenuItem::new("RGB Color", Command::Mode(0)).radio(true).into(),
                MenuItem::new("CMYK Color", Command::Mode(1)).radio(false).into(),
                MenuItem::new("Grayscale", Command::Mode(2)).radio(false).into(),
            ],
        )
        .mnemonic('M')
        .into(),
        MenuItem::submenu("Empty", vec![MenuItem::new("Nothing to run", Command::Row(0)).enabled(false).into()]).into(),
    ]
}

/// A menu of `rows` rows, taller than the window from some count on.
fn long_menu(rows: usize) -> Vec<MenuEntry<Command>> {
    (1..=rows).map(|i| MenuItem::new(format!("Row {i}"), Command::Row(i)).into()).collect()
}

/// Two buttons that open menus: Edit, and a long one that scrolls. The command chosen last is
/// shown beside them.
fn show_menus(ui: &mut Ui, args: &mut MenusArgs) {
    let size = args.size.control();
    let edit = edit_menu(args.icons);
    let long = long_menu(args.rows.max(1) as usize);
    let last = Id::unique("craft-ui-menu-story-last");
    Row::new(Gap::Section).show(ui, |ui| {
        for (label, entries) in [("Edit", &edit), ("Long", &long)] {
            let button = ui.add(Button::new(label));
            let shown = PressMenu::new(entries).size(size).show(ui, &button);
            if let Some(command) = shown.chosen {
                ui.data_mut(|d| d.insert_temp(last, format!("{command:?}")));
            }
        }
        let chosen = ui.data(|d| d.get_temp::<String>(last)).unwrap_or_else(|| "Nothing chosen".to_owned());
        ui.add(Text::new(chosen).content(Content::Muted));
    });
}

/// A focusable box with a context menu: secondary click, or Menu key and Shift+F10 once it has the
/// focus (Tab to it).
fn show_context_menu(ui: &mut Ui, args: &mut IconsArgs) {
    let size = args.size.control();
    let entries = edit_menu(args.icons);
    let last = Id::unique("craft-ui-context-menu-story-last");
    Row::new(Gap::Section).show(ui, |ui| {
        let target = ui.add(Button::new("Right-click, or focus and press Shift+F10"));
        if let Some(command) = ContextMenu::new(&entries).size(size).show(ui, &target).chosen {
            ui.data_mut(|d| d.insert_temp(last, format!("{command:?}")));
        }
        let chosen = ui.data(|d| d.get_temp::<String>(last)).unwrap_or_else(|| "Nothing chosen".to_owned());
        ui.add(Text::new(chosen).content(Content::Muted));
    });
}

/// Two press menus side by side: press, drag onto a row and release, or click to open.
fn show_press_menu(ui: &mut Ui, args: &mut IconsArgs) {
    let size = args.size.control();
    let edit = edit_menu(args.icons);
    let long = long_menu(30);
    let last = Id::unique("craft-ui-press-menu-story-last");
    Row::new(Gap::Section).show(ui, |ui| {
        for (label, entries) in [("Edit", &edit), ("Long", &long)] {
            let button = ui.add(Button::new(label));
            if let Some(command) = PressMenu::new(entries).size(size).show(ui, &button).chosen {
                ui.data_mut(|d| d.insert_temp(last, format!("{command:?}")));
            }
        }
        let chosen = ui.data(|d| d.get_temp::<String>(last)).unwrap_or_else(|| "Nothing chosen".to_owned());
        ui.add(Text::new(chosen).content(Content::Muted));
    });
}
