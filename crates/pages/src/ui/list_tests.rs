//! The page and artboard list.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::behavior::Selection;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::list::*;

struct App {
    rows: Vec<PageListRow>,
    picked: Selection<usize>,
    active: usize,
    events: Vec<PageListEvent>,
}

fn open(locked: &[usize], picked: &[usize]) -> Harness<'static, App> {
    let rows = (0..6).map(|i| PageListRow { name: format!("Board {}", i + 1), locked: locked.contains(&i) }).collect();
    let picked = Selection { items: picked.to_vec(), lead: picked.first().copied(), anchor: picked.first().copied() };
    let app = App { rows, picked, active: 0, events: Vec::new() };
    let mut harness =
        HarnessBuilder::default().with_size(vec2(320.0, 400.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let out = PageList::new("boards", &app.rows, &mut app.picked, app.active).show(ui);
                for event in out.events {
                    if let PageListEvent::Activate(row) = event {
                        app.active = row;
                    }
                    app.events.push(event);
                }
            },
            app,
        );
    harness.run();
    harness
}

fn press(harness: &mut Harness<'_, App>, key: Key) {
    harness.key_press(key);
    harness.run();
}

// A click reports Picked then Activate for row 2; Shift-click on row 5 picks 2–5; Cmd/Ctrl-click on 3 unpicks it.
#[test]
fn clicks_activate_and_pick_ranges() {
    let mut harness = open(&[], &[0]);
    harness.get_by_label("Board 3").click();
    harness.run();
    let events = &harness.state().events;
    let at = |wanted: &dyn Fn(&PageListEvent) -> bool| events.iter().position(wanted);
    let picked = at(&|e| matches!(e, PageListEvent::Picked(change) if change.row == Some(2))).expect("picked");
    let activate = at(&|e| matches!(e, PageListEvent::Activate(2))).expect("activated");
    assert!(picked < activate, "{events:?}");
    assert_eq!(harness.state().active, 2);

    harness.get_by_label("Board 6").click_modifiers(Modifiers::SHIFT);
    harness.run();
    let mut items = harness.state().picked.items.clone();
    items.sort_unstable();
    assert_eq!(items, [2, 3, 4, 5]);

    harness.get_by_label("Board 4").click_modifiers(Modifiers::COMMAND);
    harness.run();
    assert!(!harness.state().picked.items.contains(&3));
    assert!(harness.state().picked.items.contains(&4));
}

// F2 on the lead, typing "Cover" and Enter reports Rename { row, text: "Cover" } once; Esc reports nothing.
#[test]
fn rename_in_place_goes_to_every_picked_row() {
    let mut harness = open(&[], &[1, 2]);
    harness.get_by_role(Role::ListBox).focus();
    harness.run();
    press(&mut harness, Key::F2);
    harness.get_by_label("Name").focus();
    harness.run();
    harness.event(Event::Text("Cover".into()));
    harness.run();
    press(&mut harness, Key::Enter);
    let renames: Vec<_> = harness.state().events.iter().filter(|e| matches!(e, PageListEvent::Rename { .. })).collect();
    assert_eq!(renames.len(), 1, "{renames:?}");
    assert!(matches!(renames[0], PageListEvent::Rename { text, .. } if text == "Cover"));

    harness.state_mut().events.clear();
    press(&mut harness, Key::F2);
    harness.event(Event::Text("Other".into()));
    harness.run();
    press(&mut harness, Key::Escape);
    assert!(harness.state().events.iter().all(|e| !matches!(e, PageListEvent::Rename { .. })), "{:?}", harness.state().events);
}

// Delete with rows 1–3 picked and row 2 locked reports Delete([1, 3]); a row dragged onto New reports New { duplicate: Some(row), .. }.
#[test]
fn delete_leaves_locked_rows() {
    let mut harness = open(&[2], &[1, 2, 3]);
    harness.get_by_role(Role::ListBox).focus();
    harness.run();
    press(&mut harness, Key::Delete);
    assert!(harness.state().events.contains(&PageListEvent::Delete(vec![1, 3])), "{:?}", harness.state().events);

    harness.state_mut().events.clear();
    let start = harness.get_by_label("Board 5").rect().center();
    let new = harness.get_by_label("New Artboard").rect().center();
    harness.event(Event::PointerMoved(start));
    harness.run();
    harness.event(Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    for at in [start + vec2(0.0, 6.0), start + vec2(0.0, 20.0), new] {
        harness.event(Event::PointerMoved(at));
        harness.run();
    }
    harness.event(Event::PointerButton { pos: new, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();
    assert!(
        harness.state().events.iter().any(|e| matches!(e, PageListEvent::New { duplicate: Some(4), options: false })),
        "{:?}",
        harness.state().events
    );
}
