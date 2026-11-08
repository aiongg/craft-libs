//! The list trials: wheel and thumb, keys and selection, rename, the empty state, trees and large sources.

use super::kit::{Count, Dismissed, List, ROW_H, Row, Words};
use craft_behavior::PartKey;
use craft_behavior::{CancelReason, EditEvent, Selection, SelectionChange};
use craft_ui::contract::{Bind, CollectionEvent, CollectionResponse, Props as _, SelectMode, Show as _, ShowCollection as _};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::testing::focus::egui_focused;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::text::widget_id;
use craft_ui::testing::{Frames, input, pointer, trial};
use egui::{Event, Id, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const ROWS: usize = 40;
const SAME: f32 = 0.5;
const WHEEL: f32 = 60.0;

type Out = CollectionResponse<usize, Dismissed>;

/// What a frame of a list reported.
struct Seen {
    id: Cell<Option<Id>>,
    rect: Cell<Rect>,
    visible: Cell<(usize, usize)>,
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
}

impl Default for Seen {
    fn default() -> Self {
        Seen { id: Cell::new(None), rect: Cell::new(Rect::NOTHING), visible: Cell::new((0, 0)), events: RefCell::new(Vec::new()) }
    }
}

impl Seen {
    fn range(&self) -> std::ops::Range<usize> {
        self.visible.get().0..self.visible.get().1
    }

    fn take(&self, out: Out) {
        self.id.set(Some(out.response.id));
        self.rect.set(out.response.rect);
        self.visible.set((out.visible.start, out.visible.end));
        self.events.borrow_mut().extend(out.events);
    }

    fn id(&self) -> Id {
        self.id.get().expect("the list ran")
    }

    fn leads(&self) -> Vec<usize> {
        self.events.borrow().iter().filter_map(|event| if let CollectionEvent::LeadMoved { key, .. } = event { Some(*key) } else { None }).collect()
    }

    fn selections(&self) -> Vec<SelectionChange<usize>> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::Selection(change) = event { Some(change.clone()) } else { None })
            .collect()
    }
}

fn labels() -> Vec<&'static str> {
    vec!["apple", "apricot", "banana", "blueberry", "cherry", "date"]
}

fn show_words(ui: &mut Ui, list: List<'_>, words: &Words, seen: &Seen) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        seen.take(list.id_salt("list").show_items(ui, words, &mut |_, _| Row::new()));
    });
}

fn press(frames: &mut Frames, key: Key, mods: Modifiers, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![input::key_press(key, mods)], &mut *show);
    frames.frame(vec![craft_ui::testing::focus::key_release(key, mods)], &mut *show);
}

fn focus_list(frames: &mut Frames, seen: &Seen, show: &mut impl FnMut(&mut Ui)) {
    request(&frames.ctx, FocusTarget::Id(seen.id()));
    frames.idle(2, &mut *show);
}

/// The wheel and the thumb move a list in the frame that carries them.
#[trial]
fn wheel_and_thumb_scroll_show_new_rows_in_the_same_frame() {
    let words = Count(ROWS);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            seen.take(List::new(SelectMode::None).id_salt("list").show_items(ui, &words, &mut |_, _| Row::new()));
        });
    };
    frames.idle(3, &mut show);
    let view = seen.rect.get();
    assert_eq!(seen.range().start, 0);
    assert!(seen.range().end < ROWS, "the view shows a window of the rows: {:?}", seen.range());

    frames.frame(vec![pointer::moved(view.center())], &mut show);
    frames.frame(vec![pointer::wheel(-200.0)], &mut show);
    let after_wheel = seen.range().start;
    assert!(after_wheel >= 5, "the frame that carries the wheel shows the rows it scrolled to: {after_wheel}");

    let content = ROWS as f32 * ROW_H;
    let thumb_len = view.height() * view.height() / content;
    let travel = view.height() - thumb_len;
    let offset = after_wheel as f32 * ROW_H;
    let thumb = Pos2::new(view.max.x - 2.0, view.min.y + travel * offset / (content - view.height()) + thumb_len / 2.0);
    pointer::drag(&mut frames, thumb, Vec2::new(0.0, 60.0), &mut show);
    let after_thumb = seen.range().start;
    assert!(after_thumb > after_wheel + 5, "a thumb drag of 60 points moved the rows: {after_wheel} -> {after_thumb}");
}

