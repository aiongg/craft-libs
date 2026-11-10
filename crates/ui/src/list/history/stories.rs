//! Stories of the History panel.
//!
//! Every story shows the panel over a fake document kept in the story's data: the panel's events are carried out on it
//! as an app's commands would, so clicks, drops and renames change what the story shows.

use super::{
    HistoryEvent, HistoryFeatures, HistoryMenu, HistoryOrder, HistoryPanel, HistoryPanelState, HistorySource, SnapshotId, SnapshotList, StepId,
    StepRows, show_history_menu,
};
use crate::buttons::Button;
use crate::contract::{Bind, Show as _, ShowContainer as _, Trigger};
use crate::display::{Shade, Text};
use crate::layout_components::Stack;
use crate::menu::Menu;
use crate::primitives::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::IconId;
use egui::{Ui, vec2};
use std::borrow::Cow;

/// One state of the fake history.
#[derive(Clone, Debug, PartialEq)]
struct FakeStep {
    id: u64,
    label: String,
    icon: Option<&'static str>,
    depth: usize,
    off_line: bool,
}

/// The fake history the panel reads: states, the current one, and whether it is a tree or non-linear.
#[derive(Clone, Debug, PartialEq)]
struct FakeSteps {
    rows: Vec<FakeStep>,
    current: usize,
    revision: u64,
    next_id: u64,
    non_linear: bool,
    tree: bool,
}

const TOOLS: [(&str, Option<&str>); 11] = [
    ("Brush Tool", Some("brush")),
    ("Eraser Tool", Some("eraser")),
    ("Crop", Some("crop")),
    ("Move Layer", Some("move")),
    ("Gradient Tool", Some("gradient")),
    ("Pencil Tool", Some("pencil")),
    ("Lasso Tool", Some("lasso")),
    ("Blur Tool", Some("droplet")),
    ("Pen Tool", Some("pen_tool")),
    ("Layer Opacity", None),
    ("Fill Layer", None),
];

impl FakeSteps {
    /// `len` states ("Open" and `len - 1` tools), the state at row `current` current.
    fn tools(len: usize, current: usize, non_linear: bool) -> Self {
        let mut steps = FakeSteps { rows: Vec::new(), current, revision: 0, next_id: 0, non_linear, tree: false };
        steps.push("Open".to_owned(), None);
        for n in 0..len.saturating_sub(1) {
            let (label, icon) = TOOLS[n % TOOLS.len()];
            steps.push(label.to_owned(), icon);
        }
        steps
    }

    /// A tree of seven states: a branch of two states off the second state, the current line continuing at depth 0
    /// and one undone state after it.
    fn branches() -> Self {
        let rows = [
            ("Open", 0, false),
            ("Brush Tool", 0, false),
            ("Crop", 1, true),
            ("Blur Tool", 1, true),
            ("Gradient Tool", 0, false),
            ("Layer Opacity", 0, false),
            ("Fill Layer", 0, true),
        ];
        let rows = rows
            .iter()
            .enumerate()
            .map(|(at, &(label, depth, off_line))| FakeStep { id: at as u64, label: label.to_owned(), icon: None, depth, off_line })
            .collect();
        FakeSteps { rows, current: 5, revision: 0, next_id: 7, non_linear: false, tree: true }
    }

    fn push(&mut self, label: String, icon: Option<&'static str>) {
        self.rows.push(FakeStep { id: self.next_id, label, icon, depth: 0, off_line: false });
        self.next_id += 1;
    }

    /// Records a state after the current one: linear history drops the later states.
    fn record(&mut self, label: String, icon: Option<&'static str>) {
        if !self.non_linear {
            self.rows.truncate(self.current + 1);
        }
        self.push(label, icon);
        self.current = self.rows.len() - 1;
        self.revision += 1;
    }

    fn go_to(&mut self, id: StepId) {
        if let Some(at) = (0..self.rows.len()).find(|&at| self.id(at) == id) {
            self.current = at;
            self.revision += 1;
        }
    }

    fn delete(&mut self, id: StepId) {
        let Some(at) = (0..self.rows.len()).find(|&at| self.id(at) == id).filter(|&at| at > 0) else { return };
        if self.non_linear {
            self.rows.remove(at);
            if self.current >= at {
                self.current -= 1;
            }
        } else {
            self.rows.truncate(at);
            self.current = self.current.min(at - 1);
        }
        self.revision += 1;
    }

