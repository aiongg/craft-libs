//! [`SwatchesPanel`] (F04.13): Photoshop's Swatches panel on a [`SwatchLibrary`]: groups of flat
//! touching squares, click to use, the panel menu for new swatches, groups and swatch files.

use std::collections::HashSet;

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;
use craft_ui::behavior::{DropPosition, Move};
use craft_ui::composites::{NoticeCenter, Tone};
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::localized::Words;
use craft_ui::localized::words::Counted;
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::presets::{FooterEvent, PresetBrowser, PresetEvent, PresetFooter, PresetLabels, PresetReorder, PresetRow, PresetState};
use egui::{AsIdSalt, Color32, Context, CornerRadius, Id, IdSalt, Ui};

use super::PickedColor;
use crate::palette::{SwatchColor, SwatchEntry, SwatchFormat, SwatchGroup, SwatchNote, SwatchReport};

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
        /// The panel's name for a screen reader.
        title: "swatches.title" = "Swatches";
        /// The notice title after an import that changed something; the lines are the notes.
        notice_import: "swatches.noticeImport" = "Swatches imported with changes";
        /// The notice title after an export that changed something.
        notice_export: "swatches.noticeExport" = "Swatches exported with changes";
        groups_flattened: "swatches.groupsFlattened" = "The format has no groups; the swatches were written as one list.";
        /// The notes below are the plural forms of counted words; `{n}` is the count.
        book_read: "swatches.bookRead" = "{n} colour-book colours were read as black.";
        book_left_out: "swatches.bookLeftOut" = "{n} colour-book colours were left out.";
        skipped: "swatches.skipped" = "{n} colours in a model that cannot be read were skipped.";
        converted: "swatches.converted" = "{n} colours were converted to RGB.";
        kinds_dropped: "swatches.kindsDropped" = "{n} spot or global colours were written as process colours.";
        truncated: "swatches.truncated" = "{n} colours past the 256 the format holds were left out.";
        names_cut: "swatches.namesCut" = "{n} names were cut to the length the format holds.";
        /// The value read for a colour-book colour, which the panel cannot show.
        book_color: "swatches.bookColor" = "Colour book colour";
    }
}

const NOTE_BOOK_READ: Counted =
    Counted::new("swatches.bookRead", "{n} colour-book colour was read as black.", "{n} colour-book colours were read as black.");
const NOTE_BOOK_LEFT_OUT: Counted =
    Counted::new("swatches.bookLeftOut", "{n} colour-book colour was left out.", "{n} colour-book colours were left out.");
const NOTE_SKIPPED: Counted = Counted::new(
    "swatches.skipped",
    "{n} colour in a model that cannot be read was skipped.",
    "{n} colours in a model that cannot be read were skipped.",
);
const NOTE_CONVERTED: Counted = Counted::new("swatches.converted", "{n} colour was converted to RGB.", "{n} colours were converted to RGB.");
const NOTE_KINDS_DROPPED: Counted = Counted::new(
    "swatches.kindsDropped",
    "{n} spot or global colour was written as a process colour.",
    "{n} spot or global colours were written as process colours.",
);
const NOTE_TRUNCATED: Counted = Counted::new(
    "swatches.truncated",
    "{n} colour past the 256 the format holds was left out.",
    "{n} colours past the 256 the format holds were left out.",
);
const NOTE_NAMES_CUT: Counted =
    Counted::new("swatches.namesCut", "{n} name was cut to the length the format holds.", "{n} names were cut to the length the format holds.");

/// The notes' count words, for [`COUNTS`](super::COUNTS).
pub(super) const COUNTS: &[Counted] =
    &[NOTE_BOOK_READ, NOTE_BOOK_LEFT_OUT, NOTE_SKIPPED, NOTE_CONVERTED, NOTE_KINDS_DROPPED, NOTE_TRUNCATED, NOTE_NAMES_CUT];

/// What the browser names its rows by. Swatch names are unique across the library and group names
/// among groups, so both survive reordering.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Key {
    Group(String),
    Swatch(String),
}

/// The thumbnail sizes the footer slider offers; Photoshop's small thumbnail is the default.
const THUMB_RANGE: std::ops::RangeInclusive<f32> = 16.0..=64.0;
const DEFAULT_THUMB: f32 = 22.0;
/// Room under the browser for its footer.
const FOOTER_HEIGHT: f32 = 30.0;

