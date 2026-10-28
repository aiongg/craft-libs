//! A [`List`](super::List)'s row drag: what it keeps between frames, the ghost that follows the
//! pointer, and the reorder stories.

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;

use egui::mutex::Mutex;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Id, LayerId, Order, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

use super::row::{ListRow, ListRowStyle, RowEvent};
use super::toggle::ToggleColumn;
use super::view::{List, ListEvent};
use crate::behavior::{Axis, DisplayOrder, DropPosition, ListDrag, Move, Reorder, ReorderGesture, RowInfo, Selection};
use crate::controls::button::Button;
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf as _, corner_radius, role_format};

/// A drag of rows under way.
#[derive(Clone)]
pub(super) struct Dragging<K> {
    pub(super) gesture: ReorderGesture<K>,
    /// The rows dragged, in display order.
    pub(super) items: Vec<K>,
    /// The row the pointer grabbed, kept drawn while the drag lasts.
    pub(super) grabbed: K,
    pub(super) label: String,
    /// The pass the list last drew in.
    pub(super) pass: u64,
}

/// Whether every key of `items` is a row of `rows`.
pub(super) fn all_shown<K: Eq + Hash>(rows: &(impl DisplayOrder<K> + ?Sized), items: &[K]) -> bool {
    let mut missing: HashSet<&K> = items.iter().collect();
    for at in 0..rows.len() {
        if missing.is_empty() {
            break;
        }
        missing.remove(rows.key(at));
    }
    missing.is_empty()
}

/// Where the ghost's box starts from the pointer (PhotoCraft), toward the reading end.
const GHOST_OFFSET: Vec2 = Vec2::new(12.0, -10.0);

/// The ghost: the grabbed row's label in a box beside the pointer on the tooltip layer, cut to
/// `max_width`.
pub(super) fn paint_ghost(ui: &Ui, list: Id, style: &ListRowStyle, label: &str, pointer: Pos2, max_width: f32) {
    let tokens = Tokens::of(ui);
    let pad = vec2(style.pad_x, style.gap);
    let mut job = LayoutJob::single_section(label.to_owned(), role_format(&tokens, style.label_role, style.text));
    job.wrap = TextWrapping::truncate_at_width((max_width - 2.0 * pad.x).max(style.min_label));
    let painter = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, list.with("drag-ghost")));
    let galley = painter.layout_job(job);
    let size = galley.size() + 2.0 * pad;
    let left = if direction::is_rtl(ui) { pointer.x - GHOST_OFFSET.x - size.x } else { pointer.x + GHOST_OFFSET.x };
    let rect = Rect::from_min_size(pos2(left, pointer.y + GHOST_OFFSET.y), size);
    painter.rect(rect, corner_radius(style.ghost_radius), style.ghost_fill, Stroke::new(1.0, style.ghost_border), StrokeKind::Inside);
    painter.galley(rect.min + pad, galley, style.text);
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story { id: "composites/list/reorder", level: Level::Composites, component: "List", name: "Reorder", args: Args::new, show: show_flat },
        Story {
            id: "composites/list/reorder-tree",
            level: Level::Composites,
            component: "List",
            name: "Reorder a tree",
            args: Args::new,
            show: show_tree,
        },
        Story {
            id: "composites/list/reorder-refused",
            level: Level::Composites,
            component: "List",
            name: "Refused targets",
            args: Args::new,
            show: show_refused,
        },
        Story {
            id: "composites/list/reorder-copy",
            level: Level::Composites,
            component: "List",
            name: "Copy with Alt",
            args: Args::new,
            show: show_copy,
        },
    ]
}

/// Drag a layer or the selection; the edges scroll; Delete takes a dragged layer. Cmd+] and
/// Cmd+[ (Ctrl off macOS) move the selection, Shift to the ends.
pub(super) fn show_flat(ui: &mut Ui, _: &mut Args) {
    show_story(ui, Preset::Flat);
}

/// Groups take drops into their middle; double-click or Enter opens and closes a group.
pub(super) fn show_tree(ui: &mut Ui, _: &mut Args) {
    show_story(ui, Preset::Tree);
}

/// Background stays at the bottom; locked layers take no drops beside or into them unless
/// Cmd/Ctrl is held.
pub(super) fn show_refused(ui: &mut Ui, _: &mut Args) {
    show_story(ui, Preset::Refused);
}

