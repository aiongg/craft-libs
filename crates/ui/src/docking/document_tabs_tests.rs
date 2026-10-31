//! Document tab drags.

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Ui, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::document_tabs::{DocumentTab, DocumentTabEvent, DocumentTabs};
use crate::tab_strip::{TabKind, TabStrip};
use crate::theme::{Base, install};

/// Two document groups one above the other, far enough apart that the lower strip is outside the
/// upper strip's tear-off band.
struct Host {
    groups: [Vec<u32>; 2],
    events: Vec<DocumentTabEvent<u32>>,
}

fn show(ui: &mut Ui, host: &mut Host) {
    for (i, documents) in host.groups.iter().enumerate() {
        if documents.is_empty() {
            continue;
        }
        let tabs: Vec<_> = documents.iter().map(|&id| DocumentTab { id, title: format!("Doc {id}"), modified: false, closable: true }).collect();
        let strip = TabStrip::new(("strip", i), format!("Group {i}"), TabKind::Document);
        host.events.extend(DocumentTabs::new(i as u64 + 1, &tabs, tabs.first().map(|t| &t.id), strip).show(ui));
        ui.add_space(120.0);
    }
}

fn harness(groups: [Vec<u32>; 2]) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([480.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, Host { groups, events: Vec::new() });
    harness.run();
    harness.run();
    harness
}

fn tab_at(harness: &Harness<'_, Host>, label: &str, share: f32) -> Pos2 {
    let label = label.to_owned();
    let rect = harness.get_by(move |n| n.role() == Role::Tab && n.label().as_deref() == Some(label.as_str())).rect();
    pos2(rect.left() + rect.width() * share, rect.center().y)
}

fn pointer(harness: &mut Harness<'_, Host>, pos: Pos2, pressed: Option<bool>) {
    harness.event(Event::PointerMoved(pos));
    if let Some(pressed) = pressed {
        harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

/// Presses on `label`, drags to `to` in steps and releases there.
fn drag(harness: &mut Harness<'_, Host>, label: &str, to: Pos2) {
    let start = tab_at(harness, label, 0.5);
    pointer(harness, start, Some(true));
    pointer(harness, start + vec2(0.0, 8.0), None);
    pointer(harness, to, None);
    pointer(harness, to, Some(false));
}

fn drags(harness: &mut Harness<'_, Host>) -> Vec<DocumentTabEvent<u32>> {
    std::mem::take(&mut harness.state_mut().events).into_iter().filter(|e| !matches!(e, DocumentTabEvent::Selected(_))).collect()
}

#[test]
// Dragging a tab past its neighbour reorders; releasing over the canvas tears it off at the pointer.
fn drag_reorders_or_tears_off() {
    let mut harness = harness([vec![1, 2, 3], vec![]]);
    let to = tab_at(&harness, "Doc 2", 0.75);
    drag(&mut harness, "Doc 1", to);
    assert_eq!(drags(&mut harness), [DocumentTabEvent::Reordered { document: 1, before: Some(3) }]);

    let end = tab_at(&harness, "Doc 3", 0.9) + vec2(0.0, 0.0);
    drag(&mut harness, "Doc 3", end + vec2(30.0, 150.0));
    let events = drags(&mut harness);
    let [DocumentTabEvent::TornOff { document: 3, at }] = events.as_slice() else { panic!("expected a tear-off, got {events:?}") };
    assert_eq!(*at, end + vec2(30.0, 150.0) - vec2(40.0, 12.0));
}

#[test]
// Releasing over another group's strip emits DroppedInto with the insertion index under the pointer.
fn drop_into_other_group_strip() {
    let mut harness = harness([vec![1, 2], vec![10, 11]]);
    let strips = DocumentTabs::<u32>::strips(&harness.ctx);
    assert_eq!(strips.iter().map(|(g, _)| *g).collect::<std::collections::BTreeSet<_>>(), [1, 2].into());

    let into_start = tab_at(&harness, "Doc 11", 0.25);
    drag(&mut harness, "Doc 1", into_start);
    assert_eq!(drags(&mut harness), [DocumentTabEvent::DroppedInto { document: 1, group: 2, before: Some(11) }]);

    let after_last = tab_at(&harness, "Doc 11", 0.9);
    drag(&mut harness, "Doc 2", after_last);
    assert_eq!(drags(&mut harness), [DocumentTabEvent::DroppedInto { document: 2, group: 2, before: None }]);
}