    fn clear(&mut self) {
        let kept = self.rows[self.current].clone();
        self.rows = vec![kept];
        self.current = 0;
        self.revision += 1;
    }
}

impl StepRows for FakeSteps {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn id(&self, at: usize) -> StepId {
        StepId::new(self.rows[at].id)
    }

    fn label(&self, at: usize) -> Cow<'_, str> {
        Cow::Borrowed(&self.rows[at].label)
    }

    fn icon(&self, at: usize) -> Option<IconId> {
        self.rows[at].icon.map(IconId::new)
    }

    fn current(&self) -> usize {
        self.current
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn depth(&self, at: usize) -> usize {
        self.rows[at].depth
    }

    fn is_later(&self, at: usize) -> bool {
        if self.tree { self.rows[at].off_line } else { at > self.current }
    }

    fn is_non_linear(&self) -> bool {
        self.non_linear
    }
}

/// The history brush source of the fake document.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    /// The oldest state.
    Oldest,
    Chosen(HistorySource),
}

/// The fake document a story shows the panel over.
#[derive(Clone, Debug, PartialEq)]
struct Doc {
    steps: FakeSteps,
    snapshots: SnapshotList,
    panel: HistoryPanelState,
    source: Source,
    rename: Option<SnapshotId>,
    last: Option<String>,
}

impl Doc {
    fn new(steps: FakeSteps, snapshots: SnapshotList) -> Self {
        Doc { steps, snapshots, panel: HistoryPanelState::new(), source: Source::Oldest, rename: None, last: None }
    }

    fn resolved_source(&self) -> Option<HistorySource> {
        match self.source {
            Source::Oldest => Some(HistorySource::Step(self.steps.id(0))),
            Source::Chosen(HistorySource::Snapshot(id)) => self.snapshots.contains(id).then_some(HistorySource::Snapshot(id)),
            Source::Chosen(HistorySource::Step(id)) => (0..self.steps.len()).any(|at| self.steps.id(at) == id).then_some(HistorySource::Step(id)),
        }
    }

    fn snapshot(&mut self, name: &str) -> SnapshotId {
        let suggested = format!("Snapshot {}", self.snapshots.next_number());
        self.snapshots.add(name, &suggested)
    }

    /// Carries out what the panel reported, as an app does.
    fn apply(&mut self, event: HistoryEvent) {
        self.last = Some(format!("{event:?}"));
        match event {
            HistoryEvent::GoTo(id) => self.steps.go_to(id),
            HistoryEvent::SelectSnapshot(id) => {
                let name = self.snapshots.iter().find(|snapshot| snapshot.id == id).map(|snapshot| snapshot.name.clone()).unwrap_or_default();
                self.steps.record(name, Some("scan"));
                self.panel.snapshot_selected(id, self.steps.revision());
            }
            HistoryEvent::SetSource(source) => self.source = Source::Chosen(source),
            HistoryEvent::RenameSnapshot { id, text } => {
                self.snapshots.rename(id, &text);
            }
            HistoryEvent::DeleteStep(id) => self.steps.delete(id),
            HistoryEvent::DeleteSnapshot(id) => {
                self.snapshots.remove(id);
                if self.panel.selected_snapshot() == Some(id) {
                    self.panel.clear_snapshot();
                }
            }
            HistoryEvent::NewSnapshot { .. } => {
                self.snapshot("");
            }
            HistoryEvent::StepForward => {
                if let Some(next) = (self.steps.current + 1 < self.steps.len()).then(|| self.steps.id(self.steps.current + 1)) {
                    self.steps.go_to(next);
                }
            }
            HistoryEvent::StepBackward => {
                if let Some(at) = self.steps.current.checked_sub(1) {
                    self.steps.go_to(self.steps.id(at));
                }
            }
            HistoryEvent::ClearHistory => self.steps.clear(),
            _ => {}
        }
    }
}

/// `count` snapshots named after the story's three names, then "Snapshot N" suggestions.
fn snapshot_list(count: usize) -> SnapshotList {
    let mut snapshots = SnapshotList::new();
    let names = ["Before retouching", "Skin smoothed", "Final colours"];
    for n in 0..count {
        let suggested = format!("Snapshot {}", snapshots.next_number());
        snapshots.add(names.get(n).copied().unwrap_or(""), &suggested);
    }
    snapshots
}

/// How a story's panel is drawn.
struct PanelOptions {
    salt: &'static str,
    features: HistoryFeatures,
    order: HistoryOrder,
    thumbnails: bool,
    width: f32,
    height: f32,
    menu: bool,
}

