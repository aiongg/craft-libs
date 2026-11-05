//! [`SwatchesPanel`] (F04.13): Photoshop's Swatches panel on a [`SwatchLibrary`]: groups of flat
//! touching squares, click to use, the panel menu for new swatches, groups and swatch files.

use std::collections::HashSet;

use craft_cms::convert::CmykConverter;
use craft_host::{Host, PickId, PickRequest, Picked};
use craft_theme::Srgba;
use craft_ui::behavior::{DropPosition, Move};
use craft_ui::composites::{StatusLine, Tone};
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::localized::Words;
use craft_ui::localized::words::Counted;
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::presets::{PresetBrowser, PresetEvent, PresetFooter, PresetFooterEvent, PresetLabels, PresetReorder, PresetRow, PresetState};
use egui::{AsIdSalt, Color32, Context, CornerRadius, Id, IdSalt, Ui};

use super::PickedColor;
use crate::palette::{SwatchColor, SwatchEntry, SwatchFormat, SwatchGroup, SwatchNote, SwatchReport};

/// The most swatches a library holds (PhotoCraft's `MAX_SWATCHES`).
pub const MAX_SWATCHES: usize = 100_000;

/// The most groups a library holds (PhotoCraft's `MAX_GROUPS`).
pub const MAX_GROUPS: usize = 10_000;

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
        swatch_n: "swatches.swatchN" = "Color Swatch {n}";
        /// The status line after Add to Swatches; `{name}` is the swatch's name.
        added: "swatches.added" = "Added swatch {name}";
        /// The panel's name for a screen reader.
        title: "swatches.title" = "Swatches";
        /// The start of the status line after an import that changed something; the notes follow.
        notice_import: "swatches.noticeImport" = "Swatches imported with changes";
        /// The start of the status line after an export that changed something.
        notice_export: "swatches.noticeExport" = "Swatches exported with changes";
        /// The status line when a swatch file could not be imported; `{reason}` says why.
        import_failed: "swatches.importFailed" = "Swatches could not be imported: {reason}";
        /// The status line when swatches could not be exported; `{reason}` says why.
        export_failed: "swatches.exportFailed" = "Swatches could not be exported: {reason}";
        groups_flattened: "swatches.groupsFlattened" = "The format has no groups; the swatches were written as one list.";
        /// The notes below are the plural forms of counted words; `{n}` is the count.
        book_read: "swatches.bookRead" = "{n} color-book colors were read as black.";
        book_left_out: "swatches.bookLeftOut" = "{n} color-book colors were left out.";
        skipped: "swatches.skipped" = "{n} colors in a model that cannot be read were skipped.";
        converted: "swatches.converted" = "{n} colors were converted to RGB.";
        kinds_dropped: "swatches.kindsDropped" = "{n} spot or global colors were written as process colors.";
        truncated: "swatches.truncated" = "{n} colors past the 256 the format holds were left out.";
        names_cut: "swatches.namesCut" = "{n} names were cut to the length the format holds.";
        /// The value read for a colour-book colour, which the panel cannot show.
        book_color: "swatches.bookColor" = "Color book color";
    }
}

const NOTE_BOOK_READ: Counted =
    Counted::new("swatches.bookRead", "{n} color-book color was read as black.", "{n} color-book colors were read as black.");
const NOTE_BOOK_LEFT_OUT: Counted =
    Counted::new("swatches.bookLeftOut", "{n} color-book color was left out.", "{n} color-book colors were left out.");
