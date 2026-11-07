//! `List`: a list of `ListRow`s over a display order, with the selection, reorder, rename, reveal and tree settings of
//! the collection contract. The item step does the work: virtualisation, lead and selection rules, type-ahead,
//! drag with drop marks, rename lifecycle and the context-menu request. The list adds the item layout and the editor
//! the rename lifecycle shows.

use super::{ListRow, RowEvent};
use crate::contract::{
    Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, EditResponse, Props as _, ReorderRule, Role,
    SelectMode, Show as _,
};
use crate::runtime::commands::CommandScope;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use crate::text::TextField;
use craft_behavior::{ItemSource, Metrics, Selection, SpanLayout};
use egui::Ui;
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
}

impl<'a, K: Clone + Eq + Hash + Send + Sync + 'static> List<'a, K> {
    /// A list named `label`, with multiple selection.
    pub fn new(label: impl Into<Label>) -> Self {
        List {
            common: Common::new(),
            props: ContainerProps::new(),
            items: CollectionProps::new().select(SelectMode::Multi),
            selection: None,
            reveal: None,
            tree: false,
        }
        .label(label)
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
    type Style = ListStyle;
    type Output = CollectionResponse<K, RowEvent>;

    fn style(_: &StyleCx<'_>, _: ListStyleKey<K>) -> ListStyle {
        ListStyle::default()
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

    fn rename_editor(&mut self, ui: &mut Ui, draft: Bind<'_, String>) -> Option<EditResponse<String>> {
        Some(TextField::new(self.common.label.clone(), draft).show(ui))
    }
}