impl PanelOptions {
    fn new(salt: &'static str, width: f32, height: f32) -> Self {
        PanelOptions { salt, features: HistoryFeatures::default(), order: HistoryOrder::default(), thumbnails: true, width, height, menu: false }
    }
}

/// The story's document, kept under `name` and rebuilt when `signature` changes.
fn show_doc<S: craft_behavior::Value>(
    ui: &mut Ui,
    data: &mut StoryData,
    name: &'static str,
    signature: S,
    make: impl FnOnce() -> Doc,
    options: &PanelOptions,
) -> StoryOutput {
    let kept = data.value(name, (signature.clone(), None::<Doc>));
    if kept.0 != signature || kept.1.is_none() {
        *kept = (signature, Some(make()));
    }
    let Some(doc) = kept.1.as_mut() else { return StoryOutput::default() };
    show_panel(ui, doc, options)
}

fn show_panel(ui: &mut Ui, doc: &mut Doc, options: &PanelOptions) -> StoryOutput {
    let ctx = ui.ctx().clone();
    let thumbnails = Thumbnails::<SnapshotId>::in_context(&ctx, "history-story.thumbs", ThumbnailOptions::default());
    let pictures = FakeThumbSource::<SnapshotId>::new();
    let source = doc.resolved_source();
    let rename = doc.rename.take();
    let mut out = StoryOutput::default();
    let mut menu = HistoryMenu::new(options.features, doc.steps.id(0));
    let mut events = Vec::new();
    let rect = egui::Rect::from_min_size(ui.cursor().min, vec2(options.width, options.height));
    Stack::new().fill().show_at_taking_room(ui, rect, |ui, _| {
        let mut panel = HistoryPanel::new(options.salt, &doc.steps, &mut doc.panel)
            .snapshots(&doc.snapshots)
            .source(source)
            .features(options.features)
            .order(options.order);
        if options.thumbnails {
            panel = panel.thumbnails(&thumbnails, &pictures);
        }
        if let Some(id) = rename {
            panel = panel.start_rename(id);
        }
        let shown = panel.show(ui);
        menu = shown.menu;
        events = shown.events;
        out = shown.response.id.into_story();
    });
    for event in events {
        doc.apply(event);
    }
    if options.menu {
        let opener = Button::new("Panel menu").show(ui);
        let mut chosen = None;
        Menu::new("History menu").opened_by(&opener, Trigger::Click).show(ui, |ui, _| chosen = show_history_menu(&menu, ui));
        if let Some(event) = chosen {
            doc.apply(event);
        }
    }
    let text = doc.last.clone().map_or_else(|| "No event yet".to_owned(), |event| format!("Last event: {event}"));
    Text::new(text).shade(Shade::Muted).show(ui);
    out
}

trait IntoStory {
    fn into_story(self) -> StoryOutput;
}

impl IntoStory for egui::Id {
    fn into_story(self) -> StoryOutput {
        StoryOutput { id: Some(self), ..StoryOutput::default() }
    }
}

#[story(HistoryPanel<'static>, covers = [], drive = Drive::Never)]
fn history_states(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let footer = *data.value("footer", true);
    let width = *data.ranged("width", 260.0_f32, 180.0..=420.0);
    let height = *data.ranged("height", 340.0_f32, 160.0..=520.0);
    let mut options = PanelOptions::new("history-states", width, height);
    options.features = HistoryFeatures { snapshots: false, source: true, new_document: footer, delete: footer, delete_later: false, options: true };
    show_doc(ui, data, "doc-states", (), || Doc::new(FakeSteps::tools(12, 6, false), SnapshotList::new()), &options)
}

#[story(HistoryPanel<'static>, covers = [], drive = Drive::Never)]
fn history_snapshots(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let thumbnails = *data.value("thumbnails", true);
    let rename = *data.value("rename in progress", true);
    let on_snapshot = *data.value("source on a snapshot", true);
    let mut options = PanelOptions::new("history-snapshots", 280.0, 380.0);
    options.thumbnails = thumbnails;
    show_doc(
        ui,
        data,
        "doc-snapshots",
        (on_snapshot, rename),
        || {
            let mut doc = Doc::new(FakeSteps::tools(8, 5, false), snapshot_list(3));
            if on_snapshot && let Some(snapshot) = doc.snapshots.iter().nth(2) {
                doc.source = Source::Chosen(HistorySource::Snapshot(snapshot.id));
            }
            if let Some(id) = doc.snapshots.iter().nth(1).map(|snapshot| snapshot.id) {
                doc.apply(HistoryEvent::SelectSnapshot(id));
                doc.last = None;
                doc.rename = rename.then_some(id);
            }
            doc
        },
        &options,
    )
}

