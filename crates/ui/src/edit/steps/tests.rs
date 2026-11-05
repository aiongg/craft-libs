//! Behaviour tests of the step list core.

use super::{DeleteRefused, HistoryStep, StepId, StepList};

fn step(label: &str) -> HistoryStep {
    HistoryStep::new(label)
}

/// A list of `n` states: "Open", then "s1" to "s{n-1}"; the last is current.
fn list_of(n: usize) -> (StepList, Vec<StepId>) {
    let mut list = StepList::new(step("Open"));
    let mut ids = vec![list.current_id()];
    for i in 1..n {
        ids.push(list.record(step(&format!("s{i}"))).id);
    }
    (list, ids)
}

fn ids_of(list: &StepList) -> Vec<StepId> {
    (0..list.len()).map(|at| list.id_at(at).unwrap()).collect()
}

// A record makes the new state current and returns a new id, never one used before.
#[test]
fn record_adds_one_state_after_the_current_one() {
    let (mut list, ids) = list_of(3);
    assert_eq!(list.len(), 3);
    assert_eq!(list.current_index(), 2);
    assert!(ids.windows(2).all(|w| w[0] != w[1]));
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 3);
    let recorded = list.record(step("next"));
    assert!(!recorded.extended);
    assert!(!ids.contains(&recorded.id));
    assert_eq!(list.current_id(), recorded.id);
    assert_eq!(list.label_of(recorded.id), Some("next"));
}

// After back, back, record: the two later ids come out of `take_dropped` once, and a second call is empty.
#[test]
fn linear_record_drops_later_states_and_reports_them() {
    let (mut list, ids) = list_of(4);
    list.back();
    list.back();
    list.take_dropped();
    list.record(step("new"));
    assert_eq!(list.take_dropped(), vec![ids[2], ids[3]]);
    assert!(list.take_dropped().is_empty());
    assert_eq!(list.len(), 3);
}

// Non-linear: back, record keeps the later rows in order and puts the new state at the end, current.
#[test]
fn non_linear_record_keeps_later_states_before_the_new_one() {
    let (mut list, ids) = list_of(4);
    list.set_non_linear(true);
    list.back();
    list.back();
    let new = list.record(step("new")).id;
    assert_eq!(ids_of(&list), vec![ids[0], ids[1], ids[2], ids[3], new]);
    assert_eq!(list.current_id(), new);
    assert!(list.take_dropped().is_empty());
}

// `go_to` back and forward returns the `Jump`; no id is dropped; `go_to` the current id is `None`.
#[test]
fn go_to_moves_the_current_state_and_keeps_later_states() {
    let (mut list, ids) = list_of(4);
    let jump = list.go_to(ids[1]).unwrap();
    assert_eq!((jump.from, jump.to), (ids[3], ids[1]));
    assert_eq!(list.later_len(), 2);
    assert!(list.go_to(ids[1]).is_none());
    let jump = list.go_to(ids[3]).unwrap();
    assert_eq!((jump.from, jump.to), (ids[1], ids[3]));
    assert!(list.take_dropped().is_empty());
    assert!(list.forward().is_none());
}

// With limit 3 and the oldest pinned, a fifth record drops the next oldest, never the pinned or current state.
#[test]
fn limit_drops_the_oldest_past_state_but_not_the_current_or_the_source() {
    let mut list = StepList::new(step("Open"));
    let open = list.current_id();
    list.set_limit(3);
    list.pin_source(Some(open));
    let a = list.record(step("a")).id;
    let b = list.record(step("b")).id;
    let c = list.record(step("c")).id;
    assert!(list.take_dropped().is_empty());
    let d = list.record(step("d")).id;
    assert_eq!(list.take_dropped(), vec![a]);
    assert_eq!(ids_of(&list), vec![open, b, c, d]);
    assert_eq!(list.current_id(), d);
    assert_eq!(list.source(), Some(open));
}

// `evict_oldest` skips the pinned source, returns `None` once only the current and pinned states are left.
#[test]
fn evict_oldest_spares_the_pinned_source_and_the_current_state() {
    let (mut list, ids) = list_of(4);
    list.pin_source(Some(ids[0]));
    assert_eq!(list.evict_oldest(), Some(ids[1]));
    assert_eq!(list.evict_oldest(), Some(ids[2]));
    assert_eq!(list.evict_oldest(), None);
    assert_eq!(ids_of(&list), vec![ids[0], ids[3]]);
    assert_eq!(list.take_dropped(), vec![ids[1], ids[2]]);
}

// Linear: deleting row 2 of 5 (current 4) leaves rows 0 and 1, row 1 current, three ids dropped.
#[test]
fn delete_in_linear_history_drops_the_state_and_everything_after_it() {
    let (mut list, ids) = list_of(5);
    let deleted = list.delete(ids[2]).unwrap();
    assert_eq!(deleted.now, ids[1]);
    assert_eq!(ids_of(&list), vec![ids[0], ids[1]]);
    assert_eq!(list.current_index(), 1);
    assert_eq!(list.take_dropped(), vec![ids[2], ids[3], ids[4]]);
}

