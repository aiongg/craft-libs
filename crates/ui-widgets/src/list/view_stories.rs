//! Stories of `List`, `FindingsList`, `FindingsSummary`, `EventList` and `UndoList`.

use super::view_words::{EventWords, FindingsWords, UndoWords};
use super::{
    EventLevel, EventList, EventLog, EventLogState, Finding, Findings, FindingsList, FindingsSummary, List, ListRow, LogEvent, RowInk, Severity,
    ToggleCell, UndoList, UndoMark, UndoRows, footer_text,
};
use crate::contract::{
    Bind, CollectionEvent, ColumnKind, ColumnSpec, Entry, Show as _, ShowCollection as _, ToggleColumn, WordSetCx as _, request_rename,
};
use crate::menu::Menu;
use crate::runtime::env::{Env, Label};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, IconId, ItemSource, Selection};
use craft_keymap::{Chord, Key, Mods};
use egui::Ui;

/// A list of names keyed by position.
struct Names<'a>(&'a [String]);

impl DisplayOrder<usize> for Names<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        self.0.get(at).map(|name| std::borrow::Cow::Borrowed(name.as_str()))
    }
}

impl ItemSource<usize> for Names<'_> {}

fn names(count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("Layer {n}")).collect()
}

fn list(ui: &mut Ui, count: usize) -> StoryOutput {
    let names = names(count);
    List::<usize>::new("Layers").empty("No layers").show_items(ui, &Names(&names), &mut |key, _| ListRow::new(names[*key].clone())).into()
}

const EYE: ColumnKey = ColumnKey::new("eye");
const NAME: ColumnKey = ColumnKey::new("name");

/// The rows of the layers list story: names, eyes, the selection (the first row to begin with) and the line under
/// the list.
#[derive(Clone, Debug, PartialEq)]
struct LayerRows {
    names: Vec<String>,
    visible: Vec<bool>,
    selection: Selection<usize>,
    last: String,
}

impl LayerRows {
    fn new(count: usize) -> Self {
        LayerRows {
            names: names(count),
            visible: vec![true; count],
            selection: Selection::single(0),
            last: String::from("Double-click a row, press Enter or open its menu"),
        }
    }
}

/// [`LayerRows`] as the display order of a list with an eye column, keyed by position.
struct EyedNames<'a> {
    names: &'a [String],
    visible: &'a [bool],
}

impl DisplayOrder<usize> for EyedNames<'_> {
    fn len(&self) -> usize {
        self.names.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.names.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        self.names.get(at).map(|name| std::borrow::Cow::Borrowed(name.as_str()))
    }
}

impl ItemSource<usize> for EyedNames<'_> {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        *column == EYE && self.visible.get(at).copied().unwrap_or(false)
    }
}

/// Layers in the layer form with an eye column (Cmd/Ctrl+, flips the eyes of the selected layers; drag down the
/// column to sweep it), a hidden layer dimmed, and rename by double click or F2. The readout shows the last default
/// action or menu asked for.
#[story(List<'static, usize>, covers = [Default, Many], drive = Drive::Never)]
fn list_many(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let columns = vec![
        ColumnSpec::new(EYE, Label::Empty, ColumnWidth::Controls(1.0))
            .kind(ColumnKind::Toggle(ToggleColumn::sweepable().key(Chord::new(Mods::CMD, Key::Comma))))
            .icon(IconId::new("eye")),
        ColumnSpec::new(NAME, Label::Empty, ColumnWidth::fill(1.0)),
    ];
    let LayerRows { names, visible, selection, last } = data.value("rows", LayerRows::new(40));
    let out =
        List::<usize>::new("Layers").header(false).rename(true).rename_column(NAME).columns(columns).selection(Bind::bound(selection)).show_items(
            ui,
            &EyedNames { names: names.as_slice(), visible: visible.as_slice() },
            &mut |key, icx| {
                let on = visible[*key];
                let row = ListRow::new(names[*key].clone()).layer(true).cells(icx.cells()).state(icx.row.clone()).toggle(ToggleCell::eye(EYE, on));
                if on { row } else { row.ink(RowInk::Dimmed) }
            },
        );
    for event in &out.events {
        match event {
            CollectionEvent::Toggle(edit) => {
                if let Some(cell) = edit.landed()
                    && let Some(eye) = visible.get_mut(cell.key)
                {
                    *eye = cell.on;
                }
            }
            CollectionEvent::Rename(edit) => {
                if let Some(rename) = edit.value()
                    && !rename.name.trim().is_empty()
                    && let Some(name) = names.get_mut(rename.key)
                {
                    rename.name.trim().clone_into(name);
                }
            }
            CollectionEvent::Activate(key) => *last = format!("Open {}", names[*key]),
            CollectionEvent::ContextMenu(request) => *last = format!("Menu for {}", names[request.key]),
            _ => {}
        }
    }
    StoryOutput::from(out).readout(last.clone())
}

