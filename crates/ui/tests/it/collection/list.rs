//! The list trials: wheel and thumb, keys and selection, rename, the empty state, trees and large sources.

use super::kit::{Count, Dismissed, List, Row, Words, ROW_H};
use craft_behavior::{CancelReason, EditEvent, Selection, SelectionChange};
use craft_ui::contract::{Show as _, Bind, CollectionEvent, CollectionResponse, Props as _, SelectMode, ShowCollection as _};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::testing::layout::in_region;
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
        Seen {
            id: Cell::new(None),
            rect: Cell::new(Rect::NOTHING),
            visible: Cell::new((0, 0)),
            events: RefCell::new(Vec::new()),
        }
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
        self.events.borrow().iter().filter_map(|event| if let CollectionEvent::LeadMoved(key) = event { Some(*key) } else { None }).collect()
    }

    fn selections(&self) -> Vec<SelectionChange<usize>> {
        self.events.borrow().iter().filter_map(|event| if let CollectionEvent::Selection(change) = event { Some(change.clone()) } else { None }).collect()
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
        seen.events.borrow().iter().filter_map(|event| if let CollectionEvent::ContextMenu(request) = event { Some((request.key, request.at)) } else { None }).collect()
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
    assert!(seen.rect.get().contains_rect(Rect::from_min_size(asked[0].1, Vec2::ZERO)) || seen.rect.get().contains(asked[0].1), "inside the list: {asked:?}");
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
        seen.events.borrow().iter().filter_map(|event| if let CollectionEvent::Open(change) = event { Some((change.key, change.open)) } else { None }).collect()
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
    let items: Vec<_> = seen.events.borrow().iter().filter_map(|event| if let CollectionEvent::Item(key, item) = event { Some((*key, item.clone())) } else { None }).collect();
    assert_eq!(items, [(1, Dismissed::Row)]);
}