/// Alt held while dropping copies the layers, also onto their own edges.
pub(super) fn show_copy(ui: &mut Ui, _: &mut Args) {
    show_story(ui, Preset::Copy);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Preset {
    Flat,
    Tree,
    Refused,
    Copy,
}

/// A layer of a story's document, in the document's order: a group directly above its children.
#[derive(Clone)]
struct Layer {
    key: usize,
    name: String,
    depth: usize,
    group: bool,
    open: bool,
    locked: bool,
}

/// A story's document, selection and last event.
struct Layers {
    layers: Vec<Layer>,
    next: usize,
    background: Option<usize>,
    selection: Selection<usize>,
    last: String,
}

impl Layers {
    fn new(preset: Preset) -> Self {
        let flat: Vec<(String, usize, bool)> = (1..=24).map(|n| (format!("Layer {n}"), 0, false)).collect();
        let tree = [
            ("Title", 0, false),
            ("Header", 0, true),
            ("Logo", 1, false),
            ("Tagline", 1, false),
            ("Photos", 0, true),
            ("Beach", 1, false),
            ("Archive", 1, true),
            ("Harbour", 2, false),
            ("Market", 2, false),
            ("Caption", 0, false),
        ];
        let rows = match preset {
            Preset::Flat => flat,
            Preset::Copy => flat.into_iter().take(8).collect(),
            Preset::Tree => tree.iter().map(|&(name, depth, group)| (name.to_owned(), depth, group)).collect(),
            Preset::Refused => ["Text", "Frame", "Photo", "Badge", "Shape", "Background"].iter().map(|n| ((*n).to_owned(), 0, false)).collect(),
        };
        let layers: Vec<Layer> = rows
            .into_iter()
            .enumerate()
            .map(|(key, (name, depth, group))| Layer {
                locked: preset == Preset::Refused && matches!(name.as_str(), "Frame" | "Badge"),
                open: name != "Archive",
                key,
                name,
                depth,
                group,
            })
            .collect();
        let background = (preset == Preset::Refused).then(|| layers.len() - 1);
        let last = match preset {
            Preset::Flat => "Drag a layer, or press Cmd+] or Cmd+[",
            Preset::Tree => "Drop into the middle of a group",
            Preset::Refused => "Background and locked layers refuse drops",
            Preset::Copy => "Hold Alt while dropping to copy",
        };
        Layers { next: layers.len(), layers, background, selection: Selection::only(0), last: last.to_owned() }
    }

    fn at(&self, key: usize) -> Option<usize> {
        self.layers.iter().position(|l| l.key == key)
    }

    fn name(&self, key: usize) -> &str {
        self.at(key).map_or("?", |at| self.layers[at].name.as_str())
    }

    /// The layer at `at` and the layers inside it.
    fn block(&self, at: usize) -> std::ops::Range<usize> {
        let depth = self.layers[at].depth;
        let end = self.layers[at + 1..].iter().position(|l| l.depth <= depth).map_or(self.layers.len(), |n| at + 1 + n);
        at..end
    }

    /// The layers shown: closed groups' children left out.
    fn display(&self) -> Vec<RowInfo<usize>> {
        let mut closed_at: Option<usize> = None;
        let mut rows = Vec::new();
        for layer in &self.layers {
            if closed_at.is_some_and(|depth| layer.depth > depth) {
                continue;
            }
            closed_at = (layer.group && !layer.open).then_some(layer.depth);
            let row = if layer.group { RowInfo::group(layer.key, layer.open) } else { RowInfo::leaf(layer.key) };
            rows.push(row.at_depth(layer.depth));
        }
        rows
    }

    /// The blocks of `keys`, without those inside another one, in document order.
    fn blocks(&self, keys: &[usize]) -> Vec<std::ops::Range<usize>> {
        let mut blocks: Vec<std::ops::Range<usize>> = Vec::new();
        let mut starts: Vec<usize> = keys.iter().filter_map(|&k| self.at(k)).collect();
        starts.sort_unstable();
        for at in starts {
            if !blocks.iter().any(|b| b.contains(&at)) {
                blocks.push(self.block(at));
            }
        }
        blocks
    }

    /// Removes the layers of `keys` with their contents, and returns them as blocks in document
    /// order.
    fn take(&mut self, keys: &[usize]) -> Vec<Vec<Layer>> {
        let mut taken: Vec<Vec<Layer>> = self.blocks(keys).into_iter().rev().map(|block| self.layers.drain(block).collect()).collect();
        taken.reverse();
        taken
    }

    fn apply(&mut self, mv: &Move<usize>) {
        let moved: Vec<Vec<Layer>> = if mv.copy {
            let blocks: Vec<Vec<Layer>> = self.blocks(&mv.items).into_iter().map(|b| self.layers[b].to_vec()).collect();
            blocks
                .into_iter()
                .map(|block| {
                    block
                        .into_iter()
                        .map(|mut layer| {
                            layer.key = self.next;
                            self.next += 1;
                            layer.name.push_str(" copy");
                            layer
                        })
                        .collect()
                })
                .collect()
        } else {
            self.take(&mv.items)
        };
        let Some(target) = self.at(mv.target) else { return };
        let (insert, depth) = match mv.position {
            DropPosition::Before => (target, self.layers[target].depth),
            DropPosition::After => (self.block(target).end, self.layers[target].depth),
            DropPosition::Into => {
                self.layers[target].open = true;
                (target + 1, self.layers[target].depth + 1)
            }
        };
        // Each block's top layer lands at `depth`; the layers inside it keep their depth below it.
        let moved = moved.into_iter().flat_map(|block| {
            let top = block.first().map_or(0, |l| l.depth);
            block.into_iter().map(move |mut layer| {
                layer.depth = layer.depth - top + depth;
                layer
            })
        });
        self.layers.splice(insert..insert, moved.collect::<Vec<_>>());
    }

    fn describe(&self, mv: &Move<usize>) -> String {
        let names: Vec<&str> = mv.items.iter().map(|&k| self.name(k)).collect();
        let position = match mv.position {
            DropPosition::Before => "before",
            DropPosition::After => "after",
            DropPosition::Into => "into",
        };
        let verb = if mv.copy { "copy" } else { "move" };
        format!("{verb} {} {position} {}", names.join(", "), self.name(mv.target))
    }

    /// The rules of the refused-targets story: Background doesn't move and nothing goes below
    /// it (PhotoCraft); a locked layer takes no drop beside or into it unless Cmd/Ctrl is held
    /// (DesignCraft).
    fn rules(&self) -> impl Fn(&Move<usize>, egui::Modifiers) -> bool + 'static {
        let background = self.background;
        let locked: HashSet<usize> = self.layers.iter().filter(|l| l.locked).map(|l| l.key).collect();
        move |mv, modifiers| {
            let background_ok = background.is_none_or(|bg| !mv.items.contains(&bg) && !(mv.target == bg && mv.position != DropPosition::Before));
            background_ok && (modifiers.command || !locked.contains(&mv.target))
        }
    }
}

