//! [`SwatchesPanel`] (F04.13): Photoshop's Swatches panel on a [`SwatchLibrary`]: groups of flat
//! touching squares, click to use, the panel menu for new swatches, groups and swatch files.

use std::collections::HashSet;

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;
use craft_ui::behavior::{DropPosition, Move};
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::presets::{FooterEvent, PresetBrowser, PresetEvent, PresetFooter, PresetLabels, PresetReorder, PresetRow, PresetState};
use egui::{AsIdSalt, Color32, CornerRadius, Id, IdSalt, Ui};

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
        /// The panel's name for a screen reader.
        title: "swatches.title" = "Swatches";
        /// The value read for a colour-book colour, which the panel cannot show.
        book_color: "swatches.bookColor" = "Colour book colour";
    }
}

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

/// Where [`SwatchesPanel::panel_menu`] leaves the row it made, for the panel to select and rename.
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
                rows.push(PresetRow::item(Key::Swatch(entry.name.clone()), entry.name.as_str(), 1).tooltip(format!("{}, {value}", entry.name)));
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
        ui.data_mut(|d| d.insert_temp(id, st));
        response
    }

    /// The panel menu's entries, for the docking host's `panel_menu` hook: New Swatch…, New
    /// Swatch Group…, a separator, Import Swatches…, Replace Swatches…, Export Swatches… and
    /// Export Swatches for Exchange… (both disabled when the library has no swatch), a separator,
    /// Reset Swatches. New Swatch, New Group and Reset act on `library` here; the file entries
    /// come back as a [`SwatchesAction`].
    // pc: crates/ui-egui/src/panels/swatches_ui.rs:208 `panel_menu`.
    pub fn panel_menu(ui: &mut Ui, library: &mut SwatchLibrary, foreground: Srgba) -> Option<SwatchesAction> {
        enum Choice {
            New,
            Group,
            Reset,
            File(SwatchesAction),
        }
        let labels = SwatchesLabels::of(ui);
        let empty = library.total() == 0;
        let mut choice = None;
        let mut entry = |ui: &mut Ui, text: &str, enabled: bool, pick: Choice| {
            if ui.add_enabled(enabled, egui::Button::new(text)).clicked() {
                choice = Some(pick);
            }
        };
        entry(ui, &labels.new_swatch, true, Choice::New);
        entry(ui, &labels.new_group, true, Choice::Group);
        ui.separator();
        entry(ui, &labels.import, true, Choice::File(SwatchesAction::Import { replace: false }));
        entry(ui, &labels.replace, true, Choice::File(SwatchesAction::Import { replace: true }));
        entry(ui, &labels.export, !empty, Choice::File(SwatchesAction::Export(SwatchFormat::Aco)));
        entry(ui, &labels.export_exchange, !empty, Choice::File(SwatchesAction::Export(SwatchFormat::Ase)));
        ui.separator();
        entry(ui, &labels.reset, true, Choice::Reset);
        let choice = choice?;
        ui.close();
        let pending = match choice {
            Choice::New => {
                let at = library.add_numbered(SwatchColor::Rgb(foreground.to_rgb_array().map(|v| v.clamp(0.0, 1.0))), None, None, &labels.swatch_n);
                library.get(at).map(|e| Key::Swatch(e.name.clone()))
            }
            Choice::Group => {
                let at = library.new_group(&labels.group);
                Some(Key::Group(library.groups[at].name.clone()))
            }
            Choice::Reset => {
                library.groups = crate::palette::default_swatches();
                None
            }
            Choice::File(action) => return Some(action),
        };
        if let Some(key) = pending {
            ui.data_mut(|d| d.insert_temp(pending_id(), Some(key)));
        }
        None
    }
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
        let group = match &self.st.selected {
            Some(Key::Swatch(name)) => self.library.find(name).map(|at| at.group),
            Some(Key::Group(name)) => self.library.groups.iter().position(|g| g.name == *name),
            None => None,
        };
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
