//! Stories of the row types of the list family: `ListRow`.

use super::{List, ListRow, OpenSet, RowEvent, RowInk, ThumbSize, ThumbTarget, ToggleCell, TrailingFace, TrailingPart};
use crate::contract::{Bind, CollectionEvent, ColumnKind, ColumnSpec, Show as _, ShowCollection as _, ToggleColumn, ToggleEdit};
use crate::primitives::{ContentPainterCx, IconSpec};
use crate::runtime::env::Label;
use crate::style::{Corners, SurfaceKind};
use crate::testing::{Drive, StoryData, StoryOutput, show_as_only_item, story};
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, IconId, ItemSource, Selection};
use egui::{Pos2, Rect, Ui};
use std::borrow::Cow;

const EYE: ColumnKey = ColumnKey::new("eye");
const LOCK: ColumnKey = ColumnKey::new("lock");
const NAME: ColumnKey = ColumnKey::new("name");

/// A layer of the layers story.
#[derive(Clone, Debug, PartialEq)]
struct Layer {
    name: String,
    visible: bool,
    locked: bool,
}

/// The layers story's layers, top first: the Background locked.
fn layers() -> Vec<Layer> {
    let layer = |name: &str, locked| Layer { name: name.to_owned(), visible: true, locked };
    vec![
        layer("Headline", false),
        layer("Photo with a name too long for the panel to show whole", false),
        layer("Shadow", false),
        layer("Background", true),
    ]
}

/// The layers story's rows, keyed by position.
struct Layers<'a>(&'a [Layer]);

impl DisplayOrder<usize> for Layers<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        self.0.get(at).map(|layer| Cow::Borrowed(layer.name.as_str()))
    }
}

impl ItemSource<usize> for Layers<'_> {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        self.0.get(at).is_some_and(|layer| if *column == EYE { layer.visible } else { layer.locked })
    }
}

/// Sets the toggle `cell` lands on in `layers`.
fn set_toggle(layers: &mut [Layer], cell: &ToggleEdit<usize>) {
    if let Some(layer) = layers.get_mut(cell.key) {
        if cell.column == EYE {
            layer.visible = cell.on;
        } else {
            layer.locked = cell.on;
        }
    }
}

/// A layers panel: eye and lock columns, the blend mode at the trailing end (the first to go when narrow), a hidden
/// layer dimmed, and rename by double click on the label. Drag down a column to sweep it.
#[story(ListRow<'static>, covers = [], drive = Drive::Never, group = "Layers", title = "Layers")]
fn list_row_layers(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let columns = vec![
        ColumnSpec::new(EYE, Label::Empty, ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())).icon(IconId::new("eye")),
        ColumnSpec::new(LOCK, Label::Empty, ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())).icon(IconId::new("lock")),
        ColumnSpec::new(NAME, Label::Empty, ColumnWidth::fill(1.0)),
    ];
    let rows = data.value("layers", layers()).clone();
    let out = List::<usize>::new("Layers")
        .header(false)
        .natural_height(true)
        .rename(true)
        .rename_column(NAME)
        .columns(columns)
        .selection(Bind::bound(data.value("selection", Selection::single(0_usize))))
        .show_items(ui, &Layers(&rows), &mut |key, icx| {
            let layer = &rows[*key];
            let row = ListRow::new(layer.name.clone())
                .layer(true)
                .cells(icx.cells())
                .state(icx.row.clone())
                .toggle(ToggleCell::eye(EYE, layer.visible))
                .toggle(ToggleCell::lock(LOCK, layer.locked))
                .optional("Normal");
            if layer.visible { row } else { row.ink(RowInk::Dimmed) }
        });
    let kept = data.value("layers", layers());
    for event in &out.events {
        match event {
            CollectionEvent::Toggle(edit) => {
                if let Some(cell) = edit.landed() {
                    set_toggle(kept, cell);
                }
            }
            CollectionEvent::Rename(edit) => {
                if let Some(rename) = edit.value()
                    && !rename.name.trim().is_empty()
                    && let Some(layer) = kept.get_mut(rename.key)
                {
                    layer.name = rename.name.trim().to_owned();
                }
            }
            _ => {}
        }
    }
    out.into()
}

