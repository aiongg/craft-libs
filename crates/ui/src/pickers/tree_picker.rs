//! `TreePicker`: a field that shows the picked item's path and opens a tree of folders and items.
//!
//! The closed field is a `SelectFace`. The popup is a popover holding one tree `List`: folders open and close with
//! Left, Right and a click, an item is picked with Enter or a click, and type-ahead jumps to a row by its name.

#[cfg(feature = "testing")]
mod stories;

use super::words::TreePickerWords;
use super::SelectFace;
use crate::contract::overlay::{OverlayFit, ViewportShare};
use crate::contract::{
    Access, ActivateOn, Bind, BindMut, CollectionEvent, Common, Component, Cx, Edit as _, EditResponse, LeadOnOpen, MeasureCx, Overlay, OverlayUi,
    Overlays as _, PaintUi, PartsCx, Props as _, Remember as _, Role, SelectMode, ShowCollection as _, Sizing, Trigger, WordSetCx as _,
};
use crate::list::{List, ListRow, OpenSet, RowInk};
use crate::primitives::IconSpec;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, FocusLook, StyleCx, SurfaceLook};
use craft_behavior::overlay::MinWidth;
use craft_behavior::{DisplayOrder, ItemSource, Metrics, PartKey, Selection, Sense, Value};
use egui::{Rect, Vec2};
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

/// The face under the whole field.
const FACE: PartKey = PartKey::new("face");

/// The most of the viewport the popup takes before the tree scrolls.
const POPUP_SHARE: ViewportShare = ViewportShare::new(0.5, 0.4);

/// One item of the tree, in tree order: each folder is followed by what it holds.
#[derive(Clone, Debug)]
pub struct TreeEntry<K> {
    key: K,
    name: String,
    parent: Option<K>,
    folder: bool,
    blocked: Option<String>,
    icon: Option<IconSpec<'static>>,
}

impl<K> TreeEntry<K> {
    /// An item at the top, named `name`, that can be picked.
    pub fn new(key: K, name: impl Into<String>) -> Self {
        TreeEntry { key, name: name.into(), parent: None, folder: false, blocked: None, icon: None }
    }

    /// A folder at the top, named `name`: it is for browsing, not picking.
    pub fn folder(key: K, name: impl Into<String>) -> Self {
        TreeEntry { folder: true, ..TreeEntry::new(key, name) }
    }

    /// The entry sits in the folder `parent`.
    #[must_use]
    pub fn within(mut self, parent: K) -> Self {
        self.parent = Some(parent);
        self
    }

    /// The entry cannot be picked: it shows dimmed with `reason` as its tooltip.
    #[must_use]
    pub fn blocked_by(mut self, reason: impl Into<String>) -> Self {
        self.blocked = Some(reason.into());
        self
    }

    /// An icon before the name.
    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<IconSpec<'static>>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// The style of a tree picker: the control metrics and the focus ring; the closed field is its `SelectFace`.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TreePickerStyle {
    /// The field well; the face carries the padding.
    #[style(surface)]
    surface: SurfaceLook,
    /// No padding of its own: the face carries the field's.
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
}

/// What the popup keeps while it is open: the folders shown open.
#[derive(Clone)]
struct Browsing<K: Hash + Eq> {
    open: OpenSet<K>,
    shown: bool,
}

impl<K: Hash + Eq> Default for Browsing<K> {
    fn default() -> Self {
        Browsing { open: OpenSet::default(), shown: false }
    }
}

/// The parent of each entry as a position, and where each key is.
struct Tree<'a, K> {
    entries: &'a [TreeEntry<K>],
    positions: HashMap<&'a K, usize>,
    parents: Vec<Option<usize>>,
}

impl<'a, K: Hash + Eq> Tree<'a, K> {
    fn of(entries: &'a [TreeEntry<K>]) -> Self {
        let positions: HashMap<&K, usize> = entries.iter().enumerate().map(|(at, entry)| (&entry.key, at)).collect();
        let parents = entries.iter().map(|entry| entry.parent.as_ref().and_then(|parent| positions.get(parent).copied())).collect();
        Tree { entries, positions, parents }
    }

    fn position(&self, key: &K) -> Option<usize> {
        self.positions.get(key).copied()
    }

