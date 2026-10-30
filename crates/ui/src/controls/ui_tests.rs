use egui::accesskit::{Role, Toggled};
use egui::{Key, OutputCommand};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::testing::{a11y, symbols};
use crate::theme::{Base, ToEgui as _, install};
use craft_theme::{Tone, TypeRole};

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

#[test]
fn selected_draws_the_selected_fill_and_reports_toggled_without_flipping_on_click() {
    let mut harness = harness().build_ui_state(
        |ui, (on, clicks): &mut (bool, u32)| {
            if ui.add(IconButton::new(Icon::Eye, "Frame tool").selected(*on)).clicked() {
                *clicks += 1;
            }
        },
        (true, 0),
    );
    harness.run();
    let toggled = |harness: &Harness<'_, (bool, u32)>| harness.get_by_label("Frame tool").accesskit_node().toggled();
    let style = IconButton::style(&crate::theme::current(&harness.ctx), ControlSize::Md);
    let fill = style.selected_fill.rest;
    let outlined = |harness: &Harness<'_, (bool, u32)>| {
        let node = harness.get_by_label("Frame tool").rect();
        harness.output().shapes.iter().any(|c| {
            matches!(&c.shape, egui::Shape::Rect(r) if r.stroke == egui::Stroke::new(1.0, style.selected_border.rest) && r.stroke_kind == egui::StrokeKind::Inside && r.rect.contains_rect(node.shrink(1.0)))
        })
    };
    let filled = |harness: &Harness<'_, (bool, u32)>| {
        let node = harness.get_by_label("Frame tool").rect();
        harness.output().shapes.iter().any(|c| matches!(&c.shape, egui::Shape::Rect(r) if r.fill == fill && r.rect.contains_rect(node.shrink(1.0))))
    };
    assert_eq!(toggled(&harness), Some(Toggled::True));
    assert!(filled(&harness), "the selected fill is painted");
    assert!(outlined(&harness), "with a 1 pt outline inside its edge");
    harness.get_by_label("Frame tool").click();
    harness.run();
    assert_eq!(*harness.state(), (true, 1), "the click is reported and the host owns the state");
    assert_eq!(toggled(&harness), Some(Toggled::True));
    harness.state_mut().0 = false;
    harness.run();
    assert_eq!(toggled(&harness), Some(Toggled::False));
    assert!(!filled(&harness) && !outlined(&harness), "no selected look once deselected");
}

#[test]
fn show_tooltip_false_keeps_the_label_as_name_and_shows_no_tooltip() {
    let hovered_names = |show: bool| {
        let mut harness = harness().build_ui(move |ui| {
            ui.add(IconButton::new(Icon::Trash, "Delete layer").show_tooltip(show));
        });
        harness.run();
        harness.get_by_label("Delete layer").hover();
        harness.run_steps(4);
        harness.query_all_by(|n| n.label().as_deref() == Some("Delete layer") || n.value().as_deref() == Some("Delete layer")).count()
    };
    assert_eq!(hovered_names(true), 2, "the tooltip repeats the label");
    assert_eq!(hovered_names(false), 1, "the label still names the button and no tooltip shows");
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

/// PhotoCraft's `pill_tab`: separate pills of the control height, each its label plus both
/// paddings wide, `segmented.gap` apart.
#[test]
fn segmented_pills_are_separate_and_as_wide_as_their_labels() {
    let mut harness = segmented();
    harness.run();
    let style = Segmented::style(&crate::theme::current(&harness.ctx), ControlSize::Md);
    let rects: Vec<_> = ["New", "Add", "Subtract"].map(|l| harness.get_by_label(l).rect()).into();
    for rect in &rects {
        assert_eq!(rect.height(), style.height);
    }
    for pair in rects.windows(2) {
        assert!((pair[1].min.x - pair[0].max.x - style.gap).abs() < 0.01, "{pair:?}");
    }
    assert!(rects[2].width() > rects[1].width(), "a longer label makes a wider pill");
    assert!(rects[1].width() > 2.0 * style.pad_x);
}

/// A radio button as AccessKit reports it: label, position in the set, checked.
type RadioItem = (String, Option<usize>, bool);

/// The set size of the group named `group`, and its radio buttons in order.
fn radio_items<State>(harness: &Harness<'_, State>, group: &str) -> (Option<usize>, Vec<RadioItem>) {
    let group = harness.get_by(|n| n.role() == Role::RadioGroup && n.label().as_deref() == Some(group));
    let items = group
        .query_all_by(|n| n.role() == Role::RadioButton)
        .map(|n| {
            let n = n.accesskit_node();
            (n.label().unwrap_or_default(), n.position_in_set(), n.toggled() == Some(Toggled::True))
        })
        .collect();
    (group.accesskit_node().size_of_set(), items)
}

/// A key down and its frame, then the key up.
fn press_in_frame<State>(harness: &mut Harness<'_, State>, key: Key, check: impl FnOnce(&Harness<'_, State>)) {
    harness.event(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
    harness.step();
    check(harness);
    harness.event(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Default::default() });
    harness.run();
}

