//! `List`: a list of `ListRow`s over a display order, with the selection, reorder, rename, reveal and tree settings of
//! the collection contract. The item step does the work: virtualisation, lead and selection rules, type-ahead,
//! drag with drop marks, rename lifecycle and the context-menu request. The list adds the item layout and the editor
//! the rename lifecycle shows.

use super::predicate::{DropTest, RowTest, SourceTest};
use super::{ListRow, RowEvent};
use crate::contract::{
    Access, ActivateOn, Bind, BindMut, Collection, CollectionProps, CollectionResponse, ColumnSort, Common, Component, ContainerProps, Cx,
    EditResponse, LeadOnOpen, ListDrag, Props as _, ReorderRule, Role, SelectMode, Show as _, Sizing,
};
use crate::runtime::commands::CommandScope;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use crate::text::TextField;
use craft_behavior::{ItemSource, Metrics, Selection, SpanLayout};
use egui::{Id, Rect, Ui};
use std::hash::Hash;

/// The style of the list: it has no look of its own beyond its container's surface.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ListStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A list of rows: one Tab stop, the selection rules of the collection contract, type-ahead on the rows' text,
/// rename in place, drag to reorder, and the request for the app's context menu.
///
/// Rows have the height their `ListRow` measures, or the height the item source's `size_hint` gives. A list of
/// groups and their children is a tree: rows are indented by their depth in the display order, groups open and
/// close with the arrow keys, and the list reports each change as `CollectionEvent::Open`.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(collection, probe(List<'static, usize>))]
pub struct List<'a, K: Clone + Eq + Hash + Send + Sync + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    /// The selection the caller owns.
    #[prop]
    selection: Option<Bind<'a, Selection<K>>>,
    /// Scrolls the row of `key` into view once.
    #[prop]
    reveal: Option<K>,
    /// A tree: rows are indented by depth and groups open and close.
    #[prop]
    tree: bool,
    /// Which rows may be renamed; a rename request on another row does nothing.
    #[prop(func = "Fn(&K) -> bool")]
    can_rename: Option<RowTest<'a, K>>,
    /// Which rows may be dragged.
    #[prop(func = "Fn(&K) -> bool")]
    can_drag: Option<RowTest<'a, K>>,
    /// Which collections a drag may come from: the test gets the id of the list the drag started in.
    #[prop(func = "Fn(Id) -> bool")]
    accept_from: Option<SourceTest<'a>>,
    /// Where a drag may land: the test gets the drag, the row under the pointer and the position before which the
    /// dragged rows would go. A refused spot shows no drop mark.
    #[prop(func = "Fn(&ListDrag<K>, &K, usize) -> bool")]
    accepts: Option<DropTest<'a, K>>,
    /// Files dragged in from the OS may drop on the list; a drop reports `CollectionEvent::FilesDropped`.
    #[prop]
    takes_files: bool,
    /// The list is a listbox of options: the popup of a value picker.
    #[prop(flag)]
    listbox: bool,
}