fn show_story(ui: &mut Ui, preset: Preset) {
    let state_id = ui.scope_id().with(("reorder-story", preset));
    let shared = ui.data_mut(|d| d.get_temp_mut_or_insert_with(state_id, || Arc::new(Mutex::new(Layers::new(preset)))).clone());
    let mut doc = shared.lock();
    let order = doc.display();
    let indent = ListRow::<'_, usize>::style(&Tokens::of(ui)).indent;
    let mut reorder = Reorder::new(Axis::Vertical).copy_with_alt(preset == Preset::Copy);
    if preset == Preset::Refused {
        reorder = reorder.accepts(doc.rules());
    }
    let mut selection = doc.selection.clone();
    let events = ui
        .allocate_ui(vec2(300.0, 240.0), |ui| {
            let out =
                List::new("layers", "Layers", order.as_slice()).selection(&mut selection).reorder(reorder).max_height(240.0).show(ui, |_, &key| {
                    let layer = doc.at(key).map(|at| &doc.layers[at]);
                    let (name, depth, group, locked) = layer.map_or(("", 0, false, false), |l| (l.name.as_str(), l.depth, l.group, l.locked));
                    let mut row = ListRow::new(key, name).indent(depth as f32 * indent);
                    if group {
                        row = row.icon(Icon::FolderOpen);
                    }
                    if preset == Preset::Refused {
                        row = row.toggle(ToggleColumn::lock("Lock", locked));
                    }
                    row
                });
            out.events
        })
        .inner;
    doc.selection = selection;
    for event in events {
        match event {
            ListEvent::Reorder(EditEvent::Commit { value: Some(mv), .. } | EditEvent::Set { value: Some(mv), .. }) => {
                doc.last = format!("Did: {}", doc.describe(&mv));
                doc.apply(&mv);
            }
            ListEvent::Reorder(EditEvent::Update { value, .. }) => {
                doc.last = value.map_or_else(|| "No drop here".to_owned(), |mv| format!("Drop to {}", doc.describe(&mv)));
            }
            ListEvent::Reorder(EditEvent::Cancel { reason, .. }) => doc.last = format!("Drag cancelled: {reason:?}"),
            ListEvent::Row(RowEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Set { value, .. })) => {
                if let Some(at) = doc.at(value.row) {
                    doc.layers[at].locked = value.on;
                }
            }
            ListEvent::Activate(key) => {
                if let Some(layer) = doc.at(key).map(|at| &mut doc.layers[at]).filter(|l| l.group) {
                    layer.open = !layer.open;
                }
            }
            _ => {}
        }
    }
    if preset == Preset::Flat {
        let response = ui.add(Button::new("Delete").icon(Icon::Trash));
        if let Some(drag) = response.dnd_release_payload::<ListDrag<usize>>() {
            doc.take(&drag.items);
            doc.selection = Selection::default();
            doc.last = format!("Deleted {} layers", drag.items.len());
        }
    }
    ui.label(doc.last.as_str());
}
