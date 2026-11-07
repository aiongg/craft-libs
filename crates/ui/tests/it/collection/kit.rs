//! Collections for the trials, written with the derives and nothing else of the library.

use crate::contract::kit::PlainStyle;
use craft_behavior::{DisplayOrder, Intent, ItemSource, Selection, SpanLayout};
use craft_ui::contract::{
    Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, Edit as _, EditResponse, Input,
    ItemOutput, Focus as _, Outcome, Pointer as _, Role, SelectMode,
};
use craft_ui::style::StyleCx;
use egui::{Rect, Response, Ui, Vec2};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};

/// Row height of every list of the trials.
pub const ROW_H: f32 = 20.0;

/// Words in display order; rows with children and their open state come from the tree fields.
pub struct Words {
    pub words: Vec<&'static str>,
    /// `(depth, has children)` per row; empty for a flat list.
    pub tree: Vec<(usize, bool)>,
    pub open: RefCell<Vec<bool>>,
}

impl Words {
    pub fn flat(words: &[&'static str]) -> Self {
        Words { words: words.to_vec(), tree: Vec::new(), open: RefCell::new(Vec::new()) }
    }
}

impl DisplayOrder<usize> for Words {
    fn len(&self) -> usize {
        self.words.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.words.len()).then_some(*key)
    }

    fn depth(&self, at: usize) -> usize {
        self.tree.get(at).map_or(0, |row| row.0)
    }

    fn has_children(&self, at: usize) -> bool {
        self.tree.get(at).is_some_and(|row| row.1)
    }

    fn open(&self, at: usize) -> bool {
        self.open.borrow().get(at).copied().unwrap_or(false)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.words[at]))
    }
}

impl ItemSource<usize> for Words {}

/// `n` rows with no text.
pub struct Count(pub usize);

impl DisplayOrder<usize> for Count {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0).then_some(*key)
    }
}

impl ItemSource<usize> for Count {}

/// What an item reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Dismissed {
    Row,
}

/// The output of a row: its response and the event its dismiss part raised.
#[non_exhaustive]
pub struct RowOut {
    pub response: Response,
    pub event: Option<Dismissed>,
}

impl Outcome for RowOut {
    type Value = craft_ui::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<Dismissed> for RowOut {
    fn into_event(self) -> Option<Dismissed> {
        self.event
    }
}

const DISMISS: craft_behavior::PartKey = craft_behavior::PartKey::new("dismiss");

/// A row: it records its rect and, when it has a dismiss part, reports a click on it.
#[derive(craft_ui::contract::Component)]
pub struct Row {
    #[common]
    common: Common,
    dismiss: bool,
    seen: Option<&'static Cell<Rect>>,
    clicked: bool,
}

impl Row {
    pub fn new() -> Self {
        Row { common: Common::new(), dismiss: false, seen: None, clicked: false }
    }

    pub fn dismissable(mut self) -> Self {
        self.dismiss = true;
        self
    }
}

impl Component for Row {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = PlainStyle;
    type Output = RowOut;

    fn style(_: &StyleCx<'_>, _: RowStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::new(10.0, 10.0))
    }

    fn parts(&mut self, rect: Rect, cx: &mut craft_ui::contract::PartsCx<'_, Self>) {
        if self.dismiss {
            let hit = Rect::from_min_size(rect.min, Vec2::splat(ROW_H));
            cx.part(DISMISS, hit).sense(craft_behavior::Sense::CLICK).pointer_only("the list's own keys remove the row");
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.clicked = self.dismiss && matches!(cx.part(DISMISS).press.phase, craft_behavior::PressPhase::Release { click: true, .. });
        if let Some(seen) = self.seen {
            seen.set(cx.rect);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> RowOut {
        RowOut { response: cx.response.clone(), event: self.clicked.then_some(Dismissed::Row) }
    }
}

/// A list of rows at the control height, with the collection props and selection the trial gives.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
pub struct List<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    selection: Option<Bind<'a, Selection<usize>>>,
    tree: bool,
    dismissable: bool,
    on_empty: Option<&'a Cell<Rect>>,
}

impl<'a> List<'a> {
    pub fn new(select: SelectMode) -> Self {
        List {
            common: Common::new(),
            props: ContainerProps::new(),
            items: CollectionProps::new().select(select),
            selection: None,
            tree: false,
            dismissable: false,
            on_empty: None,
        }
    }

    pub fn rename(mut self) -> Self {
        self.items = self.items.rename(true);
        self
    }

    pub fn selection(mut self, selection: Bind<'a, Selection<usize>>) -> Self {
        self.selection = Some(selection);
        self
    }

    pub fn tree(mut self) -> Self {
        self.tree = true;
        self
    }

    pub fn dismissable(mut self) -> Self {
        self.dismissable = true;
        self
    }

    pub fn on_empty(mut self, seen: &'a Cell<Rect>) -> Self {
        self.on_empty = Some(seen);
        self
    }

    pub fn is_dismissable(&self) -> bool {
        self.dismissable
    }
}

impl Component for List<'_> {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: ListStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for List<'_> {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = Row;
    type ItemEvent = Dismissed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, Selection<usize>>> {
        self.selection.as_mut().map(Bind::reborrow)
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
        let layout = SpanLayout::new(source, width, ROW_H);
        if self.tree { layout.tree() } else { layout }
    }

    fn rename_editor(&mut self, ui: &mut Ui, draft: Bind<'_, String>) -> Option<EditResponse<String>> {
        Some(NameEditor::new(draft).show_in(ui))
    }

    fn empty(&mut self, ui: &mut Ui, cx: &mut craft_ui::contract::BodyCx<'_>) {
        if let Some(seen) = self.on_empty {
            seen.set(cx.rect);
        }
        let _ = ui;
    }
}

const TEXT: Input = Input::Text;

/// A text editor of a name: typed text proposes, Enter commits, Escape cancels.
#[derive(craft_ui::contract::Component)]
pub struct NameEditor<'a> {
    #[common]
    common: Common,
    draft: Bind<'a, String>,
}

impl<'a> NameEditor<'a> {
    pub fn new(draft: Bind<'a, String>) -> Self {
        NameEditor { common: Common::new(), draft }
    }

    fn show_in(self, ui: &mut Ui) -> EditResponse<String> {
        use craft_ui::contract::Show as _;
        self.show(ui)
    }
}

impl Component for NameEditor<'_> {
    const ROLE: Role = Role::TEXT_INPUT;
    const INPUT: Input = TEXT;
    type Style = PlainStyle;
    type Output = EditResponse<String>;

    fn style(_: &StyleCx<'_>, _: NameEditorStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::new(40.0, ROW_H))
    }

    fn bind(&mut self) -> Option<BindMut<'_, String>> {
        Some(self.draft.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        while let Some(text) = cx.intents().take_text() {
            let next = format!("{}{text}", cx.value().cloned().unwrap_or_default());
            cx.propose(next);
        }
        if cx.intents().take(Intent::Submit) {
            if cx.gesture().is_none() {
                cx.begin();
            }
            cx.commit();
        }
        if cx.intents().take(Intent::Cancel) && cx.gesture().is_some() {
            cx.cancel(craft_behavior::CancelReason::Escape);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<String> {
        cx.edit_response()
    }
}