/// The app sets a list's scroll position by writing the offset it owns: the next pass shows those rows, and the wheel
/// writes the new offset back.
#[trial]
fn the_app_sets_a_lists_scroll_position_through_its_scroll_binding() {
    use craft_behavior::ScrollPos;
    let words = Count(ROWS);
    let seen = Seen::default();
    let held = RefCell::new(ScrollPos::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let mut offset = *held.borrow();
            seen.take(List::new(SelectMode::None).scroll(Bind::bound(&mut offset)).id_salt("list").show_items(ui, &words, &mut |_, _| Row::new()));
            *held.borrow_mut() = offset;
        });
    };
    frames.idle(3, &mut show);
    assert_eq!(seen.range().start, 0);
    let target = (ROWS / 2) as f64 * f64::from(ROW_H);
    held.borrow_mut().y = target;
    frames.idle(2, &mut show);
    assert!(seen.range().start >= ROWS / 2 - 1, "the list shows the rows at the offset the app wrote: {:?}", seen.range());
    frames.frame(vec![pointer::moved(seen.rect.get().center())], &mut show);
    frames.frame(vec![pointer::wheel(-WHEEL)], &mut show);
    assert!(held.borrow().y > target, "the wheel wrote the offset back: {:?}", held.borrow());
}

/// A body and a list under one wheel rule: the wheel moves the one under the pointer by the wheel's points.
#[trial]
fn panel_body_and_list_share_the_wheel_rule() {
    use crate::body::kit::Slab;
    use craft_ui::contract::{ContainerProps, Scroll, ShowContainer as _};
    use craft_ui::testing::layout::Block;
    let words = Count(ROWS);
    let seen = Seen::default();
    let (row, last_row) = (Box::leak(Box::new(Cell::new(Rect::NOTHING))), Box::leak(Box::new(Cell::new(Rect::NOTHING))));
    let (panel_rect, first_top) = (Cell::new(Rect::NOTHING), Cell::new(0.0f32));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let out = Slab::new(ContainerProps::new().scroll(Scroll::vertical())).id_salt("panel").show(ui, |ui, _| {
                for line in 0..ROWS {
                    if line == 0 {
                        first_top.set(ui.next_widget_position().y);
                    }
                    Block::new(Vec2::new(100.0, ROW_H)).show(ui);
                }
            });
            panel_rect.set(out.output.rect);
        });
        in_region(ui, Rect::from_min_size(Pos2::new(350.0, 10.0), VIEW), |ui| {
            let list = List::new(SelectMode::None).id_salt("list").show_items(ui, &words, &mut |key, cx| {
                if *key == 3 {
                    row.set(cx.rect);
                }
                if *key == ROWS - 1 {
                    last_row.set(cx.rect);
                }
                Row::new()
            });
            seen.take(list);
        });
    };
    frames.idle(3, &mut show);
    let (panel, list) = (panel_rect.get(), seen.rect.get());
    let (top0, row0) = (first_top.get(), row.get().top());

    frames.frame(vec![pointer::moved(panel.center())], &mut show);
    frames.frame(vec![pointer::wheel(-WHEEL)], &mut show);
    assert!((top0 - first_top.get() - WHEEL).abs() < SAME, "the body moved by the wheel's points: {top0} -> {}", first_top.get());
    assert_eq!(seen.range().start, 0, "the list did not take the panel's wheel");

    frames.frame(vec![pointer::moved(list.center())], &mut show);
    frames.frame(vec![pointer::wheel(-WHEEL)], &mut show);
    assert!((row0 - row.get().top() - WHEEL).abs() < SAME, "the list moved by the same points: {row0} -> {}", row.get().top());
    assert!((top0 - first_top.get() - WHEEL).abs() < SAME, "the panel did not take the list's wheel");

    for _ in 0..3 {
        frames.frame(vec![pointer::moved(panel.center())], &mut show);
        frames.frame(vec![pointer::wheel(-100_000.0)], &mut show);
        frames.frame(vec![pointer::moved(list.center())], &mut show);
        frames.frame(vec![pointer::wheel(-100_000.0)], &mut show);
    }
    assert_eq!(seen.range().end, ROWS, "the list reached its last row");
    assert!((last_row.get().bottom() - list.bottom()).abs() < SAME, "the list stopped at its end, not past it: {:?} in {list:?}", last_row.get());
}

/// A natural-height list in a scrolling page: the page and the list under one wheel, and the keys.
struct Page {
    seen: Seen,
    panel: Cell<Rect>,
    first_top: Cell<f32>,
    last_row: Cell<Rect>,
}

impl Page {
    fn new() -> Self {
        Page { seen: Seen::default(), panel: Cell::new(Rect::NOTHING), first_top: Cell::new(0.0), last_row: Cell::new(Rect::NOTHING) }
    }

