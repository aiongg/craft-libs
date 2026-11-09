//! `MediaBin` behaviour: what its own logic adds to the collections it shows.

use craft_behavior::{ColumnKey, DisplayOrder, IconId, ItemSource, Selection};
use craft_browse::ui::bin::{BinEntry, BinEvent, BinItem, BinModel, BinSettings, BinState, BinView, MediaBin};
use craft_ui::contract::{Bind, ColumnSort, ListDrag, ReorderRule, ShowCollection as _, ShowContainer as _, Sizing};
use craft_ui::layout_components::Stack;
use craft_ui::list::{List, ListRow};
use craft_ui::sections::FooterAction;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Rect, pos2};
use std::cell::RefCell;
use std::rc::Rc;

const NAME: ColumnKey = ColumnKey::new("name");

struct Node {
    key: u32,
    name: &'static str,
    parent: Option<u32>,
    bin: bool,
}

/// Bins Footage and Archive and the empty bin Empty, with items inside and one at the root.
struct Project(Vec<Node>);

impl Project {
    fn new() -> Self {
        let node = |key, name, parent, bin| Node { key, name, parent, bin };
        Project(vec![
            node(1, "Footage", None, true),
            node(2, "Archive", None, true),
            node(3, "Empty", None, true),
            node(10, "Beach.mov", Some(1), false),
            node(11, "Clip.mov", Some(1), false),
            node(20, "Old beach.mov", Some(2), false),
            node(30, "Title.psd", None, false),
        ])
    }

    fn node(&self, key: u32) -> Option<&Node> {
        self.0.iter().find(|node| node.key == key)
    }
}

impl BinModel<u32> for Project {
    fn children(&self, parent: Option<&u32>, out: &mut Vec<BinEntry<u32>>) {
        for node in self.0.iter().filter(|node| node.parent == parent.copied()) {
            out.push(if node.bin { BinEntry::Bin(node.key) } else { BinEntry::Item(node.key) });
        }
    }

    fn item(&self, key: &u32, item: &mut BinItem) {
        if let Some(node) = self.node(*key) {
            item.name = node.name.to_owned();
        }
    }

    fn cell(&self, _: &u32, _: &ColumnKey) -> String {
        String::new()
    }

    fn is_bin(&self, key: &u32) -> bool {
        self.node(*key).is_some_and(|node| node.bin)
    }

    fn parent(&self, key: &u32) -> Option<u32> {
        self.node(*key).and_then(|node| node.parent)
    }

    fn count(&self, bin: &u32) -> usize {
        self.0.iter().filter(|node| node.parent == Some(*bin)).map(|node| if node.bin { self.count(&node.key) } else { 1 }).sum()
    }
}

/// A list beside the bin, whose rows are dragged into it: Old beach (in Archive) and Title (at the root).
struct Other;

impl DisplayOrder<u32> for Other {
    fn len(&self) -> usize {
        2
    }

    fn key(&self, at: usize) -> u32 {
        [20, 30][at]
    }
}

impl ItemSource<u32> for Other {}

struct Setup {
    fixture: Fixture,
    state: Rc<RefCell<BinState<u32>>>,
    selection: Rc<RefCell<Selection<u32>>>,
    events: Rc<RefCell<Vec<BinEvent<u32>>>>,
}

/// The bin over the project, with `configure` applied to its state, and a foreign list above it when `other`.
fn setup(other: bool, configure: impl FnOnce(&mut BinState<u32>)) -> Setup {
    let mut initial = BinState::<u32>::default();
    initial.expanded.set(&1, true);
    configure(&mut initial);
    let state = Rc::new(RefCell::new(initial));
    let selection = Rc::new(RefCell::new(Selection::<u32>::default()));
    let events = Rc::new(RefCell::new(Vec::new()));
    let (shown_state, shown_selection, shown_events) = (state.clone(), selection.clone(), events.clone());
    let mut fixture = Fixture::new(move |ui| {
        let project = Project::new();
        let mut state = shown_state.borrow_mut();
        let mut selection = shown_selection.borrow_mut();
        Stack::new().fill().show(ui, |ui, _| {
            if other {
                let _ = List::<u32>::new("Other")
                    .sizing(Sizing::Content)
                    .reorder(ReorderRule::new())
                    .show_items(ui, &Other, &mut |key, _| ListRow::new(format!("Other {key}")));
            }
            let columns = vec![craft_ui::contract::ColumnSpec::new(NAME, "Name", craft_behavior::ColumnWidth::fill(1.0)).sortable()];
            let action = FooterAction::new(IconId::new("plus"), "New Item").accepts_drops::<ListDrag<u32>>(|drag| !drag.keys.is_empty());
            let out = MediaBin::new("bin", Bind::bound(&mut *state), &project)
                .selection(Bind::bound(&mut *selection))
                .columns(columns)
                .footer_actions([action])
                .hover_scrub(true)
                .show(ui);
            shown_events.borrow_mut().extend(out.events);
        });
    });
    fixture.settle();
    Setup { fixture, state, selection, events }
}