#[test]
fn each_segment_is_a_tab_stop_arrows_move_the_focus_and_space_or_enter_selects() {
    let mut harness = segmented();
    harness.run();
    let item = |label: &str, at, on| (label.to_owned(), Some(at), on);
    assert_eq!(radio_items(&harness, "Combine"), (Some(3), vec![item("New", 0, true), item("Add", 1, false), item("Subtract", 2, false)]));
    let stops: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
    let stop = |label: &str| (Role::RadioButton, label.to_owned());
    assert_eq!(stops, [stop("New"), stop("Add"), stop("Subtract")]);
    assert_eq!(focused(&harness).as_deref(), Some("New"), "Tab came round to the first");
    for (key, choice, label) in [
        (Key::ArrowRight, 0, "Add"),
        (Key::Space, 1, "Add"),
        (Key::ArrowRight, 1, "Subtract"),
        (Key::Enter, 2, "Subtract"),
        (Key::ArrowLeft, 2, "Add"),
        (Key::Home, 2, "Add"),
        (Key::End, 2, "Add"),
    ] {
        press_in_frame(&mut harness, key, |harness| assert_eq!(*harness.state(), choice, "{key:?}: the key's frame"));
        assert_eq!(focused(&harness).as_deref(), Some(label), "{key:?}");
    }
    assert_eq!(radio_items(&harness, "Combine").1, [item("New", 0, false), item("Add", 1, false), item("Subtract", 2, true)]);
}

#[test]
fn arrows_move_the_focus_by_where_the_segments_sit_right_to_left() {
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
    for (key, label) in [(Key::ArrowLeft, "Add"), (Key::ArrowLeft, "Subtract"), (Key::ArrowRight, "Add")] {
        harness.key_press(key);
        harness.run();
        assert_eq!((*harness.state(), focused(&harness).as_deref()), (0, Some(label)), "{key:?}");
    }
}