    fn show(&self, ui: &mut Ui, words: &Count) {
        use crate::body::kit::Slab;
        use craft_ui::contract::{ContainerProps, Scroll, ShowContainer as _};
        use craft_ui::testing::layout::Block;
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let out = Slab::new(ContainerProps::new().scroll(Scroll::vertical())).id_salt("page").show(ui, |ui, _| {
                self.first_top.set(ui.next_widget_position().y);
                Block::new(Vec2::new(100.0, ROW_H)).show(ui);
                let list = List::new(SelectMode::Single).natural_height().id_salt("list").show_items(ui, words, &mut |key, cx| {
                    if *key == ROWS - 1 {
                        self.last_row.set(cx.rect);
                    }
                    Row::new()
                });
                self.seen.take(list);
            });
            self.panel.set(out.output.rect);
        });
    }
}

/// Every row of a natural-height list is built, though the page shows a part of the list.
#[trial]
fn a_natural_height_list_builds_every_row() {
    let (words, page) = (Count(ROWS), Page::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| page.show(ui, &words);
    frames.idle(3, &mut show);
    assert_eq!(page.seen.range(), 0..ROWS, "every row is built");
    assert!(page.last_row.get().min.y > page.panel.get().max.y, "the last row lies below the page that shows the list");
}

/// The wheel over a natural-height list scrolls the page; the list has no scroll of its own.
#[trial]
fn the_wheel_over_a_natural_height_list_scrolls_the_page() {
    let (words, page) = (Count(ROWS), Page::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| page.show(ui, &words);
    frames.idle(3, &mut show);
    let top0 = page.first_top.get();
    frames.frame(vec![pointer::moved(page.panel.get().center())], &mut show);
    frames.frame(vec![pointer::wheel(-WHEEL)], &mut show);
    assert!((top0 - page.first_top.get() - WHEEL).abs() < SAME, "the page moved by the wheel's points: {top0} -> {}", page.first_top.get());
}

/// Moving the lead to the last row scrolls the page until the row shows.
#[trial]
fn keys_in_a_natural_height_list_scroll_the_page_to_the_lead() {
    let (words, page) = (Count(ROWS), Page::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| page.show(ui, &words);
    frames.idle(2, &mut show);
    focus_list(&mut frames, &page.seen, &mut show);
    assert!(page.last_row.get().min.y > page.panel.get().max.y, "the last row starts below the page");
    press(&mut frames, Key::End, Modifiers::NONE, &mut show);
    frames.idle(3, &mut show);
    let (row, panel) = (page.last_row.get(), page.panel.get());
    assert!(row.min.y >= panel.min.y - SAME && row.max.y <= panel.max.y + SAME, "the last row is inside the page: {row:?} in {panel:?}");
}

/// Shift+F10 asks for the focused row's context menu once, below that row; nothing opens without a lead.
#[trial]
fn shift_f10_opens_the_focused_rows_context_menu() {
    let words = Words::flat(&["a", "b", "c"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single), &words, &seen);
    frames.idle(2, &mut show);
    focus_list(&mut frames, &seen, &mut show);
    let menus = || -> Vec<(usize, Pos2)> {
        seen.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::ContextMenu(request) = event { Some((request.key, request.at)) } else { None })
            .collect()
    };
    press(&mut frames, Key::F10, Modifiers::SHIFT, &mut show);
    assert!(menus().is_empty(), "no lead yet, no menu");
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    press(&mut frames, Key::F10, Modifiers::SHIFT, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    press(&mut frames, Key::F10, Modifiers::SHIFT, &mut show);
    let asked = menus();
    assert_eq!(asked.iter().map(|menu| menu.0).collect::<Vec<_>>(), [0, 1], "one request per chord, for the lead at the time");
    assert!(asked[1].1.y > asked[0].1.y, "the request sits below its own row: {asked:?}");
    assert!(
        seen.rect.get().contains_rect(Rect::from_min_size(asked[0].1, Vec2::ZERO)) || seen.rect.get().contains(asked[0].1),
        "inside the list: {asked:?}"
    );
    press(&mut frames, Key::F10, Modifiers::NONE, &mut show);
    assert_eq!(menus().len(), 2, "F10 alone opens nothing");
}

/// One tab stop; the lead moves by arrows, Home and End, Page and type-ahead; selection by Space, Shift and Ctrl.
#[trial]
fn a_list_with_one_tab_stop_moves_its_lead_and_selects() {
    let words = Words::flat(&labels());
    let seen = Seen::default();
    let selection = RefCell::new(Selection::<usize>::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut held = selection.borrow().clone();
        let list = List::new(SelectMode::Multi).selection(Bind::bound(&mut held));
        show_words(ui, list, &words, &seen);
        *selection.borrow_mut() = held;
    };
    frames.idle(2, &mut show);
    focus_list(&mut frames, &seen, &mut show);
    let last = |seen: &Seen| *seen.leads().last().expect("the lead moved");

    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    assert_eq!(last(&seen), 1);
    assert_eq!(craft_ui::testing::focus::egui_focused(&frames.ctx), Some(seen.id()), "the rows take no focus: the list is the one stop");
    press(&mut frames, Key::End, Modifiers::NONE, &mut show);
    assert_eq!(last(&seen), 5);
    press(&mut frames, Key::Home, Modifiers::NONE, &mut show);
    assert_eq!(last(&seen), 0);
    press(&mut frames, Key::PageDown, Modifiers::NONE, &mut show);
    assert!(last(&seen) > 1, "a page moves several rows: {:?}", seen.leads());
    frames.frame(vec![Event::Text("c".into())], &mut show);
    assert_eq!(last(&seen), 4, "type-ahead picks the word starting with the letter");

    assert_eq!(selection.borrow().items, vec![4], "arrows, Home, End, Page and type-ahead move the selection with the lead");
    press(&mut frames, Key::ArrowUp, Modifiers::COMMAND, &mut show);
    assert_eq!((selection.borrow().items.clone(), last(&seen)), (vec![4], 3), "Ctrl moves only the lead");
    press(&mut frames, Key::Space, Modifiers::NONE, &mut show);
    let mut items = selection.borrow().items.clone();
    items.sort_unstable();
    assert_eq!(items, vec![3, 4], "Space toggles the lead");
    press(&mut frames, Key::ArrowUp, Modifiers::SHIFT, &mut show);
    press(&mut frames, Key::ArrowUp, Modifiers::SHIFT, &mut show);
    let mut items = selection.borrow().items.clone();
    items.sort_unstable();
    assert_eq!(items, vec![1, 2, 3], "Shift extends from the anchor");
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    assert_eq!((selection.borrow().items.clone(), last(&seen)), (vec![2], 2), "a plain arrow replaces the range");
    assert!(seen.selections().len() >= 3);
}

/// Rename starts on F2, commits on Enter and cancels on Escape.
#[trial]
fn rename_starts_commits_and_cancels() {
    let words = Words::flat(&["one", "two"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).rename(), &words, &seen);
    frames.idle(2, &mut show);
    focus_list(&mut frames, &seen, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    press(&mut frames, Key::F2, Modifiers::NONE, &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![Event::Text("!".into())], &mut show);
    press(&mut frames, Key::Enter, Modifiers::NONE, &mut show);
    let renames = || -> Vec<String> {
        seen.events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                CollectionEvent::Rename(EditEvent::Begin { .. }) => Some("begin".to_owned()),
                CollectionEvent::Rename(EditEvent::Commit { after, .. }) => Some(format!("commit {}", after.name)),
                CollectionEvent::Rename(EditEvent::Cancel { reason, .. }) => Some(format!("cancel {reason:?}")),
                _ => None,
            })
            .collect()
    };
    assert_eq!(renames(), ["begin", "commit one!"]);
    assert_eq!(craft_ui::testing::focus::egui_focused(&frames.ctx), Some(seen.id()), "focus returns to the list");

    press(&mut frames, Key::F2, Modifiers::NONE, &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![Event::Text("x".into())], &mut show);
    press(&mut frames, Key::Escape, Modifiers::NONE, &mut show);
    assert_eq!(renames(), ["begin", "commit one!", "begin", format!("cancel {:?}", CancelReason::Escape).as_str()]);
}

/// A double click on the rename column opens the row's editor and does not activate it; on another column it activates.
#[trial]
fn a_double_click_on_the_rename_column_renames_and_elsewhere_activates() {
    use craft_behavior::ColumnKey;
    let words = Words::flat(&["one", "two"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show =
        |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).rename_column(ColumnKey::new("name"), ColumnKey::new("other")), &words, &seen);
    frames.idle(3, &mut show);
    let y = seen.rect.get().min.y + ROW_H / 2.0;
    let count =
        |seen: &Seen, wanted: fn(&CollectionEvent<usize, Dismissed>) -> bool| seen.events.borrow().iter().filter(|event| wanted(event)).count();
    let double = |frames: &mut Frames, at: Pos2, show: &mut dyn FnMut(&mut Ui)| {
        frames.frame(vec![pointer::moved(at)], &mut *show);
        for _ in 0..2 {
            frames.frame(vec![pointer::press(at)], &mut *show);
            frames.frame(vec![pointer::release(at)], &mut *show);
        }
        frames.idle(2, &mut *show);
    };
    double(&mut frames, Pos2::new(seen.rect.get().min.x + 150.0, y), &mut show);
    assert_eq!(count(&seen, |event| matches!(event, CollectionEvent::Activate(_))), 1, "the other column activates");
    assert_eq!(count(&seen, |event| matches!(event, CollectionEvent::Rename(_))), 0);
    double(&mut frames, Pos2::new(seen.rect.get().min.x + 50.0, y), &mut show);
    assert_eq!(count(&seen, |event| matches!(event, CollectionEvent::Rename(EditEvent::Begin { .. }))), 1, "the name column renames");
    assert_eq!(count(&seen, |event| matches!(event, CollectionEvent::Activate(_))), 1, "and does not activate");
}

/// A rename requested for a key opens that row's rename field at the next pass, with no key press on the list.
#[trial]
fn a_requested_rename_opens_the_rows_field() {
    let words = Words::flat(&["one", "two", "three"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).rename(), &words, &seen);
    frames.idle(2, &mut show);
    craft_ui::contract::request_rename(&frames.ctx, seen.id(), &2usize);
    frames.idle(2, &mut show);
    frames.frame(vec![Event::Text("!".into())], &mut show);
    press(&mut frames, Key::Enter, Modifiers::NONE, &mut show);
    let renames: Vec<String> = seen
        .events
        .borrow()
        .iter()
        .filter_map(|event| match event {
            CollectionEvent::Rename(EditEvent::Begin { .. }) => Some("begin".to_owned()),
            CollectionEvent::Rename(EditEvent::Commit { after, .. }) => Some(format!("commit {}", after.name)),
            _ => None,
        })
        .collect();
    assert_eq!(renames, ["begin", "commit three!"]);
    assert_eq!(seen.leads().last(), Some(&2), "the lead moved to the requested row");
}

/// An empty source shows the collection's empty state at its content rect and no rows.
#[trial]
fn an_empty_list_shows_its_empty_state() {
    let words = Words::flat(&[]);
    let (seen, empty) = (Seen::default(), Cell::new(Rect::NOTHING));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).on_empty(&empty), &words, &seen);
    frames.idle(3, &mut show);
    assert_eq!(empty.get(), seen.rect.get(), "the hook got the content rect");
    assert_eq!(seen.range(), 0..0);
}

