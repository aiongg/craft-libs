//! `SwatchesPanel`: the Swatches panel on a [`SwatchLibrary`]: groups of flat touching squares, click to use, the panel
//! menu for new swatches, groups and swatch files.

use std::collections::{HashMap, HashSet};

use craft_behavior::EditEvent;
use craft_cms::convert::CmykConverter;
use craft_i18n::Format;
use craft_theme::Srgba;
use craft_theme::variant::Tone;
use craft_ui::contract::{Bind, CommandSet, Held, NoValue, Outcome, Pattern, ReorderMove, ShowContainer as _, request_rename};
use craft_ui::layout_components::Stack;
use craft_ui::menu::PanelRow;
use craft_ui::patterns::presets::{
    PresetBrowser, PresetCommands, PresetEvent, PresetExport, PresetFileEvent, PresetFileRequest, PresetFiles, PresetFooterEvent,
    PresetFooterOptions, PresetReorder, PresetRow, PresetState, PresetWords, ThumbInfo,
};
use craft_ui::primitives::ContentPainterCx;
use craft_ui::runtime::commands::{CommandRef, CommandState};
use craft_ui::runtime::env::{Count, Env};
use craft_ui::runtime::toast::Toast;
use egui::{Color32, CornerRadius, Rect, Response, Ui};

use super::PickedColor;
use crate::palette::{SwatchColor, SwatchEntry, SwatchFormat, SwatchGroup, SwatchNote, SwatchReport};

#[cfg(feature = "screen-testing")]
mod stories;

/// The most swatches a library holds (PhotoCraft's `MAX_SWATCHES`).
pub const MAX_SWATCHES: usize = 100_000;

/// The most groups a library holds (PhotoCraft's `MAX_GROUPS`).
pub const MAX_GROUPS: usize = 10_000;

/// The purpose of the swatch file pickers: their folder is remembered under it.
const PICK_PURPOSE: &str = "swatches";

craft_ui::words! {
    /// The Swatches panel's words.
    #[non_exhaustive]
    pub struct SwatchesWords {
        title: "swatches.title" = "Swatches";
        /// The name of a new group.
        group: "swatches.group" = "Group";
        /// A new swatch's name; `{n}` numbers it.
        swatch_n: template "swatches.swatchN" = "Color Swatch {n}";
        /// The name of the file filter of the swatch pickers, and of an exported file.
        file: "swatches.file" = "Swatches";
        /// The start of the toast after an import that changed something; the notes follow.
        notice_import: "swatches.noticeImport" = "Swatches imported with changes";
        /// The start of the toast after an export that changed something.
        notice_export: "swatches.noticeExport" = "Swatches exported with changes";
        /// The toast when a swatch file could not be imported; `{reason}` says why.
        import_failed: template "swatches.importFailed" = "Swatches could not be imported: {reason}";
        /// The toast when swatches could not be exported; `{reason}` says why.
        export_failed: template "swatches.exportFailed" = "Swatches could not be exported: {reason}";
        groups_flattened: "swatches.groupsFlattened" = "The format has no groups; the swatches were written as one list.";
        book_read: count "swatches.bookRead" = "{n} color-book color was read as black." | "{n} color-book colors were read as black.";
        book_left_out: count "swatches.bookLeftOut" = "{n} color-book color was left out." | "{n} color-book colors were left out.";
        skipped: count "swatches.skipped" = "{n} color in a model that cannot be read was skipped." | "{n} colors in a model that cannot be read were skipped.";
        converted: count "swatches.converted" = "{n} color was converted to RGB." | "{n} colors were converted to RGB.";
        kinds_dropped: count "swatches.kindsDropped" = "{n} spot or global color was written as a process color." | "{n} spot or global colors were written as process colors.";
        truncated: count "swatches.truncated" = "{n} color past the 256 the format holds was left out." | "{n} colors past the 256 the format holds were left out.";
        names_cut: count "swatches.namesCut" = "{n} name was cut to the length the format holds." | "{n} names were cut to the length the format holds.";
        /// The value read for a colour-book colour, which the panel cannot show.
        book_color: "swatches.bookColor" = "Color book color";
    }
}

