//! A row's thumbnail click reports its press count, as a button's output does.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::{CollectionEvent, SelectMode, ShowCollection as _};
use craft_ui::list::{List, ListRow, RowEvent, ThumbSize};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Pos2, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct One;

impl DisplayOrder<usize> for One {
    fn len(&self) -> usize {
        1
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for One {}

/// A one-row list with a thumbnail, the thumbnail's rect, and the click counts its thumbnail reported.
fn rig() -> (Fixture, Rc<Cell<Rect>>, Rc<RefCell<Vec<u32>>>) {
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (place, sink) = (rect.clone(), seen.clone());
    let mut fixture = Fixture::new(move |ui| {
        let place = place.clone();
        let out = List::new("Items").select(SelectMode::Single).show_items(ui, &One, &mut |_, _| {
            let place = place.clone();
            ListRow::new("Row").thumb(ThumbSize::Medium, move |_, at| place.set(at))
        });
        for event in out.events {
            if let CollectionEvent::Item(_, RowEvent::ThumbClicked { clicks, .. }) = event {
                sink.borrow_mut().push(clicks);
            }
        }
    });
    fixture.settle();
    (fixture, rect, seen)
}

fn press_release(fixture: &mut Fixture, at: Pos2, times: usize) {
    fixture.frame(vec![pointer::moved(at)]);
    for _ in 0..times {
        fixture.frame(vec![pointer::press(at)]);
        fixture.frame(vec![pointer::release(at)]);
    }
    fixture.idle(2);
}

/// A single click on a thumbnail reports one click; a double click reports the second click as two.
#[trial]
fn a_double_click_on_a_thumbnail_reports_two_clicks() {
    let (mut fixture, rect, seen) = rig();
    let at = rect.get().center();
    press_release(&mut fixture, at, 1);
    assert_eq!(*seen.borrow(), vec![1], "a single click counts one");
    seen.borrow_mut().clear();
    fixture.advance(2.0);
    press_release(&mut fixture, at, 2);
    assert_eq!(*seen.borrow(), vec![1, 2], "the second click of a double click counts two");
}