/// What the panel keeps between passes.
#[derive(Clone)]
struct PanelState {
    browser: PresetState<Key>,
    selected: Option<Key>,
    /// A row to open the rename bar on in the next pass.
    rename: Option<Key>,
}

impl Default for PanelState {
    fn default() -> Self {
        let mut browser = PresetState::new();
        browser.thumb = DEFAULT_THUMB;
        PanelState { browser, selected: None, rename: None }
    }
}

const CMD_NEW_SWATCH: &str = "swatches.newSwatch";
const CMD_NEW_GROUP: &str = "swatches.newGroup";
const CMD_IMPORT: &str = "swatches.import";
const CMD_REPLACE: &str = "swatches.replace";
const CMD_EXPORT: &str = "swatches.export";
const CMD_EXPORT_EXCHANGE: &str = "swatches.exportExchange";
const CMD_RESET: &str = "swatches.reset";

/// The panel's selection, for the menu commands (one Swatches panel per context).
fn selection_id() -> Id {
    Id::unique("craft-color-swatches-selection")
}

/// Where [`SwatchesPanel::run_command`] leaves the row it made, for the panel to select and rename.
fn pending_id() -> Id {
    Id::unique("craft-color-swatches-pending")
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
        self.groups.iter().enumerate().find_map(|(group, g)| g.entries.iter().position(|e| e.name == name).map(|index| SwatchAt { group, index }))
    }

    pub fn get(&self, at: SwatchAt) -> Option<&SwatchEntry> {
        self.groups.get(at.group)?.entries.get(at.index)
    }

    /// Adds `color` to group `group` (the last group when `None`, a new "Swatches" group when the
    /// library is empty) under a free name ("Swatch N" when `name` is `None` or taken); where it
    /// went.
    // pc: crates/engine/src/cmd/swatch_cmds.rs (`swatches.add`).
    pub fn add(&mut self, color: SwatchColor, name: Option<&str>, group: Option<usize>) -> SwatchAt {
        self.add_numbered(color, name, group, &SwatchesLabels::english().swatch_n)
    }

    /// [`add`](SwatchLibrary::add) with the template (`{n}`) that names a swatch left without one.
    fn add_numbered(&mut self, color: SwatchColor, name: Option<&str>, group: Option<usize>, template: &str) -> SwatchAt {
        let name = name.map(str::trim).filter(|n| !n.is_empty() && self.find(n).is_none()).map_or_else(|| self.free_name(template), str::to_owned);
        if self.groups.is_empty() {
            self.groups.push(SwatchGroup { name: "Swatches".to_owned(), entries: Vec::new() });
        }
        let last = self.groups.len() - 1;
        let group = group.map_or(last, |g| g.min(last));
        let entries = &mut self.groups[group].entries;
        entries.push(SwatchEntry::new(name, color));
        SwatchAt { group, index: entries.len() - 1 }
    }

    /// The first free name of `template`, numbered from 1.
    fn free_name(&self, template: &str) -> String {
        let taken: HashSet<&str> = self.groups.iter().flat_map(|g| g.entries.iter().map(|e| e.name.as_str())).collect();
        (1..).map(|n| craft_i18n::fmt_plain(template, &[("n", &n.to_string())])).find(|name| !taken.contains(name.as_str())).unwrap_or_default()
    }

    /// Add to Swatches from the picker: in the model the user typed (CMYK inks or Lab values the
    /// picker kept), else RGB.
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:260 `add_from_picker`.
    pub fn add_picked(&mut self, picked: &PickedColor) -> SwatchAt {
        let color = match picked.model {
            Some(m) if m.model == "cmyk" => SwatchColor::Cmyk([0, 1, 2, 3].map(|i| (m.values[i] / 100.0).clamp(0.0, 1.0))),
            Some(m) if m.model == "lab" => SwatchColor::Lab([m.values[0], m.values[1], m.values[2]]),
            _ => SwatchColor::Rgb(picked.color.to_rgb_array().map(|v| v.clamp(0.0, 1.0))),
        };
        self.add(color, None, None)
    }

    /// Renames a swatch; `false` when the name is empty or taken.
    pub fn rename(&mut self, at: SwatchAt, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || self.get(at).is_none() {
            return false;
        }
        if self.find(name).is_some_and(|other| other != at) {
            return false;
        }
        self.groups[at.group].entries[at.index].name = name.to_owned();
        true
    }

    pub fn remove(&mut self, at: SwatchAt) -> Option<SwatchEntry> {
        let entries = &mut self.groups.get_mut(at.group)?.entries;
        (at.index < entries.len()).then(|| entries.remove(at.index))
    }

    /// Moves a swatch to the end of group `to`.
    pub fn move_to(&mut self, at: SwatchAt, to: usize) {
        if to >= self.groups.len() {
            return;
        }
        if let Some(entry) = self.remove(at) {
            self.groups[to].entries.push(entry);
        }
    }

    /// Adds an empty group named `name` (made unique); its index.
    pub fn new_group(&mut self, name: &str) -> usize {
        let name = self.unique_group_name(name.trim());
        self.groups.push(SwatchGroup { name, entries: Vec::new() });
        self.groups.len() - 1
    }

    /// `base`, or `base 1`, `base 2`… when a group has it.
    fn unique_group_name(&self, base: &str) -> String {
        let taken = |n: &str| self.groups.iter().any(|g| g.name == n);
        if !taken(base) { base.to_owned() } else { (1..).map(|i| format!("{base} {i}")).find(|n| !taken(n)).unwrap_or_default() }
    }

    /// Renames a group; `false` when the name is empty or another group has it.
    fn rename_group(&mut self, group: usize, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || group >= self.groups.len() || self.groups.iter().enumerate().any(|(i, g)| i != group && g.name == name) {
            return false;
        }
        self.groups[group].name = name.to_owned();
        true
    }

    /// Appends the groups read from a file (names made unique), or replaces the library with
    /// them.
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:155 `import_bytes` (`mode: append | replace`).
    pub fn import(&mut self, groups: Vec<SwatchGroup>, replace: bool) {
        if replace {
            self.groups.clear();
        }
        let mut taken: HashSet<String> = self.groups.iter().flat_map(|g| g.entries.iter().map(|e| e.name.clone())).collect();
        let mut group_taken: HashSet<String> = self.groups.iter().map(|g| g.name.clone()).collect();
        for mut group in groups {
            if group_taken.contains(&group.name) {
                group.name = (1..).map(|i| format!("{} {i}", group.name)).find(|n| !group_taken.contains(n)).unwrap_or_default();
            }
            group_taken.insert(group.name.clone());
            for entry in &mut group.entries {
                if taken.contains(&entry.name) {
                    entry.name = (2..).map(|i| format!("{} {i}", entry.name)).find(|n| !taken.contains(n)).unwrap_or_default();
                }
                taken.insert(entry.name.clone());
            }
            self.groups.push(group);
        }
    }

    /// Applies a drag: swatches go into a group (first) or before or after a swatch, groups before
    /// or after a group. Anything else leaves the library as it is.
    pub(super) fn apply_move(&mut self, mv: &Move<Key>) -> bool {
        let swatches: Option<Vec<&str>> = mv.items.iter().map(|k| if let Key::Swatch(n) = k { Some(n.as_str()) } else { None }).collect();
        let groups: Option<Vec<&str>> = mv.items.iter().map(|k| if let Key::Group(n) = k { Some(n.as_str()) } else { None }).collect();
        if let (Some(names), Key::Group(_) | Key::Swatch(_)) = (swatches, &mv.target) {
            return self.move_swatches(&names, &mv.target, mv.position);
        }
        match (groups, &mv.target) {
            (Some(names), Key::Group(target)) => self.move_groups(&names, target, mv.position),
            _ => false,
        }
    }

    fn move_swatches(&mut self, names: &[&str], target: &Key, position: DropPosition) -> bool {
        let moved = |e: &SwatchEntry| names.contains(&e.name.as_str());
        // The slot is counted in the entries that stay, so it survives taking the moved ones out.
        let (group, slot) = match (target, position) {
            (Key::Group(g), DropPosition::Into) => match self.groups.iter().position(|x| x.name == *g) {
                Some(group) => (group, 0),
                None => return false,
            },
            (Key::Swatch(n), DropPosition::Before | DropPosition::After) => {
                let Some(at) = self.find(n) else { return false };
                let entries = &self.groups[at.group].entries;
                let before = entries[..at.index].iter().filter(|e| !moved(e)).count();
                (at.group, before + usize::from(position == DropPosition::After && !moved(&entries[at.index])))
            }
            _ => return false,
        };
        let mut taken = Vec::new();
        for g in &mut self.groups {
            let (out, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut g.entries).into_iter().partition(|e| moved(e));
            g.entries = keep;
            taken.extend(out);
        }
        if taken.is_empty() {
            return false;
        }
        let entries = &mut self.groups[group].entries;
        let slot = slot.min(entries.len());
        entries.splice(slot..slot, taken);
        true
    }

    fn move_groups(&mut self, names: &[&str], target: &str, position: DropPosition) -> bool {
        let moved = |g: &SwatchGroup| names.contains(&g.name.as_str());
        let Some(at) = self.groups.iter().position(|g| g.name == target) else { return false };
        let before = self.groups[..at].iter().filter(|g| !moved(g)).count();
        let slot = match position {
            DropPosition::Before => before,
            DropPosition::After => before + usize::from(!moved(&self.groups[at])),
            DropPosition::Into => return false,
        };
        let (taken, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.groups).into_iter().partition(|g| moved(g));
        if taken.is_empty() {
            self.groups = keep;
            return false;
        }
        self.groups = keep;
        let slot = slot.min(self.groups.len());
        self.groups.splice(slot..slot, taken);
        true
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
        let SwatchesPanel { id_salt, library, cmyk, foreground } = self;
        let id = ui.make_persistent_id(id_salt);
        let labels = SwatchesLabels::of(ui);
        let mut browser_labels = PresetLabels::of(ui);
        browser_labels.new = labels.create_new.clone();
        let mut st: PanelState = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
        if let Some(key) = ui.data_mut(|d| d.remove_temp::<Option<Key>>(pending_id())).flatten() {
            st.selected = Some(key.clone()).filter(|k| matches!(k, Key::Swatch(_)));
            st.rename = Some(key);
        }
        if st.selected.as_ref().is_some_and(|k| !library.has(k)) {
            st.selected = None;
        }

        let mut rows = Vec::with_capacity(library.total() + library.groups.len());
        for group in &library.groups {
            rows.push(PresetRow::group(Key::Group(group.name.clone()), group.name.as_str(), 0).count(group.entries.len()));
            for entry in &group.entries {
                let value = value_text(&entry.color, &labels.book_color);
                let named = format!("{}, {value}", entry.name);
                rows.push(PresetRow::item(Key::Swatch(entry.name.clone()), entry.name.as_str(), 1).tooltip(named.clone()).accessible_name(named));
            }
        }
        let out = {
            let colors: std::collections::HashMap<&str, &SwatchColor> =
                library.groups.iter().flat_map(|g| g.entries.iter().map(|e| (e.name.as_str(), &e.color))).collect();
            let mut browser = PresetBrowser::new(id.with("browser"), labels.title.as_str(), &mut st.browser, &rows)
                .selected(st.selected.as_ref())
                .flush(true)
                .toolbar(true)
                .thumb_range(THUMB_RANGE)
                .max_height((ui.available_height() - FOOTER_HEIGHT).max(60.0))
                .reorder(PresetReorder::items_and_groups().accepts(|mv, _| move_allowed(mv)))
                .labels(browser_labels.clone());
            if let Some(key) = st.rename.take() {
                browser = browser.start_rename(key);
            }
            browser.show(ui, |painter, rect, info| {
                let color = match info.key {
                    Key::Swatch(name) => colors.get(name.as_str()).map_or(Srgba::rgb(0.5, 0.5, 0.5), |c| c.to_srgb(cmyk)),
                    Key::Group(_) => Srgba::rgb(0.5, 0.5, 0.5),
                };
                painter.rect_filled(rect, CornerRadius::ZERO, Color32::from(color));
            })
        };
        let footer = PresetFooter::new(&mut st.browser, &browser_labels).thumb_range(THUMB_RANGE).can_delete(st.selected.is_some()).show(ui);

        let mut response = SwatchesPanelResponse::default();
        let mut actions =
            Actions { library, st: &mut st, response: &mut response, foreground, template: &labels.swatch_n, group_name: &labels.group, cmyk };
        for event in out.events {
            actions.event(event);
        }
        for event in footer {
            actions.footer(event);
        }
        if st.rename.is_some() {
            ui.ctx().request_repaint();
        }
        let selected = st.selected.clone();
        ui.data_mut(|d| {
            d.insert_temp(selection_id(), selected);
            d.insert_temp(id, st);
        });
        response
    }

    /// The panel menu's rows, for the docking host's `panel_menu_entries` hook: New Swatch…, New
    /// Swatch Group…, a separator, Import Swatches…, Replace Swatches…, Export Swatches… and
    /// Export Swatches for Exchange… (both disabled when the library has no swatch), a separator,
    /// Reset Swatches. The commands come back as `DockEvent::PanelCommand`; the host passes them
    /// to [`run_command`](SwatchesPanel::run_command).
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:208 `panel_menu`.
    pub fn panel_menu(ctx: &Context, library: &SwatchLibrary) -> Vec<MenuEntry<PanelCommand>> {
        let labels = SwatchesLabels::of_ctx(ctx);
        let empty = library.total() == 0;
        let item = |label: &str, command: &str, enabled: bool| MenuEntry::from(MenuItem::new(label, command.to_owned()).enabled(enabled));
        vec![
            item(&labels.new_swatch, CMD_NEW_SWATCH, true),
            item(&labels.new_group, CMD_NEW_GROUP, true),
            MenuEntry::Separator,
            item(&labels.import, CMD_IMPORT, true),
            item(&labels.replace, CMD_REPLACE, true),
            item(&labels.export, CMD_EXPORT, !empty),
            item(&labels.export_exchange, CMD_EXPORT_EXCHANGE, !empty),
            MenuEntry::Separator,
            item(&labels.reset, CMD_RESET, true),
        ]
    }

    /// Carries out a command of [`panel_menu`](SwatchesPanel::panel_menu). New Swatch adds the
    /// foreground to the selected swatch's (or group's) group, as the footer's New does, and the
    /// panel selects it and opens its name for editing in its next pass; New Swatch Group, and
    /// Reset Swatches act on `library`. The file entries come back as an action.
    pub fn run_command(ctx: &Context, command: &str, library: &mut SwatchLibrary, foreground: Srgba) -> SwatchesMenuResponse {
        let labels = SwatchesLabels::of_ctx(ctx);
        let mut response = SwatchesMenuResponse::default();
        let pending = match command {
            CMD_NEW_SWATCH => {
                let selected = ctx.data(|d| d.get_temp::<Option<Key>>(selection_id())).flatten();
                let group = selected.as_ref().and_then(|key| library.group_of(key));
                let color = SwatchColor::Rgb(foreground.to_rgb_array().map(|v| v.clamp(0.0, 1.0)));
                let at = library.add_numbered(color, None, group, &labels.swatch_n);
                response.changed = true;
                library.get(at).map(|e| Key::Swatch(e.name.clone()))
            }
            CMD_NEW_GROUP => {
                let at = library.new_group(&labels.group);
                response.changed = true;
                Some(Key::Group(library.groups[at].name.clone()))
            }
            CMD_RESET => {
                library.groups = crate::palette::default_swatches();
                response.changed = true;
                None
            }
            CMD_IMPORT => {
                response.action = Some(SwatchesAction::Import { replace: false });
                None
            }
            CMD_REPLACE => {
                response.action = Some(SwatchesAction::Import { replace: true });
                None
            }
            CMD_EXPORT => {
                response.action = Some(SwatchesAction::Export(SwatchFormat::Aco));
                None
            }
            CMD_EXPORT_EXCHANGE => {
                response.action = Some(SwatchesAction::Export(SwatchFormat::Ase));
                None
            }
            _ => None,
        };
        if let Some(key) = pending {
            ctx.data_mut(|d| d.insert_temp(pending_id(), Some(key)));
            ctx.request_repaint();
        }
        response
    }

    /// The notes of a swatch file read or written as sentences, in the words installed in `ctx`;
    /// counts follow the locale's plural rules. `writing` tells an export's notes from an
    /// import's, which word a colour-book colour differently.
    pub fn note_lines(ctx: &Context, report: &SwatchReport, writing: bool) -> Vec<String> {
        let labels = SwatchesLabels::of_ctx(ctx);
        let words = Words::of_ctx(ctx);
        let count = |counted: Counted, text: &str, n: usize| words.counted(ctx, counted, text, n as u64);
        report
            .notes
            .iter()
            .map(|note| match *note {
                SwatchNote::BookColors(n) if writing => count(NOTE_BOOK_LEFT_OUT, &labels.book_left_out, n),
                SwatchNote::BookColors(n) => count(NOTE_BOOK_READ, &labels.book_read, n),
                SwatchNote::Skipped(n) => count(NOTE_SKIPPED, &labels.skipped, n),
                SwatchNote::GroupsFlattened => labels.groups_flattened.clone(),
                SwatchNote::ConvertedToRgb(n) => count(NOTE_CONVERTED, &labels.converted, n),
                SwatchNote::KindsDropped(n) => count(NOTE_KINDS_DROPPED, &labels.kinds_dropped, n),
                SwatchNote::Truncated(n) => count(NOTE_TRUNCATED, &labels.truncated, n),
                SwatchNote::NamesCut(n) => count(NOTE_NAMES_CUT, &labels.names_cut, n),
            })
            .collect()
    }

    /// Posts the notes of an import (or, with `writing`, an export) to the app's notice center as
    /// a warning, one line per note; `None` when there is nothing to say.
    pub fn post_notes(ctx: &Context, center: &mut NoticeCenter, report: &SwatchReport, writing: bool) -> Option<u64> {
        let lines = Self::note_lines(ctx, report, writing);
        if lines.is_empty() {
            return None;
        }
        let labels = SwatchesLabels::of_ctx(ctx);
        let title = if writing { labels.notice_export } else { labels.notice_import };
        center.post(title, lines, Tone::Warn, None, &[])
    }
}

