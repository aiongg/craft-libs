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

    harness.get_by_label("Item 2").click();
    harness.run();
    assert_eq!(rings_of(&harness), 0, "no ring after a click");

    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.query_by(|n| n.is_focused()).is_some());
    let colour = *harness.state();
    a11y::assert_focus_ring(&harness, colour);

    harness.get_by_label("Item 1").click();
    harness.run();
    assert_eq!(rings_of(&harness), 0, "a click hides the ring again");
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