/// What the browser names its rows by. Swatch names are unique across the library and group names among groups, so
/// both survive reordering.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SwatchKey {
    /// A group, by name.
    Group(String),
    /// A swatch, by name.
    Swatch(String),
}

/// What the Swatches panel keeps between passes, owned by the host: the browser's view, the selected row, the row to
/// rename in place next, and the notes of an export in flight.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwatchesState {
    /// The browser's view settings: thumbnail size, open groups.
    pub browser: PresetState<SwatchKey>,
    /// The selected row.
    pub selected: Option<SwatchKey>,
    /// A row to open the rename on in the next pass.
    pub rename: Option<SwatchKey>,
    /// The notes of the export whose file the host is writing.
    export_notes: Option<SwatchReport>,
}

impl Default for SwatchesState {
    fn default() -> Self {
        SwatchesState { browser: PresetState::new(), selected: None, rename: None, export_notes: None }
    }
}

/// The app's swatches: groups of named swatches, names unique across the library. Kept by the
/// app (a settings section or its presets store).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct SwatchAt {
    /// Index of the group.
    pub group: usize,
    /// Index of the swatch within its group.
    pub index: usize,
}

impl SwatchAt {
    /// The swatch at `index` of group `group`.
    pub const fn new(group: usize, index: usize) -> Self {
        SwatchAt { group, index }
    }
}

impl SwatchLibrary {
    /// A library of `groups`.
    pub fn new(groups: Vec<SwatchGroup>) -> Self {
        SwatchLibrary { groups }
    }

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
        self.add_numbered(color, name, group, SwatchesWords::default().swatch_n.as_str())
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

    /// Applies a reorder of the browser: swatches go into a group at a place, or groups to a place among the
    /// groups. `to` counts in the flat rows (a header, then its swatches) that remain once the moved ones are
    /// taken out. Anything else leaves the library as it is.
    pub fn apply_reorder(&mut self, mv: &ReorderMove<SwatchKey>) -> bool {
        if mv.copy {
            return false;
        }
        let swatches: Option<Vec<&str>> = mv.keys.iter().map(|k| if let SwatchKey::Swatch(n) = k { Some(n.as_str()) } else { None }).collect();
        let groups: Option<Vec<&str>> = mv.keys.iter().map(|k| if let SwatchKey::Group(n) = k { Some(n.as_str()) } else { None }).collect();
        if let Some(names) = swatches {
            return self.move_swatches(&names, mv.to, mv.parent.as_ref());
        }
        groups.is_some_and(|names| self.move_groups(&names, mv.to))
    }

    /// The flat rows of the browser: each group's header, then its swatches.
    fn flat(&self) -> Vec<SwatchKey> {
        let mut rows = Vec::new();
        for group in &self.groups {
            rows.push(SwatchKey::Group(group.name.clone()));
            rows.extend(group.entries.iter().map(|e| SwatchKey::Swatch(e.name.clone())));
        }
        rows
    }

