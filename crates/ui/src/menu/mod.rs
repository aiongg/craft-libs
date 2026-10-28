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
//! - [`mnemonics`]: whether menus have mnemonics here (off on macOS); [`set_mnemonics`] overrides.
//!
//! Menus draw with the list box rows' tokens (`listbox.*`) and their own `menu.*` (least width,
//! shortcut gap, scroll strip, the accent-filled active row); [`Menu::style`] is the pure style
//! function, dumped under `crates/ui/tests/dumps/components/menu.txt`.

mod item;
mod mnemonic;
mod nav;
mod popup;
mod row;
#[cfg(test)]
mod tests;

pub use item::{ItemKind, MenuEntry, MenuItem};
pub use mnemonic::{mnemonics, set_mnemonics};
pub use nav::MenuStep;
pub use popup::{Menu, MenuResponse};
pub use row::MenuStyle;

use egui::{Id, Key, KeyboardShortcut, Modifiers, Popup, Ui};

use crate::controls::{Button, Kbd, SIZE_NAMES, size_arg};
use crate::icons::Icon;
use crate::primitives::{Content, Gap, Row, Text};
use crate::story::{Args, Level, Story};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Menu"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "composites/menu/menus",
        level: Level::Composites,
        component: "Menu",
        name: "Menus",
        args: || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("icons", true).with_number("rows", 40.0, 5.0, 200.0),
        show: show_menus,
    }]
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
fn show_menus(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let edit = edit_menu(args.bool("icons"));
    let long = long_menu(args.number("rows").round().max(1.0) as usize);
    let last = Id::unique("craft-ui-menu-story-last");
    Row::new(Gap::Section).show(ui, |ui| {
        for (label, entries) in [("Edit", &edit), ("Long", &long)] {
            let button = ui.add(Button::new(label));
            let shown = Menu::new(entries).size(size).show(ui, Popup::menu(&button));
            if let Some(command) = shown.chosen {
                ui.data_mut(|d| d.insert_temp(last, format!("{command:?}")));
            }
        }
        let chosen = ui.data(|d| d.get_temp::<String>(last)).unwrap_or_else(|| "Nothing chosen".to_owned());
        ui.add(Text::new(chosen).content(Content::Muted));
    });
}