#[test]
fn every_segment_of_the_segmented_stories_is_a_tab_stop() {
    for (id, stops) in [
        ("controls/segmented/text", ["New", "Add", "Subtract", "New", "Add", "Subtract", "Intersect", "Exclude"]),
        ("controls/segmented/icons", ["Move", "Hand", "Brush", "Move", "Hand", "Brush", "Eraser", "Eyedropper"]),
    ] {
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
fn a_link_activates_on_enter_and_space() {
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
    assert_eq!(*harness.state(), 1, "Space");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 2, "Enter");
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
fn arrow_keys_in_consecutive_frames_move_on_past_the_last_segment() {
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
    // Key downs in consecutive frames: each comes in the frame after the last moved focus.
    // (`key_press` spends a frame on each key up.)
    let arrow = |pressed| egui::Event::Key { key: Key::ArrowRight, physical_key: None, pressed, repeat: false, modifiers: Default::default() };
    for _ in 0..2 {
        harness.event(arrow(true));
        harness.step();
    }
    assert_eq!((*harness.state(), focused(&harness).as_deref()), (0, Some("Subtract")));
    harness.event(arrow(true));
    harness.step();
    harness.event(arrow(false));
    harness.run();
    assert_eq!((*harness.state(), focused(&harness).as_deref()), (0, Some("Next")), "the arrows go on to the next control");
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
fn each_radio_button_is_a_tab_stop_arrows_move_the_focus_and_space_or_enter_selects_in_the_key_s_frame() {
    let mut harness = radio_group();
    harness.run();
    let item = |label: &str, at, on| (label.to_owned(), Some(at), on);
    assert_eq!(radio_items(&harness, "Units"), (Some(3), vec![item("Pixels", 0, true), item("Millimetres", 1, false), item("Inches", 2, false)]));
    let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|s| s.label).collect();
    assert_eq!(stops, ["Pixels", "Millimetres", "Inches"]);
    assert_eq!(focused_label(&harness).as_deref(), Some("Pixels"), "Tab came round to the first");
    let checked = |harness: &Harness<'_, usize>| radio_items(harness, "Units").1.into_iter().filter(|i| i.2).map(|i| i.0).collect::<Vec<_>>();
    for (key, unit, label) in [
        (Key::ArrowDown, 0, "Millimetres"),
        (Key::Space, 1, "Millimetres"),
        (Key::ArrowDown, 1, "Inches"),
        (Key::Enter, 2, "Inches"),
        (Key::ArrowUp, 2, "Millimetres"),
    ] {
        press_in_frame(&mut harness, key, |harness| {
            assert_eq!(checked(harness), [["Pixels", "Millimetres", "Inches"][unit]], "{key:?}: the key's frame");
        });
        assert_eq!((*harness.state(), focused_label(&harness).as_deref()), (unit, Some(label)), "{key:?}");
    }

    harness.get_by_label("Pixels").click();
    harness.step();
    assert_eq!((*harness.state(), checked(&harness)), (0, vec!["Pixels".to_owned()]), "a click: the click's frame");
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
}

#[test]
fn every_enabled_option_of_the_radio_group_story_is_a_tab_stop() {
    let story = crate::story::registry().into_iter().find(|s| s.id == "controls/radio/groups").unwrap();
    let mut args = story.defaults();
    let mut harness = harness().with_size([640.0, 480.0]).build_ui(move |ui| (story.show)(ui, &mut args));
    harness.run();
    let order: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|s| s.label).collect();
    assert_eq!(order, ["Pixels", "Millimetres", "Inches", "Pixels", "Millimetres", "Inches", "Picas"], "Points is disabled");
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
fn a_radio_set_selects_in_the_click_or_key_s_frame_and_each_of_its_places_is_a_tab_stop() {
    let mut harness = radio_set();
    harness.run();
    let item = |label: &str, at, on| (label.to_owned(), Some(at), on);
    assert_eq!(
        radio_items(&harness, "Units"),
        (Some(4), vec![item("Pixels", 0, true), item("Millimetres", 1, false), item("Inches", 2, false), item("Points", 3, false)]),
        "every option is in the group"
    );
    let stops: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
    let stop = |role, label: &str| (role, label.to_owned());
    assert_eq!(
        stops,
        [stop(Role::RadioButton, "Pixels"), stop(Role::CheckBox, "Snap"), stop(Role::RadioButton, "Millimetres"), stop(Role::RadioButton, "Inches")]
    );

    let checked = |harness: &Harness<'_, usize>| radio_items(harness, "Units").1.into_iter().filter(|i| i.2).map(|i| i.0).collect::<Vec<_>>();
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
    // The arrows go by place: the check box between the options takes the focus on the way.
    for (key, unit, label) in
        [(Key::ArrowDown, 0, "Snap"), (Key::ArrowDown, 0, "Millimetres"), (Key::ArrowDown, 0, "Inches"), (Key::Space, 2, "Inches")]
    {
        press_in_frame(&mut harness, key, |harness| assert_eq!(*harness.state(), unit, "{key:?}: the key's frame"));
        assert_eq!(focused_label(&harness).as_deref(), Some(label), "{key:?}");
    }
    assert!(harness.query_by_label("now 2").is_some());
}

/// Options A to E with B, D and E disabled: Tab and the arrows pass over them, in a radio group
/// and in a segmented control.
#[test]
fn tab_and_the_arrows_pass_over_disabled_options_and_home_and_end_do_nothing() {
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
        assert_eq!(stops, ["A", "C"], "segmented {segmented}: the enabled options, not the selected disabled one");
        assert_eq!(harness.get_by_label("B").accesskit_node().toggled(), Some(Toggled::True), "segmented {segmented}");
        assert_eq!(focused_label(&harness).as_deref(), Some("A"), "segmented {segmented}: Tab came round to the first");
        let (next, previous) = if segmented { (Key::ArrowRight, Key::ArrowLeft) } else { (Key::ArrowDown, Key::ArrowUp) };
        for (key, choice, focus) in
            [(next, 1, "C"), (next, 1, "C"), (Key::Home, 1, "C"), (Key::Space, 2, "C"), (previous, 2, "A"), (Key::End, 2, "A")]
        {
            harness.key_press(key);
            harness.run();
            assert_eq!((*harness.state(), focused_label(&harness)), (choice, Some(focus.to_owned())), "segmented {segmented}: {key:?}");
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
fn a_tooltip_shows_after_the_pointer_rests_for_the_theme_s_delay() {
    let mut harness =
        Harness::builder().with_size([320.0, 160.0]).with_step_dt(0.05).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(
            |ui| {
                ui.add(Button::new("Save").id_salt("save")).tooltip("Save the document");
            },
        );
    harness.run();
    assert_eq!(harness.ctx.global_style().interaction.tooltip_delay, Tooltip::DELAY);
    let shown = |harness: &Harness<'_, ()>| harness.query_all_by(|n| n.value().as_deref() == Some("Save the document")).count();
    // The first step takes the move; each one after it is 0.05 s later.
    harness.get_by_label("Save").hover();
    harness.run_steps(7);
    assert_eq!(shown(&harness), 0, "0.3 s");
    harness.run_steps(2);
    assert_eq!(shown(&harness), 1, "0.4 s");
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

#[test]
fn text_fields_report_their_role_name_value_placeholder_and_state() {
    let mut harness = harness().build_ui(|ui| {
        let _ = TextField::new("name", "Name", "Layer 1").placeholder("Untitled").show(ui);
        let _ = TextField::new("code", "Code", "x/1").invalid(true).show(ui);
        let _ = TextField::new("secret", "Password", "hunter2").password(true).show(ui);
        let _ = TextField::new("locked", "Locked", "").placeholder("Nothing here").enabled(false).show(ui);
        let _ = SearchField::new("search", "Search layers", "sky", "Clear").placeholder("Search").show(ui);
    });
    harness.run();
    a11y::assert_labelled(&harness);
    {
        let name = harness.get_by_label("Name");
        let name = name.accesskit_node();
        assert_eq!((name.role(), name.value().as_deref(), name.data().placeholder()), (Role::TextInput, Some("Layer 1"), Some("Untitled")));
        assert!(!name.is_disabled() && name.data().invalid().is_none());
        assert_eq!(harness.get_by_label("Code").accesskit_node().data().invalid(), Some(egui::accesskit::Invalid::True));
        let secret = harness.get_by_label("Password");
        let secret = secret.accesskit_node();
        assert_eq!(secret.role(), Role::PasswordInput);
        assert_ne!(secret.value().as_deref(), Some("hunter2"), "a password is not read out");
        let locked = harness.get_by_label("Locked");
        let locked = locked.accesskit_node();
        assert!(locked.is_disabled() && !locked.is_read_only());
        assert_eq!(locked.placeholder(), Some("Nothing here"));
        let search = harness.get_by_label("Search layers");
        let search = search.accesskit_node();
        assert_eq!((search.role(), search.value().as_deref()), (Role::SearchInput, Some("sky")));
        assert_eq!(harness.get_by_label("Clear").accesskit_node().role(), Role::Button);
    }

    // One Tab stop per enabled field: the clear button and the disabled field are none.
    let stops: Vec<_> = a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect();
    assert_eq!(stops, ["Name", "Code", "Password", "Search layers"]);
}

/// The baseline of every text shape whose text is `text`: the galley's top plus its first row's
/// baseline offset (the row's bottom less the glyph descent, here the row's `max_y`).
fn baseline_of(harness: &Harness<'_>, text: &str) -> Vec<f32> {
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        collect_baselines(&clipped.shape, text, &mut found);
    }
    found
}

fn collect_baselines(shape: &egui::Shape, text: &str, found: &mut Vec<f32>) {
    match shape {
        egui::Shape::Text(t) if t.galley.job.text.contains(text) => {
            let row = &t.galley.rows[0];
            found.push(t.pos.y + row.pos.y + row.row.glyphs[0].pos.y);
        }
        egui::Shape::Vec(v) => v.iter().for_each(|s| collect_baselines(s, text, found)),
        _ => {}
    }
}

#[test]
fn a_link_in_running_text_shares_the_baseline_of_the_text_around_it() {
    use crate::primitives::Text;
    for role in [TypeRole::Caption, TypeRole::Body, TypeRole::Title] {
        // Row heights: the line's own, the default, and a control-sized row.
        for row_height in [0.0, 24.0, 40.0] {
            for wrapped in [true, false] {
                let mut harness = harness().build_ui(move |ui| {
                    ui.spacing_mut().interact_size.y = row_height;
                    let add = |ui: &mut Ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add(Text::new("before ").role(role));
                        ui.add(Link::new("linked").role(role).url("https://example.com").id_salt("link"));
                        ui.add(Text::new(" after").role(role));
                    };
                    if wrapped {
                        ui.horizontal_wrapped(add)
                    } else {
                        ui.horizontal(add)
                    };
                });
                harness.run();
                let at = |text| baseline_of(&harness, text);
                let (before, linked, after) = (at("before"), at("linked"), at("after"));
                assert_eq!(before.len(), 1, "{role:?}");
                let context = format!("{role:?}, row {row_height}, wrapped {wrapped}");
                assert_eq!(before, linked, "{context}");
                assert_eq!(before, after, "{context}");
            }
        }
    }
}

#[test]
fn a_toggle_button_label_takes_its_type_role() {
    let family_of = |role: Option<TypeRole>| {
        let mut on = false;
        let mut harness = harness().build_ui(move |ui| {
            let toggle = ToggleButton::new("tokens.color.accent", &mut on);
            ui.add(if let Some(role) = role { toggle.role(role) } else { toggle });
        });
        harness.run();
        let mut families = Vec::new();
        for clipped in &harness.output().shapes {
            text_families(&clipped.shape, "tokens.color.accent", &mut families);
        }
        families
    };
    assert_eq!(family_of(Some(TypeRole::Mono)), [egui::FontFamily::Monospace]);
    assert_eq!(family_of(None), [egui::FontFamily::Proportional]);
}

fn text_families(shape: &egui::Shape, text: &str, found: &mut Vec<egui::FontFamily>) {
    match shape {
        egui::Shape::Text(t) if t.galley.job.text.contains(text) => found.push(t.galley.job.sections[0].format.font_id.family.clone()),
        egui::Shape::Vec(v) => v.iter().for_each(|s| text_families(s, text, found)),
        _ => {}
    }
}

#[test]
fn a_secondary_button_of_the_suite_default_is_a_28_point_outlined_pill() {
    let mut harness = harness().build_ui(|ui| {
        ui.add(Button::new("Cancel"));
    });
    harness.run();
    let style = Button::style(&crate::theme::current(&harness.ctx), ButtonVariant::Secondary, ControlSize::Md);
    let node = harness.get_by_label("Cancel").rect();
    assert_eq!((node.height(), style.border_width), (28.0, 1.5));
    let outline = harness.output().shapes.iter().any(|c| {
        matches!(&c.shape, egui::Shape::Rect(r)
            if r.stroke.width == 1.5 && r.stroke.color == style.border.rest && r.stroke_kind == egui::StrokeKind::Inside && (r.rect.size() - node.size()).abs().max_elem() < 1.0)
    });
    assert!(outline, "the outline is drawn 1.5 points wide inside the button");
    assert_eq!(style.fill.rest, egui::Color32::TRANSPARENT);
}

#[test]
fn a_toned_icon_button_paints_its_icon_in_the_status_colour() {
    let painted = |tone: Tone| {
        let mut harness = harness().build_ui(move |ui| {
            ui.add(IconButton::new(Icon::Warning, "Comps").tone(tone));
        });
        harness.run();
        let ctx = harness.ctx.clone();
        let tokens = crate::theme::current(&ctx);
        let warn = tokens.component.notice.fg(Tone::Warn).to_egui();
        let ghost = IconButton::style(&tokens, ControlSize::Md).text.rest;
        let icon =
            |color| harness.output().shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Mesh(m) if m.vertices.iter().all(|v| v.color == color)));
        (icon(warn), icon(ghost))
    };
    assert_eq!(painted(Tone::Warn), (true, false));
    assert_eq!(painted(Tone::Neutral), (false, true));
}
