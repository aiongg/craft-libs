//! Stories of the History panel and the undo list.
//!
//! Every story uses fakes: a `StepList` and a `SnapshotList` built in the story,
//! `FakeThumbSource<SnapshotId>` for the snapshot thumbnails; no document and no platform. Every
//! string goes through story words. The panel's events are carried out on the story's fake
//! document, so clicks, drops and renames change what the story shows. A panel story draws the
//! panel at the tile's width, as a dock does.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/history-panel/states` | 12 states with tool icons, the current state in the middle, later states dimmed, footer |
//! | `composites/history-panel/snapshots` | three snapshots above the states, one selected, the source on a snapshot, a rename in progress |
//! | `composites/history-panel/source` | the source on a state, on the default (oldest), and none (state gone) |
//! | `composites/history-panel/non-linear` | non-linear history with later states deletable by a drop on the trash |
//! | `composites/history-panel/branches` | EffectCraft-style tree rows with depth and off-line rows dimmed (a `StepRows` fake) |
//! | `composites/history-panel/newest-first` | LightCraft's order, no footer features |
//! | `composites/history-panel/short-dock` | 6 snapshots and 20 states in a 160 pt dock: the rows scroll above the footer |
//! | `composites/history-panel/menu` | the panel menu beside a real panel: its rows enable from what the panel stores, and a chosen row runs on the document |
//! | `composites/undo-list/popup` | WordCraft's list with 30 steps, three marked, footer "Undo 3 Actions", and the Cancel row |
//! | `composites/history-panel/new-snapshot` | the New Snapshot form with Name and From |
//! | `composites/history-panel/options` | the History Options form with one extra row |

use egui::{Ui, vec2};
use std::borrow::Cow;