/// Rows with the "More" button of `row_menu`, shown on the hovered and the selected row: a click on it opens the host's
/// row menu, whose Rename opens the row's rename field.
#[story(List<'static, usize>, covers = [], drive = Drive::Never)]
fn list_row_menu(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let names = names(4);
    let out = List::<usize>::new("Layers")
        .rename(true)
        .row_menu()
        .selection(Bind::bound(data.value("selection", Selection::single(1_usize))))
        .show_items(ui, &Names(&names), &mut |key, _| ListRow::new(names[*key].clone()));
    let menu_row = data.value("menu", None::<usize>);
    for event in &out.events {
        if let CollectionEvent::ContextMenu(request) = event {
            *menu_row = Some(request.key);
        }
    }
    if let Some(row) = *menu_row {
        let entries = [
            Entry::new(String::from("rename"), "Rename"),
            Entry::new(String::from("duplicate"), "Duplicate"),
            Entry::new(String::from("delete"), "Delete"),
        ];
        let menu = Menu::context("Layer", &out.response).pick(ui, &entries);
        if menu.picked.as_deref() == Some("rename") {
            request_rename(ui.ctx(), out.response.id, &row);
        }
        if menu.closed.is_some() {
            *menu_row = None;
        }
    }
    out.into()
}

#[story(List<'static, usize>, covers = [One], drive = Drive::Never)]
fn list_one(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    list(ui, 1)
}

#[story(List<'static, usize>, covers = [Empty], drive = Drive::Never)]
fn list_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    list(ui, 0)
}

fn findings(count: usize) -> Vec<Finding> {
    (0..count)
        .map(|n| {
            let id = n as u64;
            match n % 3 {
                0 => Finding::new(id, Severity::Info, format!("Layer {} has no name", n + 1)),
                1 => Finding::new(id, Severity::Warning, format!("Font {} is missing", n + 1)).detail(format!("page {}", n + 1)),
                _ => Finding::new(id, Severity::Error, format!("Page {} overflows its frame", n + 1)),
            }
        })
        .collect()
}

fn findings_list(ui: &mut Ui, findings: &[Finding]) -> StoryOutput {
    let source = Findings::new(findings);
    FindingsList::new("Findings").show_items(ui, &source, &mut |key, icx| findings[*key as usize].row(&icx.words::<FindingsWords>())).into()
}

#[story(FindingsList<'static>, covers = [Default, Many], drive = Drive::Never)]
fn findings_many(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    findings_list(ui, &findings(30))
}

#[story(FindingsList<'static>, covers = [One], drive = Drive::Never)]
fn findings_one(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    findings_list(ui, &findings(1))
}

#[story(FindingsList<'static>, covers = [Empty], drive = Drive::Never)]
fn findings_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    findings_list(ui, &[])
}

#[story(FindingsSummary<'static>, covers = [Default], drive = Drive::Never)]
fn findings_summary(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    FindingsSummary::new(&findings(3)).show(ui).into()
}

/// The time the event stories take as now, in milliseconds.
const NOW_MS: u64 = 3_600_000;

fn events(count: usize) -> Vec<LogEvent> {
    let levels = [EventLevel::Info, EventLevel::Warning, EventLevel::Error];
    (0..count)
        .map(|n| {
            LogEvent::new(n as u64 + 1, levels[n % 3], "Render", format!("Event {}", n + 1), NOW_MS - (n as u64 + 1) * 60_000)
                .count(1 + (n % 2) as u32)
        })
        .collect()
}

fn event_list(ui: &mut Ui, count: usize) -> StoryOutput {
    let events = events(count);
    let log = EventLog::new(&events, &EventLogState::default());
    EventList::new("Events")
        .show_items(ui, &log, &mut |_, icx| {
            let Some(event) = log.event(icx.at) else { return ListRow::spacer() };
            let age = format!("{} min ago", event.age_seconds(NOW_MS) / 60);
            event.row(&age, &icx.words::<EventWords>(), icx.format(), icx.row.striped)
        })
        .into()
}

#[story(EventList<'static>, covers = [Default, Many], drive = Drive::Never)]
fn event_list_many(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    event_list(ui, 12)
}

#[story(EventList<'static>, covers = [One], drive = Drive::Never)]
fn event_list_one(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    event_list(ui, 1)
}

#[story(EventList<'static>, covers = [Empty], drive = Drive::Never)]
fn event_list_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    event_list(ui, 0)
}

/// An undo list of `count` actions, "Step 1" the oldest, starting with `mark`, and its footer as the readout: the
/// actions a pick undoes, or Cancel while none is marked.
fn undo_list(ui: &mut Ui, data: &mut StoryData, count: usize, mark: UndoMark) -> StoryOutput {
    let labels: Vec<String> = (1..=count).rev().map(|n| format!("Step {n}")).collect();
    let mark = data.value("mark", mark);
    let rows = UndoRows::new(&labels);
    let out = UndoList::new("Undo History").selection(Bind::bound(&mut *mark)).show_items(ui, &rows, &mut |key, _| rows.row(*key));
    let env = Env::of(ui);
    let words = env.words::<UndoWords>();
    let footer = footer_text(&words, env.format(), mark).map_or_else(|| words.cancel.clone(), Label::from);
    StoryOutput::from(out).readout(footer.as_str())
}

/// WordCraft's undo list: 30 actions, the newest three marked.
#[story(UndoList<'static>, covers = [Default, Many], drive = Drive::Never)]
fn undo_list_many(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    undo_list(ui, data, 30, UndoMark::at(2))
}

#[story(UndoList<'static>, covers = [One], drive = Drive::Never)]
fn undo_list_one(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    undo_list(ui, data, 1, UndoMark::default())
}

#[story(UndoList<'static>, covers = [Empty], drive = Drive::Never)]
fn undo_list_empty(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    undo_list(ui, data, 0, UndoMark::default())
}
