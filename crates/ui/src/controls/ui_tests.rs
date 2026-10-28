use egui::accesskit::{Role, Toggled};
use egui::{Key, OutputCommand};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::testing::{a11y, symbols};
use crate::theme::{Base, ToEgui as _, install};

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([640.0, 320.0]).with_context_setup(|ctx| {
        install(ctx, &Base::suite_default());
        symbols::install(ctx);
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
    })
}

/// A harness for macOS key chords: their symbols come from craft-fonts' symbol faces, so without
/// `CRAFT_FONTS_DIR` it draws a box for them.
fn kbd_harness<State>() -> HarnessBuilder<State> {
    if symbols::available() { harness() } else { harness().allow_missing_glyphs() }
}

#[test]
fn every_control_story_names_every_control() {
    for story in crate::story::registry().into_iter().filter(|s| s.id.starts_with("controls/")) {
        let mut args = story.defaults();
        let builder = if story.id.starts_with("controls/kbd/") { kbd_harness() } else { harness() };
        let mut harness = builder.build_ui(move |ui| (story.show)(ui, &mut args));
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

fn toggled<State>(harness: &Harness<'_, State>, label: &str) -> Option<Toggled> {
    harness.get_by_label(label).accesskit_node().toggled()
}

#[test]
fn a_mixed_checkbox_reports_mixed_and_becomes_checked_on_click_and_on_space() {
    let mut harness = harness().build_ui_state(
        |ui, state: &mut CheckState| {
            let _ = Checkbox::new("Visible", state).id_salt("visible").show(ui);
        },
        CheckState::Mixed,
    );
    harness.run();
    assert_eq!(harness.get_by_label("Visible").accesskit_node().role(), Role::CheckBox);
    assert_eq!(toggled(&harness, "Visible"), Some(Toggled::Mixed));
    harness.get_by_label("Visible").click();
    harness.step();
    assert_eq!((*harness.state(), toggled(&harness, "Visible")), (CheckState::Checked, Some(Toggled::True)), "the click's frame");
    harness.run();
    harness.get_by_label("Visible").click();
    harness.run();
    assert_eq!((*harness.state(), toggled(&harness, "Visible")), (CheckState::Unchecked, Some(Toggled::False)));

    *harness.state_mut() = CheckState::Mixed;
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.get_by_label("Visible").is_focused());
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!((*harness.state(), toggled(&harness, "Visible")), (CheckState::Checked, Some(Toggled::True)), "Space");
}

#[test]
fn switches_and_radio_buttons_report_their_role_and_state_in_the_click_frame_and_ignore_clicks_when_disabled() {
    let mut harness = harness().build_ui_state(
        |ui, (on, unit, locked): &mut (bool, usize, bool)| {
            let _ = Switch::new("Autosave", on).id_salt("autosave").show(ui);
            let mut set = RadioSet::new("units", "Units", unit).item(0).item(1).begin(ui);
            let _ = set.radio(ui, 0, "Pixels");
            let _ = set.radio(ui, 1, "Inches");
            let _ = set.end(ui);
            let _ = Switch::new("Locked", locked).enabled(false).id_salt("locked").show(ui);
        },
        (false, 0, false),
    );
    harness.run();
    a11y::assert_labelled(&harness);
    let role = |harness: &Harness<'_, _>, label| harness.get_by_label(label).accesskit_node().role();
    assert_eq!([role(&harness, "Autosave"), role(&harness, "Pixels"), role(&harness, "Locked")], [Role::Switch, Role::RadioButton, Role::Switch]);
    assert_eq!(toggled(&harness, "Autosave"), Some(Toggled::False));
    harness.get_by_label("Autosave").click();
    harness.step();
    assert_eq!((harness.state().0, toggled(&harness, "Autosave")), (true, Some(Toggled::True)), "the click's frame");
    harness.run();
    harness.get_by_label("Inches").click();
    harness.step();
    assert_eq!((harness.state().1, toggled(&harness, "Inches")), (1, Some(Toggled::True)), "the click's frame");
    harness.run();
    assert!(harness.get_by_label("Locked").accesskit_node().is_disabled());
    harness.get_by_label("Locked").click();
    harness.run();
    assert!(!harness.state().2, "a disabled switch ignores clicks");
}

/// A radio group of three options named "Units".
fn radio_group<'a>() -> Harness<'a, usize> {
    harness().build_ui_state(
        |ui, unit: &mut usize| {
            let _ = RadioGroup::new("units", "Units", unit).item(0, "Pixels").item(1, "Millimetres").item(2, "Inches").show(ui);
        },
        0,
    )
}

#[test]
fn a_radio_group_is_one_tab_stop_and_arrows_select_and_report_in_the_same_frame() {
    let mut harness = radio_group();
    harness.run();
    let group = harness.get_by(|n| n.role() == Role::RadioGroup);
    assert_eq!(group.accesskit_node().label().as_deref(), Some("Units"));
    let radios: Vec<_> = group.accesskit_node().children().filter(|n| n.role() == Role::RadioButton).map(|n| n.label().unwrap_or_default()).collect();
    assert_eq!(radios, ["Pixels", "Millimetres", "Inches"]);
    let stops: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
    assert_eq!(stops, [(Role::RadioButton, "Pixels".to_owned())]);

    harness.key_press(Key::Tab);
    harness.run();
    let checked = |harness: &Harness<'_, usize>| {
        harness
            .query_all_by(|n| n.role() == Role::RadioButton && n.toggled() == Some(Toggled::True))
            .map(|n| n.accesskit_node().label().unwrap_or_default())
            .collect::<Vec<_>>()
    };
    for (key, unit, label) in
        [(Key::ArrowDown, 1, "Millimetres"), (Key::ArrowRight, 2, "Inches"), (Key::ArrowDown, 0, "Pixels"), (Key::ArrowUp, 2, "Inches")]
    {
        harness.event(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
        harness.step();
        assert_eq!((*harness.state(), checked(&harness)), (unit, vec![label.to_owned()]), "{key:?}: the key's frame");
        harness.event(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Default::default() });
        harness.run();
        assert_eq!(focused_label(&harness).as_deref(), Some(label), "{key:?}: focus follows");
    }
    let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|s| s.label).collect();
    assert_eq!(stops, ["Inches"], "the Tab stop follows the selection");

    harness.get_by_label("Millimetres").click();
    harness.step();
    assert_eq!((*harness.state(), checked(&harness)), (1, vec!["Millimetres".to_owned()]), "a click: the click's frame");
}