/// The rows of the mask thumbnail story.
const MASK_ROWS: [&str; 3] = ["Layer with masks", "Clipped layer", "Background"];

/// Rows named by fixed labels, keyed by position.
struct Labels(&'static [&'static str]);

impl DisplayOrder<usize> for Labels {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        self.0.get(at).map(|label| Cow::Borrowed(*label))
    }
}

impl ItemSource<usize> for Labels {}

fn layer_thumb(cx: &ContentPainterCx, rect: Rect) {
    cx.surface(rect, SurfaceKind::Raised, Corners::None);
}

fn mask_thumb(cx: &ContentPainterCx, rect: Rect) {
    cx.painter.rect_filled(rect, 0.0, cx.ink_on(SurfaceKind::Raised));
}

/// A layer with a layer mask and a vector mask, each behind its own link glyph, a clipped layer and an italic
/// Background. A click on a thumbnail moves the target brackets to it; a click on a link glyph flips it.
#[story(ListRow<'static>, covers = [], drive = Drive::Never, group = "Mask thumbnail", title = "Mask thumbnail")]
fn list_row_masks(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let linked = *data.value("linked", [true; 2]);
    let target = *data.value("target", ThumbTarget::Extra(0));
    let out = List::<usize>::new("Layers")
        .natural_height(true)
        .selection(Bind::bound(data.value("selection", Selection::single(0_usize))))
        .show_items(ui, &Labels(&MASK_ROWS), &mut |key, icx| {
            let row = ListRow::new(MASK_ROWS[*key]).state(icx.row.clone()).thumb(ThumbSize::Medium, layer_thumb);
            match *key {
                0 => row
                    .extra_thumb(mask_thumb)
                    .thumb_link(linked[0], "Link the layer mask to {row}")
                    .extra_thumb(mask_thumb)
                    .thumb_link(linked[1], "Link the vector mask to {row}")
                    .thumb_target(target),
                1 => row.clipped(true),
                _ => row.italic(true),
            }
        });
    for event in &out.events {
        match event {
            CollectionEvent::Item(0, RowEvent::LinkToggled { thumb, linked: on, .. }) => {
                data.value("linked", [true; 2])[(*thumb).min(1)] = *on;
            }
            CollectionEvent::Item(0, RowEvent::ThumbClicked { target: picked, .. }) => *data.value("target", ThumbTarget::Extra(0)) = *picked,
            _ => {}
        }
    }
    out.into()
}

/// The layer tree of the tree story: each node's name and depth, in display order.
const TREE: [(&str, usize); 15] = [
    ("Poster", 0),
    ("Background", 1),
    ("Photos", 1),
    ("Hero photograph with a name too long for a narrow panel", 2),
    ("Detail", 2),
    ("Frames", 2),
    ("Thin frame", 3),
    ("Wide frame", 3),
    ("Text", 1),
    ("Headline", 2),
    ("Body", 2),
    ("Guides", 0),
    ("Margins", 1),
    ("Columns", 1),
    ("Notes", 0),
];

/// Whether the node at `at` of [`TREE`] has children: the next node is deeper.
fn is_group(at: usize) -> bool {
    TREE.get(at + 1).is_some_and(|next| next.1 > TREE[at].1)
}

/// The groups the tree story opens with: Poster and Photos.
fn first_open() -> OpenSet<usize> {
    let mut open = OpenSet::new();
    open.set(&0, true);
    open.set(&2, true);
    open
}

/// The nodes of [`TREE`] that show, in order: every node whose groups above it are all open.
fn shown_nodes(open: &OpenSet<usize>) -> Vec<usize> {
    let mut shown = Vec::new();
    let mut closed_at: Option<usize> = None;
    for (at, &(_, depth)) in TREE.iter().enumerate() {
        if closed_at.is_some_and(|closed| depth > closed) {
            continue;
        }
        closed_at = (is_group(at) && !open.is_open(&at)).then_some(depth);
        shown.push(at);
    }
    shown
}

/// The shown nodes of [`TREE`] as the display order of a tree list, keyed by their place in the tree.
struct TreeNodes<'a> {
    shown: &'a [usize],
    open: &'a OpenSet<usize>,
}

