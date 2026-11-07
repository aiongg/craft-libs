//! The choice family: `RadioGroup` and `Radio` are one Tab stop where arrows move focus and a click or Space selects;
//! `Rating` and `Segmented` change their value from clicks and keys.

use crate::contract::kit::{Probe, Seen};
use craft_behavior::Sense;
use craft_ui::choice::{Radio, RadioGroup, Rating, Segment, Segmented};
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::testing::focus::{egui_focused, is_focused, key_release};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Id, Key, Modifiers, Rect, Ui};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What the last frame showed.
#[derive(Default)]
struct Shown {
    radios: Vec<(Rect, Id)>,
    before: Option<Id>,
    group: Option<Id>,
}

fn frame(ui: &mut Ui, value: &mut u8, shown: &mut Shown) {
    shown.radios.clear();
    let seen = RefCell::new(Seen::default());
    shown.before = Some(Probe::new(&seen).label("Before").id_salt("before").show(ui).id);
    let out = RadioGroup::new("Size", Bind::bound(value)).id_salt("group").show(ui, |ui, _| {
        ui.horizontal(|ui| {
            for v in 1..=3u8 {
                let response = Radio::new(v).label(format!("Option {v}")).id_salt(("radio", v)).show(ui);
                shown.radios.push((response.rect, response.id));
            }
        });
    });
    shown.group = Some(out.output.response.id);
}

fn tap(frames: &mut Frames, key: Key, value: &mut u8, shown: &mut Shown) {
    frames.frame(vec![key_press(key, Modifiers::NONE)], |ui| frame(ui, value, shown));
    frames.frame(vec![key_release(key, Modifiers::NONE)], |ui| frame(ui, value, shown));
}

/// Skeleton `frames_pointer::radio_group_click_selects_one_tab_stop_arrows_move`.
#[trial]
fn radio_group_click_selects_one_tab_stop_arrows_move() {
    let mut frames = Frames::new();
    let (mut value, mut shown) = (1u8, Shown::default());
    frames.idle(3, |ui| frame(ui, &mut value, &mut shown));
    assert_eq!(shown.radios.len(), 3);
    let rects: Vec<Rect> = shown.radios.iter().map(|(rect, _)| *rect).collect();
    assert!(rects[0].max.x <= rects[1].min.x && rects[1].max.x <= rects[2].min.x, "radios shown in a horizontal layout sit side by side");

    let second = rects[1].center();
    click(&mut frames, second, &mut |ui| frame(ui, &mut value, &mut shown));
    assert_eq!(value, 2, "a click on the second radio selects it");

    // The group is one Tab stop: Tab reaches the button, then the group's root, then wraps. No radio holds egui focus.
    let (group, before) = (shown.group, shown.before);
    let mut order = Vec::new();
    for _ in 0..3 {
        tap(&mut frames, Key::Tab, &mut value, &mut shown);
        order.push(egui_focused(&frames.ctx));
    }
    assert_eq!(order.iter().filter(|focused| **focused == group).count(), 1, "one stop in three Tab presses: {order:?}");
    assert!(order.iter().all(|focused| *focused == group || *focused == before || focused.is_none()), "no radio takes egui focus: {order:?}");

    while egui_focused(&frames.ctx) != group {
        tap(&mut frames, Key::Tab, &mut value, &mut shown);
    }
    let focused_member = |frames: &Frames, shown: &Shown| shown.radios.iter().position(|(_, id)| is_focused(&frames.ctx, *id));
    assert_eq!(focused_member(&frames, &shown), Some(1), "Tab arrives at the selected member");
    tap(&mut frames, Key::ArrowRight, &mut value, &mut shown);
    assert_eq!(value, 2, "ArrowRight moves focus, not the selection");
    assert_eq!(
        (focused_member(&frames, &shown), egui_focused(&frames.ctx)),
        (Some(2), group),
        "focus moved to the third member; egui focus stays on the group"
    );
    tap(&mut frames, Key::Space, &mut value, &mut shown);
    assert_eq!(value, 3, "Space selects the focused radio");
    tap(&mut frames, Key::ArrowLeft, &mut value, &mut shown);
    tap(&mut frames, Key::ArrowLeft, &mut value, &mut shown);
    tap(&mut frames, Key::Enter, &mut value, &mut shown);
    assert_eq!(value, 1, "two ArrowLeft presses move focus to the first radio; Enter selects it");
}

