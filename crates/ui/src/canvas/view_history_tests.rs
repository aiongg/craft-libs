//! Go Back and Go Forward.

use kurbo::Point;

use super::{ViewHistory, ViewPlace};

fn at(key: u32) -> ViewPlace<u32> {
    ViewPlace { key, center: Point::ZERO, zoom: 1.0 }
}

fn visited(keys: &[u32]) -> ViewHistory<u32> {
    let mut history = ViewHistory::default();
    for &key in keys {
        history.visit(at(key));
    }
    history
}

fn key(place: Option<ViewPlace<u32>>) -> Option<u32> {
    place.map(|p| p.key)
}

const ALL: fn(&u32) -> bool = |_| true;

// Visiting 1, 2, 3 then going back twice shows 1 and forward twice shows 3.
#[test]
fn going_back_and_forward_retraces_the_places() {
    let mut history = visited(&[1, 2, 3]);
    assert!(!history.can_go(1, ALL));
    assert_eq!(key(history.go(-1, ALL)), Some(2));
    assert_eq!(key(history.go(-1, ALL)), Some(1));
    assert_eq!(history.go(-1, ALL), None);
    assert_eq!(key(history.go(1, ALL)), Some(2));
    assert_eq!(key(history.go(1, ALL)), Some(3));
    assert_eq!(history.go(1, ALL), None);
}

// Leave records centre and zoom; Go Back returns to them.
#[test]
fn an_entry_keeps_the_place_a_jump_left() {
    let mut history = visited(&[1, 2]);
    let left = ViewPlace { key: 2, center: Point::new(-200.0, 30.0), zoom: 2.5 };
    history.leave(left);
    history.visit(at(3));
    assert_eq!(history.go(-1, ALL), Some(left));
    assert_eq!(key(history.go(-1, ALL)), Some(1), "scrolling added no entry");
}

// After back then a new visit, can_go(1) is false.
#[test]
fn visiting_after_going_back_clears_go_forward() {
    let mut history = visited(&[1, 2, 3]);
    history.go(-1, ALL);
    history.visit(at(4));
    assert!(!history.can_go(1, ALL));
    assert_eq!(key(history.go(-1, ALL)), Some(2));
}

// A deleted page is skipped by can_go and go.
#[test]
fn places_that_no_longer_exist_are_skipped() {
    let mut history = visited(&[1, 2, 3]);
    let alive = |k: &u32| *k != 2;
    assert!(history.can_go(-1, alive));
    assert_eq!(key(history.go(-1, alive)), Some(1));
    let only_three = |k: &u32| *k == 3;
    assert!(!history.can_go(-1, only_three));
    assert_eq!(key(history.go(1, only_three)), Some(3));
}

// After 300 visits 256 entries remain, the oldest dropped.
#[test]
fn the_history_keeps_the_newest_entries() {
    let mut history = visited(&(0..300).collect::<Vec<_>>());
    let mut steps = 0;
    let mut oldest = 299;
    while let Some(place) = history.go(-1, ALL) {
        steps += 1;
        oldest = place.key;
    }
    assert_eq!(steps, ViewHistory::<u32>::LIMIT - 1);
    assert_eq!(oldest, 300 - ViewHistory::<u32>::LIMIT as u32);
}