/// The empty-state text of the collection's props is a label node of the list.
#[trial]
fn the_empty_state_text_is_a_label_node() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let _ = craft_ui::list::List::<usize>::new("Items")
                .empty("Nothing here")
                .show_items(ui, &Count(0), &mut |_, _| craft_ui::list::ListRow::new(""));
        });
    };
    frames.idle(3, &mut show);
    let tree = frames.tree();
    let label = tree.keys().into_iter().find(|key| tree.name_of(*key) == Some("Nothing here")).expect("the empty text is a node");
    assert_eq!(tree.role_of(label), Some(craft_ui::contract::Role::LABEL));
}

/// A collection that overrides the empty hooks shows its own empty state, not the text of its props beside it.
#[trial]
fn an_overridden_empty_state_replaces_the_empty_text() {
    let words = Words::flat(&[]);
    let seen = Seen::default();
    let empty = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).empty_text("Nothing here").on_empty(&empty), &words, &seen);
    frames.idle(3, &mut show);
    assert!(empty.get().is_positive(), "the override ran");
    let tree = frames.tree();
    assert!(tree.keys().into_iter().all(|key| tree.name_of(key) != Some("Nothing here")), "the props text is not shown beside it");
}

/// A tree opens on Right and closes on Left; Left again goes to the parent.
#[trial]
fn a_tree_expands_and_collapses_by_key() {
    let mut words = Words::flat(&["root", "child", "leaf"]);
    words.tree = vec![(0, true), (1, true), (2, false)];
    words.open = RefCell::new(vec![false, false, false]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).tree(), &words, &seen);
    frames.idle(2, &mut show);
    focus_list(&mut frames, &seen, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    let opens = || -> Vec<(usize, bool)> {
        seen.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::Open(change) = event { Some((change.key, change.open)) } else { None })
            .collect()
    };
    press(&mut frames, Key::ArrowRight, Modifiers::NONE, &mut show);
    assert_eq!(opens(), [(0, true)]);
    words.open.borrow_mut()[0] = true;
    press(&mut frames, Key::ArrowLeft, Modifiers::NONE, &mut show);
    assert_eq!(opens(), [(0, true), (0, false)]);
}