    /// The folders holding entry `at`, nearest first.
    fn ancestors(&self, at: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.parents[at], |&parent| self.parents[parent]).take(self.entries.len())
    }

    /// The positions of the entries shown with the folders in `open` open, and the depth of every entry.
    fn visible(&self, open: &OpenSet<K>) -> (Vec<usize>, Vec<usize>) {
        let mut depth = vec![0usize; self.entries.len()];
        let mut shown = vec![true; self.entries.len()];
        for at in 0..self.entries.len() {
            if let Some(parent) = self.parents[at].filter(|&parent| parent < at) {
                depth[at] = depth[parent] + 1;
                shown[at] = shown[parent] && open.is_open(&self.entries[parent].key);
            }
        }
        ((0..self.entries.len()).filter(|&at| shown[at]).collect(), depth)
    }
}

/// The visible entries as a collection source, keyed by the entry's key.
struct Rows<'a, 'k, K> {
    tree: &'k Tree<'a, K>,
    shown: &'k [usize],
    depth: &'k [usize],
    open: &'k OpenSet<K>,
}

impl<K: Clone + Hash + Eq> DisplayOrder<K> for Rows<'_, '_, K> {
    fn len(&self) -> usize {
        self.shown.len()
    }

    fn key(&self, at: usize) -> K {
        self.tree.entries[self.shown[at]].key.clone()
    }

    fn depth(&self, at: usize) -> usize {
        self.depth[self.shown[at]]
    }

    fn has_children(&self, at: usize) -> bool {
        self.tree.entries[self.shown[at]].folder
    }

    fn open(&self, at: usize) -> bool {
        let entry = &self.tree.entries[self.shown[at]];
        entry.folder && self.open.is_open(&entry.key)
    }

    fn enabled(&self, at: usize) -> bool {
        self.tree.entries[self.shown[at]].blocked.is_none()
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.tree.entries[self.shown[at]].name.as_str()))
    }
}

impl<K: Clone + Hash + Eq> ItemSource<K> for Rows<'_, '_, K> {}

/// A field that opens a tree of items and folders and sets the binding to the picked item's key. The field shows the
/// item's path, folder names joined by the path separator, or the placeholder while nothing is picked.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay, probe(TreePicker<'static, u32>))]
pub struct TreePicker<'a, K: Value + Eq + Hash> {
    #[common]
    common: Common,
    value: Bind<'a, Option<K>>,
    entries: &'a [TreeEntry<K>],
    /// Shown while nothing is picked.
    #[prop(into)]
    placeholder: Option<Label>,
}

impl<'a, K: Value + Eq + Hash> TreePicker<'a, K> {
    /// A picker named `label` over `value`, choosing among `entries` in tree order.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<K>>, entries: &'a [TreeEntry<K>]) -> Self {
        TreePicker { common: Common::new(), value, entries, placeholder: None }.label(label)
    }

    /// The names from the top folder down to the entry with `key`, joined with `separator`.
    fn path(&self, key: &K, separator: &str) -> Option<String> {
        let tree = Tree::of(self.entries);
        let at = tree.position(key)?;
        let mut names: Vec<&str> = std::iter::once(at).chain(tree.ancestors(at)).map(|at| self.entries[at].name.as_str()).collect();
        names.reverse();
        Some(names.join(separator))
    }

    /// What the field shows: the picked item's path, else the placeholder in its muted look.
    fn face(&self, value: Option<&Option<K>>, words: &TreePickerWords, env: &crate::runtime::env::EnvView<'_>) -> SelectFace {
        let path = value.and_then(Option::as_ref).and_then(|key| self.path(key, words.separator.as_str()));
        match path {
            Some(path) => SelectFace::new(path),
            None => SelectFace::new(env.resolve(&self.placeholder.clone().unwrap_or_else(|| words.placeholder.clone()))).placeholder(),
        }
    }
}