/// Members shown in nested layouts (a non-arena container, a horizontal `Ui`) with an ordinary control between them
/// join the group's arena: a click on each selects it, the control between them is not picked, and AccessKit reports
/// the group with each member a radio button at its place in the set.
#[trial]
fn members_of_a_group_form_in_a_nested_layout_join_its_arena() {
    use crate::contract::kit::Panel;
    use craft_ui::contract::Role;
    let mut frames = Frames::new();
    let mut value = 1u8;
    let rects = RefCell::new(Vec::<(Rect, Id)>::new());
    let mut group = None;
    let mut show = |ui: &mut Ui, value: &mut u8| {
        rects.borrow_mut().clear();
        let out = RadioGroup::new("Pages", Bind::bound(value)).id_salt("group").show(ui, |ui, _| {
            Panel::new().id_salt("outer").show(ui, |ui, _| {
                ui.horizontal(|ui| {
                    let first = Radio::new(1u8).label("All").id_salt("all").show(ui);
                    let seen = RefCell::new(Seen::default());
                    Probe::new(&seen).label("Between").sense(Sense::HOVER).id_salt("between").show(ui);
                    let second = Radio::new(2u8).label("Range").id_salt("range").show(ui);
                    rects.borrow_mut().extend([(first.rect, first.id), (second.rect, second.id)]);
                });
            });
        });
        group = Some(out.output.response.id);
    };
    frames.idle(3, |ui| show(ui, &mut value));
    let second = rects.borrow()[1].0;
    click(&mut frames, second.center(), &mut |ui| show(ui, &mut value));
    assert_eq!(value, 2, "a radio inside a nested container selects through the group's arena");
    let tree = frames.tree();
    let group = group.expect("the group showed");
    assert_eq!(tree.role(group), Some(Role::RADIO_GROUP));
    assert_eq!(tree.name(group), Some("Pages"));
    let ids: Vec<Id> = rects.borrow().iter().map(|(_, id)| *id).collect();
    for (at, id) in ids.iter().enumerate() {
        assert_eq!(tree.role(*id), Some(Role::RADIO_BUTTON));
        let mut up = craft_ui::testing::access::NodeKey::of(*id);
        let mut inside = false;
        while let Some(parent) = tree.parent_of_key(up) {
            inside |= parent == craft_ui::testing::access::NodeKey::of(group);
            up = parent;
        }
        assert!(inside, "members are descendants of the group node");
        assert_eq!(tree.position(*id), Some((at, 2)), "each member reports its place in the set");
    }
}

fn rating_rig(start: u8) -> (Fixture, Rc<Cell<u8>>) {
    let value = Rc::new(Cell::new(start));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut stars = shown.get();
        Rating::new(Bind::bound(&mut stars), 5).label("Rating").show(ui);
        shown.set(stars);
    });
    fixture.settle();
    (fixture, value)
}

fn press(fixture: &mut Fixture, key: Key) {
    fixture.key(key, Modifiers::NONE);
    fixture.idle(2);
}

/// A click on a star sets the rating to it; a click on the current star clears it.
#[trial]
fn a_click_on_the_current_star_clears_the_rating() {
    let (mut fixture, value) = rating_rig(0);
    let star = fixture.by_label("3 stars").expect("the third star is a node");
    fixture.click(&star);
    assert_eq!(value.get(), 3);
    fixture.click(&star);
    assert_eq!(value.get(), 0);
}

/// With focus in the row, the arrows step by one and stop at the ends, Home and End jump to them, and a digit sets
/// that many stars.
#[trial]
fn rating_keys_step_jump_and_set() {
    let (mut fixture, value) = rating_rig(1);
    press(&mut fixture, Key::Tab);
    press(&mut fixture, Key::ArrowRight);
    press(&mut fixture, Key::ArrowRight);
    assert_eq!(value.get(), 3);
    press(&mut fixture, Key::Num5);
    assert_eq!(value.get(), 5);
    press(&mut fixture, Key::ArrowRight);
    assert_eq!(value.get(), 5, "the arrows stop at the last star");
    press(&mut fixture, Key::Home);
    assert_eq!(value.get(), 0);
    press(&mut fixture, Key::End);
    assert_eq!(value.get(), 5);
}

/// The arrows pass over a disabled segment, and a click on one picks nothing.
#[trial]
fn segmented_arrows_skip_disabled_segments() {
    let value = Rc::new(Cell::new(1u8));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let segments = [Segment::new(1u8, "Draft"), Segment::new(2, "Review").enabled(false), Segment::new(3, "Final")];
        let mut pick = shown.get();
        Segmented::new(Bind::bound(&mut pick), &segments).label("Stage").show(ui);
        shown.set(pick);
    });
    fixture.settle();
    let review = fixture.by_label("Review").expect("every segment is a node");
    fixture.click(&review);
    assert_eq!(value.get(), 1, "a disabled segment is not picked");
    press(&mut fixture, Key::Tab);
    press(&mut fixture, Key::ArrowRight);
    assert_eq!(value.get(), 3);
    press(&mut fixture, Key::ArrowLeft);
    assert_eq!(value.get(), 1);
}