fn focused_label<State>(harness: &Harness<'_, State>) -> Option<String> {
    harness.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label())
}

#[test]
fn radio_groups_checkboxes_and_switches_mirror_right_to_left() {
    let mut harness = harness()
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, Direction::Rtl);
        })
        .build_ui_state(
            |ui, unit: &mut usize| {
                ui.with_layout(direction::column(ui), |ui| {
                    let _ = RadioGroup::new("units", "Units", unit).item(0, "Pixels").item(1, "Millimetres").item(2, "Inches").show(ui);
                    let _ = Checkbox::bool("Grid", &mut true).id_salt("grid").show(ui);
                });
            },
            0,
        );
    harness.run();
    let rect = |harness: &Harness<'_, usize>, label: &str| harness.get_by_label(label).rect();
    let (pixels, inches, grid) = (rect(&harness, "Pixels"), rect(&harness, "Inches"), rect(&harness, "Grid"));
    assert_eq!(pixels.right(), inches.right(), "options line up at the right");
    assert_eq!(pixels.right(), grid.right());
    harness.key_press(Key::Tab);
    harness.run();
    for (key, unit) in [(Key::ArrowLeft, 1), (Key::ArrowLeft, 2), (Key::ArrowRight, 1)] {
        harness.key_press(key);
        harness.run();
        assert_eq!(*harness.state(), unit, "{key:?}");
    }
}

#[test]
fn the_radio_group_story_has_one_tab_stop_per_group() {
    let story = crate::story::registry().into_iter().find(|s| s.id == "controls/radio/groups").unwrap();
    let mut args = story.defaults();
    let mut harness = harness().with_size([640.0, 480.0]).build_ui(move |ui| (story.show)(ui, &mut args));
    harness.run();
    let order: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
    assert_eq!(order, [(Role::RadioButton, "Pixels".to_owned()), (Role::RadioButton, "Inches".to_owned())]);
}