impl<'a, K: Clone + Eq + Hash + Send + Sync + 'static> List<'a, K> {
    /// A list named `label`, with multiple selection.
    pub fn new(label: impl Into<Label>) -> Self {
        List {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Multi),
            selection: None,
            reveal: None,
            tree: false,
            can_rename: None,
            can_drag: None,
            accept_from: None,
            accepts: None,
            takes_files: false,
            listbox: false,
        }
        .label(label)
    }

    /// The options list of a value picker's popup (completions, number presets, `PopupList`): one selection, a click
    /// activates, the lead starts on the first row and follows the pointer, and a press leaves the key focus on the
    /// opener, whose list keys the host forwards while the popup is open. Its rows are `ListRow::option()`. Menus are
    /// `Menu`s, not option lists.
    pub fn options(label: impl Into<Label>) -> Self {
        Self::new(label)
            .select(SelectMode::Single)
            .activate(ActivateOn::Click)
            .lead_on_open(LeadOnOpen::First)
            .lead_follows_hover(true)
            .focus_on_press(false)
            .sizing(Sizing::Content)
            .listbox()
    }

    /// What the user can select.
    #[must_use]
    pub fn select(mut self, mode: SelectMode) -> Self {
        self.items = self.items.select(mode);
        self
    }

    /// Rows reorder by dragging and by Alt with the arrow keys.
    #[must_use]
    pub fn reorder(mut self, rule: ReorderRule) -> Self {
        self.items = self.items.reorder(rule);
        self
    }

    /// Rows rename in place: a double click on the label, F2 or the rename command opens the editor.
    #[must_use]
    pub fn rename(mut self, on: bool) -> Self {
        self.items = self.items.rename(on);
        self
    }

    /// Odd rows are tinted: the item reads `ItemCx::row.striped`.
    #[must_use]
    pub fn striped(mut self, on: bool) -> Self {
        self.items = self.items.striped(on);
        self
    }

    /// A press on the empty space below the rows empties the selection.
    #[must_use]
    pub fn allow_empty(mut self, on: bool) -> Self {
        self.items = self.items.allow_empty(on);
        self
    }

    /// A press picks and activates a row without moving key focus to the list: a popup list whose opener keeps focus.
    #[must_use]
    pub fn focus_on_press(mut self, on: bool) -> Self {
        self.items = self.items.focus_on_press(on);
        self
    }

    /// Which gesture activates a row.
    #[must_use]
    pub fn activate(mut self, on: ActivateOn) -> Self {
        self.items = self.items.activate(on);
        self
    }

    /// The column the rows are sorted by, drawn on its header.
    #[must_use]
    pub fn sort(mut self, sort: impl Into<Option<ColumnSort>>) -> Self {
        self.items = self.items.sort(sort);
        self
    }

    /// Where the lead lands when the list opens.
    #[must_use]
    pub fn lead_on_open(mut self, at: LeadOnOpen) -> Self {
        self.items = self.items.lead_on_open(at);
        self
    }

    /// The rows are the set `stamp` names; the lead starts over when the stamp changes.
    #[must_use]
    pub fn rows_stamp(mut self, stamp: u64) -> Self {
        self.items = self.items.rows_stamp(stamp);
        self
    }

    /// The lead follows the pointer over the rows.
    #[must_use]
    pub fn lead_follows_hover(mut self, on: bool) -> Self {
        self.items = self.items.lead_follows_hover(on);
        self
    }

    /// How the list takes its size: to its content or to the room it has.
    #[must_use]
    pub fn sizing(mut self, sizing: Sizing) -> Self {
        self.props = self.props.sizing(sizing);
        self
    }

    /// Commands of `scope` reach the list while focus is in it.
    #[must_use]
    pub fn command_scope(mut self, scope: CommandScope) -> Self {
        self.props = self.props.command_scope(scope);
        self
    }

    /// The text of the empty state.
    #[must_use]
    pub fn empty(mut self, text: impl Into<Label>) -> Self {
        self.items = self.items.empty(text);
        self
    }

    /// The surface the list paints behind its rows.
    #[must_use]
    pub fn surface(mut self, surface: SurfaceKind) -> Self {
        self.props = self.props.surface(surface);
        self
    }
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> Component for List<'_, K> {
    const ROLE: Role = Role::LIST;
    const ALT_ROLES: &'static [Role] = &[Role::LIST_BOX];
    type Style = ListStyle;
    type Output = CollectionResponse<K, RowEvent>;

    fn style(_: &StyleCx<'_>, _: ListStyleKey<K>) -> ListStyle {
        ListStyle::default()
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        if self.listbox {
            access.role(Role::LIST_BOX);
        }
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> Collection for List<'_, K> {
    type Key = K;
    type ItemLayout = SpanLayout;
    type Selection = Selection<K>;
    type Item<'r> = ListRow<'r>;
    type ItemEvent = RowEvent;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, Selection<K>>> {
        self.selection.as_mut().map(Bind::reborrow)
    }

    fn reveal(&self) -> Option<&K> {
        self.reveal.as_ref()
    }

    fn item_layout(&self, source: &dyn ItemSource<K>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        let layout = SpanLayout::sized(source, width, cx.control().height);
        if self.tree { layout.tree() } else { layout }
    }

    fn can_rename(&self, key: &K) -> bool {
        self.can_rename.as_ref().is_none_or(|test| test.passes(key))
    }

    fn can_drag(&self, key: &K) -> bool {
        self.can_drag.as_ref().is_none_or(|test| test.passes(key))
    }

    fn accept_from(&self, from: Id) -> bool {
        self.accept_from.as_ref().is_none_or(|test| test.passes(from))
    }

    fn accepts(&self, drag: &ListDrag<K>, over: &K, insert: usize) -> bool {
        self.accepts.as_ref().is_none_or(|test| test.passes(drag, over, insert))
    }

    fn takes_files(&self) -> bool {
        self.takes_files
    }

    fn rename_editor(&mut self, ui: &mut Ui, draft: Bind<'_, String>, label: Rect, name: &str) -> Option<EditResponse<String>> {
        Some(TextField::new(name.to_owned(), draft).show_at(ui, label))
    }
}