    fn move_swatches(&mut self, names: &[&str], to: usize, parent: Option<&SwatchKey>) -> bool {
        let moved = |e: &SwatchEntry| names.contains(&e.name.as_str());
        let remaining: Vec<SwatchKey> =
            self.flat().into_iter().filter(|k| !matches!(k, SwatchKey::Swatch(n) if names.contains(&n.as_str()))).collect();
        let before = &remaining[..to.min(remaining.len())];
        // The slot is counted in the entries that stay, so it survives taking the moved ones out.
        let (group, slot) = match before.last() {
            Some(SwatchKey::Swatch(n)) => {
                let Some(at) = self.find(n) else { return false };
                let entries = &self.groups[at.group].entries;
                let slot = entries[..=at.index].iter().filter(|e| !moved(e)).count();
                match parent {
                    Some(SwatchKey::Group(g)) if self.groups[at.group].name != *g => match self.groups.iter().position(|x| x.name == *g) {
                        Some(group) => (group, 0),
                        None => return false,
                    },
                    _ => (at.group, slot),
                }
            }
            Some(SwatchKey::Group(g)) => match self.groups.iter().position(|x| x.name == *g) {
                Some(group) => (group, 0),
                None => return false,
            },
            None => (0, 0),
        };
        if group >= self.groups.len() {
            return false;
        }
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

    fn move_groups(&mut self, names: &[&str], to: usize) -> bool {
        let moved = |g: &SwatchGroup| names.contains(&g.name.as_str());
        let remaining: Vec<SwatchKey> = {
            let mut rows = Vec::new();
            for group in self.groups.iter().filter(|g| !moved(g)) {
                rows.push(SwatchKey::Group(group.name.clone()));
                rows.extend(group.entries.iter().map(|e| SwatchKey::Swatch(e.name.clone())));
            }
            rows
        };
        let slot = remaining[..to.min(remaining.len())].iter().filter(|k| matches!(k, SwatchKey::Group(_))).count();
        let (taken, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.groups).into_iter().partition(|g| moved(g));
        self.groups = keep;
        if taken.is_empty() {
            return false;
        }
        let slot = slot.min(self.groups.len());
        self.groups.splice(slot..slot, taken);
        true
    }
}

/// What [`SwatchesPanel::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the swatch"]
pub struct SwatchesPanelOutput {
    /// The panel's response.
    pub response: Response,
    /// A swatch was clicked: its colour (through the document's converter) for the foreground, or for the background
    /// when Alt (Ctrl or Cmd too, as PhotoCraft) was held.
    pub used: Option<(SwatchAt, Srgba, bool)>,
    /// The library changed (new, rename, delete, move, group edits): the app saves it.
    pub changed: bool,
}

impl Outcome for SwatchesPanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The Swatches panel on a [`SwatchLibrary`], built on `PresetBrowser`: groups as folders, swatches as flat touching
/// squares with no gap or corner radius (Photoshop), the size the browser's zoom chooses.
///
/// - Click uses the swatch for the foreground; Alt-click (or Ctrl or Cmd) for the background. The swatch is selected.
///   Double-click or F2 renames in place; Delete removes; dragging onto a group moves it. Swatches do not drop onto
///   the canvas.
/// - The footer's + is New Swatch: the foreground colour into the selected swatch's group, named "Color Swatch N", then
///   renamed in place.
/// - Tooltips name the swatch and its value; AccessKit as the preset browser's grid, each swatch named.
/// - The files of Import, Replace and Export go out as [`PresetFileRequest`]s from the panel menu; the host answers
///   them with [`SwatchesPanel::apply_file`].
#[must_use = "show the panel with `show`"]
#[derive(Pattern)]
pub struct SwatchesPanel<'a> {
    library: Bind<'a, SwatchLibrary>,
    state: Bind<'a, SwatchesState>,
    cmyk: Option<&'a CmykConverter>,
    foreground: Srgba,
}

impl<'a> SwatchesPanel<'a> {
    /// The panel over `library` and the host-owned `state`; `foreground` is what New Swatch adds.
    pub fn new(library: Bind<'a, SwatchLibrary>, state: Bind<'a, SwatchesState>, foreground: Srgba) -> Self {
        SwatchesPanel { library, state, cmyk: None, foreground }
    }

    /// The document's CMYK converter, which draws CMYK swatches and gives their colours when used; without it they
    /// use the built-in coated CMYK.
    #[must_use]
    pub fn cmyk(mut self, converter: &'a CmykConverter) -> Self {
        self.cmyk = Some(converter);
        self
    }