/// What [`SwatchesPanel::run_command`] did.
#[derive(Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app saves a changed library and carries out the action"]
pub struct SwatchesMenuResponse {
    /// The library changed: the app saves it.
    pub changed: bool,
    /// A file entry the app carries out.
    pub action: Option<SwatchesAction>,
}

/// The events of one pass applied to the library and the panel's state.
struct Actions<'a> {
    library: &'a mut SwatchLibrary,
    st: &'a mut PanelState,
    response: &'a mut SwatchesPanelResponse,
    foreground: Srgba,
    template: &'a str,
    group_name: &'a str,
    cmyk: &'a CmykConverter,
}

impl Actions<'_> {
    fn event(&mut self, event: PresetEvent<Key>) {
        match event {
            PresetEvent::Select { key: Key::Swatch(name), modifiers } => {
                self.st.selected = Some(Key::Swatch(name.clone()));
                if let Some(at) = self.library.find(&name)
                    && let Some(entry) = self.library.get(at)
                {
                    let background = modifiers.alt || modifiers.command || modifiers.ctrl;
                    self.response.used = Some((at, entry.color.to_srgb(self.cmyk), background));
                }
            }
            PresetEvent::Activate { key: Key::Swatch(name), .. } => {
                self.st.selected = Some(Key::Swatch(name.clone()));
                self.st.rename = Some(Key::Swatch(name));
            }
            PresetEvent::Rename { key: Key::Swatch(old), text } => {
                if let Some(at) = self.library.find(&old)
                    && self.library.rename(at, &text)
                {
                    self.response.changed = true;
                    if self.st.selected == Some(Key::Swatch(old)) {
                        self.st.selected = Some(Key::Swatch(text.trim().to_owned()));
                    }
                }
            }
            PresetEvent::Rename { key: Key::Group(old), text } => {
                if let Some(group) = self.library.groups.iter().position(|g| g.name == old)
                    && self.library.rename_group(group, &text)
                {
                    self.response.changed = true;
                }
            }
            PresetEvent::Delete(key) => self.delete(&key),
            PresetEvent::MoveTo { key: Key::Swatch(name), group: Key::Group(to) } => {
                if let (Some(at), Some(to)) = (self.library.find(&name), self.library.groups.iter().position(|g| g.name == to)) {
                    self.library.move_to(at, to);
                    self.response.changed = true;
                }
            }
            PresetEvent::Reorder(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                self.response.changed |= self.library.apply_move(&value);
            }
            _ => {}
        }
    }

    fn footer(&mut self, event: FooterEvent) {
        match event {
            FooterEvent::New => self.new_swatch(),
            FooterEvent::NewGroup => {
                let at = self.library.new_group(self.group_name);
                self.st.rename = Some(Key::Group(self.library.groups[at].name.clone()));
                self.response.changed = true;
            }
            FooterEvent::Delete => {
                if let Some(key) = self.st.selected.clone() {
                    self.delete(&key);
                }
            }
            _ => {}
        }
    }

    /// The foreground colour into the selected swatch's group, named and selected, then renamed
    /// in place (Photoshop asks for the name).
    fn new_swatch(&mut self) {
        let group = self.st.selected.as_ref().and_then(|key| self.library.group_of(key));
        let color = SwatchColor::Rgb(self.foreground.to_rgb_array().map(|v| v.clamp(0.0, 1.0)));
        let at = self.library.add_numbered(color, None, group, self.template);
        if let Some(entry) = self.library.get(at) {
            let key = Key::Swatch(entry.name.clone());
            self.st.selected = Some(key.clone());
            self.st.rename = Some(key);
        }
        self.response.changed = true;
    }

    fn delete(&mut self, key: &Key) {
        let removed = match key {
            Key::Swatch(name) => self.library.find(name).and_then(|at| self.library.remove(at)).is_some(),
            Key::Group(name) => match self.library.groups.iter().position(|g| g.name == *name) {
                Some(group) => {
                    self.library.groups.remove(group);
                    true
                }
                None => false,
            },
        };
        if removed {
            self.response.changed = true;
        }
    }
}