/// Three options of a set named "Units", each in a place of its own with other widgets between
/// them; the last is disabled.
fn radio_set<'a>() -> Harness<'a, usize> {
    harness().build_ui_state(
        |ui, unit: &mut usize| {
            let mut set = RadioSet::new("units", "Units", unit).item(0).item(1).item(2).item(3).item_enabled(false).begin(ui);
            ui.horizontal(|ui| {
                let _ = set.radio(ui, 0, "Pixels");
                ui.label(format!("now {unit}"));
            });
            let _ = Checkbox::bool("Snap", &mut true).id_salt("snap").show(ui);
            let _ = set.radio(ui, 1, "Millimetres");
            ui.horizontal(|ui| {
                let _ = set.radio(ui, 2, "Inches");
                let _ = set.radio(ui, 3, "Points");
            });
            let _ = set.end(ui);
        },
        0,
    )
}

#[test]
fn a_radio_set_selects_in_the_click_frame_and_is_one_tab_stop_with_arrows_across_its_places() {
    let mut harness = radio_set();
    harness.run();
    let group = harness.get_by(|n| n.role() == Role::RadioGroup);
    assert_eq!(group.accesskit_node().label().as_deref(), Some("Units"));
    let radios: Vec<_> = group.query_all_by(|n| n.role() == Role::RadioButton).map(|n| n.accesskit_node().label().unwrap_or_default()).collect();
    assert_eq!(radios, ["Pixels", "Millimetres", "Inches", "Points"], "every option is in the group");
    let stops: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
    assert_eq!(stops, [(Role::RadioButton, "Pixels".to_owned()), (Role::CheckBox, "Snap".to_owned())]);

    let checked = |harness: &Harness<'_, usize>| {
        harness
            .query_all_by(|n| n.role() == Role::RadioButton && n.toggled() == Some(Toggled::True))
            .map(|n| n.accesskit_node().label().unwrap_or_default())
            .collect::<Vec<_>>()
    };
    // A click on an option drawn after the selected one, and a screen reader's click on one drawn
    // before it: both report in their own frame, as does the code between the options.
    harness.get_by_label("Inches").click();
    harness.step();
    assert_eq!((*harness.state(), checked(&harness)), (2, vec!["Inches".to_owned()]), "a click: its frame");
    assert!(harness.query_by_label("now 2").is_some());
    harness.run();
    harness.get_by_label("Pixels").click_accesskit();
    harness.step();
    assert_eq!((*harness.state(), checked(&harness)), (0, vec!["Pixels".to_owned()]), "a screen reader's click: its frame");
    harness.run();
    harness.get_by_label("Points").click();
    harness.run();
    assert_eq!(*harness.state(), 0, "a disabled option ignores clicks");

    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(focused_label(&harness).as_deref(), Some("Pixels"));
    for (key, unit, label) in
        [(Key::ArrowDown, 1, "Millimetres"), (Key::ArrowRight, 2, "Inches"), (Key::ArrowDown, 0, "Pixels"), (Key::ArrowUp, 2, "Inches")]
    {
        harness.event(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
        harness.step();
        assert_eq!((*harness.state(), checked(&harness)), (unit, vec![label.to_owned()]), "{key:?}: the key's frame");
        harness.event(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Default::default() });
        harness.run();
        assert_eq!(focused_label(&harness).as_deref(), Some(label), "{key:?}: focus follows");
    }
    let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|s| s.label).collect();
    assert_eq!(stops, ["Snap", "Inches"], "the Tab stop follows the selection");
}

