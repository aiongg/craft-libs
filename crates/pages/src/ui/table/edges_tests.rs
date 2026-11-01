//! The border edge picker.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::edges::*;

fn open(edges: BorderEdges) -> Harness<'static, BorderEdges> {
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(240.0, 240.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, edges: &mut BorderEdges| {
                EdgePicker::new("edges", edges).show(ui);
            },
            edges,
        )
}

fn click(harness: &mut Harness<'_, BorderEdges>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        harness.step();
    }
}

fn on(harness: &Harness<'_, BorderEdges>) -> Vec<EdgeKey> {
    EdgeKey::ALL.into_iter().filter(|&k| harness.state().get(k)).collect()
}

// A click on the top line toggles it; a double-click on it leaves the four outer lines on and the inner off; a triple-click turns all on, again all off.
#[test]
fn clicks_toggle_double_picks_a_group_triple_toggles_all() {
    let mut harness = open(BorderEdges::default());
    harness.run();
    let top = harness.get_by_role_and_label(Role::CheckBox, "Top Border").rect().center();

    click(&mut harness, top);
    assert_eq!(on(&harness), vec![EdgeKey::Top]);
    click(&mut harness, top);
    assert_eq!(on(&harness), vec![EdgeKey::Top, EdgeKey::Bottom, EdgeKey::Left, EdgeKey::Right], "the double-click picks the outer group");
    click(&mut harness, top);
    assert_eq!(on(&harness), EdgeKey::ALL.to_vec(), "the triple-click turns all on, the choice before it being none");

    // Past the multi-click time, so the next click starts a new sequence.
    for _ in 0..20 {
        harness.step();
    }
    for _ in 0..3 {
        click(&mut harness, top);
    }
    assert_eq!(on(&harness), Vec::<EdgeKey>::new(), "a triple-click from all on turns all off");

    // One Tab stop: ↓ moves from the top line to the left one, Space toggles it.
    harness.get_by_role_and_label(Role::CheckBox, "Top Border").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(on(&harness), vec![EdgeKey::Left]);
}

// Outside sets the four outer edges and leaves the inner; Edge(Top) toggles the top edge.
#[test]
fn presets_set_or_toggle_edges() {
    let mut inner = BorderEdges::default();
    inner.set(EdgeKey::InsideVertical, true);
    let outside = BorderPreset::Outside.apply(inner);
    assert_eq!(EdgeKey::ALL.map(|k| outside.get(k)), [true, true, true, true, false, true]);
    let toggled = BorderPreset::Edge(EdgeKey::Top).apply(outside);
    assert!(!toggled.get(EdgeKey::Top) && toggled.get(EdgeKey::Bottom));
    assert_eq!(BorderPreset::None.apply(outside), BorderEdges::default());
}