impl SwatchLibrary {
    /// The index of the group `key` is, or is in.
    fn group_of(&self, key: &Key) -> Option<usize> {
        match key {
            Key::Swatch(name) => self.find(name).map(|at| at.group),
            Key::Group(name) => self.groups.iter().position(|g| g.name == *name),
        }
    }

    /// Whether `key` names a row of the library.
    fn has(&self, key: &Key) -> bool {
        match key {
            Key::Swatch(name) => self.find(name).is_some(),
            Key::Group(name) => self.groups.iter().any(|g| g.name == *name),
        }
    }
}

/// Whether a drag is one the library takes: swatches into a group or beside a swatch, groups
/// beside a group. A copy is not offered.
fn move_allowed(mv: &Move<Key>) -> bool {
    if mv.copy {
        return false;
    }
    let swatches = mv.items.iter().all(|k| matches!(k, Key::Swatch(_)));
    let groups = mv.items.iter().all(|k| matches!(k, Key::Group(_)));
    match (&mv.target, mv.position) {
        (Key::Group(_), DropPosition::Into) | (Key::Swatch(_), DropPosition::Before | DropPosition::After) => swatches,
        (Key::Group(_), DropPosition::Before | DropPosition::After) => groups,
        _ => false,
    }
}

/// A swatch's value in its own model, for the tooltip and a screen reader.
fn value_text(color: &SwatchColor, book: &str) -> String {
    let pct = |v: f32| (v * 100.0).round();
    match *color {
        SwatchColor::Rgb(v) => {
            let [r, g, b] = v.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
            format!("#{r:02x}{g:02x}{b:02x}")
        }
        SwatchColor::Hsb([h, s, b]) => format!("H {} S {}% B {}%", h.round(), pct(s), pct(b)),
        SwatchColor::Cmyk([c, m, y, k]) => format!("C {}% M {}% Y {}% K {}%", pct(c), pct(m), pct(y), pct(k)),
        SwatchColor::Lab([l, a, b]) => format!("L {} a {} b {}", l.round(), a.round(), b.round()),
        SwatchColor::Gray(k) => format!("K {}%", pct(k)),
        SwatchColor::Book { .. } => book.to_owned(),
    }
}
