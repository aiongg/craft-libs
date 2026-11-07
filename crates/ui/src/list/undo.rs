//! `UndoList`: the actions that can be undone, newest first, as a listbox for a popup the app opens from its own
//! drop-down arrow. Marking a row marks it and every newer one; picking a row undoes through it.

use super::view_words::UndoWords;
use crate::contract::{
    ActivateOn, Bind, BindMut, Collection, CollectionEvent, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, MeasureCx,
    PaintUi, Props as _, Role, SelectMode, Sizing,
};
use crate::controls::select::row_look;
use crate::primitives::TextRun;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, Ink, StyleColor, StyleCx, SurfaceLook};
use craft_behavior::{DisplayOrder, ItemSource, Metrics, SelectionChange, SelectionModel, SpanLayout};
use egui::{Response, Vec2};
use std::borrow::Cow;

/// The marked actions of an undo list: the row at `marked` and every newer one (a smaller position).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UndoMark {
    marked: Option<usize>,
}

impl UndoMark {
    /// A mark on the row at `position`, 0 being the newest action.
    pub const fn at(position: usize) -> Self {
        UndoMark { marked: Some(position) }
    }

    /// How many actions the mark covers: the marked row and every newer one.
    pub const fn actions(&self) -> usize {
        match self.marked {
            Some(position) => position + 1,
            None => 0,
        }
    }
}

impl SelectionModel<usize> for UndoMark {
    fn contains(&self, key: &usize) -> bool {
        self.marked.is_some_and(|marked| *key <= marked)
    }

    fn lead(&self) -> Option<&usize> {
        self.marked.as_ref()
    }

    fn anchor(&self) -> Option<&usize> {
        self.marked.as_ref()
    }

    fn apply(&mut self, change: SelectionChange<usize>) {
        self.marked = match change {
            SelectionChange::Set(keys) => keys.last().copied(),
            SelectionChange::Toggle(key) => Some(key),
            SelectionChange::Clear => None,
            SelectionChange::Cells(range) => range.rows.last().copied(),
            _ => self.marked,
        };
    }
}

/// The footer text of an undo list: "Undo 3 Actions" while rows are marked, `None` while none is (the footer is the
/// Cancel row then).
pub fn footer_text(words: &UndoWords, mark: &UndoMark) -> Option<String> {
    let actions = mark.actions();
    (actions > 0).then(|| words.actions.text(actions as u64, &actions.to_string()))
}

/// The actions of the undo list as an item source: the label of each, newest first, keyed by position.
#[derive(Clone, Copy, Debug)]
pub struct UndoRows<'a>(&'a [String]);

impl<'a> UndoRows<'a> {
    /// The actions labelled `labels`, the newest first.
    pub const fn new(labels: &'a [String]) -> Self {
        UndoRows(labels)
    }
}

impl DisplayOrder<usize> for UndoRows<'_> {
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
        self.0.get(at).map(|label| Cow::Borrowed(label.as_str()))
    }
}

impl ItemSource<usize> for UndoRows<'_> {}

/// How many actions an undo list asks the app to undo: through the row picked by a click or Enter, `None` while
/// no row was picked. The app undoes that many and closes the popup.
pub fn undo_count(events: &[CollectionEvent<usize, ()>]) -> Option<usize> {
    events.iter().find_map(|event| match event {
        CollectionEvent::Activate(position) => Some(position + 1),
        _ => None,
    })
}

/// The style of the undo list: it has no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct UndoListStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The undo list: one Tab stop, arrows mark rows through the row, Enter or a click picks it.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(collection)]
pub struct UndoList<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    /// The mark the caller owns.
    #[prop]
    selection: Option<Bind<'a, UndoMark>>,
}

impl<'a> UndoList<'a> {
    /// An undo list named `label`.
    pub fn new(label: impl Into<Label>) -> Self {
        UndoList {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Single).activate(ActivateOn::Click).lead_follows_hover(true),
            selection: None,
        }
        .label(label)
    }
}

impl Component for UndoList<'_> {
    const ROLE: Role = Role::LIST_BOX;
    type Style = UndoListStyle;
    type Output = CollectionResponse<usize, ()>;

    fn style(_: &StyleCx<'_>, _: UndoListStyleKey) -> UndoListStyle {
        UndoListStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for UndoList<'_> {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = UndoMark;
    type Item<'r> = UndoRow<'r>;
    type ItemEvent = ();

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, UndoMark>> {
        self.selection.as_mut().map(Bind::reborrow)
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(source, width, cx.control().height)
    }
}

/// The style of an undo row: the marked fill, the control's padding and the text colour.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct UndoRowStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    ink: StyleColor,
}

/// A row of the undo list: the label of an action. The list's item step handles the click and Enter that pick it.
#[derive(crate::contract::Component)]
pub struct UndoRow<'r> {
    #[common]
    common: Common,
    label: &'r str,
}

impl<'r> UndoRow<'r> {
    /// A row showing `label`.
    pub fn new(label: &'r str) -> Self {
        UndoRow { common: Common::new(), label }.label(label.to_owned())
    }
}

impl Component for UndoRow<'_> {
    const ROLE: Role = Role::LIST_BOX_OPTION;
    const OVERLAYS: bool = false;
    type Style = UndoRowStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: UndoRowStyleKey) -> UndoRowStyle {
        UndoRowStyle { surface: row_look(cx), metrics: cx.control().metrics(), ink: cx.ink(Ink::OnPanel) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(self.label)).size()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.content_rect, &TextRun::new(self.label).elide().color(cx.style.ink));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