/// Options A to E with B, D and E disabled: the keys and the Tab stop skip them, in a radio group
/// and in a segmented control.
#[test]
fn arrows_home_and_end_skip_disabled_options_and_a_disabled_option_is_never_the_tab_stop() {
    let options = ["A", "B", "C", "D", "E"];
    let enabled = |i: usize| i == 0 || i == 2;
    for segmented in [false, true] {
        let mut harness = harness().build_ui_state(
            move |ui, choice: &mut usize| {
                if segmented {
                    let mut control = Segmented::new("letters", "Letters", choice);
                    for (i, label) in options.into_iter().enumerate() {
                        control = control.item(i, label).item_enabled(enabled(i));
                    }
                    control.show(ui);
                } else {
                    let mut group = RadioGroup::new("letters", "Letters", choice);
                    for (i, label) in options.into_iter().enumerate() {
                        group = group.item(i, label).item_enabled(enabled(i));
                    }
                    let _ = group.show(ui);
                }
            },
            1,
        );
        harness.run();
        let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|s| s.label).collect();
        assert_eq!(stops, ["A"], "segmented {segmented}: the first enabled option, not the selected disabled one");
        assert_eq!(harness.get_by_label("B").accesskit_node().toggled(), Some(Toggled::True), "segmented {segmented}");

        harness.key_press(Key::Tab);
        harness.run();
        for (key, choice) in
            [(Key::ArrowDown, 2), (Key::ArrowDown, 0), (Key::ArrowUp, 2), (Key::Home, 0), (Key::End, 2), (Key::ArrowRight, 0), (Key::ArrowLeft, 2)]
        {
            harness.key_press(key);
            harness.run();
            assert_eq!((*harness.state(), focused_label(&harness)), (choice, Some(options[choice].to_owned())), "segmented {segmented}: {key:?}");
        }
        harness.get_by_label("D").click();
        harness.run();
        assert_eq!(*harness.state(), 2, "segmented {segmented}: a disabled option ignores clicks");
    }
}

fn numeric<State>(harness: &Harness<'_, State>, label: &str) -> (Role, Option<f64>, Option<f64>, Option<f64>) {
    let node = harness.get_by_label(label);
    let n = node.accesskit_node();
    (n.role(), n.numeric_value(), n.min_numeric_value(), n.max_numeric_value())
}

#[test]
fn swatches_are_buttons_with_the_colour_as_value_report_clicks_and_ignore_them_when_disabled() {
    let mut harness = harness().build_ui_state(
        |ui, clicks: &mut u32| {
            let blue = craft_theme::Rgba::rgb(0x3e, 0x63, 0xdd);
            for (label, swatch) in [
                ("Fill", Swatch::new("fill", "Fill", blue)),
                ("Glass", Swatch::new("glass", "Glass", craft_theme::Rgba([0xff, 0, 0, 0x80]))),
                ("Clear", Swatch::new("clear", "Clear", craft_theme::Rgba([0xff, 0, 0, 0]))),
                ("Faint", Swatch::new("faint", "Faint", craft_theme::Rgba([0x12, 0x34, 0x56, 3]))),
                ("Mixed", Swatch::new("mixed", "Mixed", blue).mixed(true)),
                ("Chosen", Swatch::new("chosen", "Chosen", blue).selected(true)),
                ("Locked", Swatch::new("locked", "Locked", blue).enabled(false)),
            ] {
                if ui.add(swatch).clicked() {
                    *clicks += 1;
                    let _ = label;
                }
            }
        },
        0,
    );
    harness.run();
    a11y::assert_labelled(&harness);
    let value = |harness: &Harness<'_, u32>, label: &str| harness.get_by_label(label).accesskit_node().value();
    assert_eq!(harness.get_by_label("Fill").accesskit_node().role(), Role::Button);
    assert_eq!(value(&harness, "Fill").as_deref(), Some("#3e63dd"));
    assert_eq!(value(&harness, "Glass").as_deref(), Some("#ff000080"));
    assert_eq!(value(&harness, "Clear").as_deref(), Some("#ff000000"), "alpha 0 keeps its channels");
    assert_eq!(value(&harness, "Faint").as_deref(), Some("#12345603"));
    assert_eq!(value(&harness, "Mixed"), None, "a mixed well has no colour");
    assert!(harness.get_by_label("Chosen").accesskit_node().is_selected().unwrap_or(false));
    assert!(harness.get_by_label("Locked").accesskit_node().is_disabled());
    harness.get_by_label("Fill").click();
    harness.step();
    assert_eq!(*harness.state(), 1, "reported in the click's frame");
    harness.get_by_label("Locked").click();
    harness.run();
    assert_eq!(*harness.state(), 1, "a disabled swatch takes no click");
}

