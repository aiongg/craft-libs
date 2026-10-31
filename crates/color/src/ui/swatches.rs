//! [`SwatchesPanel`] (F04.13): Photoshop's Swatches panel on a [`SwatchLibrary`]: groups of flat
//! touching squares, click to use, the panel menu for new swatches, groups and swatch files.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use super::PickedColor;
use crate::palette::{SwatchColor, SwatchEntry, SwatchFormat, SwatchGroup};

craft_ui::words! {
    /// The Swatches panel's words, English by default.
    #[non_exhaustive]
    pub struct SwatchesLabels {
        new_swatch: "swatches.newSwatch" = "New Swatch…";
        new_group: "swatches.newGroup" = "New Swatch Group…";
        import: "swatches.import" = "Import Swatches…";
        replace: "swatches.replace" = "Replace Swatches…";
        export: "swatches.export" = "Export Swatches…";
        export_exchange: "swatches.exportExchange" = "Export Swatches for Exchange…";
        reset: "swatches.reset" = "Reset Swatches";
        create_new: "swatches.createNew" = "Create new swatch";
        /// The name of a new group.
        group: "swatches.group" = "Group";
        /// A new swatch's name; `{n}` numbers it.
        swatch_n: "swatches.swatchN" = "Swatch {n}";
        /// The status line after Add to Swatches; `{name}` is the swatch's name.
        added: "swatches.added" = "Added swatch {name}";
    }
}

/// The app's swatches: groups of named swatches, names unique across the library. Kept by the
/// app (a settings section or its presets store).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SwatchLibrary {
    pub groups: Vec<SwatchGroup>,
}

impl Default for SwatchLibrary {
    /// The default swatches ([`default_swatches`](crate::palette::default_swatches)).
    fn default() -> Self {
        SwatchLibrary { groups: crate::palette::default_swatches() }
    }
}

/// Where a swatch is: its group's index and its index in the group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SwatchAt {
    pub group: usize,
    pub index: usize,
}

impl SwatchLibrary {
    /// Every swatch, in order.
    pub fn total(&self) -> usize {
        self.groups.iter().map(|g| g.entries.len()).sum()
    }

    /// The swatch named `name`.
    pub fn find(&self, name: &str) -> Option<SwatchAt> {
        todo!()
    }

    pub fn get(&self, at: SwatchAt) -> Option<&SwatchEntry> {
        self.groups.get(at.group)?.entries.get(at.index)
    }

    /// Adds `color` to group `group` (the last group when `None`, a new "Swatches" group when the
    /// library is empty) under a free name ("Swatch N" when `name` is `None` or taken); where it
    /// went.
    // pc: crates/engine/src/cmd/swatch_cmds.rs (`swatches.add`).
    pub fn add(&mut self, color: SwatchColor, name: Option<&str>, group: Option<usize>) -> SwatchAt {
        todo!()
    }

    /// Add to Swatches from the picker: in the model the user typed (CMYK inks or Lab values the
    /// picker kept), else RGB.
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:260 `add_from_picker`.
    pub fn add_picked(&mut self, picked: &PickedColor) -> SwatchAt {
        todo!()
    }

    /// Renames a swatch; `false` when the name is empty or taken.
    pub fn rename(&mut self, at: SwatchAt, name: &str) -> bool {
        todo!()
    }

    pub fn remove(&mut self, at: SwatchAt) -> Option<SwatchEntry> {
        todo!()
    }

    /// Moves a swatch to the end of group `to`.
    pub fn move_to(&mut self, at: SwatchAt, to: usize) {
        todo!()
    }

    /// Adds an empty group named `name` (made unique); its index.
    pub fn new_group(&mut self, name: &str) -> usize {
        todo!()
    }

    /// Appends the groups read from a file (names made unique), or replaces the library with
    /// them.
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:155 `import_bytes` (`mode: append | replace`).
    pub fn import(&mut self, groups: Vec<SwatchGroup>, replace: bool) {
        todo!()
    }
}

/// A Swatches panel menu entry the app carries out: the file entries need the app's file dialogs
/// (`craft-dialogs`), which this crate does not depend on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwatchesAction {
    /// Import Swatches… (append) or Replace Swatches…: the app opens a file, then calls
    /// [`read_swatches`](crate::palette::read_swatches) and [`SwatchLibrary::import`].
    Import { replace: bool },
    /// Export Swatches… (`.aco`) or Export Swatches for Exchange… (`.ase`): the app asks for a
    /// path, then calls [`write_swatches`](crate::palette::write_swatches) and shows the
    /// [`SwatchReport`](crate::palette::SwatchReport).
    Export(SwatchFormat),
}

/// What [`SwatchesPanel::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
#[must_use = "the host applies the swatch"]
pub struct SwatchesPanelResponse {
    /// A swatch was clicked: its colour (through the document's converter) for the foreground, or
    /// for the background when Alt (Ctrl or Cmd too, as PhotoCraft) was held.
    pub used: Option<(SwatchAt, Srgba, bool)>,
    /// The library changed (new, rename, delete, move, group edits): the app saves it.
    pub changed: bool,
}

/// The Swatches panel on a [`SwatchLibrary`], built on craft-ui's
/// [`PresetBrowser`](craft_ui::presets::PresetBrowser): groups as folders, swatches as flat
/// touching squares with no gap or corner radius (Photoshop, PhotoCraft #5103), the size the
/// browser's view menu chooses.
///
/// - Click uses the swatch for the foreground; Alt-click (or Ctrl or Cmd) for the background.
///   The swatch is selected. Double-click or Enter renames in place; Delete removes; dragging
///   onto a group moves it. Swatches do not drop onto the canvas.
/// - The footer's + is New Swatch: the foreground colour into the selected swatch's group, named
///   "Swatch N", then renamed in place (Photoshop asks for the name).
/// - Tooltips name the swatch; AccessKit as the preset browser's grid, each swatch named.
pub struct SwatchesPanel<'a> {
    id_salt: IdSalt,
    library: &'a mut SwatchLibrary,
    cmyk: &'a CmykConverter,
    foreground: Srgba,
}

impl<'a> SwatchesPanel<'a> {
    /// The panel over `library`; `cmyk` draws CMYK swatches (the document's converter);
    /// `foreground` is what New Swatch adds.
    pub fn new(id_salt: impl AsIdSalt, library: &'a mut SwatchLibrary, cmyk: &'a CmykConverter, foreground: Srgba) -> SwatchesPanel<'a> {
        SwatchesPanel { id_salt: IdSalt::new(id_salt), library, cmyk, foreground }
    }

    // pc: crates/ui-egui/src/panels/swatches_ui.rs:63 `panel` (the colour table cached per library
    // at :33 `colors`, `browser` with `flush: true`), :86 `handle` (Select → use, Activate → rename, Delete,
    // Move, group events), :121 `new_swatch`, :53 `use_swatch`.
    pub fn show(self, ui: &mut Ui) -> SwatchesPanelResponse {
        todo!()
    }

    /// The panel menu's entries, for the docking host's `panel_menu` hook: New Swatch…, New
    /// Swatch Group…, a separator, Import Swatches…, Replace Swatches…, Export Swatches… and
    /// Export Swatches for Exchange… (both disabled when the library has no swatch), a separator,
    /// Reset Swatches. New Swatch, New Group and Reset act on `library` here; the file entries
    /// come back as a [`SwatchesAction`].
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:208 `panel_menu`.
    pub fn panel_menu(ui: &mut Ui, library: &mut SwatchLibrary, foreground: Srgba) -> Option<SwatchesAction> {
        todo!()
    }
}
