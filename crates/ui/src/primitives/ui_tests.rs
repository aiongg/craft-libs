use egui::accesskit::{Role, Toggled};
use egui::{Color32, Key, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::testing::a11y;
use crate::theme::{Base, ToEgui, TokensOf, install};

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 200.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

/// `line.focus` where `ui` draws.
fn focus_colour(ui: &Ui) -> Color32 {
    Tokens::of(ui).color.line.focus.to_egui()
}

/// Rings of `colour` around the focused node, if any node has focus.
fn rings_of(harness: &Harness<'_, Color32>) -> usize {
    a11y::focus_rings(harness).map_or(0, |(_, rings)| rings.iter().filter(|r| r.stroke.color == *harness.state()).count())
}

#[test]
fn focus_rings_show_after_keyboard_input_only() {
    let story = crate::story::registry().into_iter().find(|s| s.id == "primitives/focus-ring/focus").unwrap();
    let mut args = story.defaults();
    let mut harness = harness().build_ui_state(
        move |ui, colour: &mut Color32| {
            *colour = focus_colour(ui);
            (story.show)(ui, &mut args);
        },
        Color32::TRANSPARENT,
    );
    harness.run();
    assert_eq!(rings_of(&harness), 0, "no ring before any input");

    harness.key_press(Key::Tab);
    harness.run();
    let focused = |harness: &Harness<'_, Color32>| harness.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label());
    assert_eq!(focused(&harness).as_deref(), Some("Item 1"));
    let colour = *harness.state();
    a11y::assert_focus_ring(&harness, colour);

    harness.get_by_label("Item 1").click();
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("Item 1"), "a click keeps the focus");
    assert_eq!(rings_of(&harness), 0, "a click hides the ring");

    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("Item 2"));
    a11y::assert_focus_ring(&harness, colour);
}

/// The ring is drawn after the shapes the focused item's neighbours draw later in the same
/// layer, and is not cut by the item's own clip rect.
#[test]
fn focus_rings_draw_over_later_neighbours_and_outside_their_clip_rect() {
    let mut harness = harness().build_ui_state(
        |ui, colour: &mut Color32| {
            *colour = focus_colour(ui);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.scope(|ui| {
                    let clip = ui.available_rect_before_wrap().intersect(egui::Rect::from_min_size(ui.cursor().min, vec2(80.0, 24.0)));
                    ui.set_clip_rect(clip);
                    let _ = Interactive::new("first", Role::Button, "First").show(ui, vec2(80.0, 24.0));
                });
                let (rect, _) = ui.allocate_exact_size(vec2(80.0, 24.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(1, 2, 3));
            });
        },
        Color32::TRANSPARENT,
    );
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    let colour = *harness.state();
    let shapes = &harness.output().shapes;
    let rect_at = |i: usize| match &shapes[i].shape {
        egui::Shape::Rect(r) => Some(r),
        _ => None,
    };
    let ring = (0..shapes.len()).find(|&i| rect_at(i).is_some_and(|r| r.stroke.color == colour)).expect("a ring");
    let neighbour = (0..shapes.len()).find(|&i| rect_at(i).is_some_and(|r| r.fill == Color32::from_rgb(1, 2, 3))).expect("the neighbour");
    assert!(ring > neighbour, "ring at {ring}, neighbour at {neighbour}");
    let outer = rect_at(ring).map(|r| r.rect.expand(r.stroke.width)).unwrap_or(egui::Rect::NOTHING);
    assert!(shapes[ring].clip_rect.contains_rect(outer), "{:?} clips {outer:?}", shapes[ring].clip_rect);
}

#[test]
fn interactive_reports_role_label_and_state_and_activates_on_enter_and_space() {
    let mut harness = harness().build_ui_state(
        |ui, count: &mut u32| {
            if Interactive::new("save", Role::Button, "Save").show(ui, vec2(80.0, 24.0)).clicked() {
                *count += 1;
            }
            let _ = Interactive::new("grid", Role::CheckBox, "Show grid").checked(true).show(ui, vec2(80.0, 24.0));
            let _ = Interactive::new("layers", Role::Tab, "Layers").selected(true).value("3 layers").show(ui, vec2(80.0, 24.0));
        },
        0,
    );
    harness.run();
    a11y::assert_labelled(&harness);
    let stops: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
    assert_eq!(stops, [(Role::Button, "Save".to_owned()), (Role::CheckBox, "Show grid".to_owned()), (Role::Tab, "Layers".to_owned())]);
    assert_eq!(harness.get_by_label("Show grid").accesskit_node().data().toggled(), Some(Toggled::True));
    let tab = harness.get_by_label("Layers");
    assert_eq!((tab.accesskit_node().data().is_selected(), tab.value()), (Some(true), Some("3 layers".to_owned())));

    harness.get_by_label("Save").focus();
    harness.run();
    for (key, count) in [(Key::Enter, 1), (Key::Space, 2)] {
        harness.key_press(key);
        harness.run();
        assert_eq!(*harness.state(), count, "{key:?}");
    }
}