impl Setup {
    fn rect(&self, label: &str) -> Rect {
        self.fixture.by_label(label).unwrap_or_else(|| panic!("{label} is shown")).rect
    }

    /// Drags the control labelled `from` to the centre of the one labelled `to`.
    fn drag_to(&mut self, from: &str, to: egui::Pos2) {
        let target = self.fixture.by_label(from).unwrap_or_else(|| panic!("{from} is shown"));
        let by = to - target.rect.center();
        self.fixture.drag(&target, by);
        self.fixture.settle();
    }

    /// A point in the bin's empty room, above the footer.
    fn empty_room(&self) -> egui::Pos2 {
        let room = self.fixture.shown();
        pos2(room.center().x, room.bottom() - 70.0)
    }

    /// The drops so far as the dropped keys and the target bin.
    fn drops(&self) -> Vec<(Vec<u32>, Option<u32>)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| if let BinEvent::Drop { items, into, .. } = event { Some((items.clone(), *into)) } else { None })
            .collect()
    }
}

/// A search lists the matching items of the sub-bins, flat, and clearing it shows the bin again.
#[trial]
fn a_search_shows_matches_from_sub_bins() {
    let mut bin = setup(false, |_| {});
    assert!(bin.fixture.by_label("Clip.mov").is_some());
    bin.state.borrow_mut().search = "BEACH".to_owned();
    bin.fixture.settle();
    assert!(bin.fixture.by_label("Old beach.mov").is_some(), "a match inside Archive");
    assert!(bin.fixture.by_label("Beach.mov").is_some());
    assert!(bin.fixture.by_label("Clip.mov").is_none());
    assert!(bin.fixture.by_label("Archive").is_none(), "bins are left out of results");
    bin.state.borrow_mut().search.clear();
    bin.fixture.settle();
    assert!(bin.fixture.by_label("Archive").is_some());
}

/// An item dragged onto a bin row's middle is a drop into the bin.
#[trial]
fn dropping_on_a_bin_row_reports_the_target() {
    let mut bin = setup(false, |_| {});
    let archive = bin.rect("Archive").center();
    bin.drag_to("Beach.mov", archive);
    assert_eq!(bin.drops(), [(vec![10], Some(2))], "{:?}", bin.events.borrow());
}

/// In a sorted list a release on the empty room drops into the shown bin, except items already in it.
#[trial]
fn a_release_on_the_background_drops_into_the_shown_bin_unless_they_are_there() {
    let mut bin = setup(false, |state| state.settings.sort = Some(ColumnSort::ascending(NAME)));
    let room = bin.empty_room();
    bin.drag_to("Clip.mov", room);
    assert_eq!(bin.drops(), [(vec![11], None)], "{:?}", bin.events.borrow());
    bin.events.borrow_mut().clear();
    bin.drag_to("Title.psd", room);
    assert!(bin.drops().is_empty(), "Title is in the root, which is the shown bin: {:?}", bin.events.borrow());
}

/// In User Order a release past the last row moves a row of the shown bin to the end; it is not a drop.
#[trial]
fn a_release_past_the_last_row_in_user_order_reorders() {
    let mut bin = setup(false, |_| {});
    let room = bin.empty_room();
    bin.drag_to("Archive", room);
    let events = bin.events.borrow();
    assert!(events.iter().any(|event| matches!(event, BinEvent::Reorder(_))), "{events:?}");
    assert!(!events.iter().any(|event| matches!(event, BinEvent::Drop { .. })));
}

