//! `UndoList`: the actions that can be undone, newest first, as a listbox for a popup the app opens from its own
//! drop-down arrow. Marking a row marks it and every newer one; picking a row undoes through it.

use super::view_words::UndoWords;
use crate::contract::{
    Bind, BindMut, Collection, CollectionEvent, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, ItemOutput, MeasureCx,
    NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Props as _, Role, SelectMode,
};
use crate::controls::select::row_look;
use crate::primitives::TextRun;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, Ink, StyleColor, StyleCx, SurfaceLook};
use craft_behavior::{DisplayOrder, ItemSource, Metrics, PartKey, PressPhase, SelectionChange, SelectionModel, Sense, SpanLayout};
use egui::{Rect, Response, Vec2};
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

/// What a row of the undo list reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum UndoRowEvent {
    /// The row was clicked.
    Picked,
}

/// How many actions an undo list asks the app to undo: through the row picked by a click or Enter, `None` while
/// no row was picked. The app undoes that many and closes the popup.
pub fn undo_count(events: &[CollectionEvent<usize, UndoRowEvent>]) -> Option<usize> {
    events.iter().find_map(|event| match event {
        CollectionEvent::Activate(position) | CollectionEvent::Item(position, UndoRowEvent::Picked) => Some(position + 1),
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
        UndoList { common: Common::new(), props: ContainerProps::new(), items: CollectionProps::new().select(SelectMode::Single), selection: None }
            .label(label)
    }
}

impl Component for UndoList<'_> {
    const ROLE: Role = Role::LIST_BOX;
    type Style = UndoListStyle;
    type Output = CollectionResponse<usize, UndoRowEvent>;

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
    type ItemEvent = UndoRowEvent;

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

const PICK: PartKey = PartKey::new("pick");

/// What a row of the undo list returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct UndoRowOutput {
    /// The row's response.
    pub response: Response,
    /// What the row reported this pass.
    pub event: Option<UndoRowEvent>,
}

impl Outcome for UndoRowOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<UndoRowEvent> for UndoRowOutput {
    fn into_event(self) -> Option<UndoRowEvent> {
        self.event
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

/// A row of the undo list: the label of an action, and a click part that picks it.
#[derive(crate::contract::Component)]
pub struct UndoRow<'r> {
    #[common]
    common: Common,
    label: &'r str,
    picked: bool,
}

impl<'r> UndoRow<'r> {
    /// A row showing `label`.
    pub fn new(label: &'r str) -> Self {
        UndoRow { common: Common::new(), label, picked: false }
    }
}

impl Component for UndoRow<'_> {
    const ROLE: Role = Role::LIST_BOX_OPTION;
    type Style = UndoRowStyle;
    type Output = UndoRowOutput;

    fn style(cx: &StyleCx<'_>, _: UndoRowStyleKey) -> UndoRowStyle {
        UndoRowStyle { surface: row_look(cx), metrics: cx.control().metrics(), ink: cx.ink(Ink::OnPanel) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(self.label)).size()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(PICK, rect).sense(Sense::CLICK).pointer_only("Enter in the list picks the marked row");
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.picked = matches!(cx.part(PICK).press.phase, PressPhase::Release { click: true, .. });
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.content_rect, &TextRun::new(self.label).elide().color(cx.style.ink));
    }

    fn finish(self, cx: Cx<'_, Self>) -> UndoRowOutput {
        UndoRowOutput { response: cx.response.clone(), event: self.picked.then_some(UndoRowEvent::Picked) }
    }
}