impl DisplayOrder<usize> for TreeNodes<'_> {
    fn len(&self) -> usize {
        self.shown.len()
    }

    fn key(&self, at: usize) -> usize {
        self.shown[at]
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        self.shown.iter().position(|node| node == key)
    }

    fn depth(&self, at: usize) -> usize {
        TREE[self.shown[at]].1
    }

    fn has_children(&self, at: usize) -> bool {
        is_group(self.shown[at])
    }

    fn open(&self, at: usize) -> bool {
        self.open.is_open(&self.shown[at])
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(TREE[self.shown[at]].0))
    }
}

impl ItemSource<usize> for TreeNodes<'_> {}

#[story(ListRow<'static>, covers = [Default], drive = Drive::Activate)]
fn list_row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Background").show(ui).into()
}

#[story(ListRow<'static>, covers = [LongText], drive = Drive::Never)]
fn list_row_long_text(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let label = "A layer whose name is far longer than the room its row gives the label";
    ListRow::new(label).detail("A second line under it").show(ui).into()
}

/// A layers tree: Up and Down move through the rows shown, Right opens a closed group or enters an open one, Left
/// closes an open group or leaves it, and Alt with either, or Alt-click on a triangle, opens or closes every group.
#[story(ListRow<'static>, covers = [], drive = Drive::Never, title = "Tree group")]
fn list_row_tree_group(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let open = data.value("open", first_open()).clone();
    let shown = shown_nodes(&open);
    let out = List::<usize>::new("Layers")
        .tree(true)
        .natural_height(true)
        .selection(Bind::bound(data.value("selection", Selection::single(0_usize))))
        .show_items(ui, &TreeNodes { shown: &shown, open: &open }, &mut |key, _| {
            let (name, depth) = TREE[*key];
            let row = ListRow::new(name).depth(depth);
            if is_group(*key) {
                row.group(open.is_open(key)).icon(IconSpec::svg(IconId::new("layers")))
            } else {
                row.icon(IconSpec::svg(IconId::new("check")))
            }
        });
    let groups: Vec<usize> = (0..TREE.len()).filter(|&at| is_group(at)).collect();
    for event in &out.events {
        if let CollectionEvent::Open(change) = event {
            data.value("open", first_open()).apply(&change.key, change.open, change.all, &groups);
        }
    }
    out.into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_clipped_dimmed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Shadow").clipped(true).ink(RowInk::Dimmed).italic(true).lines("Clipped to the layer below, hidden").show(ui).into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_text_cell(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    const NAME: ColumnKey = ColumnKey::new("name");
    const SIZE: ColumnKey = ColumnKey::new("size");
    let (left, width, height) = (ui.cursor().min.x, ui.available_width(), ui.spacing().interact_size.y);
    let split = left + width * 0.6;
    let cells = [
        (NAME, Rect::from_min_max(Pos2::new(left, 0.0), Pos2::new(split, height))),
        (SIZE, Rect::from_min_max(Pos2::new(split, 0.0), Pos2::new(left + width, height))),
    ];
    show_as_only_item(ui, ListRow::new("report.pdf").cells(&cells).cell(SIZE, "2.4 MB")).into()
}

fn trailing_row() -> ListRow<'static> {
    ListRow::new("Background copy")
        .trailing("100%")
        .trailing_icon(IconId::new("lock"), "Locked")
        .trailing_part(TrailingPart::new(0), TrailingFace::disclosure(false), "Effects")
        .optional("Multiply")
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_trailing_items(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    show_as_only_item(ui, trailing_row()).into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_trailing_items_narrow(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ui.set_max_width(ui.available_width() * 0.3);
    show_as_only_item(ui, trailing_row()).into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_clamped_description_with_rail(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let text = "Retouched the sky, then rebuilt the horizon from the second exposure and matched its grain and tone to the foreground";
    let row = ListRow::new("Version 3").lines(text).line_limit(3).lead_painter(|cx, rect| {
        cx.painter.circle_filled(rect.center(), rect.width() / 4.0, cx.ink());
    });
    show_as_only_item(ui, row).into()
}