/// A release past the last row puts the item at the top level of the shown bin: an item of an open sub-bin leaves
/// its bin, which is a drop into the shown bin, not a reorder.
#[trial]
fn a_release_past_the_last_row_takes_an_item_out_of_its_bin() {
    let mut bin = setup(false, |_| {});
    let room = bin.empty_room();
    bin.drag_to("Clip.mov", room);
    assert_eq!(bin.drops(), [(vec![11], None)], "{:?}", bin.events.borrow());
    assert!(!bin.events.borrow().iter().any(|event| matches!(event, BinEvent::Reorder(_))));
}

/// A drag from another list over an item row is a drop on the background, never a reorder; over a bin row it
/// drops into the bin.
#[trial]
fn a_foreign_drag_over_rows_is_a_drop_not_a_reorder() {
    let mut bin = setup(true, |_| {});
    let clip = bin.rect("Clip.mov").center();
    bin.drag_to("Other 20", clip);
    assert_eq!(bin.drops(), [(vec![20], None)], "{:?}", bin.events.borrow());
    assert!(bin.events.borrow().iter().all(|event| !matches!(event, BinEvent::Reorder(_))));

    bin.events.borrow_mut().clear();
    let footage = bin.rect("Footage").center();
    bin.drag_to("Other 30", footage);
    assert_eq!(bin.drops(), [(vec![30], Some(1))], "{:?}", bin.events.borrow());
}

/// "{selected} of {total}" counts only the selected keys the bin shows.
#[trial]
fn the_selection_count_leaves_out_keys_that_are_not_shown() {
    let mut bin = setup(false, |state| state.shown = Some(1));
    *bin.selection.borrow_mut() = Selection::single(20);
    bin.fixture.settle();
    assert!(bin.fixture.by_label("2 items").is_some());
    *bin.selection.borrow_mut() = Selection::single(10);
    bin.fixture.settle();
    assert!(bin.fixture.by_label("1 of 2 items selected").is_some());
}

/// Icon view: bins are tiles before the items, and the pointer over an item's picture reports the scrub.
#[trial]
fn the_icon_view_shows_bins_first_and_scrubs_items() {
    let mut bin = setup(false, |state| state.settings.view = BinView::Icon);
    let (footage, archive, title) = (bin.rect("Footage"), bin.rect("Archive"), bin.rect("Title.psd"));
    assert!(footage.left() < archive.left() && archive.left() < title.left(), "bins first: {footage:?} {archive:?} {title:?}");
    assert!(bin.fixture.by_label("Beach.mov").is_none(), "a bin's items show when it is opened");

    bin.fixture.frame(vec![pointer::moved(pos2(title.left() + title.width() * 0.5, title.top() + 12.0))]).settle();
    let scrub = bin
        .events
        .borrow()
        .iter()
        .rev()
        .find_map(|event| if let BinEvent::Scrub { key, fraction, .. } = event { Some((*key, *fraction)) } else { None });
    let (key, fraction) = scrub.expect("a scrub");
    assert_eq!(key, 30);
    assert!(fraction.is_finite());

    bin.events.borrow_mut().clear();
    bin.fixture.frame(vec![pointer::moved(footage.center())]).settle();
    assert!(bin.events.borrow().iter().all(|event| !matches!(event, BinEvent::Scrub { .. })), "a bin has no picture to scrub");
}

/// Icon view: a drag from another list onto a bin tile drops into it, onto the background into the shown bin.
#[trial]
fn icon_cards_drop_into_bins_and_the_background() {
    let mut bin = setup(true, |state| state.settings.view = BinView::Icon);
    let archive = bin.rect("Archive").center();
    bin.drag_to("Other 30", archive);
    assert_eq!(bin.drops(), [(vec![30], Some(2))], "{:?}", bin.events.borrow());

    bin.events.borrow_mut().clear();
    let room = bin.empty_room();
    bin.drag_to("Other 20", room);
    assert_eq!(bin.drops(), [(vec![20], None)], "{:?}", bin.events.borrow());
}

/// The List view's sort, the Icon view's sort and the dragged column widths survive the host's save and load.
#[trial]
fn the_sorts_and_column_widths_are_saved() {
    let mut settings = BinSettings::default();
    settings.sort = Some(ColumnSort::new(NAME, true));
    settings.icon_sort = Some(ColumnSort::new(ColumnKey::new("rating"), false));
    settings.widths = vec![(NAME, 140.0)];
    let text = serde_json::to_string(&settings).expect("the settings serialize");
    let loaded: BinSettings = serde_json::from_str(&text).expect("the settings load");
    assert_eq!(loaded, settings);
}
