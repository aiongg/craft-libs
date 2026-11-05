//! The page and artboard list.
#![allow(clippy::expect_used, reason = "tests")]

use std::sync::Arc;

use craft_host::{AppInfo, Host};
use craft_i18n::{Embedded, I18n};
use craft_ui::behavior::Selection;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Shape, vec2};
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
    open_many(6, locked, picked)
}

fn open_many(count: usize, locked: &[usize], picked: &[usize]) -> Harness<'static, App> {
    let rows = (0..count).map(|i| PageListRow { name: format!("Board {}", i + 1), locked: locked.contains(&i) }).collect();
    let picked = Selection { items: picked.to_vec(), lead: picked.first().copied(), anchor: picked.first().copied() };
    let app = App { rows, picked, active: 0, events: Vec::new() };
    let mut harness =
        HarnessBuilder::default().with_size(vec2(320.0, 400.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let out = PageList::new("boards", &app.rows, &mut app.picked, app.active).show(ui);
                for event in out.events {
                    if let PageListEvent::Go(row) = event {
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
    let picked = at(&|e| matches!(e, PageListEvent::Selection(change) if change.row == Some(2))).expect("picked");
    let activate = at(&|e| matches!(e, PageListEvent::Go(2))).expect("activated");
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

// Backspace deletes like Delete, and Shift+Backspace is not a Delete.
#[test]
fn backspace_deletes_the_picked_rows() {
    let mut harness = open(&[], &[1, 2]);
    harness.get_by_role(Role::ListBox).focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Backspace);
    harness.run();
    assert!(harness.state().events.iter().all(|e| !matches!(e, PageListEvent::Delete(_))), "{:?}", harness.state().events);
    press(&mut harness, Key::Backspace);
    assert!(harness.state().events.contains(&PageListEvent::Delete(vec![1, 2])), "{:?}", harness.state().events);
}

/// Where each text drawn in the last pass starts and ends on x.
fn text_spans(harness: &Harness<'_, App>, wanted: &str) -> Option<(f32, f32)> {
    let mut spans = Vec::new();
    for clipped in &harness.output().shapes {
        let mut stack = vec![&clipped.shape];
        while let Some(shape) = stack.pop() {
            match shape {
                Shape::Vec(inner) => stack.extend(inner),
                Shape::Text(text) if text.galley.text() == wanted => spans.push((text.pos.x, text.pos.x + text.galley.size().x)),
                _ => {}
            }
        }
    }
    spans.first().copied()
}

// With 12 rows the numbers are right-aligned in a slot as wide as "12": "1" and "12" end at one x and the names start after the slot; with 6 rows the slot is narrower, so names start further left.
#[test]
fn the_number_slot_is_as_wide_as_the_widest_number() {
    let wide = open_many(12, &[], &[]);
    let (one, twelve) = (text_spans(&wide, "1").expect("number 1"), text_spans(&wide, "12").expect("number 12"));
    assert!((one.1 - twelve.1).abs() < 0.5, "right-aligned: {one:?} {twelve:?}");
    assert!(twelve.0 < one.0, "12 is wider than 1");
    let name = text_spans(&wide, "Board 1").expect("name").0;
    assert!(name >= twelve.1, "the name starts after the widest number: {name} vs {twelve:?}");

    let narrow = open_many(6, &[], &[]);
    let narrow_name = text_spans(&narrow, "Board 1").expect("name").0;
    assert!(narrow_name < name, "the slot follows the widest number: {narrow_name} vs {name}");
}

static CATALOGS: &Embedded = &[("de", "@id\tpageList.new\tNeue Tafel\n", "")];

// The default labels come from the installed catalog.
#[test]
fn default_labels_are_translated() {
    let (i18n, problems) = I18n::new(CATALOGS, &["de"]);
    assert!(problems.is_empty(), "{problems:?}");
    let de = i18n.from_code("de").expect("de");
    let i18n = Arc::new(i18n);
    let rows = vec![PageListRow { name: "Tafel 1".to_owned(), locked: false }];
    let app = App { rows, picked: Selection::new(), active: 0, events: Vec::new() };
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(320.0, 400.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            Host::builder(AppInfo { id: "test", name: "Test", version: "1.0" }).i18n(i18n.clone()).lang(de).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                let _ = PageList::new("boards", &app.rows, &mut app.picked, app.active).show(ui);
            },
            app,
        );
    harness.run();
    let _ = harness.get_by_label("Neue Tafel");
}