    /// Draws the panel and returns the user's actions this pass.
    pub fn show(self, ui: &mut Ui) -> SwatchesPanelOutput {
        let SwatchesPanel { library, state, cmyk, foreground } = self;
        let cmyk = cmyk.unwrap_or_else(|| CmykConverter::coated());
        let words = Env::of(ui).words::<SwatchesWords>();
        let mut held_library = Held::new(library);
        let mut held_state = Held::new(state);
        let (mut used, mut changed) = (None, false);
        let shown = Stack::new().show(ui, |ui, cx| {
            let (Some(before), Some(before_state)) = (held_library.read(cx), held_state.read(cx)) else { return };
            let (mut lib, mut st) = (before.clone(), before_state.clone());
            if st.selected.as_ref().is_some_and(|k| !lib.has(k)) {
                st.selected = None;
            }
            if let Some(key) = st.rename.take() {
                request_rename(ui.ctx(), ui.scope_id().with("browser-rows"), &key);
            }
            let mut rows = Vec::with_capacity(lib.total() + lib.groups.len());
            for group in &lib.groups {
                rows.push(PresetRow::group(SwatchKey::Group(group.name.clone()), group.name.as_str(), 0).count(group.entries.len()));
                for entry in &group.entries {
                    let named = format!("{}, {}", entry.name, value_text(&entry.color, words.book_color.as_str()));
                    rows.push(
                        PresetRow::item(SwatchKey::Swatch(entry.name.clone()), entry.name.as_str(), 1).tooltip(named.clone()).accessible_name(named),
                    );
                }
            }
            let out = {
                let colors: HashMap<&str, &SwatchColor> =
                    lib.groups.iter().flat_map(|g| g.entries.iter().map(|e| (e.name.as_str(), &e.color))).collect();
                let mut browser = PresetBrowser::new(words.title.clone(), Bind::bound(&mut st.browser), &rows)
                    .toolbar(true)
                    .reorder(PresetReorder::items_and_groups())
                    .footer(PresetFooterOptions::new().can_delete(st.selected.is_some()))
                    .thumb(|cx: &ContentPainterCx<'_>, rect: Rect, info: ThumbInfo<'_, SwatchKey>| {
                        if let SwatchKey::Swatch(name) = info.key
                            && let Some(color) = colors.get(name.as_str())
                        {
                            cx.painter.rect_filled(rect, CornerRadius::ZERO, Color32::from(color.to_srgb(cmyk)));
                        }
                    });
                if let Some(key) = &st.selected {
                    browser = browser.selected(key.clone());
                }
                browser.show(ui)
            };
            let mut actions = Actions {
                library: &mut lib,
                st: &mut st,
                used: &mut used,
                changed: &mut changed,
                foreground,
                template: words.swatch_n.as_str(),
                group_name: words.group.as_str(),
                cmyk,
            };
            for event in out.events {
                actions.event(event);
            }
            if changed {
                held_library.write(cx, &before, lib, words.title.as_str());
            }
            if st != before_state {
                held_state.write(cx, &before_state, st, words.title.as_str());
            }
        });
        SwatchesPanelOutput { response: shown.output, used, changed }
    }

    /// Carries out the host's answer to a file request of the panel menu: Import and Replace read the file into
    /// `library`, a finished export reports the notes it kept, and a failure says why. Returns what changed and the
    /// toast the host posts, if any.
    pub fn apply_file(
        library: &mut SwatchLibrary,
        state: &mut SwatchesState,
        event: &PresetFileEvent,
        words: &SwatchesWords,
        presets: &PresetWords,
        format: &Format,
    ) -> SwatchesFileOutput {
        let mut output = SwatchesFileOutput { changed: false, report: None, toast: None };
        match event {
            PresetFileEvent::Loaded { file_name, bytes, replace } => match crate::palette::read_swatches(bytes, file_name) {
                Ok((groups, report)) => {
                    library.import(groups, *replace);
                    output.changed = true;
                    output.toast = Self::notes_toast(&report, false, words, format);
                    output.report = Some(report);
                }
                Err(error) => {
                    output.toast = Some(Toast::new(words.import_failed.fill(&[("reason", &error.to_string())])).tone(Tone::Bad));
                }
            },
            PresetFileEvent::Saved { .. } => {
                if let Some(report) = state.export_notes.take() {
                    output.toast = Self::notes_toast(&report, true, words, format);
                    output.report = Some(report);
                }
            }
            PresetFileEvent::Failed { .. } => {
                state.export_notes = None;
                output.toast = Some(Toast::new(event.status(presets)).tone(Tone::Bad));
            }
            _ => {}
        }
        output
    }

