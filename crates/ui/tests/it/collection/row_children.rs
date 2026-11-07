//! Rows with focusable children (17.5): the children are components shown in the row, their stops are members of
//! the row's roving group, and body parts of a leaf join the bar's roving group.

use super::kit::{Dismissed, ROW_H, RowOut, Words};
use crate::contract::kit::PlainStyle;
use craft_behavior::{Selection, SpanLayout};
use craft_ui::bars::{Bar, BarItem, BarItems, BarKind};
use craft_ui::buttons::{Button, IconButton};
use craft_ui::choice::Switch;
use craft_ui::contract::{
    Bind, Collection, CollectionEvent, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, MeasureCx, PaintUi, Props as _,
    Role, SelectMode, Show as _, ShowCollection as _, ShowContainer as _, Sizing,
};
use craft_ui::primitives::IconSpec;
use craft_ui::runtime::commands::EditCommands;
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::style::StyleCx;
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::focus::is_focused;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers, Rect, Vec2};
use std::cell::{Cell, RefCell};

const ROW_W: f32 = ROW_H * 3.0;

/// What the trials' components saw.
#[derive(Default)]
struct Log {
    /// The row component id of each key, as of the last pass that showed it.
    rows: RefCell<Vec<(usize, Id)>>,
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    list: Cell<Option<Id>>,
    before: Cell<Option<Id>>,
    after: Cell<Option<Id>>,
    icon: Cell<Option<Id>>,
    items: Cell<Option<Id>>,
}

impl Log {
    fn new() -> &'static Log {
        Box::leak(Box::default())
    }

    fn row(&self, key: usize) -> Id {
        self.rows.borrow().iter().rev().find(|(row, _)| *row == key).map(|(_, id)| *id).expect("the row ran")
    }

    fn list(&self) -> Id {
        self.list.get().expect("the list ran")
    }

    fn leads(&self) -> Vec<usize> {
        self.events.borrow().iter().filter_map(|event| if let CollectionEvent::LeadMoved(key) = event { Some(*key) } else { None }).collect()
    }

    /// The row and child that hold focus, `None` while the list or anything else does.
    fn child_focus(&self, ctx: &egui::Context) -> Option<(usize, &'static str)> {
        let rows = self.rows.borrow();
        rows.iter().rev().find_map(|(key, id)| ["a", "b"].into_iter().find(|name| is_focused(ctx, id.with(*name))).map(|name| (*key, name)))
    }
}

/// A row that shows two switches as its children.
#[derive(craft_ui::contract::Component)]
struct ChildRow {
    #[common]
    common: Common,
    key: usize,
    log: &'static Log,
}

impl ChildRow {
    fn new(key: usize, log: &'static Log) -> Self {
        ChildRow { common: Common::new(), key, log }
    }
}

impl Component for ChildRow {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = PlainStyle;
    type Output = RowOut;

    fn style(_: &StyleCx<'_>, _: ChildRowStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(ROW_W, ROW_H)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let first = Rect::from_min_size(cx.rect.min, Vec2::splat(ROW_H));
        ui.child("a", first, Switch::new(Bind::kept()).label("A"));
        ui.child("b", first.translate(Vec2::new(ROW_H, 0.0)), Switch::new(Bind::kept()).label("B"));
        self.log.rows.borrow_mut().push((self.key, cx.response.id));
    }

    fn finish(self, cx: Cx<'_, Self>) -> RowOut {
        RowOut { response: cx.response.clone(), event: None }
    }
}

/// A list of `ChildRow`s, a tree when the source has depths.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
struct ChildList {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
}

impl ChildList {
    fn new() -> Self {
        ChildList {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Content),
            items: CollectionProps::new().select(SelectMode::Single),
        }
    }
}

impl Component for ChildList {
    const ROLE: Role = Role::TREE;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: ChildListStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for ChildList {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = ChildRow;
    type ItemEvent = Dismissed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn item_layout(&self, source: &dyn craft_behavior::ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(source, width, ROW_H).tree()
    }
}

/// A fixture of a button, the list over `words` and a button, as one tab sequence.
fn list_fixture(log: &'static Log, words: &'static Words) -> Fixture {
    Fixture::new(move |ui| {
        log.before.set(Some(Button::new("Before").show(ui).id));
        let out = ChildList::new().id_salt("list").show_items(ui, words, &mut |key, _| ChildRow::new(*key, log));
        log.list.set(Some(out.response.id));
        for event in &out.events {
            if let CollectionEvent::Open(change) = event {
                words.open.borrow_mut()[change.key] = change.open;
            }
        }
        log.events.borrow_mut().extend(out.events);
        log.after.set(Some(Button::new("After").show(ui).id));
    })
}

fn tree_words() -> &'static Words {
    let mut words = Words::flat(&["group", "layer", "other"]);
    words.tree = vec![(0, true), (1, false), (0, false)];
    words.open = RefCell::new(vec![false; 3]);
    Box::leak(Box::new(words))
}