const NOTE_SKIPPED: Counted = Counted::new(
    "swatches.skipped",
    "{n} color in a model that cannot be read was skipped.",
    "{n} colors in a model that cannot be read were skipped.",
);
const NOTE_CONVERTED: Counted = Counted::new("swatches.converted", "{n} color was converted to RGB.", "{n} colors were converted to RGB.");
const NOTE_KINDS_DROPPED: Counted = Counted::new(
    "swatches.kindsDropped",
    "{n} spot or global color was written as a process color.",
    "{n} spot or global colors were written as process colors.",
);
const NOTE_TRUNCATED: Counted = Counted::new(
    "swatches.truncated",
    "{n} color past the 256 the format holds was left out.",
    "{n} colors past the 256 the format holds were left out.",
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

/// The panel's selection, for the menu commands.
fn selection_id(salt: IdSalt) -> Id {
    Id::unique(("craft-color-swatches-selection", salt))
}

/// Where [`SwatchesPanel::run_command`] leaves the row it made, for the panel to select and rename.
fn pending_id(salt: IdSalt) -> Id {
    Id::unique(("craft-color-swatches-pending", salt))
}

/// The file pick a menu command started, until the panel has the answer.
fn file_id(salt: IdSalt) -> Id {
    Id::unique(("craft-color-swatches-file", salt))
}

/// What a swatch file pick is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileJob {
    Import { replace: bool },
    Export(SwatchFormat),
}

/// A file pick in flight.
#[derive(Clone, Copy)]
struct FilePick {
    ticket: PickId,
    job: FileJob,
}

/// The purpose of the swatch file pickers: their folder is remembered under it.
const PICK_PURPOSE: &str = "swatches";