// Non-linear: deleting a later state drops only it; deleting the current state makes the one before it current.
#[test]
fn delete_in_non_linear_history_drops_only_that_state() {
    let (mut list, ids) = list_of(5);
    list.set_non_linear(true);
    list.go_to(ids[2]);
    list.delete(ids[4]).unwrap();
    assert_eq!(ids_of(&list), vec![ids[0], ids[1], ids[2], ids[3]]);
    assert_eq!(list.current_id(), ids[2]);
    let deleted = list.delete(ids[2]).unwrap();
    assert_eq!(deleted.now, ids[1]);
    assert_eq!(ids_of(&list), vec![ids[0], ids[1], ids[3]]);
    assert_eq!(list.take_dropped(), vec![ids[4], ids[2]]);
}

// `delete(oldest)` is `Err(Oldest)` and changes nothing, revision included.
#[test]
fn the_oldest_state_cannot_be_deleted() {
    let (mut list, ids) = list_of(3);
    let revision = list.revision();
    assert_eq!(list.delete(ids[0]), Err(DeleteRefused::Oldest));
    assert_eq!(list.delete(StepId::new(999)), Err(DeleteRefused::Unknown));
    assert_eq!(list.revision(), revision);
    assert_eq!(list.len(), 3);
}

// Linear: `delete(later)` is `Err(Later)`.
#[test]
fn a_later_state_cannot_be_deleted_in_linear_history() {
    let (mut list, ids) = list_of(3);
    list.back();
    assert_eq!(list.delete(ids[2]), Err(DeleteRefused::Later));
    assert_eq!(list.len(), 3);
}

// Clear leaves one row and drops nothing; restore brings rows, current and pin back; after clear then record the set-aside ids are dropped.
#[test]
fn clear_keeps_only_the_current_state_and_restore_brings_the_rest_back() {
    let (mut list, ids) = list_of(4);
    list.back();
    list.pin_source(Some(ids[1]));
    list.clear();
    assert_eq!(ids_of(&list), vec![ids[2]]);
    assert!(list.take_dropped().is_empty());
    assert!(list.can_restore_cleared());
    assert!(list.restore_cleared());
    assert_eq!(ids_of(&list), ids);
    assert_eq!(list.current_id(), ids[2]);
    assert_eq!(list.pinned(), Some(ids[1]));
    assert!(!list.restore_cleared());

    list.clear();
    list.record(step("after"));
    let mut dropped = list.take_dropped();
    dropped.sort();
    assert_eq!(dropped, vec![ids[0], ids[1], ids[3]]);
    assert!(!list.can_restore_cleared());
}

// Clear keeps a pin on the current state, so the limit cannot evict it; a pin on another state is set aside.
#[test]
fn clear_keeps_a_pin_on_the_current_state() {
    let (mut list, ids) = list_of(4);
    list.back();
    list.pin_source(Some(ids[2]));
    list.clear();
    assert_eq!((list.pinned(), list.source()), (Some(ids[2]), Some(ids[2])));
    assert!(list.restore_cleared());
    assert_eq!(list.pinned(), Some(ids[2]));
}

// `source()` is the oldest without a pin; after deleting the pinned state it is `None`.
#[test]
fn source_defaults_to_the_oldest_state_and_is_gone_after_its_state_is_deleted() {
    let (mut list, ids) = list_of(4);
    assert_eq!(list.source(), Some(ids[0]));
    list.set_source(ids[2]);
    assert_eq!(list.source(), Some(ids[2]));
    list.delete(ids[2]).unwrap();
    assert_eq!(list.source(), None);
    assert_eq!(list.pinned(), None);
    list.pin_source(None);
    assert_eq!(list.source(), Some(ids[0]));
}

// Every call that changes the rows bumps `revision`; reads, refused calls, the pin and the mode do not.
#[test]
fn revision_changes_on_every_change_and_not_on_reads() {
    let (mut list, ids) = list_of(4);
    let mut last = list.revision();
    let mut bumped = |list: &StepList| {
        let now = list.revision();
        let changed = now != last;
        last = now;
        changed
    };
    list.back();
    assert!(bumped(&list));
    list.forward();
    assert!(bumped(&list));
    assert!(list.set_label(ids[1], "renamed"));
    assert!(bumped(&list));
    list.set_label(ids[1], "renamed");
    assert!(!bumped(&list));
    // The panel reads the mode and the source from the list; a bump would follow-scroll and deselect.
    list.set_non_linear(true);
    assert!(!bumped(&list));
    list.pin_source(Some(ids[1]));
    assert!(!bumped(&list));
    list.set_limit(2);
    assert!(bumped(&list));
    list.record(step("x"));
    assert!(bumped(&list));
    list.clear();
    assert!(bumped(&list));
    list.restore_cleared();
    assert!(bumped(&list));
    let _ = (list.len(), list.can_back(), list.source(), list.label_of(ids[0]));
    list.take_dropped();
    assert!(!bumped(&list));
    list.go_to(StepId::new(999));
    assert!(!bumped(&list));
}