use super::labels::{HistoryOptionsLabels, SnapshotFormLabels};
use super::{HistoryEvent, HistoryFeatures, HistoryOrder, HistoryPanel, HistoryPanelState, UndoList};
use crate::canvas::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use crate::controls::Button;
use crate::dialog::{CommandForm, Param, ParamKind, StackDialog};
use crate::edit::snapshots::{HistoryOptions, HistorySource, HistorySourceChoice, SnapshotId, SnapshotList, history_options_form, new_snapshot_form};
use crate::edit::steps::{HistoryStep, StepId, StepList, StepRows};
use crate::icons::Icon;
use crate::localized::{Labels as _, LocaleNumbers};
use crate::menu::PressMenu;
use crate::primitives::{Content, Text};
use crate::story::{self, Level, NoArgs, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

crate::words! {
    /// The sample text of the History stories, English by default: the names of the steps and
    /// snapshots, the panel menu's button and the From options of the New Snapshot form.
    #[non_exhaustive]
    pub struct HistoryStoryLabels {
        open: "historyStory.open" = "Open";
        brush: "historyStory.brush" = "Brush Tool";
        eraser: "historyStory.eraser" = "Eraser Tool";
        crop: "historyStory.crop" = "Crop";
        move_layer: "historyStory.moveLayer" = "Move Layer";
        gradient: "historyStory.gradient" = "Gradient Tool";
        pencil: "historyStory.pencil" = "Pencil Tool";
        lasso: "historyStory.lasso" = "Lasso Tool";
        blur: "historyStory.blur" = "Blur Tool";
        pen: "historyStory.pen" = "Pen Tool";
        opacity: "historyStory.opacity" = "Layer Opacity";
        fill: "historyStory.fill" = "Fill Layer";
        snapshot_before: "historyStory.snapshotBefore" = "Before retouching";
        snapshot_skin: "historyStory.snapshotSkin" = "Skin smoothed";
        snapshot_final: "historyStory.snapshotFinal" = "Final colours";
        step_n: "historyStory.stepN" = "Step {n}";
        panel_menu: "historyStory.panelMenu" = "Panel menu";
        from_document: "historyStory.fromDocument" = "Full Document";
        from_merged: "historyStory.fromMerged" = "Merged Layers";
        from_layer: "historyStory.fromLayer" = "Current Layer";
        visibility_undoable: "historyStory.visibilityUndoable" = "Make Layer Visibility Changes Undoable";
    }
}

impl HistoryStoryLabels {
    /// The tool steps the stories cycle through, with their icons; the last two have none, to show
    /// the empty icon slot.
    fn tools(&self) -> [(&str, Option<Icon>); 11] {
        [
            (&self.brush, Some(Icon::Brush)),
            (&self.eraser, Some(Icon::Eraser)),
            (&self.crop, Some(Icon::Crop)),
            (&self.move_layer, Some(Icon::Move)),
            (&self.gradient, Some(Icon::Gradient)),
            (&self.pencil, Some(Icon::Pencil)),
            (&self.lasso, Some(Icon::Lasso)),
            (&self.blur, Some(Icon::Droplet)),
            (&self.pen, Some(Icon::PenTool)),
            (&self.opacity, None),
            (&self.fill, None),
        ]
    }

    /// A step list of `len` states ("Open" and `len - 1` tools), the state at row `current` current
    /// (the rows after it are later states).
    pub(super) fn step_list(&self, len: usize, current: usize, non_linear: bool) -> StepList {
        let mut steps = StepList::new(HistoryStep::new(self.open.as_str()));
        steps.set_limit(len.max(StepList::DEFAULT_LIMIT));
        steps.set_non_linear(non_linear);
        let tools = self.tools();
        for n in 0..len.saturating_sub(1) {
            let (label, icon) = tools[n % tools.len()];
            let step = HistoryStep::new(label);
            let _ = steps.record(match icon {
                Some(icon) => step.icon(icon),
                None => step,
            });
        }
        if let Some(id) = steps.id_at(current) {
            let _ = steps.go_to(id);
        }
        steps
    }

    /// `count` snapshots named after the story's three names, then "Snapshot N" suggestions.
    pub(super) fn snapshot_list(&self, count: usize) -> SnapshotList {
        let labels = SnapshotFormLabels::english();
        let numbers = LocaleNumbers::from_tag("en");
        let mut snapshots = SnapshotList::new();
        let names = [&self.snapshot_before, &self.snapshot_skin, &self.snapshot_final];
        for n in 0..count {
            snapshots.add(names.get(n).map(|name| name.as_str()), &labels, &numbers);
        }
        snapshots
    }
}

/// A fake `StepRows` with a tree: EffectCraft's branches. Row `n` has id `n`.
pub(super) struct BranchRows {
    /// Label, depth and whether the row is off the current line.
    rows: Vec<(String, usize, bool)>,
    current: usize,
}

impl BranchRows {
    /// A tree of seven states: a branch of two states off the second state, the current line
    /// continuing at depth 0 and one undone state after it.
    pub(super) fn new(words: &HistoryStoryLabels) -> Self {
        let rows = [
            (&words.open, 0, false),
            (&words.brush, 0, false),
            (&words.crop, 1, true),
            (&words.blur, 1, true),
            (&words.gradient, 0, false),
            (&words.opacity, 0, false),
            (&words.fill, 0, true),
        ];
        Self { rows: rows.map(|(label, depth, off)| (label.clone(), depth, off)).to_vec(), current: 5 }
    }

    /// Makes row `id` current.
    fn go_to(&mut self, id: StepId) {
        if let Some(at) = (0..self.rows.len()).find(|&at| self.id(at) == id) {
            self.current = at;
        }
    }
}

impl StepRows for BranchRows {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn id(&self, at: usize) -> StepId {
        StepId::new(at as u64)
    }

    fn label(&self, at: usize) -> Cow<'_, str> {
        Cow::Borrowed(&self.rows[at].0)
    }

    fn icon(&self, _at: usize) -> Option<Icon> {
        None
    }

    fn current(&self) -> usize {
        self.current
    }

    fn revision(&self) -> u64 {
        self.current as u64
    }

    fn depth(&self, at: usize) -> usize {
        self.rows[at].1
    }

    fn is_later(&self, at: usize) -> bool {
        self.rows[at].2
    }
}

/// The fake document a story shows the panel over: the app's state, changed by the panel's events
/// the way an app's commands would.
pub(super) struct Doc {
    pub(super) steps: StepList,
    pub(super) snapshots: SnapshotList,
    pub(super) panel: HistoryPanelState,
    pub(super) source: HistorySourceChoice,
    /// The rows of a story that shows its own `StepRows`; the panel reads these instead of `steps`.
    pub(super) rows: Option<BranchRows>,
    /// The last event the panel reported.
    pub(super) last: Option<HistoryEvent>,
    /// A snapshot whose rename the next pass starts.
    pub(super) rename: Option<SnapshotId>,
}

impl Doc {
    pub(super) fn new(steps: StepList, snapshots: SnapshotList) -> Self {
        Doc { steps, snapshots, panel: HistoryPanelState::new(), source: HistorySourceChoice::default(), rows: None, last: None, rename: None }
    }

