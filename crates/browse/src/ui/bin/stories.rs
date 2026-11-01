//! Stories of the media bin: `composites/media-bin/list`,
//! `composites/media-bin/icon`, `composites/media-bin/freeform`.
//!
//! The spec's ids `browse/bin-list`, `browse/bin-icon` and `browse/bin-freeform` become
//! `composites/media-bin/…`: the gallery requires a story id to start with its level.
//!
//! Each story keeps a [`Demo`] in egui memory: a small project tree ([`Project`], the bin's
//! `BinModel`), the bin's state, the selection, the Freeform cards and the last event. The story
//! applies the bin's events as an app would: view state through `BinState::apply`, and renames,
//! drops, reorders, removals, new items and bins, and Freeform moves to its own project. The three
//! stories differ in the view they open in and their args; the footer switches views in any of
//! them. Pictures come from a `FakeThumbSource` whose renders are jobs on a `ManualJobs`, one per
//! "Finish next". Nothing decodes, spawns a thread or polls on a timer.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use craft_host::ManualJobs;
use craft_theme::Rgba;
use craft_ui::behavior::{DropPosition, Selection};
use craft_ui::canvas::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::controls::{BadgeTone, Button};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{TableColumn, ThumbSize};
use craft_ui::localized::Labels as _;
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::typed_story;
use egui::{Ui, Vec2, vec2};

use super::{BinEntry, BinEvent, BinFooterAction, BinItem, BinModel, BinState, BinView, FreeformCard, MediaBin, OpenHow};
use crate::ui::grid::{BadgeKind, BadgeSlot, TileBadge, TileLabels, TileStatus};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Media bin"];

const COMPONENT: &str = "Media bin";