/// A million rows: the last row places and hits exactly through the runtime.
#[trial]
fn a_million_row_layout_places_and_hits_the_last_row_exactly() {
    const COUNT: usize = 1_048_576;
    let words = Count(COUNT);
    let (seen, last) = (Seen::default(), Cell::new(Rect::NOTHING));
    let last = &*Box::leak(Box::new(last));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let list = List::new(SelectMode::Single).id_salt("list").show_items(ui, &words, &mut |key, cx| {
                if *key == COUNT - 1 {
                    last.set(cx.rect);
                }
                Row::new()
            });
            seen.take(list);
        });
    };
    frames.idle(2, &mut show);
    focus_list(&mut frames, &seen, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    press(&mut frames, Key::End, Modifiers::NONE, &mut show);
    frames.idle(2, &mut show);
    assert_eq!(seen.range().end, COUNT);
    let view = seen.rect.get();
    assert_eq!(last.get().bottom(), view.bottom(), "the last row's edge is exact: {:?} in {view:?}", last.get());
    let click = last.get().center();
    pointer::click(&mut frames, click, &mut show);
    assert!(seen.leads().contains(&(COUNT - 1)), "a press at the last row's centre hits it");
}

/// A findings-style list on the contract only: a row's dismiss part reports an item event.
#[trial]
fn an_item_part_reports_through_the_collection() {
    let words = Words::flat(&["warn one", "warn two"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let rows = Box::leak(Box::new([Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING)]));
    let rows: &'static [Cell<Rect>; 2] = rows;
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let out = List::new(SelectMode::Single).id_salt("list").show_items(ui, &words, &mut |key, cx| {
                rows[*key].set(cx.rect);
                Row::new().dismissable()
            });
            seen.take(out);
        });
    };
    frames.idle(3, &mut show);
    let hit = rows[1].get().min + Vec2::splat(ROW_H / 2.0);
    pointer::click(&mut frames, hit, &mut show);
    let items: Vec<_> = seen
        .events
        .borrow()
        .iter()
        .filter_map(|event| if let CollectionEvent::Item(key, item) = event { Some((*key, item.clone())) } else { None })
        .collect();
    assert_eq!(items, [(1, Dismissed::Row)]);
}