    /// The notes of a swatch file read or written as sentences; counts follow the locale's plural rules. `writing`
    /// tells an export's notes from an import's, which word a colour-book colour differently.
    pub fn note_lines(report: &SwatchReport, writing: bool, words: &SwatchesWords, format: &Format) -> Vec<String> {
        let count = |row: &Count, n: usize| row.text(n as u64, &format.integer(n as u64));
        report
            .notes
            .iter()
            .map(|note| match *note {
                SwatchNote::BookColors(n) if writing => count(&words.book_left_out, n),
                SwatchNote::BookColors(n) => count(&words.book_read, n),
                SwatchNote::Skipped(n) => count(&words.skipped, n),
                SwatchNote::GroupsFlattened => words.groups_flattened.as_str().to_owned(),
                SwatchNote::ConvertedToRgb(n) => count(&words.converted, n),
                SwatchNote::KindsDropped(n) => count(&words.kinds_dropped, n),
                SwatchNote::Truncated(n) => count(&words.truncated, n),
                SwatchNote::NamesCut(n) => count(&words.names_cut, n),
            })
            .collect()
    }

    /// The toast for the notes of an import (or, with `writing`, an export): the title and the notes in one line, as
    /// a warning; `None` when there is nothing to say.
    fn notes_toast(report: &SwatchReport, writing: bool, words: &SwatchesWords, format: &Format) -> Option<Toast> {
        let lines = Self::note_lines(report, writing, words, format);
        if lines.is_empty() {
            return None;
        }
        let title = if writing { &words.notice_export } else { &words.notice_import };
        Some(Toast::new(format!("{title}: {}", lines.join(" "))).tone(Tone::Warn))
    }
}

/// What [`SwatchesPanel::apply_file`] did.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host saves a changed library and posts the toast"]
pub struct SwatchesFileOutput {
    /// The library changed: the app saves it.
    pub changed: bool,
    /// The notes of the swatch file that was read or written, once the panel has handled it.
    pub report: Option<SwatchReport>,
    /// The toast for the notes that lost something or for a failure.
    pub toast: Option<Toast>,
}

/// The commands of the Swatches panel menu, in Photoshop's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CommandSet)]
#[command(prefix = "swatches")]
#[non_exhaustive]
pub enum SwatchesCommand {
    /// Add the foreground colour as a swatch.
    #[command(label = "New Swatch…")]
    NewSwatch,
    /// Add an empty group.
    #[command(label = "New Swatch Group…")]
    NewGroup,
    /// Add the swatches of a file.
    #[command(label = "Import Swatches…")]
    Import,
    /// Replace the swatches with those of a file.
    #[command(label = "Replace Swatches…")]
    Replace,
    /// Write the swatches to a Photoshop swatch file.
    #[command(label = "Export Swatches…")]
    Export,
    /// Write the swatches to an exchange file with their groups.
    #[command(label = "Export Swatches for Exchange…")]
    ExportExchange,
    /// Put the default swatches back.
    #[command(label = "Reset Swatches")]
    Reset,
}

impl SwatchesCommand {
    /// Every command, in menu order.
    pub const ALL: [SwatchesCommand; 7] = [
        SwatchesCommand::NewSwatch,
        SwatchesCommand::NewGroup,
        SwatchesCommand::Import,
        SwatchesCommand::Replace,
        SwatchesCommand::Export,
        SwatchesCommand::ExportExchange,
        SwatchesCommand::Reset,
    ];
}

/// What [`SwatchesMenu::run_command`] did.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app saves a changed library and answers the file request"]
pub struct SwatchesMenuOutput {
    /// The library changed: the app saves it.
    pub changed: bool,
    /// The file dialog the command asks the host for; the host answers with [`SwatchesPanel::apply_file`].
    pub file: Option<PresetFileRequest>,
    /// A toast for a command that failed before it asked for a file.
    pub toast: Option<Toast>,
}

/// The Swatches panel menu: the rows for the dock's panel menu, the states the host publishes, and the carrying out
/// of a fired command.
pub struct SwatchesMenu;

impl SwatchesMenu {
    /// The rows: New Swatch…, New Swatch Group…, a separator, Import, Replace, Export and Export for Exchange, a
    /// separator, Reset Swatches.
    pub fn rows() -> Vec<PanelRow> {
        let row = |command: SwatchesCommand| PanelRow::Command(CommandRef::from(command));
        vec![
            row(SwatchesCommand::NewSwatch),
            row(SwatchesCommand::NewGroup),
            PanelRow::Separator,
            row(SwatchesCommand::Import),
            row(SwatchesCommand::Replace),
            row(SwatchesCommand::Export),
            row(SwatchesCommand::ExportExchange),
            PanelRow::Separator,
            row(SwatchesCommand::Reset),
        ]
    }

