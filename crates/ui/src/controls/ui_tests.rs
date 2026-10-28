use egui::accesskit::{Role, Toggled};
use egui::{Key, OutputCommand};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::testing::a11y;
use crate::theme::{Base, install};

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([640.0, 320.0]).with_context_setup(|ctx| {
        install(ctx, &Base::suite_default());
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
    })
}

#[test]
fn every_control_story_names_every_control() {
    for story in crate::story::registry().into_iter().filter(|s| s.id.starts_with("controls/")) {
        let mut args = story.defaults();
        let mut harness = harness().build_ui(move |ui| (story.show)(ui, &mut args));
        // Loading buttons spin: run a fixed number of frames.
        harness.run_steps(4);
        a11y::assert_labelled(&harness);
    }
}

#[test]
fn icon_buttons_are_named_by_their_label_with_a_tooltip_and_report_a_toggle_in_the_click_frame() {
    let mut harness = harness().build_ui_state(
        |ui, on: &mut bool| {
            ui.add(IconButton::new(Icon::Trash, "Delete layer"));
            ui.add(IconButton::new(Icon::Eye, "Visible").toggle(on));
        },
        false,
    );
    harness.run();
    a11y::assert_labelled(&harness);
    let named = |harness: &Harness<'_, bool>, label: &str| {
        harness.query_all_by(|n| n.label().as_deref() == Some(label) || n.value().as_deref() == Some(label)).count()
    };
    let delete = harness.get_by_label("Delete layer");
    assert_eq!(delete.accesskit_node().role(), Role::Button);
    assert_eq!(named(&harness, "Delete layer"), 1);
    delete.hover();
    harness.run_steps(4);
    assert_eq!(named(&harness, "Delete layer"), 2, "the tooltip shows the label");

    let toggled = |harness: &Harness<'_, bool>| harness.get_by_label("Visible").accesskit_node().toggled();
    assert_eq!(toggled(&harness), Some(Toggled::False));
    for on in [true, false] {
        harness.get_by_label("Visible").click();
        harness.step();
        assert_eq!((*harness.state(), toggled(&harness)), (on, Some(if on { Toggled::True } else { Toggled::False })), "the click's frame");
        harness.run();
    }
}

/// Three segments in a group named "Combine".
fn segmented<'a>() -> Harness<'a, usize> {
    harness().build_ui_state(
        |ui, choice: &mut usize| {
            Segmented::new("combine", "Combine", choice).item(0, "New").item(1, "Add").item(2, "Subtract").show(ui);
        },
        0,
    )
}

fn focused(harness: &Harness<'_, usize>) -> Option<String> {
    harness.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label())
}

#[test]
fn segmented_controls_are_radio_groups_with_the_selected_segment_checked() {
    let mut harness = segmented();
    harness.run();
    let group = harness.get_by(|n| n.role() == Role::RadioGroup);
    assert_eq!(group.accesskit_node().label().as_deref(), Some("Combine"));
    let radios: Vec<_> = group.accesskit_node().children().filter(|n| n.role() == Role::RadioButton).collect();
    let labels: Vec<_> = radios.iter().map(|n| n.label().unwrap_or_default()).collect();
    assert_eq!(labels, ["New", "Add", "Subtract"]);
    let checked = |harness: &Harness<'_, usize>| {
        harness
            .query_all_by(|n| n.role() == Role::RadioButton && n.toggled() == Some(Toggled::True))
            .map(|n| n.accesskit_node().label().unwrap_or_default())
            .collect::<Vec<_>>()
    };
    assert_eq!(checked(&harness), ["New"]);

    harness.get_by_label("Subtract").click();
    harness.step();
    assert_eq!((*harness.state(), checked(&harness)), (2, vec!["Subtract".to_owned()]), "every segment reports the click in its frame");
}

#[test]
fn a_segmented_control_is_one_tab_stop_and_arrows_move_the_selection() {
    let mut harness = segmented();
    harness.run();
    let stops: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
    assert_eq!(stops, [(Role::RadioButton, "New".to_owned())]);

    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("New"));
    for (key, choice, label) in [
        (Key::ArrowRight, 1, "Add"),
        (Key::ArrowDown, 2, "Subtract"),
        (Key::ArrowRight, 0, "New"),
        (Key::ArrowLeft, 2, "Subtract"),
        (Key::ArrowUp, 1, "Add"),
        (Key::Home, 0, "New"),
        (Key::End, 2, "Subtract"),
    ] {
        harness.key_press(key);
        harness.run();
        assert_eq!((*harness.state(), focused(&harness).as_deref()), (choice, Some(label)), "{key:?}");
    }
    let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|s| s.label).collect();
    assert_eq!(stops, ["Subtract"], "the Tab stop follows the selection");
}

#[test]
fn arrows_follow_the_reading_direction_right_to_left() {
    let mut harness = harness()
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, Direction::Rtl);
        })
        .build_ui_state(
            |ui, choice: &mut usize| {
                Segmented::new("combine", "Combine", choice).item(0, "New").item(1, "Add").item(2, "Subtract").show(ui);
            },
            0,
        );
    harness.run();
    let x = |harness: &Harness<'_, usize>, label: &str| harness.get_by_label(label).rect().center().x;
    assert!(x(&harness, "New") > x(&harness, "Add"), "segments start at the right");
    harness.key_press(Key::Tab);
    harness.run();
    for (key, choice) in [(Key::ArrowLeft, 1), (Key::ArrowLeft, 2), (Key::ArrowRight, 1)] {
        harness.key_press(key);
        harness.run();
        assert_eq!(*harness.state(), choice, "{key:?}");
    }
}

#[test]
fn the_segmented_stories_have_one_tab_stop_per_control() {
    for (id, stops) in [("controls/segmented/text", ["New", "Subtract"]), ("controls/segmented/icons", ["Move", "Brush"])] {
        let story = crate::story::registry().into_iter().find(|s| s.id == id).unwrap();
        let mut args = story.defaults();
        let mut harness = harness().build_ui(move |ui| (story.show)(ui, &mut args));
        harness.run();
        let order: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
        assert_eq!(order, stops.map(|l| (Role::RadioButton, l.to_owned())), "{id}");
    }
}

#[test]
fn a_link_with_a_url_opens_it_on_click() {
    const URL: &str = "https://example.com/notes";
    let mut harness = harness().build_ui(|ui| {
        ui.add(Link::new("Release notes").url(URL));
    });
    harness.run();
    let link = harness.get_by_label("Release notes");
    assert_eq!(link.accesskit_node().role(), Role::Link);
    link.click();
    harness.step();
    let opened: Vec<&OutputCommand> = harness.output().platform_output.commands.iter().filter(|c| matches!(c, OutputCommand::OpenUrl(_))).collect();
    assert_eq!(opened, [&OutputCommand::OpenUrl(egui::OpenUrl::same_tab(URL))]);
}