/// A menu list starts with its first row as the lead, and the lead follows a moving pointer but not a still one.
#[trial]
fn a_menu_list_opens_on_its_first_row_and_follows_the_pointer() {
    let words = Words::flat(&["one", "two", "three"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).menu(), &words, &seen);
    frames.idle(3, &mut show);
    focus_list(&mut frames, &seen, &mut show);
    press(&mut frames, Key::Enter, Modifiers::NONE, &mut show);
    assert!(seen.events.borrow().iter().any(|event| matches!(event, CollectionEvent::Activate(0))), "Enter activates the first row");
    let over = |row: usize| ORIGIN + Vec2::new(50.0, ROW_H * (row as f32 + 0.5));
    frames.frame(vec![pointer::moved(over(0))], &mut show);
    frames.frame(vec![pointer::moved(over(2))], &mut show);
    assert_eq!(seen.leads(), [2], "the pointer moves the lead to the row under it");
    frames.idle(2, &mut show);
    assert_eq!(seen.selections().len(), 1, "the selection follows the lead as it does for the arrow keys");
    assert_eq!(seen.leads(), [2], "a still pointer leaves the lead alone");
}

/// Enter with Shift, Alt, Ctrl or Cmd held activates the lead row like a plain Enter; the component reads the held modifiers.
#[trial]
fn a_modified_enter_activates_the_lead_row() {
    for modifiers in [Modifiers::SHIFT, Modifiers::ALT, Modifiers::COMMAND, Modifiers::CTRL] {
        let words = Words::flat(&["one", "two"]);
        let seen = Seen::default();
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).menu(), &words, &seen);
        frames.idle(3, &mut show);
        focus_list(&mut frames, &seen, &mut show);
        press(&mut frames, Key::Enter, modifiers, &mut show);
        let activations = seen.events.borrow().iter().filter(|event| matches!(event, CollectionEvent::Activate(0))).count();
        assert_eq!(activations, 1, "{modifiers:?}+Enter activates the first row");
    }
}

/// A list sized by its content keeps the height of its empty state while it has no rows.
#[trial]
fn an_empty_content_sized_list_keeps_the_height_of_its_empty_state() {
    let words = Words::flat(&[]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).content_sized(), &words, &seen);
    frames.idle(4, &mut show);
    assert_eq!(seen.rect.get().height(), ROW_H, "the list is as tall as its empty state: {:?}", seen.rect.get());
}