/// The app's swatches: groups of named swatches, names unique across the library. Kept by the
/// app (a settings section or its presets store).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SwatchLibrary {
    /// The groups, in display order.
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
    /// Index of the group.
    pub group: usize,
    /// Index of the swatch within its group.
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

    /// The swatch at `at`, or `None` when out of range.
    pub fn get(&self, at: SwatchAt) -> Option<&SwatchEntry> {
        self.groups.get(at.group)?.entries.get(at.index)
    }

    /// Adds `color` to group `group` (the first group when `None`, a new "Swatches" group when
    /// the library is empty) under a free name ("Color Swatch N" when `name` is `None` or taken);
    /// where it went, or `None` when the library holds [`MAX_SWATCHES`].
    // pc: crates/engine/src/cmd/swatch_cmds.rs:150-171 (`swatches.add`: the cap, the name, group 0).
    pub fn add(&mut self, color: SwatchColor, name: Option<&str>, group: Option<usize>) -> Option<SwatchAt> {
        self.add_numbered(color, name, group, &SwatchesLabels::english().swatch_n)
    }

    /// [`add`](SwatchLibrary::add) with the template (`{n}`) that names a swatch left without one.
    fn add_numbered(&mut self, color: SwatchColor, name: Option<&str>, group: Option<usize>, template: &str) -> Option<SwatchAt> {
        if self.groups.iter().map(|g| g.entries.len()).sum::<usize>() >= MAX_SWATCHES {
            return None;
        }
        let name = name.map(str::trim).filter(|n| !n.is_empty() && self.find(n).is_none()).map_or_else(|| self.free_name(template), str::to_owned);
        if self.groups.is_empty() {
            self.groups.push(SwatchGroup { name: "Swatches".to_owned(), entries: Vec::new() });
        }
        let group = group.unwrap_or(0).min(self.groups.len() - 1);
        let entries = &mut self.groups[group].entries;
        entries.push(SwatchEntry::new(name, color));
        Some(SwatchAt { group, index: entries.len() - 1 })
    }

    /// The first free name of `template`, numbered from 1.
    fn free_name(&self, template: &str) -> String {
        let taken: HashSet<&str> = self.groups.iter().flat_map(|g| g.entries.iter().map(|e| e.name.as_str())).collect();
        (1..).map(|n| craft_i18n::fmt_plain(template, &[("n", &n.to_string())])).find(|name| !taken.contains(name.as_str())).unwrap_or_default()
    }

    /// Add to Swatches from the picker: in the model the user typed (CMYK inks or Lab values the
    /// picker kept), else RGB; `None` when the library is full.
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:260 `add_from_picker`.
    pub fn add_picked(&mut self, picked: &PickedColor) -> Option<SwatchAt> {
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

    /// Removes and returns the swatch at `at`, or `None` when out of range.
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

    /// Adds an empty group named `name` (made unique); its index, or `None` when the library
    /// holds [`MAX_GROUPS`].
    pub fn new_group(&mut self, name: &str) -> Option<usize> {
        if self.groups.len() >= MAX_GROUPS {
            return None;
        }
        let name = self.unique_group_name(name.trim());
        self.groups.push(SwatchGroup { name, entries: Vec::new() });
        Some(self.groups.len() - 1)
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

/// What [`SwatchesPanel::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
#[must_use = "the host applies the swatch"]
pub struct SwatchesPanelResponse {
    /// A swatch was clicked: its colour (through the document's converter) for the foreground, or
    /// for the background when Alt (Ctrl or Cmd too, as PhotoCraft) was held.
    pub used: Option<(SwatchAt, Srgba, bool)>,
    /// The library changed (new, rename, delete, move, group edits, an import): the app saves it.
    pub changed: bool,
    /// The notes of the swatch file Import or Replace Swatches… read, once the panel has imported
    /// it (empty notes when nothing was lost). The swatches are in the library.
    pub imported: Option<SwatchReport>,
    /// The notes of the swatch file Export Swatches… wrote, once it is written.
    pub exported: Option<SwatchReport>,
    /// The message for the status bar: the notes of an import or export that lost something, or
    /// why a file could not be read or written.
    pub status: Option<StatusLine>,
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
    cmyk: Option<&'a CmykConverter>,
    foreground: Srgba,
}

impl<'a> SwatchesPanel<'a> {
    /// The panel over `library`; `foreground` is what New Swatch adds.
    pub fn new(id_salt: impl AsIdSalt, library: &'a mut SwatchLibrary, foreground: Srgba) -> SwatchesPanel<'a> {
        SwatchesPanel { id_salt: IdSalt::new(id_salt), library, cmyk: None, foreground }
    }

    /// The document's CMYK converter, which draws CMYK swatches and gives their colours when used;
    /// without it they use the built-in coated CMYK.
    pub fn cmyk(mut self, converter: &'a CmykConverter) -> SwatchesPanel<'a> {
        self.cmyk = Some(converter);
        self
    }

    // pc: crates/ui-egui/src/panels/swatches_ui.rs:63 `panel` (the colour table cached per library
    // at :33 `colors`, `browser` with `flush: true`), :86 `handle` (Select → use, Activate → rename, Delete,
    // Move, group events), :121 `new_swatch`, :53 `use_swatch`.
    /// Draws the panel and returns the user's actions this pass.
    pub fn show(self, ui: &mut Ui) -> SwatchesPanelResponse {
        let SwatchesPanel { id_salt, library, cmyk, foreground } = self;
        let cmyk = cmyk.unwrap_or_else(|| CmykConverter::coated());
        let id = ui.make_persistent_id(id_salt);
        let labels = SwatchesLabels::of(ui);
        let mut response = SwatchesPanelResponse::default();
        if let Some(host) = Host::try_of(ui) {
            finish_file_pick(ui.ctx(), &host, file_id(id_salt), library, &mut response);
        }
        let mut browser_labels = PresetLabels::of(ui);
        browser_labels.new = labels.create_new.clone();
        let mut st: PanelState = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
        if let Some(key) = ui.data_mut(|d| d.remove_temp::<Option<Key>>(pending_id(id_salt))).flatten() {
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
        let footer = PresetFooter::new("swatches-footer", &mut st.browser)
            .labels(&browser_labels)
            .thumb_range(THUMB_RANGE)
            .can_delete(st.selected.is_some())
            .show(ui);

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
            d.insert_temp(selection_id(id_salt), selected);
            d.insert_temp(id, st);
        });
        response
    }

    /// The panel menu's rows, for the docking host's `panel_menu_entries` hook: New Swatch…, New
    /// Swatch Group…, a separator, Import Swatches…, Replace Swatches…, Export Swatches… and
    /// Export Swatches for Exchange… (both disabled when the library has no swatch), a separator,
    /// Reset Swatches. The commands come back as `DockEvent::PanelCommand`; the host passes them
    /// to [`run_command`](SwatchesPanel::run_command). `id_salt` is the panel's, as the other
    /// panels' menus take it; these rows depend on `library` alone.
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:208 `panel_menu`.
    pub fn panel_menu(ctx: &Context, _id_salt: impl AsIdSalt, library: &SwatchLibrary) -> Vec<MenuEntry<PanelCommand>> {
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

    /// Carries out a command of [`panel_menu`](SwatchesPanel::panel_menu) for the panel `id_salt`.
    /// New Swatch adds the foreground to the selected swatch's (or group's) group, as the footer's
    /// New does, and the panel selects it and opens its name for editing in its next pass; New
    /// Swatch Group, and Reset Swatches act on `library`. Import, Replace and the two exports ask
    /// the installed [`Host`]'s file service for a file (purpose `"swatches"`; without a host they
    /// do nothing); the panel reads or writes the file in the pass the answer arrives, and the
    /// result is in [`SwatchesPanelResponse`].
    pub fn run_command(
        ctx: &Context,
        id_salt: impl AsIdSalt,
        library: &mut SwatchLibrary,
        command: &PanelCommand,
        foreground: Srgba,
    ) -> SwatchesMenuResponse {
        let salt = IdSalt::new(id_salt);
        let labels = SwatchesLabels::of_ctx(ctx);
        let mut response = SwatchesMenuResponse::default();
        let pending = match command.as_str() {
            CMD_NEW_SWATCH => {
                let selected = ctx.data(|d| d.get_temp::<Option<Key>>(selection_id(salt))).flatten();
                let group = selected.as_ref().and_then(|key| library.group_of(key));
                let color = SwatchColor::Rgb(foreground.to_rgb_array().map(|v| v.clamp(0.0, 1.0)));
                let at = library.add_numbered(color, None, group, &labels.swatch_n);
                response.changed = at.is_some();
                at.and_then(|at| library.get(at)).map(|e| Key::Swatch(e.name.clone()))
            }
            CMD_NEW_GROUP => {
                let at = library.new_group(&labels.group);
                response.changed = at.is_some();
                at.map(|at| Key::Group(library.groups[at].name.clone()))
            }
            CMD_RESET => {
                library.groups = crate::palette::default_swatches();
                response.changed = true;
                None
            }
            CMD_IMPORT => {
                start_file_pick(ctx, salt, FileJob::Import { replace: false }, &labels.import);
                None
            }
            CMD_REPLACE => {
                start_file_pick(ctx, salt, FileJob::Import { replace: true }, &labels.replace);
                None
            }
            CMD_EXPORT => {
                start_file_pick(ctx, salt, FileJob::Export(SwatchFormat::Aco), &labels.export);
                None
            }
            CMD_EXPORT_EXCHANGE => {
                start_file_pick(ctx, salt, FileJob::Export(SwatchFormat::Ase), &labels.export_exchange);
                None
            }
            _ => None,
        };
        if let Some(key) = pending {
            ctx.data_mut(|d| d.insert_temp(pending_id(salt), Some(key)));
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

    /// The status-bar message for the notes of an import (or, with `writing`, an export): the
    /// title and the notes in one line, as a warning; `None` when there is nothing to say.
    pub fn status_line(ctx: &Context, report: &SwatchReport, writing: bool) -> Option<StatusLine> {
        let lines = Self::note_lines(ctx, report, writing);
        if lines.is_empty() {
            return None;
        }
        let labels = SwatchesLabels::of_ctx(ctx);
        let title = if writing { labels.notice_export } else { labels.notice_import };
        Some(StatusLine { text: format!("{title}: {}", lines.join(" ")), tone: Tone::Warn })
    }
}

/// Asks the host's file service for the file of `job`; a pick still open is dropped. Without a
/// host nothing is asked.
fn start_file_pick(ctx: &Context, salt: IdSalt, job: FileJob, label: &str) {
    let Some(host) = Host::try_of_ctx(ctx) else { return };
    let key = file_id(salt);
    if let Some(old) = ctx.data(|d| d.get_temp::<FilePick>(key)) {
        host.files().forget(old.ticket);
    }
    let title = label.trim_end_matches('…');
    let request = match job {
        FileJob::Import { .. } => {
            let readable: Vec<&str> = SwatchFormat::ALL.iter().filter(|f| f.can_read()).map(|f| f.extension()).collect();
            PickRequest::open(PICK_PURPOSE).title(title).filter("Swatches", &readable)
        }
        FileJob::Export(format) => {
            PickRequest::save(PICK_PURPOSE).title(title).suggested_extension(format.extension()).filter("Swatches", &[format.extension()])
        }
    };
    let ticket = host.files().pick(request);
    ctx.data_mut(|d| d.insert_temp(key, FilePick { ticket, job }));
}

/// Carries out the file pick of a menu command once it is answered: imports the swatch file into
/// `library`, or writes the library to the chosen path, and reports the result in `response`.
fn finish_file_pick(ctx: &Context, host: &Host, key: Id, library: &mut SwatchLibrary, response: &mut SwatchesPanelResponse) {
    let Some(pick) = ctx.data(|d| d.get_temp::<FilePick>(key)) else { return };
    let Some(picked) = host.files().take_answer(pick.ticket) else { return };
    ctx.data_mut(|d| d.remove::<FilePick>(key));
    let labels = SwatchesLabels::of_ctx(ctx);
    let failed = |template: &str, reason: &dyn std::fmt::Display| StatusLine {
        text: craft_i18n::fmt(template, &[("reason", &reason.to_string())]),
        tone: Tone::Bad,
    };
    match pick.job {
        FileJob::Import { replace } => {
            let file = match picked {
                Picked::Cancelled => return,
                Picked::Paths(paths) => paths.into_iter().next().map(|path| {
                    let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    host.files().read(&path).map(|bytes| (name, bytes))
                }),
                Picked::Contents(files) => files.into_iter().next().map(|(name, bytes)| Ok((name, bytes))),
            };
            let result = match file {
                None => return,
                Some(Err(error)) => Err(failed(&labels.import_failed, &error)),
                Some(Ok((name, bytes))) => crate::palette::read_swatches(&bytes, &name).map_err(|error| failed(&labels.import_failed, &error)),
            };
            match result {
                Ok((groups, report)) => {
                    library.import(groups, replace);
                    response.changed = true;
                    response.status = SwatchesPanel::status_line(ctx, &report, false);
                    response.imported = Some(report);
                }
                Err(status) => response.status = Some(status),
            }
        }
        FileJob::Export(format) => {
            let Some(path) = picked.first_path() else { return };
            let written = crate::palette::write_swatches(&library.groups, format).map_err(|error| failed(&labels.export_failed, &error)).and_then(
                |(bytes, report)| host.files().write(&path, &bytes).map(|()| report).map_err(|error| failed(&labels.export_failed, &error)),
            );
            match written {
                Ok(report) => {
                    response.status = SwatchesPanel::status_line(ctx, &report, true);
                    response.exported = Some(report);
                }
                Err(status) => response.status = Some(status),
            }
        }
    }
}

/// What [`SwatchesPanel::run_command`] did.
#[derive(Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app saves a changed library"]
pub struct SwatchesMenuResponse {
    /// The library changed: the app saves it.
    pub changed: bool,
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

    fn footer(&mut self, event: PresetFooterEvent<Key>) {
        match event {
            PresetFooterEvent::New => self.new_swatch(),
            PresetFooterEvent::NewGroup => {
                if let Some(at) = self.library.new_group(self.group_name) {
                    self.st.rename = Some(Key::Group(self.library.groups[at].name.clone()));
                    self.response.changed = true;
                }
            }
            PresetFooterEvent::Delete => {
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
        let Some(at) = self.library.add_numbered(color, None, group, self.template) else {
            return;
        };
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
