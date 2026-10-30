//! The menu commands as command-palette entries.

use craft_keymap::{CommandRef, Keymap};
use craft_ui::behavior::PaletteEntry;
use craft_ui::menu::MenuItem;

use crate::bar::{Convert, EnglishHook, keymap_platform};
use crate::model::Builtin;
use crate::{Gate, LayoutOptions, MenuBar, Target, apply_keymap, layout};

impl MenuBar {
    /// The commands of the menus as a command palette lists them, in menu order.
    ///
    /// The menus are laid out for `target` first, so the paths are the titles the user sees in the
    /// bar ("File › Export › PNG…"; on a Mac the app menu carries the app's name), and the
    /// shortcuts are the `keymap`'s, user overrides included, written for the target's platform.
    /// A disabled command is listed with [`PaletteEntry::is_enabled`] false. Separators, submenu
    /// rows, AppKit's own items and the [`Builtin`] commands (which the crate runs itself) are not
    /// listed; the open documents of the Window menu are. A check or radio item is a plain entry:
    /// the palette has no checked state.
    ///
    /// `os` is the host's operating system (`ctx.os()`), which picks the platform the shortcuts
    /// are written for on a non-Mac target. Building the list lays out the whole bar, so a host
    /// caches the result and rebuilds it when the bar or the keymap changes, not every frame.
    ///
    /// `english` gives a command's or menu title's English name (as [`crate::MenuBarUi::english`]),
    /// asked on every target, so the search finds a translated command by its English name.
    pub fn palette_entries(
        &self,
        keymap: &Keymap,
        target: Target,
        options: &LayoutOptions,
        english: Option<&EnglishHook<'_>>,
        os: egui::os::OperatingSystem,
    ) -> Vec<PaletteEntry<CommandRef>> {
        let mut bar = self.clone();
        apply_keymap(&mut bar, keymap);
        let laid_out = layout(&bar, target, options).bar;
        let convert = Convert { gate: Gate::default(), platform: keymap_platform(target, os), mnemonics: true, english, preferred: &[] };
        let menus: Vec<MenuItem<CommandRef>> = laid_out.menus.iter().map(|m| convert.sub(&m.title, &m.children)).collect();
        let mut entries = PaletteEntry::from_menu(&menus);
        entries.retain(|e| Builtin::of(e.command()).is_none());
        entries
    }
}
