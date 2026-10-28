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

#[test]
fn a_link_activates_on_enter_but_not_space() {
    let mut harness = harness().build_ui_state(
        |ui, clicks: &mut u32| {
            if ui.add(Link::new("Details").id_salt("details")).clicked() {
                *clicks += 1;
            }
        },
        0,
    );
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.get_by_label("Details").is_focused());
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(*harness.state(), 0, "Space");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 1, "Enter");
}

#[test]
fn a_loading_button_is_busy_and_ignores_clicks_enter_and_space() {
    let mut harness = harness().build_ui_state(
        |ui, clicks: &mut u32| {
            if ui.add(Button::new("Export").loading(true).id_salt("export")).clicked() {
                *clicks += 1;
            }
        },
        0,
    );
    // The spinner never settles: run a fixed number of frames.
    harness.run_steps(2);
    assert!(harness.get_by_label("Export").accesskit_node().is_busy());
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert!(harness.get_by_label("Export").is_focused(), "a loading button keeps its Tab stop");
    for key in [Key::Enter, Key::Space] {
        harness.key_press(key);
        harness.run_steps(2);
    }
    harness.get_by_label("Export").click();
    harness.run_steps(2);
    assert_eq!(*harness.state(), 0);
}

#[test]
fn focus_follows_the_selection_of_a_focused_segmented_control() {
    let mut harness = segmented();
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("New"));
    harness.get_by_label("Subtract").click();
    harness.run();
    assert_eq!((*harness.state(), focused(&harness).as_deref()), (2, Some("Subtract")), "a click");
    *harness.state_mut() = 1;
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("Add"), "a change from outside");
}

#[test]
fn arrow_keys_in_consecutive_frames_stay_in_the_segmented_control() {
    let mut harness = harness().build_ui_state(
        |ui, choice: &mut usize| {
            ui.horizontal(|ui| {
                Segmented::new("combine", "Combine", choice).item(0, "New").item(1, "Add").item(2, "Subtract").show(ui);
                ui.add(Button::new("Next").id_salt("next"));
            });
        },
        0,
    );
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    // Key downs in two consecutive frames: the second comes in the frame after the first moved
    // focus. (`key_press` spends a frame on each key up.)
    let arrow = |pressed| egui::Event::Key { key: Key::ArrowRight, physical_key: None, pressed, repeat: false, modifiers: Default::default() };
    for _ in 0..2 {
        harness.event(arrow(true));
        harness.step();
    }
    harness.event(arrow(false));
    harness.run();
    assert_eq!((*harness.state(), focused(&harness).as_deref()), (2, Some("Subtract")));
}

#[test]
fn a_disabled_segmented_control_keeps_its_radio_buttons_in_the_group_and_its_tooltips() {
    let mut harness = harness().build_ui_state(
        |ui, choice: &mut usize| {
            Segmented::new("tool", "Tool", choice).icon_item(0, Icon::Move, "Move").icon_item(1, Icon::Hand, "Hand").enabled(false).show(ui);
        },
        0,
    );
    harness.run();
    let group = harness.get_by(|n| n.role() == Role::RadioGroup);
    let children: Vec<_> = group.accesskit_node().children().map(|n| (n.role(), n.label().unwrap_or_default(), n.is_disabled())).collect();
    assert_eq!(children, [(Role::RadioButton, "Move".to_owned(), true), (Role::RadioButton, "Hand".to_owned(), true)]);
    harness.get_by_label("Hand").hover();
    harness.run_steps(4);
    let named = harness.query_all_by(|n| n.label().as_deref() == Some("Hand") || n.value().as_deref() == Some("Hand")).count();
    assert_eq!(named, 2, "the tooltip shows the label");
}

/// A slider at 40 in steps of 5 and a range slider at 20 to 60, as the host applies them, and the
/// focus ring colour.
type Sliders = (f64, (f64, f64), egui::Color32);

fn sliders<'a>() -> Harness<'a, Sliders> {
    harness().build_ui_state(
        |ui, (level, range, ring): &mut Sliders| {
            *ring = crate::theme::ToEgui::to_egui(<crate::theme::Tokens as crate::theme::TokensOf>::of(ui).color.line.focus);
            let edit = Slider::new("opacity", "Opacity", *level, 0.0..=100.0).step(5.0).show(ui);
            if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
                *level = *value;
            }
            let edit = RangeSlider::new("levels", ("Levels from", "Levels to"), *range, 0.0..=100.0).show(ui);
            if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
                *range = *value;
            }
        },
        (40.0, (20.0, 60.0), egui::Color32::TRANSPARENT),
    )
}

#[test]
fn sliders_report_their_value_bounds_and_step_and_take_screen_reader_requests() {
    let mut harness = sliders();
    harness.run();
    a11y::assert_labelled(&harness);
    let numeric = |harness: &Harness<'_, Sliders>, label: &str| {
        let node = harness.get_by_label(label).accesskit_node();
        (node.role(), node.numeric_value(), node.min_numeric_value(), node.max_numeric_value(), node.numeric_value_step())
    };
    assert_eq!(numeric(&harness, "Opacity"), (Role::Slider, Some(40.0), Some(0.0), Some(100.0), Some(5.0)));
    // Each thumb's bounds stop at the other thumb; without a step a key moves a hundredth.
    assert_eq!(numeric(&harness, "Levels from"), (Role::Slider, Some(20.0), Some(0.0), Some(60.0), Some(1.0)));
    assert_eq!(numeric(&harness, "Levels to"), (Role::Slider, Some(60.0), Some(20.0), Some(100.0), Some(1.0)));
    let order: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect();
    assert_eq!(order, ["Opacity", "Levels from", "Levels to"]);

    harness.get_by_label("Opacity").increment_accesskit();
    harness.run();
    assert_eq!(harness.state().0, 45.0);
    harness.get_by_label("Levels to").set_value_accesskit(30.0);
    harness.run();
    assert_eq!(harness.state().1, (20.0, 30.0));
    harness.get_by_label("Levels to").set_value_accesskit(10.0);
    harness.run();
    assert_eq!(harness.state().1, (20.0, 20.0), "the upper thumb can't pass the lower");
    assert_eq!(numeric(&harness, "Levels from").3, Some(20.0));
}

#[test]
fn a_focused_slider_and_a_focused_thumb_show_the_focus_ring() {
    let mut harness = sliders();
    harness.run();
    for label in ["Opacity", "Levels from", "Levels to"] {
        harness.key_press(Key::Tab);
        harness.run();
        assert!(harness.get_by_label(label).is_focused(), "{label}");
        a11y::assert_focus_ring(&harness, harness.state().2);
    }
}