#[test]
fn a_palette_row_is_a_tab_stop_per_swatch_with_one_selected() {
    let story = crate::story::registry().into_iter().find(|s| s.id == "controls/swatch/palette").unwrap();
    let mut args = story.defaults();
    let mut harness = harness().build_ui(move |ui| (story.show)(ui, &mut args));
    harness.run();
    let stops = a11y::tab_order(&mut harness);
    assert_eq!(stops.len(), 12);
    assert!(stops.iter().all(|s| s.role == Role::Button));
    let selected = harness.query_all_by(|n| n.role() == Role::Button && n.is_selected() == Some(true)).count();
    assert_eq!(selected, 1);
    harness.get_by_label("Colour 3").click();
    harness.run();
    assert!(harness.get_by_label("Colour 3").accesskit_node().is_selected().unwrap_or(false));
}

/// In Studio dark the selection ring and the focus ring are both `accent.solid`: focus draws a second
/// ring outside the selection ring, a gap away, not inside its band.
#[test]
fn focusing_a_selected_swatch_draws_the_focus_ring_outside_the_selection_ring() {
    use craft_theme::{AppDefaults, ModeChoice, Size, SystemAppearance, ThemePair, builtins};
    let studio = builtins().iter().find(|t| t.id.as_str() == "craft.studio").unwrap().id.clone();
    let defaults = AppDefaults { theme: ThemePair::same(studio), mode: ModeChoice::Dark, size: Size::Compact };
    let base =
        Base::resolve(&craft_theme::SuiteChoice::default().with_defaults(&defaults), SystemAppearance::default(), &defaults, None, &mut |id| {
            builtins().iter().find(|t| t.id == *id).cloned()
        });
    let tokens = &base.shown(craft_theme::Mode::Dark).resolved.tokens;
    let accent = tokens.color.accent.solid.to_egui();
    assert_eq!(tokens.color.line.focus.to_egui(), accent, "Studio's focus ring is the accent");
    let mut harness = Harness::builder().with_size([200.0, 100.0]).build_ui(move |ui| {
        install(ui.ctx(), &base);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.add(Swatch::new("chosen", "Chosen", craft_theme::Rgba::rgb(0x30, 0xa4, 0x6c)).selected(true));
        });
    });
    harness.run();
    let rings = |harness: &Harness<'_, ()>| -> Vec<egui::Rect> {
        let node = harness.get_by_label("Chosen").rect();
        let mut rects: Vec<_> = harness
            .output()
            .shapes
            .iter()
            .filter_map(|c| if let egui::Shape::Rect(r) = &c.shape { Some(r.clone()) } else { None })
            .filter(|r| r.stroke.color == accent && !r.stroke.is_empty() && r.rect.contains_rect(node.shrink(4.0)))
            .map(|r| r.visual_bounding_rect())
            .collect();
        rects.sort_by(|a, b| a.width().total_cmp(&b.width()));
        rects
    };
    let unfocused = rings(&harness);
    assert_eq!(unfocused.len(), 1, "the selection ring: {unfocused:?}");
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.get_by_label("Chosen").is_focused());
    let focused = rings(&harness);
    assert_eq!(focused.len(), 2, "the focus ring as well: {focused:?}");
    let (selection, focus) = (focused[0], focused[1]);
    assert_eq!(selection, unfocused[0]);
    assert!(focus.shrink(1.0).contains_rect(selection.expand(0.5)), "the focus ring {focus:?} clears the selection ring {selection:?}");
    let node = harness.get_by_label("Chosen").rect();
    assert!(node.contains_rect(selection), "the selection ring is inside the swatch's own rect, so nothing clips it");
}

#[test]
fn badges_are_labels_with_their_text_and_counts_cap_at_99_plus() {
    let mut harness = harness().build_ui(|ui| {
        ui.add(Badge::new("Beta").tone(BadgeTone::Accent));
        for n in [0, 99, 100, 1234] {
            ui.add(Badge::count(n).tone(BadgeTone::Bad).style(BadgeStyle::Solid));
        }
    });
    harness.run();
    a11y::assert_labelled(&harness);
    let texts: Vec<String> = harness.query_all_by(|n| n.role() == Role::Label).filter_map(|n| n.accesskit_node().value()).collect();
    assert_eq!(texts, ["Beta", "0", "99", "99+", "99+"]);
    assert!(a11y::tab_order(&mut harness).is_empty(), "no Tab stop");
}