/// Tab from before the list lands on the list and the next Tab leaves it: the switches of the rows are no Tab
/// stops.
#[trial]
fn tab_passes_a_list_whose_rows_hold_controls_in_one_stop() {
    let (log, words) = (Log::new(), Box::leak(Box::new(Words::flat(&["one", "two"]))));
    let mut fixture = list_fixture(log, words);
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    assert!(is_focused(fixture.ctx(), log.before.get().unwrap()));
    fixture.key(Key::Tab, Modifiers::NONE);
    assert!(is_focused(fixture.ctx(), log.list()), "Tab lands on the list");
    fixture.key(Key::Tab, Modifiers::NONE);
    assert!(is_focused(fixture.ctx(), log.after.get().unwrap()), "Tab leaves the list, past the rows' switches");
}

/// Forward enters the lead row's first child and moves on, Back returns, and Escape from a child returns to the row.
#[trial]
fn forward_and_back_move_through_a_rows_children_and_escape_returns_to_the_row() {
    let (log, words) = (Log::new(), Box::leak(Box::new(Words::flat(&["one", "two"]))));
    let mut fixture = list_fixture(log, words);
    fixture.settle();
    request(fixture.ctx(), FocusTarget::Id(log.list()));
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(log.leads().last(), Some(&0));
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(log.child_focus(fixture.ctx()), Some((0, "a")), "Forward enters the first child");
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(log.child_focus(fixture.ctx()), Some((0, "b")));
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(log.child_focus(fixture.ctx()), None);
    assert!(is_focused(fixture.ctx(), log.list()), "Back from the first child returns to the row");

    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.key(Key::Escape, Modifiers::NONE);
    assert_eq!(log.child_focus(fixture.ctx()), None);
    assert!(is_focused(fixture.ctx(), log.list()), "Escape from a child returns to its row");
    assert_eq!(log.leads().last(), Some(&0), "the lead stays on the row");
}

/// In a tree, Forward on a closed parent expands it and Forward again enters its first child; Back on a child row
/// moves to the parent row.
#[trial]
fn a_tree_row_expands_then_enters_its_children_and_back_moves_to_the_parent() {
    let (log, words) = (Log::new(), tree_words());
    let mut fixture = list_fixture(log, words);
    fixture.settle();
    request(fixture.ctx(), FocusTarget::Id(log.list()));
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(log.leads().last(), Some(&0));
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert!(words.open.borrow()[0], "Forward on a closed parent expands it");
    assert_eq!(log.child_focus(fixture.ctx()), None, "expanding does not enter the children");
    fixture.settle();
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(log.child_focus(fixture.ctx()), Some((0, "a")), "Forward on an open parent enters its first child");
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(log.leads().last(), Some(&1), "Down moves to the child row");
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(log.leads().last(), Some(&0), "Back on a child row moves to the parent row");
}

/// A child's node is a child of its row's node, so a screen reader reaches each control through the row.
#[trial]
fn a_rows_children_are_nodes_below_the_row_node() {
    let (log, words) = (Log::new(), Box::leak(Box::new(Words::flat(&["one"]))));
    let mut fixture = list_fixture(log, words);
    fixture.settle();
    let row = log.row(0);
    let tree = fixture.tree();
    for name in ["a", "b"] {
        assert_eq!(tree.parent_of(row.with(name)), Some(NodeKey::of(row)), "the {name} switch");
    }
}

/// `BarItems` and an icon button in a bar are one Tab stop, and the arrows cross from the items to the button and
/// back.
#[trial]
fn bar_items_and_an_icon_button_are_one_stop_with_arrows_across() {
    let log = Log::new();
    let mut fixture = Fixture::new(move |ui| {
        log.before.set(Some(Button::new("Before").show(ui).id));
        Bar::new("Tools", BarKind::Tool).show(ui, |ui, _| {
            log.items.set(Some(BarItems::new(vec![BarItem::new(EditCommands::Undo), BarItem::new(EditCommands::Redo)]).show(ui).id));
            log.icon.set(Some(IconButton::new(IconSpec::svg(craft_behavior::IconId::new("eye"))).label("Visible").show(ui).id));
        });
        log.after.set(Some(Button::new("After").show(ui).id));
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::Tab, Modifiers::NONE);
    assert!(is_focused(fixture.ctx(), log.items.get().unwrap()), "Tab arrives at the bar's first member");
    fixture.key(Key::Tab, Modifiers::NONE);
    assert!(is_focused(fixture.ctx(), log.after.get().unwrap()), "the bar is one Tab stop");
    fixture.key(Key::Tab, Modifiers::SHIFT);
    fixture.key(Key::End, Modifiers::NONE);
    assert!(is_focused(fixture.ctx(), log.icon.get().unwrap()), "the last member of the bar's group is the icon button");
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    assert!(is_focused(fixture.ctx(), log.items.get().unwrap()), "Back from the icon button reaches the last item");
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert!(is_focused(fixture.ctx(), log.icon.get().unwrap()), "Forward from the last item reaches the icon button");
}