    /// The state of each command: both exports are disabled while the library has no swatch.
    pub fn states(library: &SwatchLibrary) -> Vec<(SwatchesCommand, CommandState)> {
        let exports = library.total() > 0;
        SwatchesCommand::ALL
            .into_iter()
            .map(|command| {
                let enabled = !matches!(command, SwatchesCommand::Export | SwatchesCommand::ExportExchange) || exports;
                (command, CommandState::new().enabled(enabled))
            })
            .collect()
    }

    /// Carries out `command`. New Swatch adds the foreground to the selected swatch's (or group's) group, as the
    /// footer's New does, and the panel selects it and opens its name for editing in its next pass; New Swatch Group
    /// and Reset Swatches act on `library`. Import, Replace and the two exports give the file request for the host.
    pub fn run_command(
        library: &mut SwatchLibrary,
        state: &mut SwatchesState,
        command: SwatchesCommand,
        foreground: Srgba,
        words: &SwatchesWords,
    ) -> SwatchesMenuOutput {
        let mut output = SwatchesMenuOutput { changed: false, file: None, toast: None };
        let readable: Vec<&str> = SwatchFormat::ALL.iter().filter(|f| f.can_read()).map(|f| f.extension()).collect();
        let import = || PresetFiles::<SwatchKey>::new(PICK_PURPOSE, words.file.as_str(), &readable).replace(true);
        match command {
            SwatchesCommand::NewSwatch => {
                let group = state.selected.as_ref().and_then(|key| library.group_of(key));
                let color = SwatchColor::Rgb(foreground.to_rgb_array().map(|v| v.clamp(0.0, 1.0)));
                let at = library.add_numbered(color, None, group, words.swatch_n.as_str());
                output.changed = at.is_some();
                if let Some(entry) = at.and_then(|at| library.get(at)) {
                    let key = SwatchKey::Swatch(entry.name.clone());
                    state.selected = Some(key.clone());
                    state.rename = Some(key);
                }
            }
            SwatchesCommand::NewGroup => {
                let at = library.new_group(words.group.as_str());
                output.changed = at.is_some();
                if let Some(at) = at {
                    state.rename = Some(SwatchKey::Group(library.groups[at].name.clone()));
                }
            }
            SwatchesCommand::Reset => {
                library.groups = crate::palette::default_swatches();
                output.changed = true;
            }
            SwatchesCommand::Import => output.file = import().request(PresetCommands::Load, None),
            SwatchesCommand::Replace => output.file = import().request(PresetCommands::Replace, None),
            SwatchesCommand::Export => Self::export(library, state, SwatchFormat::Aco, words, &mut output),
            SwatchesCommand::ExportExchange => Self::export(library, state, SwatchFormat::Ase, words, &mut output),
        }
        output
    }

    /// The file request that writes the library as `format`; a library the format cannot hold is a toast.
    fn export(library: &SwatchLibrary, state: &mut SwatchesState, format: SwatchFormat, words: &SwatchesWords, output: &mut SwatchesMenuOutput) {
        match crate::palette::write_swatches(&library.groups, format) {
            Ok((bytes, report)) => {
                let file_name = format!("{}.{}", words.file.as_str(), format.extension());
                let files = PresetFiles::<SwatchKey>::new(PICK_PURPOSE, words.file.as_str(), &[format.extension()])
                    .export(move |_| Some(PresetExport::new(file_name.clone(), bytes.clone())));
                output.file = files.request(PresetCommands::Save, None);
                state.export_notes = Some(report);
            }
            Err(error) => output.toast = Some(Toast::new(words.export_failed.fill(&[("reason", &error.to_string())])).tone(Tone::Bad)),
        }
    }
}

/// The events of one pass applied to the library and the panel's state.
struct Actions<'a> {
    library: &'a mut SwatchLibrary,
    st: &'a mut SwatchesState,
    used: &'a mut Option<(SwatchAt, Srgba, bool)>,
    changed: &'a mut bool,
    foreground: Srgba,
    template: &'a str,
    group_name: &'a str,
    cmyk: &'a CmykConverter,
}