/// A toggle and a select report the state after a click in the frame of the click.
#[test]
fn bound_states_report_the_state_after_a_click_in_the_same_frame() {
    let mut harness = harness().build_ui_state(
        |ui, (checked, selected): &mut (bool, bool)| {
            let _ = Interactive::new("grid", Role::CheckBox, "Show grid").toggle(checked).show(ui, vec2(80.0, 24.0));
            let _ = Interactive::new("layers", Role::Tab, "Layers").select(selected).show(ui, vec2(80.0, 24.0));
        },
        (false, false),
    );
    harness.run();
    let toggled = |harness: &Harness<'_, (bool, bool)>| harness.get_by_label("Show grid").accesskit_node().data().toggled();
    let selected = |harness: &Harness<'_, (bool, bool)>| harness.get_by_label("Layers").accesskit_node().data().is_selected();
    assert_eq!((toggled(&harness), selected(&harness)), (Some(Toggled::False), Some(false)));
    for round in [1, 2] {
        harness.get_by_label("Show grid").click();
        harness.get_by_label("Layers").click();
        harness.step();
        let on = round == 1;
        assert_eq!(*harness.state(), (on, true), "round {round}");
        assert_eq!(toggled(&harness), Some(if on { Toggled::True } else { Toggled::False }), "round {round}");
        assert_eq!(selected(&harness), Some(true), "a click on a selected tab keeps it selected");
    }
}

/// A handle is a labelled graphics object with its value, takes clicks on its `handle.hit`
/// square, and is not a Tab stop.
#[test]
fn handles_are_in_the_accessibility_tree_but_not_the_tab_order() {
    let centre = egui::pos2(200.0, 100.0);
    let mut harness = harness().build_ui_state(
        move |ui, (clicks, hit): &mut (u32, f32)| {
            *hit = Tokens::of(ui).component.handle.hit;
            let _ = Interactive::new("save", Role::Button, "Save").show(ui, vec2(80.0, 24.0));
            let response = Handle::new(HandleShape::Square, "Top left corner").value("10, 20 px").show(ui, centre);
            *clicks += u32::from(response.clicked());
        },
        (0, 0.0),
    );
    harness.run();
    let hit = harness.state().1;
    let handle = harness.get_by_label("Top left corner");
    assert_eq!((handle.accesskit_node().role(), handle.value()), (Role::GraphicsObject, Some("10, 20 px".to_owned())));
    assert_eq!(handle.rect().size(), vec2(hit, hit));
    let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|s| s.label).collect();
    assert_eq!(stops, ["Save"]);

    // Outside the drawn handle (size plus halo), inside the hit square.
    let pos = centre + vec2(hit / 2.0 - 1.0, 0.0);
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
    }
    harness.run();
    assert_eq!(harness.state().0, 1);
}

#[test]
fn truncated_text_keeps_the_full_text_in_its_tooltip_and_accesskit_node() {
    const FULL: &str = "A long layer name that does not fit in the panel";
    let mut harness = harness()
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
        })
        .build_ui(|ui| {
            ui.allocate_ui(vec2(120.0, 40.0), |ui| {
                ui.set_max_width(120.0);
                ui.add(Text::new(FULL).truncate());
            });
        });
    harness.run();
    let full = |harness: &Harness<'_>| harness.query_all_by(|n| n.value().as_deref() == Some(FULL) || n.label().as_deref() == Some(FULL)).count();
    let text = harness.query_by(|n| n.value().as_deref() == Some(FULL)).expect("the AccessKit node has the full text");
    assert!(text.rect().width() <= 120.0, "{:?}", text.rect());
    assert_eq!(full(&harness), 1);

    text.hover();
    harness.run_steps(4);
    assert_eq!(full(&harness), 2, "the tooltip shows the full text");
}