craft_ui::words! {
    /// The words the media bin stories add around the panel.
    pub(crate) struct BinStoryLabels {
        name: "binStory.name" = "Name";
        duration: "binStory.duration" = "Duration";
        new_item: "binStory.newItem" = "New Item";
        new_sequence: "binStory.newSequence" = "Sequence {n}";
        new_bin: "binStory.newBin" = "Bin {n}";
        linked: "binStory.linked" = "Linked";
        finish_next: "binStory.finishNext" = "Finish next";
        /// The line under a story; `{event}` is the last event the bin reported.
        last_event: "binStory.lastEvent" = "Last event: {event}";
        no_event: "binStory.noEvent" = "No event yet.";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/media-bin/list", Level::Composites, COMPONENT, "List", ListArgs { thumbnails: true }, show_list),
        typed_story!("composites/media-bin/icon", Level::Composites, COMPONENT, "Icon", IconArgs { hover_scrub: true }, show_icon),
        typed_story!("composites/media-bin/freeform", Level::Composites, COMPONENT, "Freeform", FreeformArgs { snap: true }, show_freeform),
    ]
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct ListArgs {
    /// List-view thumbnails (Small) beside the names.
    thumbnails: bool,
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct IconArgs {
    /// Scrubbing over a card reports the pointer's place in the clip.
    #[arg(name = "hover scrub")]
    hover_scrub: bool,
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct FreeformArgs {
    /// Cards snap to a 16-point grid while they are dragged.
    snap: bool,
}

/// The grid step of the Freeform story's snap, in points.
const SNAP_STEP: f32 = 16.0;

/// Room between Freeform cards when the story lays them out, in points.
const CELL: Vec2 = vec2(136.0, 104.0);

/// The height of the bin's box.
const HEIGHT: f32 = 440.0;

/// The widest the bin's box gets.
const WIDTH: f32 = 760.0;

/// One entry of the story's project.
#[derive(Clone)]
struct Node {
    key: u32,
    name: String,
    parent: Option<u32>,
    bin: bool,
    icon: Option<Icon>,
    duration: Option<&'static str>,
    status: TileStatus,
    linked: bool,
    range: Option<(f32, f32)>,
    label_color: Option<Rgba>,
}

impl Node {
    fn item(key: u32, name: &str, parent: Option<u32>, icon: Icon, duration: Option<&'static str>) -> Self {
        Self {
            key,
            name: name.to_owned(),
            parent,
            bin: false,
            icon: Some(icon),
            duration,
            status: TileStatus::Normal,
            linked: false,
            range: None,
            label_color: None,
        }
    }

    fn bin(key: u32, name: &str, parent: Option<u32>) -> Self {
        Self {
            key,
            name: name.to_owned(),
            parent,
            bin: true,
            icon: None,
            duration: None,
            status: TileStatus::Normal,
            linked: false,
            range: None,
            label_color: None,
        }
    }
}

/// The app's project: the `BinModel` the bin reads and the story edits.
struct Project {
    nodes: Vec<Node>,
    linked_tip: String,
}

impl Project {
    fn new(labels: &BinStoryLabels, tiles: &TileLabels) -> Self {
        let mut beach = Node::item(10, "Beach.mov", Some(1), Icon::Image, Some("00:12:04"));
        beach.range = Some((0.2, 0.7));
        beach.linked = true;
        let mut clip = Node::item(11, "Clip.mov", Some(1), Icon::Image, Some("00:03:10"));
        clip.label_color = Some(Rgba::rgb(0xd0, 0x6a, 0x3a));
        let mut wide = Node::item(13, "Wide shot.mov", Some(1), Icon::Image, Some("00:01:45"));
        wide.status = TileStatus::Offline(tiles.offline.clone());
        let mut drone = Node::item(14, "Drone.mov", Some(1), Icon::Image, Some("00:08:20"));
        drone.status = TileStatus::Failed;
        Self {
            nodes: vec![
                Node::bin(1, "Footage", None),
                Node::bin(2, "Archive", None),
                Node::bin(3, "Interviews", Some(1)),
                beach,
                clip,
                Node::item(12, "Narration.wav", Some(1), Icon::File, Some("00:45:00")),
                wide,
                drone,
                Node::item(15, "Host.wav", Some(3), Icon::File, Some("00:22:10")),
                Node::item(20, "Old beach.mov", Some(2), Icon::Image, Some("00:09:30")),
                Node::item(30, "Title.psd", None, Icon::Image, None),
                Node::item(31, "Rough cut", None, Icon::Layers, Some("00:04:00")),
            ],
            linked_tip: labels.linked.clone(),
        }
    }

    fn node(&self, key: u32) -> Option<&Node> {
        self.nodes.iter().find(|n| n.key == key)
    }

    fn next_key(&self) -> u32 {
        self.nodes.iter().map(|n| n.key).max().unwrap_or(0) + 1
    }

    /// Whether `key` is `ancestor` or inside it.
    fn within(&self, key: u32, ancestor: u32) -> bool {
        let mut at = Some(key);
        for _ in 0..64 {
            match at {
                Some(k) if k == ancestor => return true,
                Some(k) => at = self.node(k).and_then(|n| n.parent),
                None => return false,
            }
        }
        false
    }

    /// Takes `keys` and everything inside them out of the project.
    fn remove(&mut self, keys: &[u32]) {
        let gone: Vec<u32> = self.nodes.iter().filter(|n| keys.iter().any(|k| self.within(n.key, *k))).map(|n| n.key).collect();
        self.nodes.retain(|n| !gone.contains(&n.key));
    }

    /// Moves `keys` into `bin` (`None`: the root), unless that would put a bin inside itself.
    fn reparent(&mut self, keys: &[u32], bin: Option<u32>) {
        for &key in keys {
            if bin.is_some_and(|b| self.within(b, key)) {
                continue;
            }
            if let Some(node) = self.nodes.iter_mut().find(|n| n.key == key) {
                node.parent = bin;
            }
        }
    }

    /// Puts `items` before or after `target` in the model's order (the bin's User Order), or into
    /// `target` when it is a bin and the drop is `Into`.
    fn reorder(&mut self, items: &[u32], target: u32, position: DropPosition) {
        if position == DropPosition::Into {
            self.reparent(items, Some(target));
            return;
        }
        let parent = self.node(target).and_then(|n| n.parent);
        let moved: Vec<Node> = items.iter().filter(|k| **k != target).filter_map(|k| self.node(*k).cloned()).collect();
        self.nodes.retain(|n| !moved.iter().any(|m| m.key == n.key));
        let Some(at) = self.nodes.iter().position(|n| n.key == target) else { return };
        let at = if position == DropPosition::After { at + 1 } else { at };
        for (offset, mut node) in moved.into_iter().enumerate() {
            node.parent = parent;
            self.nodes.insert(at + offset, node);
        }
    }
}

impl BinModel<u32> for Project {
    fn children(&self, parent: Option<&u32>, out: &mut Vec<BinEntry<u32>>) {
        for node in &self.nodes {
            if node.parent == parent.copied() {
                out.push(if node.bin { BinEntry::Bin(node.key) } else { BinEntry::Item(node.key) });
            }
        }
    }

    fn item(&self, key: &u32, item: &mut BinItem) {
        let Some(node) = self.node(*key) else { return };
        item.name.clone_from(&node.name);
        item.icon = node.icon;
        item.label_color = node.label_color;
        item.status = node.status.clone();
        item.duration = node.duration.map(str::to_owned);
        item.range = node.range;
        if node.linked {
            item.badges.push(TileBadge {
                slot: BadgeSlot::EndTop,
                kind: BadgeKind::Glyph(Icon::Link),
                tone: BadgeTone::Accent,
                tip: self.linked_tip.clone(),
            });
        }
    }

    fn cell(&self, key: &u32, column: &str) -> String {
        match (column, self.node(*key)) {
            ("duration", Some(node)) => node.duration.unwrap_or_default().to_owned(),
            _ => String::new(),
        }
    }

    fn is_bin(&self, key: &u32) -> bool {
        self.node(*key).is_some_and(|n| n.bin)
    }

    fn parent(&self, key: &u32) -> Option<u32> {
        self.node(*key).and_then(|n| n.parent)
    }

    fn count(&self, bin: &u32) -> usize {
        self.nodes.iter().filter(|n| n.parent == Some(*bin)).map(|n| if n.bin { self.count(&n.key) } else { 1 }).sum()
    }
}

/// What a story keeps between passes.
struct Demo {
    project: Project,
    state: BinState<u32>,
    selection: Selection<u32>,
    columns: Vec<TableColumn<String>>,
    actions: Vec<BinFooterAction>,
    cards: Vec<FreeformCard<u32>>,
    jobs: Arc<ManualJobs>,
    last_event: Option<String>,
}

impl Demo {
    fn new(view: BinView, labels: &BinStoryLabels, tiles: &TileLabels) -> Self {
        let project = Project::new(labels, tiles);
        let mut state = BinState::default();
        state.settings.view = view;
        state.expanded.insert(1);
        let mut demo = Self {
            project,
            state,
            selection: Selection::new(),
            columns: vec![
                TableColumn::new("name".to_owned(), labels.name.clone()),
                TableColumn::new("duration".to_owned(), labels.duration.clone()).sortable(true),
            ],
            actions: vec![BinFooterAction::new("newItem", Icon::Plus, labels.new_item.clone()).accepts_drops(true)],
            cards: Vec::new(),
            jobs: Arc::new(ManualJobs::new()),
            last_event: None,
        };
        demo.lay_out();
        demo
    }

    /// Cards for every node, in rows of four within each bin; "Narration" and "Host" share a stack.
    fn lay_out(&mut self) {
        let mut parents: Vec<Option<u32>> = self.project.nodes.iter().map(|n| n.parent).collect();
        parents.dedup();
        for parent in parents {
            let keys: Vec<u32> = self.project.nodes.iter().filter(|n| n.parent == parent).map(|n| n.key).collect();
            for (i, key) in keys.into_iter().enumerate() {
                if self.cards.iter().any(|c| c.key == key) {
                    continue;
                }
                let pos = vec2((i % 4) as f32 * CELL.x + 16.0, (i / 4) as f32 * CELL.y + 16.0);
                let stack = matches!(key, 11 | 12).then_some(1);
                let pos = if key == 12 { vec2(pos.x - CELL.x, pos.y) } else { pos };
                self.cards.push(FreeformCard { key, pos, stack });
            }
        }
    }

    /// Applies one event as an app would.
    fn apply(&mut self, event: &BinEvent<u32>, labels: &BinStoryLabels) {
        self.state.apply(event);
        match event {
            BinEvent::Rename { key, name } => {
                if let Some(node) = self.project.nodes.iter_mut().find(|n| n.key == *key) {
                    node.name.clone_from(name);
                }
            }
            BinEvent::Drop { items, into } => {
                let into = (*into).or(self.state.shown);
                self.project.reparent(items, into);
            }
            BinEvent::Reorder(EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => {
                self.project.reorder(&value.items, value.target, value.position);
            }
            BinEvent::Remove(keys) => {
                self.project.remove(keys);
                self.cards.retain(|c| self.project.node(c.key).is_some());
                self.selection = Selection::new();
            }
            BinEvent::Footer("newItem") => self.add_sequence(labels),
            BinEvent::FooterDrop { .. } => self.add_sequence(labels),
            BinEvent::NewBin => {
                let key = self.project.next_key();
                let name = craft_i18n::fmt(&labels.new_bin, &[("n", &key.to_string())]);
                self.project.nodes.push(Node::bin(key, &name, self.state.shown));
                self.lay_out();
            }
            BinEvent::FreeformMoved { keys, to } => {
                for (key, pos) in keys.iter().zip(to) {
                    if let Some(card) = self.cards.iter_mut().find(|c| c.key == *key) {
                        card.pos = *pos;
                    }
                }
            }
            _ => {}
        }
        self.last_event = Some(describe(event));
    }

    fn add_sequence(&mut self, labels: &BinStoryLabels) {
        let key = self.project.next_key();
        let name = craft_i18n::fmt(&labels.new_sequence, &[("n", &key.to_string())]);
        self.project.nodes.push(Node::item(key, &name, self.state.shown, Icon::Layers, None));
        self.lay_out();
    }
}

/// The event's name and payload, for the line under the bin.
fn describe(event: &BinEvent<u32>) -> String {
    match event {
        BinEvent::OpenBin { key, how } => format!(
            "OpenBin {{ key: {key}, how: {} }}",
            match how {
                OpenHow::InPlace => "InPlace",
                OpenHow::NewTab => "NewTab",
                OpenHow::NewWindow => "NewWindow",
            }
        ),
        other => format!("{other:?}"),
    }
}

fn shared(ui: &Ui, name: &str, init: impl FnOnce() -> Demo) -> Arc<Mutex<Demo>> {
    let key = ui.make_persistent_id(("craft_browse::bin::stories", name));
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, || Arc::new(Mutex::new(init()))).clone())
}

fn lock(demo: &Mutex<Demo>) -> MutexGuard<'_, Demo> {
    demo.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Draws the bin for the story `name`, applies its events and shows the last one.
fn show_bin(ui: &mut Ui, name: &str, view: BinView, configure: impl FnOnce(&mut Demo) -> (bool, Option<f32>)) {
    let labels = BinStoryLabels::of(ui);
    let tiles = TileLabels::of(ui);
    let demo = shared(ui, name, || Demo::new(view, &labels, &tiles));
    let mut demo = lock(&demo);
    let (hover_scrub, snap) = configure(&mut demo);

    let options = ThumbnailOptions { lanes: 1, jobs: Some(demo.jobs.clone()), ..ThumbnailOptions::default() };
    let thumbs = Thumbnails::<u32>::in_context(ui.ctx(), ("craft_browse::bin::stories", name), options);
    let source = FakeThumbSource::<u32>::new().aspect(16, 9).later(true);

    if ui.add(Button::new(labels.finish_next.clone()).enabled(demo.jobs.waiting() > 0)).clicked() {
        demo.jobs.run_next();
    }
    let width = ui.available_width().min(WIDTH);
    let events = ui
        .allocate_ui(vec2(width, HEIGHT), |ui| {
            let Demo { project, state, selection, columns, actions, cards, .. } = &mut *demo;
            MediaBin::new(name, state, &*project)
                .selection(selection)
                .columns(columns)
                .footer_actions(actions)
                .thumbnails(&thumbs, &source)
                .hover_scrub(hover_scrub)
                .freeform(cards)
                .snap(snap)
                .show(ui)
                .events
        })
        .inner;
    for event in &events {
        demo.apply(event, &labels);
    }
    let line = match &demo.last_event {
        Some(event) => craft_i18n::fmt(&labels.last_event, &[("event", event.as_str())]),
        None => labels.no_event.clone(),
    };
    ui.label(line);
}

fn show_list(ui: &mut Ui, args: &mut ListArgs) {
    show_bin(ui, "list", BinView::List, |demo| {
        demo.state.settings.thumbnails = if args.thumbnails { ThumbSize::Small } else { ThumbSize::None };
        (false, None)
    });
}

fn show_icon(ui: &mut Ui, args: &mut IconArgs) {
    show_bin(ui, "icon", BinView::Icon, |_| (args.hover_scrub, None));
}

fn show_freeform(ui: &mut Ui, args: &mut FreeformArgs) {
    show_bin(ui, "freeform", BinView::Freeform, |_| (false, args.snap.then_some(SNAP_STEP)));
}