    /// Carries out what the panel reported, as an app does: jumps, restores a snapshot as one new
    /// state, sets the source, renames and deletes.
    pub(super) fn apply(&mut self, event: HistoryEvent) {
        let labels = SnapshotFormLabels::english();
        match &event {
            HistoryEvent::GoTo(id) => match &mut self.rows {
                Some(rows) => rows.go_to(*id),
                None => {
                    let _ = self.steps.go_to(*id);
                }
            },
            HistoryEvent::SelectSnapshot(id) => {
                let name = self.snapshots.get(*id).map(|snapshot| snapshot.name.clone()).unwrap_or_default();
                let _ = self.steps.record(HistoryStep::new(name).icon(Icon::Scan));
                self.panel.snapshot_selected(*id, self.steps.revision());
            }
            HistoryEvent::SetSource(source) => self.source.set(*source, &mut self.steps),
            HistoryEvent::RenameSnapshot { id, text } => {
                self.snapshots.rename(*id, text);
            }
            HistoryEvent::DeleteStep(id) => {
                let _ = self.steps.delete(*id);
            }
            HistoryEvent::DeleteSnapshot(id) => {
                self.snapshots.remove(*id);
                if self.panel.selected_snapshot() == Some(*id) {
                    self.panel.clear_snapshot();
                }
            }
            HistoryEvent::NewSnapshot { .. } => {
                self.snapshots.add(None, &labels, &LocaleNumbers::from_tag("en"));
            }
            HistoryEvent::StepForward => {
                let _ = self.steps.forward();
            }
            HistoryEvent::StepBackward => {
                let _ = self.steps.back();
            }
            HistoryEvent::ClearHistory => self.steps.clear(),
            HistoryEvent::NewDocument(_) | HistoryEvent::DeleteLater | HistoryEvent::Options => {}
        }
        self.last = Some(event);
    }
}

/// How a story's panel is drawn.
struct PanelOptions {
    /// The panel's id salt.
    salt: &'static str,
    features: HistoryFeatures,
    order: HistoryOrder,
    thumbnails: bool,
    /// The dock's width, kept to the space the story has.
    width: f32,
    height: f32,
    /// Show the panel menu's button under the panel.
    menu: bool,
}

impl PanelOptions {
    fn new(salt: &'static str, width: f32, height: f32) -> Self {
        PanelOptions { salt, features: HistoryFeatures::default(), order: HistoryOrder::default(), thumbnails: true, width, height, menu: false }
    }
}

/// The story's document, kept per story instance under `name` and rebuilt when `signature`
/// changes or Reset is pressed; `show` draws the story over it.
fn with_doc<S: PartialEq + Clone + Send + Sync + 'static>(
    ui: &mut Ui,
    name: &str,
    signature: S,
    make: impl FnOnce(&HistoryStoryLabels) -> Doc,
    show: impl FnOnce(&mut Ui, &mut Doc, &HistoryStoryLabels),
) {
    let words = HistoryStoryLabels::of(ui);
    let slot = story::shared(ui, name, || None::<(S, Doc)>);
    let mut slot = story::lock(&slot);
    if ui.add(Button::new(&StoryLabels::of(ui).reset)).clicked() {
        *slot = None;
    }
    if slot.as_ref().is_none_or(|(kept, _)| *kept != signature) {
        *slot = Some((signature, make(&words)));
    }
    if let Some((_, doc)) = slot.as_mut() {
        show(ui, doc, &words);
    }
}

/// Names the last event the panel reported under it.
fn show_last_event(ui: &mut Ui, doc: &Doc) {
    let words = StoryLabels::of(ui);
    let text = match &doc.last {
        Some(event) => craft_i18n::fmt(&words.last_event, &[("event", &format!("{event:?}"))]),
        None => words.no_event.clone(),
    };
    ui.add(Text::new(text).content(Content::Muted));
}