/// A press on a row's click part leaves the selection alone and moves focus to the row (17.3 step 8).
#[trial]
fn a_press_on_an_item_part_moves_focus_to_the_row_and_does_not_select_it() {
    let words = Words::flat(&["warn one", "warn two"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let rows = Box::leak(Box::new([Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING)]));
    let rows: &'static [Cell<Rect>; 2] = rows;
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let out = List::new(SelectMode::Single).id_salt("list").show_items(ui, &words, &mut |key, cx| {
                rows[*key].set(cx.rect);
                Row::new().dismissable()
            });
            seen.take(out);
        });
    };
    frames.idle(3, &mut show);
    let hit = rows[1].get().min + Vec2::splat(ROW_H / 2.0);
    pointer::click(&mut frames, hit, &mut show);
    assert!(seen.selections().is_empty(), "the part's press does not select the row: {:?}", seen.selections());
    assert_eq!(seen.leads(), [1], "the row takes the lead");
    let beside = rows[1].get().min + Vec2::new(ROW_H * 3.0, ROW_H / 2.0);
    pointer::click(&mut frames, beside, &mut show);
    assert_eq!(seen.selections().len(), 1, "a press beside the part selects the row");
}

/// A text field renaming a row keeps Escape while its draft is open: the first Escape cancels the edit and focus
/// goes back to the list, not to the row before the cancel.
#[trial]
fn escape_reaches_a_text_part_with_an_open_draft_before_the_row() {
    let words = Words::flat(&["one", "two"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).rename().text_editor(), &words, &seen);
    frames.idle(2, &mut show);
    focus_list(&mut frames, &seen, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    press(&mut frames, Key::F2, Modifiers::NONE, &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![Event::Text("x".into())], &mut show);
    frames.idle(1, &mut show);
    press(&mut frames, Key::Escape, Modifiers::NONE, &mut show);
    let cancels = seen
        .events
        .borrow()
        .iter()
        .filter(|event| matches!(event, CollectionEvent::Rename(EditEvent::Cancel { reason: CancelReason::Escape, .. })))
        .count();
    assert_eq!(cancels, 1, "the text part took the first Escape");
}

/// A right-click on a row asks for its menu on the release, selects the row when it was not selected, and names it.
#[trial]
fn a_secondary_release_on_a_row_asks_for_its_context_menu() {
    let words = Words::flat(&["a", "b", "c"]);
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single), &words, &seen);
    frames.idle(3, &mut show);
    let at = ORIGIN + Vec2::new(30.0, ROW_H * 1.5);
    frames.frame(vec![pointer::moved(at)], &mut show);
    frames.frame(vec![pointer::secondary(at, true)], &mut show);
    assert!(!seen.events.borrow().iter().any(|event| matches!(event, CollectionEvent::ContextMenu(_))), "nothing opens on the press");
    frames.frame(vec![pointer::secondary(at, false)], &mut show);
    frames.idle(1, &mut show);
    let menus: Vec<usize> = seen
        .events
        .borrow()
        .iter()
        .filter_map(|event| if let CollectionEvent::ContextMenu(request) = event { Some(request.key) } else { None })
        .collect();
    assert_eq!(menus, [1]);
    assert!(seen.leads().contains(&1), "the row under the pointer holds the lead");
}

/// A right-click on a row of a multiple selection keeps the whole selection, so the menu acts on all of it; on a row
/// outside the selection it selects that row alone.
#[trial]
fn a_secondary_press_on_a_selected_row_keeps_the_multiple_selection() {
    let words = Words::flat(&["a", "b", "c", "d"]);
    let seen = Seen::default();
    let selection = RefCell::new(Selection::<usize>::default());
    selection.borrow_mut().items = vec![0, 1];
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut held = selection.borrow().clone();
        show_words(ui, List::new(SelectMode::Multi).selection(Bind::bound(&mut held)), &words, &seen);
        *selection.borrow_mut() = held;
    };
    frames.idle(3, &mut show);
    fn right_click(frames: &mut Frames, show: &mut impl FnMut(&mut Ui), row: f32) {
        let at = ORIGIN + Vec2::new(30.0, ROW_H * row);
        frames.frame(vec![pointer::moved(at)], &mut *show);
        frames.frame(vec![pointer::secondary(at, true)], &mut *show);
        frames.frame(vec![pointer::secondary(at, false)], &mut *show);
        frames.idle(1, &mut *show);
    }
    right_click(&mut frames, &mut show, 1.5);
    let mut items = selection.borrow().items.clone();
    items.sort_unstable();
    assert_eq!(items, vec![0, 1], "the menu's row is in the selection: all of it stays");
    right_click(&mut frames, &mut show, 3.5);
    assert_eq!(selection.borrow().items, vec![3], "a row outside the selection replaces it");
}

