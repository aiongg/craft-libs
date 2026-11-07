//! `SectionStack`: Alt+Up and Alt+Down and a header drag move a section, each section reports its place, and Alt+Down
//! in a stack's section moves it instead of closing it.

use craft_ui::contract::{Role, ShowContainer as _};
use craft_ui::runtime::drag;
use craft_ui::sections::{Section, SectionStack};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const TITLES: [&str; 3] = ["Layers", "Effects", "Paths"];

/// What the stack's scene records.
#[derive(Default)]
struct Seen {
    /// The app's order of the sections, by key.
    order: Vec<u8>,
    /// Every move reported: the section's key and its new position.
    moves: Vec<(u8, usize)>,
    /// The sections' ids in the order shown last frame.
    ids: Vec<Id>,
    /// The keys whose body ran last frame.
    bodies: Vec<u8>,
}

/// A stack of the three sections whose moves the scene applies to its order.
fn scene() -> (Fixture, Rc<RefCell<Seen>>) {
    let seen = Rc::new(RefCell::new(Seen { order: vec![0, 1, 2], ..Seen::default() }));
    let shared = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        let order = shared.borrow().order.clone();
        let (mut ids, mut bodies) = (Vec::new(), Vec::new());
        let shown = SectionStack::new(&order, |ui, key, slot| {
            let out = Section::new(TITLES[usize::from(*key)])
                .in_stack(slot)
                .show(ui, |ui, _| {
                    bodies.push(*key);
                    ui.label("Body");
                })
                .output;
            ids.push(out.response.id);
            out
        })
        .show(ui);
        let mut seen = shared.borrow_mut();
        seen.ids = ids;
        seen.bodies = bodies;
        for edit in &shown.moves {
            if let Some(moved) = edit.value()
                && let Some(from) = seen.order.iter().position(|key| *key == moved.key)
            {
                seen.moves.push((moved.key, moved.to));
                let key = seen.order.remove(from);
                seen.order.insert(moved.to, key);
            }
        }
    });
    fixture.settle();
    (fixture, seen)
}

/// Puts focus in the section titled `title`: two clicks on its disclosure close and reopen it.
fn focus_section(fixture: &mut Fixture, title: &str) {
    let lead = fixture.by_role(Role::BUTTON).into_iter().find(|button| fixture.tree().name_of(button.node) == Some(title)).expect("a disclosure");
    fixture.click(&lead);
    fixture.click(&lead);
    fixture.settle();
}

#[trial]
fn alt_down_in_the_first_section_moves_it_to_position_one_and_alt_up_moves_nothing() {
    let (mut fixture, seen) = scene();
    focus_section(&mut fixture, TITLES[0]);
    fixture.key(Key::ArrowUp, Modifiers::ALT);
    assert!(seen.borrow().moves.is_empty(), "the first section cannot move up");
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    assert_eq!(seen.borrow().moves, [(0, 1)], "Alt+Down moves the first section one place toward the end");
    assert_eq!(seen.borrow().order, [1, 0, 2]);
}

#[trial]
fn alt_down_in_a_stack_section_does_not_close_it() {
    let (mut fixture, seen) = scene();
    focus_section(&mut fixture, TITLES[0]);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.settle();
    assert!(seen.borrow().bodies.contains(&0), "the moved section is still open");
}

#[trial]
fn dragging_the_third_header_above_the_first_moves_it_to_position_zero_and_marks_the_drop() {
    let (mut fixture, seen) = scene();
    let ids = seen.borrow().ids.clone();
    let first = fixture.node_of(ids[0]).expect("the first section").rect;
    let third = fixture.node_of(ids[2]).expect("the third section").rect;
    let grab = egui::pos2(third.center().x, third.min.y + 8.0);
    let by = Vec2::new(0.0, first.min.y + 2.0 - grab.y);
    fixture.frame(vec![moved(grab)]);
    fixture.frame(vec![press(grab)]);
    for step in 1..=4 {
        fixture.frame(vec![moved(grab + by * (step as f32 / 4.0))]);
    }
    assert!(drag::mark(fixture.ctx()).is_some(), "the drop mark shows while a header is dragged");
    fixture.frame(vec![release(grab + by)]);
    fixture.frame(Vec::new());
    assert_eq!(seen.borrow().moves, [(2, 0)], "one move, to the front");
    assert_eq!(seen.borrow().order, [2, 0, 1]);
}

#[trial]
fn each_section_reports_its_place_in_the_stack() {
    let (fixture, seen) = scene();
    let ids = seen.borrow().ids.clone();
    assert_eq!(ids.len(), TITLES.len());
    for (index, id) in ids.iter().enumerate() {
        assert_eq!(fixture.tree().position(*id), Some((index, TITLES.len())), "section {index} is {} of {}", index + 1, TITLES.len());
    }
}

#[trial]
fn a_stack_is_as_high_as_its_sections_and_stays_so() {
    let (mut fixture, _) = scene();
    let settled = fixture.shown().height();
    fixture.idle(3);
    assert_eq!(fixture.shown().height(), settled, "sections fill the width, not the height, so the stack does not grow");
}
