use egui::accesskit::{Role, Toggled};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::direction::{self, Direction};
use crate::theme::{Base, install};

struct Host {
    picked: Option<Anchor9>,
    size: RefSize,
    sets: usize,
}

fn harness(dir: Direction, picked: Option<Anchor9>, size: RefSize) -> Harness<'static, Host> {
    Harness::<Host>::builder()
        .with_size([200.0, 120.0])
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(
            |ui, host: &mut Host| {
                ui.add_space(10.0);
                let edit = ReferencePoint::new("pivot", host.picked).size(host.size).show(ui);
                if let Some(EditEvent::Set { value, .. }) = edit.event {
                    host.picked = value;
                    host.sets += 1;
                }
            },
            Host { picked, size, sets: 0 },
        )
}

fn radios<'a>(h: &'a Harness<'_, Host>) -> Vec<egui_kittest::Node<'a>> {
    h.query_all(egui_kittest::kittest::by().role(Role::RadioButton)).collect()
}

fn focused(h: &Harness<'_, Host>) -> Option<String> {
    h.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label())
}

fn click(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run();
}

#[test]
fn nine_radio_buttons_in_a_group_with_the_picked_one_on() {
    let mut h = harness(Direction::Ltr, Some(Anchor9::Right), RefSize::Compact);
    h.run();
    let group = h.get_by_label("Reference point");
    assert_eq!(group.accesskit_node().role(), Role::RadioGroup);
    let names: Vec<_> = radios(&h).iter().map(|n| n.accesskit_node().label().unwrap_or_default()).collect();
    assert_eq!(names, ["Top left", "Top", "Top right", "Left", "Center", "Right", "Bottom left", "Bottom", "Bottom right"]);
    let on: Vec<_> = radios(&h).iter().map(|n| n.accesskit_node().toggled() == Some(Toggled::True)).collect();
    assert_eq!(on.iter().filter(|&&on| on).count(), 1);
    assert!(on[Anchor9::Right as usize]);
    assert_eq!(group.accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("Right"));
}

#[test]
fn a_custom_reference_point_lights_no_cell() {
    let mut h = harness(Direction::Ltr, None, RefSize::Large);
    h.run();
    assert!(radios(&h).iter().all(|n| n.accesskit_node().toggled() != Some(Toggled::True)));
    assert!(h.get_by_label("Reference point").accesskit_node().active_descendant().is_none());
}

#[test]
fn arrows_move_a_keyboard_cell_and_space_or_enter_picks_it() {
    let mut h = harness(Direction::Ltr, Some(Anchor9::Center), RefSize::Compact);
    h.run();
    h.get_by_label("Reference point").focus();
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(h.state().sets, 0, "moving picks nothing");
    assert_eq!(h.get_by_label("Reference point").accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("Bottom right"));
    h.key_press(Key::Space);
    h.run();
    assert_eq!((h.state().picked, h.state().sets), (Some(Anchor9::BottomRight), 1));
    // The grid has edges.
    h.key_press(Key::ArrowRight);
    h.run();
    h.key_press(Key::ArrowDown);
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().picked, Some(Anchor9::BottomRight));
    for _ in 0..3 {
        h.key_press(Key::ArrowLeft);
        h.run();
    }
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().picked, Some(Anchor9::BottomLeft));
}

#[test]
fn a_click_picks_at_once_and_the_grid_does_not_mirror_right_to_left() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let mut h = harness(dir, Some(Anchor9::Center), RefSize::Large);
        h.run();
        let group = h.get_by_label("Reference point").rect();
        click(&mut h, pos2(group.left() + 4.0, group.top() + 4.0));
        assert_eq!(h.state().picked, Some(Anchor9::TopLeft), "{dir:?}: the left is left");
        click(&mut h, pos2(group.right() - 4.0, group.bottom() - 4.0));
        assert_eq!(h.state().picked, Some(Anchor9::BottomRight), "{dir:?}");
        let cells = radios(&h);
        assert!(cells[0].rect().left() < cells[8].rect().left(), "{dir:?}: cells keep their document order");
    }
}

#[test]
fn the_picker_is_one_tab_stop_of_the_set_size() {
    for (size, side) in [(RefSize::Compact, 16.0), (RefSize::Large, 72.0)] {
        let mut h = harness(Direction::Ltr, Some(Anchor9::Center), size);
        h.run();
        let rect = h.get_by_label("Reference point").rect();
        assert_eq!((rect.width(), rect.height()), (side, side));
        h.key_press(Key::Tab);
        h.run();
        h.run();
        // AccessKit reports the focus on the group's active descendant.
        assert_eq!(focused(&h).as_deref(), Some("Center"));
        h.key_press(Key::Tab);
        h.run();
        h.run();
        assert_eq!(focused(&h).as_deref(), Some("Center"), "Tab wraps to the one stop, it doesn't visit cells");
    }
}