#[story(HistoryPanel<'static>, covers = [], drive = Drive::Never)]
fn history_source(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let kind = *data.choice("source", 0_u8, &[("state", 0), ("default (oldest)", 1), ("none (state gone)", 2)]);
    let options = PanelOptions::new("history-source", 260.0, 320.0);
    show_doc(
        ui,
        data,
        "doc-source",
        kind,
        || {
            let mut doc = Doc::new(FakeSteps::tools(9, 8, false), SnapshotList::new());
            let chosen = doc.steps.id(4);
            match kind {
                0 => doc.source = Source::Chosen(HistorySource::Step(chosen)),
                2 => {
                    doc.source = Source::Chosen(HistorySource::Step(chosen));
                    doc.steps.delete(chosen);
                }
                _ => {}
            }
            doc
        },
        &options,
    )
}

#[story(HistoryPanel<'static>, covers = [], drive = Drive::Never)]
fn history_non_linear(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let non_linear = *data.value("non-linear", true);
    let options = PanelOptions::new("history-non-linear", 260.0, 320.0);
    show_doc(ui, data, "doc-non-linear", non_linear, || Doc::new(FakeSteps::tools(10, 4, non_linear), SnapshotList::new()), &options)
}

#[story(HistoryPanel<'static>, covers = [], drive = Drive::Never)]
fn history_branches(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut options = PanelOptions::new("history-branches", 260.0, 280.0);
    options.features = HistoryFeatures::NONE;
    show_doc(ui, data, "doc-branches", (), || Doc::new(FakeSteps::branches(), SnapshotList::new()), &options)
}

#[story(HistoryPanel<'static>, covers = [], drive = Drive::Never)]
fn history_newest_first(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let newest_first = *data.value("newest first", true);
    let mut options = PanelOptions::new("history-newest-first", 260.0, 320.0);
    options.features = HistoryFeatures::NONE;
    options.order = if newest_first { HistoryOrder::NewestFirst } else { HistoryOrder::OldestFirst };
    show_doc(ui, data, "doc-newest-first", (), || Doc::new(FakeSteps::tools(12, 8, false), SnapshotList::new()), &options)
}

#[story(HistoryPanel<'static>, covers = [], drive = Drive::Never)]
fn history_short_dock(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let height = *data.ranged("height", 160.0_f32, 120.0..=400.0);
    let options = PanelOptions::new("history-short-dock", 260.0, height);
    show_doc(
        ui,
        data,
        "doc-short-dock",
        (),
        || {
            let mut snapshots = snapshot_list(3);
            for _ in 0..3 {
                let suggested = format!("Snapshot {}", snapshots.next_number());
                snapshots.add("", &suggested);
            }
            Doc::new(FakeSteps::tools(20, 14, false), snapshots)
        },
        &options,
    )
}

#[story(HistoryPanel<'static>, covers = [], drive = Drive::Never)]
fn history_menu(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut options = PanelOptions::new("history-menu", 260.0, 300.0);
    options.menu = true;
    show_doc(ui, data, "doc-menu", (), || Doc::new(FakeSteps::tools(8, 4, false), snapshot_list(3)), &options)
}

#[story(crate::list::UndoList<'static>, covers = [], drive = Drive::Never)]
fn undo_list_popup(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    use crate::contract::ShowCollection as _;
    use crate::list::{UndoList, UndoMark, UndoRows, UndoWords, footer_text};
    let steps = *data.ranged("steps", 30_usize, 1.0..=40.0);
    let marked = *data.ranged("marked", 3_usize, 0.0..=40.0);
    let labels: Vec<String> = (1..=steps).rev().map(|n| format!("Step {n}")).collect();
    let mark = data.value("mark", UndoMark::at(marked.min(steps).saturating_sub(1)));
    let rows = UndoRows::new(&labels);
    let shown = UndoList::new("Undo History").selection(Bind::bound(mark)).show_items(ui, &rows, &mut |key, _| rows.row(*key));
    let env = crate::runtime::env::Env::of(ui);
    let words = env.words::<UndoWords>();
    let format = env.format();
    let footer = footer_text(&words, format, mark).unwrap_or_else(|| words.cancel.as_str().to_owned());
    Text::new(footer).show(ui);
    shown.into()
}