/// A plain click on a row asks for its menu on the release when the list is set to, and not otherwise.
#[trial]
fn a_plain_click_on_a_row_asks_for_its_menu_only_when_the_list_is_set_to() {
    let menus_after_click = |on: bool| {
        let words = Words::flat(&["a", "b", "c"]);
        let seen = Seen::default();
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).menu_on_click(on), &words, &seen);
        frames.idle(3, &mut show);
        let at = ORIGIN + Vec2::new(30.0, ROW_H * 1.5);
        frames.frame(vec![pointer::moved(at)], &mut show);
        frames.frame(vec![pointer::press(at)], &mut show);
        assert!(!seen.events.borrow().iter().any(|event| matches!(event, CollectionEvent::ContextMenu(_))), "nothing opens on the press");
        frames.frame(vec![pointer::release(at)], &mut show);
        frames.idle(1, &mut show);
        seen.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::ContextMenu(request) = event { Some(request.key) } else { None })
            .collect::<Vec<usize>>()
    };
    assert_eq!(menus_after_click(true), [1], "the click names the row under the pointer");
    assert!(menus_after_click(false).is_empty(), "a list that is not set to takes a click as a pick only");
}

/// A press on a row of a popup list whose opener keeps focus picks the row while the opener's text field stays
/// focused with its draft open.
#[trial]
fn a_click_in_a_popup_list_picks_while_the_field_keeps_focus() {
    let words = Words::flat(&labels());
    let seen = Seen::default();
    let text = RefCell::new(String::from("a"));
    let field = Cell::new((Id::NULL, Rect::NOTHING));
    let commits = Cell::new(0);
    let list_at = Pos2::new(10.0, 100.0);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, Vec2::new(200.0, 30.0)), |ui| {
            let out = craft_ui::text::TextField::new("Name", Bind::bound(&mut *text.borrow_mut())).id_salt("name").show(ui);
            field.set((out.response.id, out.response.rect));
            if matches!(out.event, Some(EditEvent::Commit { .. })) {
                commits.set(commits.get() + 1);
            }
        });
        in_region(ui, Rect::from_min_size(list_at, VIEW), |ui| {
            seen.take(List::new(SelectMode::Single).keep_focus().id_salt("list").show_items(ui, &words, &mut |_, _| Row::new()));
        });
    };
    frames.idle(3, &mut show);
    pointer::click(&mut frames, field.get().1.center(), &mut show);
    frames.frame(vec![Event::Text("b".into())], &mut show);
    frames.idle(2, &mut show);
    let widget = widget_id(field.get().0, PartKey::new("text"));
    assert_eq!(egui_focused(&frames.ctx), Some(widget), "the field has focus and a draft before the pick");
    let before = commits.get();
    pointer::click(&mut frames, list_at + Vec2::new(50.0, ROW_H * 1.5), &mut show);
    assert_eq!(seen.selections(), [SelectionChange::Set(vec![1])], "the press picks the row");
    assert_eq!(egui_focused(&frames.ctx), Some(widget), "focus stays in the field");
    assert_eq!(commits.get(), before, "the field does not blur or commit before the pick");
}

/// A click on the selected row of a single list that allows an empty selection clears it on release; a click on
/// another row picks that row.
#[trial]
fn a_click_on_the_selected_row_of_a_single_list_that_allows_empty_clears_it() {
    let words = Words::flat(&labels());
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_words(ui, List::new(SelectMode::Single).allow_empty(), &words, &seen);
    frames.idle(3, &mut show);
    let over = |row: usize| ORIGIN + Vec2::new(50.0, ROW_H * (row as f32 + 0.5));
    pointer::click(&mut frames, over(1), &mut show);
    assert_eq!(seen.selections(), [SelectionChange::Set(vec![1])], "a click on an unselected row picks it");
    pointer::click(&mut frames, over(1), &mut show);
    assert_eq!(seen.selections(), [SelectionChange::Set(vec![1]), SelectionChange::Clear], "a click on the selected row empties the selection");
    pointer::click(&mut frames, over(2), &mut show);
    assert_eq!(seen.selections().last(), Some(&SelectionChange::Set(vec![2])), "a click on another row picks it");
}