/// Draws the panel over `doc` in a dock of the options' size (at most the width the story has),
/// carries out its events and names the last one under it.
fn show_panel(ui: &mut Ui, doc: &mut Doc, words: &HistoryStoryLabels, options: &PanelOptions) {
    let ctx = ui.ctx().clone();
    let thumbnails = Thumbnails::<SnapshotId>::in_context(&ctx, "history-story.thumbs", ThumbnailOptions::default());
    let pictures = FakeThumbSource::<SnapshotId>::new();
    let source = doc.source.resolve(&doc.steps, &doc.snapshots);
    let rename = doc.rename.take();
    let mut events = Vec::new();
    let width = options.width.min(ui.available_width());
    ui.allocate_ui(vec2(width, options.height), |ui| {
        let rows: &dyn StepRows = match &doc.rows {
            Some(rows) => rows,
            None => &doc.steps,
        };
        let mut panel = HistoryPanel::new(options.salt, rows, &mut doc.panel)
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
        events = panel.show(ui).events;
    });
    for event in events {
        doc.apply(event);
    }
    if options.menu {
        // The menu reads what the panel stored in this pass.
        let entries = HistoryPanel::panel_menu(&ctx, options.salt, &doc.panel);
        let button = ui.add(Button::new(&words.panel_menu));
        if let Some(command) = PressMenu::new(&entries).show(ui, &button).chosen
            && let Some(event) = HistoryPanel::run_command(&ctx, options.salt, &mut doc.panel, &command)
        {
            doc.apply(event);
        }
    }
    show_last_event(ui, doc);
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StatesArgs {
    footer: bool,
    #[arg(range = 180.0..=420.0)]
    width: f32,
    #[arg(range = 160.0..=520.0)]
    height: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SnapshotsArgs {
    thumbnails: bool,
    #[arg(name = "rename in progress")]
    rename: bool,
    #[arg(name = "source on a snapshot")]
    source_on_snapshot: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq, Eq)]
enum SourceArg {
    State,
    #[choice(name = "default (oldest)")]
    Oldest,
    #[choice(name = "none (state gone)")]
    Gone,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SourceArgs {
    source: SourceArg,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct NonLinearArgs {
    #[arg(name = "non-linear")]
    non_linear: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct NewestFirstArgs {
    #[arg(name = "newest first")]
    newest_first: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ShortDockArgs {
    #[arg(range = 120.0..=400.0)]
    height: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct UndoArgs {
    #[arg(range = 1..=40)]
    steps: usize,
    #[arg(range = 0..=40)]
    marked: usize,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FormArgs {
    open: bool,
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["History panel", "Undo list"];

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/history-panel/states",
            Level::Composites,
            "History panel",
            "States",
            StatesArgs { footer: true, width: 260.0, height: 340.0 },
            show_states
        ),
        typed_story!(
            "composites/history-panel/snapshots",
            Level::Composites,
            "History panel",
            "Snapshots",
            SnapshotsArgs { thumbnails: true, rename: true, source_on_snapshot: true },
            show_snapshots
        ),
        typed_story!(
            "composites/history-panel/source",
            Level::Composites,
            "History panel",
            "Source",
            SourceArgs { source: SourceArg::State },
            show_source
        ),
        typed_story!(
            "composites/history-panel/non-linear",
            Level::Composites,
            "History panel",
            "Non-linear",
            NonLinearArgs { non_linear: true },
            show_non_linear
        ),
        typed_story!("composites/history-panel/branches", Level::Composites, "History panel", "Branches", NoArgs {}, show_branches),
        typed_story!(
            "composites/history-panel/newest-first",
            Level::Composites,
            "History panel",
            "Newest first",
            NewestFirstArgs { newest_first: true },
            show_newest_first
        ),
        typed_story!(
            "composites/history-panel/short-dock",
            Level::Composites,
            "History panel",
            "Short dock",
            ShortDockArgs { height: 160.0 },
            show_short_dock
        ),
        typed_story!("composites/history-panel/menu", Level::Composites, "History panel", "Menu", NoArgs {}, show_menu),
        typed_story!("composites/undo-list/popup", Level::Composites, "Undo list", "Popup", UndoArgs { steps: 30, marked: 3 }, show_undo_list),
        typed_story!(
            "composites/history-panel/new-snapshot",
            Level::Composites,
            "History panel",
            "New Snapshot form",
            FormArgs { open: true },
            show_new_snapshot
        ),
        typed_story!(
            "composites/history-panel/options",
            Level::Composites,
            "History panel",
            "History Options form",
            FormArgs { open: true },
            show_options
        ),
    ]
}

/// The footer features the stories without a footer use.
const NO_FOOTER: HistoryFeatures =
    HistoryFeatures { snapshots: false, source: false, new_document: false, delete: false, delete_later: false, options: false };

/// Twelve states, the seventh current, the later ones dimmed; the footer is the args' choice.
fn show_states(ui: &mut Ui, args: &mut StatesArgs) {
    let (footer, width, height) = (args.footer, args.width, args.height);
    with_doc(
        ui,
        "states",
        (),
        |words| Doc::new(words.step_list(12, 6, false), SnapshotList::new()),
        |ui, doc, words| {
            let mut options = PanelOptions::new("history-states", width, height);
            options.features =
                HistoryFeatures { snapshots: false, source: true, new_document: footer, delete: footer, delete_later: false, options: true };
            show_panel(ui, doc, words, &options);
        },
    );
}

/// Three snapshots above the states, the second selected, the source on the third snapshot, and a
/// rename running on the second (started again when the args change).
fn show_snapshots(ui: &mut Ui, args: &mut SnapshotsArgs) {
    let (thumbnails, rename, on_snapshot) = (args.thumbnails, args.rename, args.source_on_snapshot);
    with_doc(
        ui,
        "snapshots",
        (on_snapshot, rename),
        |words| {
            let mut doc = Doc::new(words.step_list(8, 5, false), words.snapshot_list(3));
            if on_snapshot && let Some(snapshot) = doc.snapshots.iter().nth(2) {
                doc.source.set(HistorySource::Snapshot(snapshot.id), &mut doc.steps);
            }
            if let Some(id) = doc.snapshots.iter().nth(1).map(|snapshot| snapshot.id) {
                doc.apply(HistoryEvent::SelectSnapshot(id));
                doc.last = None;
                doc.rename = rename.then_some(id);
            }
            doc
        },
        |ui, doc, words| {
            let mut options = PanelOptions::new("history-snapshots", 280.0, 380.0);
            options.thumbnails = thumbnails;
            show_panel(ui, doc, words, &options);
        },
    );
}

/// The source on a state, on the oldest state by default, or gone with its deleted state.
fn show_source(ui: &mut Ui, args: &mut SourceArgs) {
    let kind = args.source;
    with_doc(
        ui,
        "source",
        kind as u8,
        |words| {
            let mut doc = Doc::new(words.step_list(9, 8, false), SnapshotList::new());
            let chosen = doc.steps.id_at(4);
            match (kind, chosen) {
                (SourceArg::State, Some(id)) => doc.source.set(HistorySource::Step(id), &mut doc.steps),
                (SourceArg::Gone, Some(id)) => {
                    doc.source.set(HistorySource::Step(id), &mut doc.steps);
                    let _ = doc.steps.delete(id);
                }
                _ => {}
            }
            doc
        },
        |ui, doc, words| show_panel(ui, doc, words, &PanelOptions::new("history-source", 260.0, 320.0)),
    );
}

/// Non-linear history: later states stay after a jump back and a drop on the trash deletes one.
fn show_non_linear(ui: &mut Ui, args: &mut NonLinearArgs) {
    let non_linear = args.non_linear;
    with_doc(
        ui,
        "non-linear",
        (),
        |words| Doc::new(words.step_list(10, 4, true), SnapshotList::new()),
        |ui, doc, words| {
            if doc.steps.is_non_linear() != non_linear {
                doc.steps.set_non_linear(non_linear);
            }
            show_panel(ui, doc, words, &PanelOptions::new("history-non-linear", 260.0, 320.0));
        },
    );
}

/// EffectCraft's tree: rows indented by depth, the other branch and the undone state dimmed.
fn show_branches(ui: &mut Ui, _args: &mut NoArgs) {
    with_doc(
        ui,
        "branches",
        (),
        |words| {
            let mut doc = Doc::new(words.step_list(1, 0, false), SnapshotList::new());
            doc.rows = Some(BranchRows::new(words));
            doc
        },
        |ui, doc, words| {
            let mut options = PanelOptions::new("history-branches", 260.0, 280.0);
            options.features = NO_FOOTER;
            show_panel(ui, doc, words, &options);
        },
    );
}

/// LightCraft's order: the newest state on top and no footer.
fn show_newest_first(ui: &mut Ui, args: &mut NewestFirstArgs) {
    let newest_first = args.newest_first;
    with_doc(
        ui,
        "newest-first",
        (),
        |words| Doc::new(words.step_list(12, 8, false), SnapshotList::new()),
        |ui, doc, words| {
            let mut options = PanelOptions::new("history-newest-first", 260.0, 320.0);
            options.features = NO_FOOTER;
            options.order = if newest_first { HistoryOrder::NewestFirst } else { HistoryOrder::OldestFirst };
            show_panel(ui, doc, words, &options);
        },
    );
}

/// Six snapshots and twenty states in a short dock: the rows scroll above the footer.
fn show_short_dock(ui: &mut Ui, args: &mut ShortDockArgs) {
    let height = args.height;
    with_doc(
        ui,
        "short-dock",
        (),
        |words| {
            let mut snapshots = words.snapshot_list(3);
            let labels = SnapshotFormLabels::english();
            let numbers = LocaleNumbers::from_tag("en");
            for _ in 0..3 {
                snapshots.add(None, &labels, &numbers);
            }
            Doc::new(words.step_list(20, 14, false), snapshots)
        },
        |ui, doc, words| show_panel(ui, doc, words, &PanelOptions::new("history-short-dock", 260.0, height)),
    );
}

/// The panel menu beside a real panel: its rows enable from what the panel stores, so jumping to
/// the oldest state or selecting a snapshot changes them, and a chosen row runs on the document.
fn show_menu(ui: &mut Ui, _args: &mut NoArgs) {
    with_doc(
        ui,
        "menu",
        (),
        |words| Doc::new(words.step_list(8, 4, false), words.snapshot_list(3)),
        |ui, doc, words| {
            let mut options = PanelOptions::new("history-menu", 260.0, 300.0);
            options.menu = true;
            show_panel(ui, doc, words, &options);
        },
    );
}

/// WordCraft's undo list: `steps` actions, `marked` of them marked, in a popup frame. Clicking a
/// row or pressing Enter names the count under it.
fn show_undo_list(ui: &mut Ui, args: &mut UndoArgs) {
    let words = HistoryStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let mut steps = StepList::new(HistoryStep::new(words.open.as_str()));
    steps.set_limit(args.steps.max(StepList::DEFAULT_LIMIT));
    for n in 1..=args.steps {
        let _ = steps.record(HistoryStep::new(craft_i18n::fmt(&words.step_n, &[("n", &numbers.count(n as u64))])));
    }
    let marked = args.marked.min(args.steps);
    let reported = story::shared(ui, "undo-event", || None::<String>);
    egui::Frame::popup(ui.style()).show(ui, |ui| {
        // The mark applies on a list's first show: a new mark is a new list.
        let list = UndoList::new(("history-undo-list", marked), &steps).marked(marked.checked_sub(1));
        if let Some(event) = list.show(ui).event {
            *story::lock(&reported) = Some(format!("{event:?}"));
        }
    });
    let story_words = StoryLabels::of(ui);
    let text = match &*story::lock(&reported) {
        Some(event) => craft_i18n::fmt(&story_words.last_event, &[("event", event)]),
        None => story_words.no_event.clone(),
    };
    ui.add(Text::new(text).content(Content::Muted));
}

/// The New Snapshot form with its suggested name and the three From options.
fn show_new_snapshot(ui: &mut Ui, args: &mut FormArgs) {
    let labels = SnapshotFormLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let words = HistoryStoryLabels::of(ui);
    let from = vec![
        ("fullDocument".to_owned(), words.from_document.clone()),
        ("mergedLayers".to_owned(), words.from_merged.clone()),
        ("currentLayer".to_owned(), words.from_layer.clone()),
    ];
    let name = SnapshotList::new().suggested_name(&labels, &numbers);
    let describe = describe_fields(ui);
    story::drive(ui, "new-snapshot", &mut args.open, (), || new_snapshot_form("story-new-snapshot", &labels, &name, from), |_| {}, describe);
}

/// The History Options form with the app's extra check box after Allow Non-Linear History.
fn show_options(ui: &mut Ui, args: &mut FormArgs) {
    let labels = HistoryOptionsLabels::of(ui);
    let words = HistoryStoryLabels::of(ui);
    let extra = vec![Param::new("visibilityUndoable", &words.visibility_undoable, ParamKind::Bool { default: false })];
    let describe = describe_fields(ui);
    story::drive(
        ui,
        "options",
        &mut args.open,
        (),
        || history_options_form("story-history-options", &labels, &HistoryOptions { non_linear: false }, extra),
        |_| {},
        describe,
    );
}

/// Words the fields a form returned: `key = value` pairs.
fn describe_fields(ui: &Ui) -> impl FnOnce(<CommandForm as StackDialog>::Result) -> String + use<> {
    let template = StoryLabels::of(ui).result_ok.clone();
    move |done| {
        let fields: Vec<String> = done.fields.iter().map(|(key, value)| format!("{key} = {value:?}")).collect();
        craft_i18n::fmt(&template, &[("value", &fields.join(", "))])
    }
}
