//! Collections for the trials, written with the derives and nothing else of the library.

use crate::contract::kit::PlainStyle;
use craft_behavior::{DisplayOrder, Intent, ItemSource, Selection, SpanLayout};
use craft_ui::contract::{
    Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, Edit as _, EditResponse, Focus as _,
    Input, ItemOutput, Outcome, Pointer as _, Role, SelectMode,
};
use craft_ui::style::StyleCx;
use egui::{Rect, Response, Ui, Vec2};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};

/// The payload a list maps its row drag to for other drop targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RowKey(pub usize);

/// Row height of every list of the trials.
pub const ROW_H: f32 = 20.0;

#[non_exhaustive]
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
#[non_exhaustive]
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
    scroll: Option<Bind<'a, craft_behavior::ScrollPos>>,
    tree: bool,
    on_empty: Option<&'a Cell<Rect>>,
    text_editor: bool,
    /// Where `parts` records the rect `PartsCx::rect_of` gives for the key.
    parts_probe: Option<(usize, &'a Cell<Option<Rect>>)>,
}

impl<'a> List<'a> {
    pub fn new(select: SelectMode) -> Self {
        List {
            common: Common::new(),
            props: ContainerProps::new().sizing(craft_ui::contract::Sizing::Fill),
            items: CollectionProps::new().select(select),
            selection: None,
            scroll: None,
            tree: false,
            on_empty: None,
            text_editor: false,
            parts_probe: None,
        }
    }

    /// Renames in a library text field, whose open draft owns Escape.
    pub fn text_editor(mut self) -> Self {
        self.text_editor = true;
        self
    }

    pub fn reorder(mut self) -> Self {
        self.items = self.items.reorder(craft_ui::contract::ReorderRule::new());
        self
    }

    /// Reordering may land inside the groups of a tree.
    pub fn reorder_into_groups(mut self) -> Self {
        self.items = self.items.reorder(craft_ui::contract::ReorderRule::new().into_groups());
        self
    }

    /// A row drag also carries the dragged row's key as a `RowKey`.
    pub fn row_key_payload(mut self) -> Self {
        self.items = self.items.drag_payload(|drag: &craft_ui::contract::ListDrag<usize>| Some(RowKey(drag.grabbed)));
        self
    }

    /// A row drag carries the dragged row's key as a `RowKey`, except the row `skip`, which carries none.
    pub fn row_key_payload_except(mut self, skip: usize) -> Self {
        self.items =
            self.items.drag_payload(move |drag: &craft_ui::contract::ListDrag<usize>| (drag.grabbed != skip).then_some(RowKey(drag.grabbed)));
        self
    }

    /// `parts` records `rect_of(key)` into `seen`.
    pub fn probe_parts(mut self, key: usize, seen: &'a Cell<Option<Rect>>) -> Self {
        self.parts_probe = Some((key, seen));
        self
    }

    /// A menu's rule: the lead starts on the first row and follows the pointer.
    pub fn menu(mut self) -> Self {
        self.items = self.items.lead_on_open(craft_ui::contract::LeadOnOpen::First).lead_follows_hover(true);
        self
    }

    /// The list takes the height of its rows, or of its empty state.
    pub fn content_sized(mut self) -> Self {
        self.props = self.props.sizing(craft_ui::contract::Sizing::Content);
        self
    }

    pub fn rename(mut self) -> Self {
        self.items = self.items.rename(true);
        self
    }

    /// Two headerless columns, the first of which a double click renames from.
    pub fn rename_column(mut self, name: craft_behavior::ColumnKey, other: craft_behavior::ColumnKey) -> Self {
        use craft_behavior::ColumnWidth;
        self.items = self.items.rename(true).rename_column(name.clone()).header(false).columns(vec![
            craft_ui::contract::ColumnSpec::new(name, "Name", ColumnWidth::Fixed(100.0)),
            craft_ui::contract::ColumnSpec::new(other, "Other", ColumnWidth::Fixed(100.0)),
        ]);
        self
    }

    /// A plain click on a row asks for its menu.
    pub fn menu_on_click(mut self, on: bool) -> Self {
        self.items = self.items.menu_on_click(on);
        self
    }

    /// A press on the empty space, or on the selected row of a single selection, empties the selection.
    pub fn allow_empty(mut self) -> Self {
        self.items = self.items.allow_empty(true);
        self
    }

    /// A press picks a row and leaves key focus where it is.
    pub fn keep_focus(mut self) -> Self {
        self.items = self.items.focus_on_press(false);
        self
    }

    /// The list takes the height of all its rows and leaves the scroll to the page it sits in.
    pub fn natural_height(mut self) -> Self {
        self.items = self.items.natural_height(true);
        self.props = self.props.sizing(craft_ui::contract::Sizing::FillWidth);
        self
    }

    /// Row `key` is the current one.
    pub fn current(mut self, key: usize) -> Self {
        self.items = self.items.current(Some(key));
        self
    }

    pub fn selection(mut self, selection: Bind<'a, Selection<usize>>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// The app owns the scroll offset.
    pub fn scroll(mut self, scroll: Bind<'a, craft_behavior::ScrollPos>) -> Self {
        self.scroll = Some(scroll);
        self
    }

    pub fn tree(mut self) -> Self {
        self.tree = true;
        self
    }

    /// The collection's own empty-state text.
    pub fn empty_text(mut self, text: &'static str) -> Self {
        self.items = self.items.empty(text);
        self
    }

    pub fn on_empty(mut self, seen: &'a Cell<Rect>) -> Self {
        self.on_empty = Some(seen);
        self
    }
}

impl Component for List<'_> {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: ListStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn scroll(&mut self) -> Option<BindMut<'_, craft_behavior::ScrollPos>> {
        self.scroll.as_mut().map(Bind::reborrow)
    }

    fn parts(&mut self, _: Rect, cx: &mut craft_ui::contract::PartsCx<'_, Self>) {
        if let Some((key, seen)) = self.parts_probe {
            seen.set(cx.rect_of(&key));
        }
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

    fn rename_editor(&mut self, ui: &mut Ui, draft: Bind<'_, String>, _: Rect, _: &str) -> Option<EditResponse<String>> {
        if self.text_editor {
            use craft_ui::contract::Show as _;
            return Some(craft_ui::text::TextField::new("Name", draft).show(ui));
        }
        Some(NameEditor::new(draft).shown(ui))
    }

    fn empty(&mut self, _ui: &mut Ui, cx: &mut craft_ui::contract::BodyCx<'_>) {
        if let Some(seen) = self.on_empty {
            seen.set(cx.rect);
        }
    }

    fn empty_height(&self, _: f32, _: &StyleCx<'_>) -> f32 {
        ROW_H
    }
}

const TEXT: Input = Input::Text;
const ESCAPE: craft_keymap::Chord = craft_keymap::Chord::plain(craft_keymap::Key::Escape);

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

    fn shown(self, ui: &mut Ui) -> EditResponse<String> {
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
        // An edit under way claims Escape from the next frame on, so the row does not take focus back first.
        if cx.take_claimed(ESCAPE) || (cx.intents().take(Intent::Cancel) && cx.gesture().is_some()) {
            cx.cancel(craft_behavior::CancelReason::Escape);
        }
        if cx.gesture().is_some() {
            cx.claim(ESCAPE);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<String> {
        cx.edit_response()
    }
}