impl Actions<'_> {
    fn event(&mut self, event: PresetEvent<SwatchKey>) {
        match event {
            PresetEvent::Select { key: SwatchKey::Swatch(name), mods } => {
                self.st.selected = Some(SwatchKey::Swatch(name.clone()));
                if let Some(at) = self.library.find(&name)
                    && let Some(entry) = self.library.get(at)
                {
                    let background = mods.alt || mods.cmd || mods.ctrl;
                    *self.used = Some((at, entry.color.to_srgb(self.cmyk), background));
                }
            }
            PresetEvent::Select { key, .. } | PresetEvent::Activate { key, .. } => {
                self.st.selected = Some(key).filter(|k| matches!(k, SwatchKey::Swatch(_)))
            }
            PresetEvent::Rename { key: SwatchKey::Swatch(old), text } => {
                if let Some(at) = self.library.find(&old)
                    && self.library.rename(at, &text)
                {
                    *self.changed = true;
                    if self.st.selected == Some(SwatchKey::Swatch(old)) {
                        self.st.selected = Some(SwatchKey::Swatch(text.trim().to_owned()));
                    }
                }
            }
            PresetEvent::Rename { key: SwatchKey::Group(old), text } => {
                if let Some(group) = self.library.groups.iter().position(|g| g.name == old)
                    && self.library.rename_group(group, &text)
                {
                    *self.changed = true;
                }
            }
            PresetEvent::Delete(key) => self.delete(&key),
            PresetEvent::MoveTo { key: SwatchKey::Swatch(name), group: SwatchKey::Group(to) } => {
                if let (Some(at), Some(to)) = (self.library.find(&name), self.library.groups.iter().position(|g| g.name == to)) {
                    self.library.move_to(at, to);
                    *self.changed = true;
                }
            }
            PresetEvent::Reorder(EditEvent::Commit { after, .. } | EditEvent::Set { after, .. }) => {
                *self.changed |= self.library.apply_reorder(&after);
            }
            PresetEvent::Footer(footer) => self.footer(footer),
            _ => {}
        }
    }

    fn footer(&mut self, event: PresetFooterEvent<SwatchKey>) {
        match event {
            PresetFooterEvent::New => self.new_swatch(),
            PresetFooterEvent::NewGroup => {
                if let Some(at) = self.library.new_group(self.group_name) {
                    self.st.rename = Some(SwatchKey::Group(self.library.groups[at].name.clone()));
                    *self.changed = true;
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

    /// The foreground colour into the selected swatch's group, named and selected, then renamed in place.
    fn new_swatch(&mut self) {
        let group = self.st.selected.as_ref().and_then(|key| self.library.group_of(key));
        let color = SwatchColor::Rgb(self.foreground.to_rgb_array().map(|v| v.clamp(0.0, 1.0)));
        let Some(at) = self.library.add_numbered(color, None, group, self.template) else {
            return;
        };
        if let Some(entry) = self.library.get(at) {
            let key = SwatchKey::Swatch(entry.name.clone());
            self.st.selected = Some(key.clone());
            self.st.rename = Some(key);
        }
        *self.changed = true;
    }

    fn delete(&mut self, key: &SwatchKey) {
        let removed = match key {
            SwatchKey::Swatch(name) => self.library.find(name).and_then(|at| self.library.remove(at)).is_some(),
            SwatchKey::Group(name) => match self.library.groups.iter().position(|g| g.name == *name) {
                Some(group) => {
                    self.library.groups.remove(group);
                    true
                }
                None => false,
            },
        };
        if removed {
            *self.changed = true;
        }
    }
}

impl SwatchLibrary {
    /// The index of the group `key` is, or is in.
    fn group_of(&self, key: &SwatchKey) -> Option<usize> {
        match key {
            SwatchKey::Swatch(name) => self.find(name).map(|at| at.group),
            SwatchKey::Group(name) => self.groups.iter().position(|g| g.name == *name),
        }
    }

    /// Whether `key` names a row of the library.
    fn has(&self, key: &SwatchKey) -> bool {
        match key {
            SwatchKey::Swatch(name) => self.find(name).is_some(),
            SwatchKey::Group(name) => self.groups.iter().any(|g| g.name == *name),
        }
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