impl<K: Value + Eq + Hash> Component for TreePicker<'_, K> {
    const ROLE: Role = Role::COMBO_BOX;
    type Style = TreePickerStyle;
    type Output = EditResponse<Option<K>>;

    fn style(cx: &StyleCx<'_>, _: TreePickerStyleKey<K>) -> TreePickerStyle {
        TreePickerStyle { surface: cx.field(), metrics: Metrics::new(), focus: cx.focus_look() }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.face(self.face(cx.value(), &cx.words::<TreePickerWords>(), &cx.env))
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(FACE, rect).sense(Sense::HOVER);
    }

    fn bind(&mut self) -> Option<BindMut<'_, Option<K>>> {
        Some(self.value.reborrow())
    }

    /// A click, Enter, Space or Alt+Down toggles the popup.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.popup("tree").trigger(Trigger::Click, cx);
    }

    /// The popup: the tree with the folders closed except those holding the picked item, which is selected and in
    /// view. A click or Enter on a folder opens or closes it; on an item it sets the binding and closes the popup. A
    /// blocked item takes neither.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("tree");
        let entries = self.entries;
        let tree = Tree::of(entries);
        let chosen = cx.value().and_then(|value| value.clone());
        let opened = popup.is_open(cx);
        let fresh = opened && !cx.memory::<Browsing<K>>().shown;
        if fresh {
            let mut open = OpenSet::new();
            for folder in chosen.as_ref().and_then(|key| tree.position(key)).into_iter().flat_map(|at| tree.ancestors(at)) {
                open.set(&entries[folder].key, true);
            }
            cx.memory::<Browsing<K>>().open = open;
        }
        cx.memory::<Browsing<K>>().shown = opened;
        let open = cx.memory::<Browsing<K>>().open.clone();
        let (shown, depth) = tree.visible(&open);
        let rows = Rows { tree: &tree, shown: &shown, depth: &depth, open: &open };
        let name = cx.label();
        let overlay = Overlay::popover().fit(OverlayFit::ContentCapped(POPUP_SHARE)).min_width(MinWidth::Anchor);
        let events = ui.show(popup, overlay, |ui, _| {
            let selection = chosen.clone().map_or_else(Selection::default, |key| Selection::new(vec![key.clone()], Some(key.clone()), Some(key)));
            let mut list = List::<K>::new(name.clone())
                .select(SelectMode::Single)
                .allow_empty(false)
                .activate(ActivateOn::Click)
                .lead_on_open(LeadOnOpen::First)
                .tree(true)
                .sizing(Sizing::Content)
                .selection(Bind::given(selection));
            if let Some(key) = chosen.clone().filter(|_| fresh) {
                list = list.reveal(key);
            }
            list.show_items(ui, &rows, &mut |key, _| {
                let Some(at) = tree.position(key) else { return ListRow::spacer() };
                let entry = &entries[at];
                let mut row = ListRow::new(Label::from(entry.name.clone())).depth(depth[at]);
                if entry.folder {
                    row = row.group(open.is_open(&entry.key));
                }
                if let Some(icon) = &entry.icon {
                    row = row.icon(icon.clone());
                }
                match &entry.blocked {
                    Some(reason) => row.enabled(false).ink(RowInk::Dimmed).tooltip(reason.clone()),
                    None => row,
                }
            })
            .events
        });
        for event in events.into_iter().flatten() {
            match event {
                CollectionEvent::Activate(key) => {
                    let Some(entry) = tree.position(&key).map(|at| &entries[at]) else { continue };
                    if entry.folder {
                        let browsing = cx.memory::<Browsing<K>>();
                        let now = browsing.open.is_open(&key);
                        browsing.open.set(&key, !now);
                    } else if entry.blocked.is_none() {
                        cx.set(Some(key));
                        popup.close(cx);
                    }
                }
                CollectionEvent::Open(change) => {
                    let folders = entries.iter().filter(|entry| entry.folder).map(|entry| &entry.key);
                    cx.memory::<Browsing<K>>().open.apply(&change.key, change.open, change.all, folders);
                }
                _ => {}
            }
        }
    }

    /// The picked item's path as the field's value.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let separator = cx.words::<TreePickerWords>().separator.clone();
        if let Some(path) = cx.value().and_then(Option::as_ref).and_then(|key| self.path(key, separator.as_str())) {
            access.value(path);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.face(FACE, self.face(cx.value(), &cx.words::<TreePickerWords>(), &cx.env));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<Option<K>> {
        cx.edit_response()
    }
}