#[test]
fn a_key_chord_is_a_label_with_the_key_names_and_no_tab_stop() {
    let mut harness = kbd_harness().build_ui(|ui| {
        let chord = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, Key::Z);
        ui.add(Kbd::new(&chord).os(egui::os::OperatingSystem::Mac));
        ui.add(Kbd::new(&chord).keycaps(true).os(egui::os::OperatingSystem::Windows));
        ui.add(Kbd::keys(&["Shift", "Click"]));
    });
    harness.run();
    let texts: Vec<String> = harness.query_all_by(|n| n.role() == Role::Label).filter_map(|n| n.accesskit_node().value()).collect();
    assert_eq!(texts, ["Shift+Cmd+Z", "Ctrl+Shift+Z", "Shift+Click"], "spoken as names whatever the symbols");
    assert!(a11y::tab_order(&mut harness).is_empty());
}

#[test]
fn progress_bars_report_their_fraction_between_0_and_1_and_indeterminate_ones_none() {
    let mut harness = harness().build_ui(|ui| {
        ui.add(Progress::new(0.25).label("Export"));
        ui.add(Progress::new(7.0).label("Overfull"));
        ui.add(Progress::new(f32::NAN).label("Unknown"));
        ui.add(Progress::indeterminate().label("Working"));
        ui.add(Spinner::new().label("Loading"));
    });
    harness.run_steps(3);
    a11y::assert_labelled(&harness);
    assert_eq!(numeric(&harness, "Export"), (Role::ProgressIndicator, Some(0.25), Some(0.0), Some(1.0)));
    assert_eq!(numeric(&harness, "Overfull").1, Some(1.0));
    assert_eq!(numeric(&harness, "Unknown").1, Some(0.0));
    let working = numeric(&harness, "Working");
    assert_eq!((working.0, working.1), (Role::ProgressIndicator, None));
    assert!(harness.get_by_label("Working").accesskit_node().is_busy());
    assert_eq!(numeric(&harness, "Loading").0, Role::ProgressIndicator);
    assert!(harness.get_by_label("Loading").accesskit_node().is_busy());
}

#[test]
fn tooltips_show_text_the_shortcut_and_custom_contents_on_enabled_and_disabled_controls() {
    let mut harness = harness().build_ui(|ui| {
        let undo = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, Key::Z);
        ui.horizontal(|ui| {
            ui.add(Button::new("Undo").id_salt("undo")).tooltip_with("Undo the last change", &undo);
            ui.add(Button::new("Off").enabled(false).id_salt("off")).tooltip("Not available");
            ui.add(Button::new("Info").id_salt("info")).tooltip_ui(|ui| {
                ui.label("Custom contents");
            });
        });
    });
    harness.run();
    let shown = |harness: &Harness<'_, ()>, text: &str| harness.query_all_by(|n| n.value().as_deref() == Some(text)).count();
    let undo = harness.get_by_label("Undo").accesskit_node();
    assert_eq!(undo.data().keyboard_shortcut(), Some("Ctrl+Z"), "the shortcut is the node's");
    assert_eq!(undo.data().description(), Some("Undo the last change"));
    harness.get_by_label("Undo").hover();
    harness.run_steps(4);
    assert_eq!(shown(&harness, "Undo the last change"), 1);
    assert_eq!(shown(&harness, "Ctrl+Z"), 1, "the shortcut follows the text in a Kbd");
    harness.get_by_label("Off").hover();
    harness.run_steps(4);
    assert_eq!(shown(&harness, "Not available"), 1, "a disabled control keeps its tooltip");
    harness.get_by_label("Info").hover();
    harness.run_steps(4);
    assert_eq!(shown(&harness, "Custom contents"), 1);
}

#[test]
fn a_tooltip_shortcut_follows_its_text_in_reading_order() {
    let mut harness = harness().build_ui(|ui| {
        direction::apply(ui.ctx(), Direction::Rtl);
        let undo = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, Key::Z);
        ui.add(Button::new("Undo").id_salt("undo")).tooltip_with("Undo the last change", &undo);
    });
    harness.run();
    harness.get_by_label("Undo").hover();
    harness.run_steps(4);
    let x = |harness: &Harness<'_, ()>, text: &str| harness.get_by_value(text).rect().center().x;
    assert!(x(&harness, "Ctrl+Z") < x(&harness, "Undo the last change"), "right to left, the shortcut is on the left");
}
