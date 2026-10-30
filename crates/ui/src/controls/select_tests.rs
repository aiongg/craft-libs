use egui::accesskit::{self, Role};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::edit::{self, CancelReason, EditEvent};
use crate::icons::Icon;
use crate::menu::{MenuEntry, MenuItem};
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, install};

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

const MODES: [&str; 6] = ["Normal", "Multiply", "Screen", "Overlay", "Darken", "Lighten"];

/// A host's value and every event it got.
#[derive(Default)]
struct Host {
    value: usize,
    events: Vec<EditEvent<usize>>,
    /// The select isn't drawn.
    hidden: bool,
    disabled: bool,
}

/// Blend modes 0 to 5, with "Overlay" (3) disabled, a separator after "Normal"; Ctrl+Z cancels
/// every gesture as an undo would.
fn blend() -> Harness<'static, Host> {
    harness().build_ui_state(
        move |ui, host: &mut Host| {
            if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
                edit::cancel_all(ui.ctx(), CancelReason::UndoDuringGesture);
            }
            if host.hidden {
                return;
            }
            let mut select = Select::new("blend", "Blend mode", &mut host.value).enabled(!host.disabled);
            for (i, mode) in MODES.into_iter().enumerate() {
                select = select.option(i, mode).option_enabled(i != 3);
                if i == 0 {
                    select = select.separator();
                }
            }
            if let Some(event) = select.show(ui).event {
                host.events.push(event);
            }
        },
        Host::default(),
    )
}

fn options<State>(harness: &Harness<'_, State>) -> Vec<String> {
    harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().label()).collect()
}

fn option<'h, State>(harness: &'h Harness<'_, State>, label: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(label))
}

fn expanded<State>(harness: &Harness<'_, State>) -> Option<bool> {
    harness.get_by_role(Role::ComboBox).accesskit_node().data().is_expanded()
}

fn active<State>(harness: &Harness<'_, State>) -> Option<String> {
    harness.get_by_role(Role::ComboBox).accesskit_node().active_descendant().and_then(|n| n.label())
}

fn press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// Keyboard focus on the closed box.
fn focus_box<State>(harness: &mut Harness<'_, State>) {
    harness.get_by_role(Role::ComboBox).focus();
    harness.run();
}

const D_MODES: [&str; 6] = ["Normal", "Darken", "Dissolve", "Multiply", "Difference", "Screen"];

fn d_modes() -> Harness<'static, Host> {
    harness().build_ui_state(
        move |ui, host: &mut Host| {
            let mut select = Select::new("d-modes", "Blend mode", &mut host.value);
            for (i, mode) in D_MODES.into_iter().enumerate() {
                select = select.option(i, mode);
            }
            if let Some(event) = select.show(ui).event {
                host.events.push(event);
            }
        },
        Host::default(),
    )
}

#[test]
fn a_repeated_letter_moves_through_the_options_with_that_initial_and_other_text_is_a_prefix() {
    let mut harness = d_modes();
    harness.run();
    focus_box(&mut harness);
    for expected in ["Darken", "Dissolve", "Difference", "Darken"] {
        harness.event(Event::Text("d".into()));
        harness.run();
        assert_eq!(active(&harness).as_deref(), Some(expected), "d, d, ...");
    }

    let mut harness = d_modes();
    harness.run();
    focus_box(&mut harness);
    harness.event(Event::Text("d".into()));
    harness.event(Event::Text("i".into()));
    harness.event(Event::Text("f".into()));
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("Difference"), "\"dif\" finds by prefix");
}

#[test]
fn a_press_dragged_onto_an_option_chooses_it_on_release() {
    let mut harness = blend();
    harness.run();
    let field = harness.get_by_role(Role::ComboBox).rect().center();
    harness.hover_at(field);
    harness.event(press(field, true));
    harness.run();
    assert_eq!(expanded(&harness), Some(true), "the press opens the list");
    let screen = option(&harness, "Screen").rect().center();
    harness.hover_at(screen);
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("Screen"), "the option under the pointer is active");
    assert_eq!(harness.state().value, 0, "the pointer only highlights");
    harness.event(press(screen, false));
    harness.run();
    assert_eq!(harness.state().value, 2);
    let events = &harness.state().events;
    assert!(matches!(events[..], [EditEvent::Begin { .. }, EditEvent::Commit { value: 2, changed: true, .. }]), "{events:?}");
    assert_eq!(expanded(&harness), Some(false));
    assert!(options(&harness).is_empty(), "the list closed");
}

#[test]
fn a_click_opens_the_list_until_a_second_click_and_a_release_on_the_box_keeps_it_open() {
    let mut harness = blend();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    assert_eq!(options(&harness), MODES);
    let field = harness.get_by_role(Role::ComboBox).rect();
    harness.event(press(field.center(), true));
    harness.run();
    assert_eq!(expanded(&harness), Some(false), "a press on the box closes the open list");
    harness.event(press(field.center(), false));
    harness.run();
    let inside = field.right_center() - egui::vec2(4.0, 0.0);
    harness.event(press(field.center(), true));
    harness.run();
    harness.hover_at(inside);
    harness.event(press(inside, false));
    harness.run();
    assert_eq!(expanded(&harness), Some(true), "a release over the box keeps it open");
    option(&harness, "Overlay").click();
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (0, Some(true)), "a disabled option can't be chosen");
    option(&harness, "Darken").click();
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (4, Some(false)));
    assert!(harness.get_by_role(Role::ComboBox).is_focused(), "the box keeps the focus");
}

#[test]
fn keys_in_the_open_list_move_the_value_over_enabled_options_and_enter_chooses_it() {
    let mut harness = blend();
    harness.state_mut().value = 2;
    harness.run();
    focus_box(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run();
    assert_eq!((expanded(&harness), active(&harness).as_deref()), (Some(true), Some("Screen")), "Alt+Down opens on the chosen option");
    let steps = [
        (Key::ArrowDown, "Darken"),
        (Key::ArrowDown, "Lighten"),
        (Key::ArrowDown, "Lighten"),
        (Key::Home, "Normal"),
        (Key::ArrowUp, "Normal"),
        (Key::End, "Lighten"),
        (Key::PageUp, "Normal"),
        (Key::PageDown, "Lighten"),
        (Key::ArrowUp, "Darken"),
        (Key::ArrowUp, "Screen"),
    ];
    for (key, expected) in steps {
        harness.key_press(key);
        harness.run();
        assert_eq!(active(&harness).as_deref(), Some(expected), "{key:?}");
        assert_eq!(MODES[harness.state().value], expected, "{key:?}: the value follows");
        assert_eq!(expanded(&harness), Some(true), "{key:?}: the list stays open");
    }
    harness.event(Event::Text("m".into()));
    harness.run();
    assert_eq!((active(&harness).as_deref(), harness.state().value), (Some("Multiply"), 1), "type-ahead");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (1, Some(false)));
    assert!(harness.get_by_role(Role::ComboBox).is_focused());
    let events = std::mem::take(&mut harness.state_mut().events);
    assert!(matches!(events[..], [EditEvent::Begin { .. }, .., EditEvent::Commit { value: 1, changed: true, .. }]), "one gesture: {events:?}");
    assert_eq!(events.iter().filter(|e| matches!(e, EditEvent::Begin { .. })).count(), 1, "{events:?}");

    harness.event(Event::Text("l".into()));
    harness.run();
    assert_eq!((expanded(&harness), active(&harness).as_deref()), (Some(true), Some("Lighten")), "typing on the closed box opens it");
    assert_eq!(harness.state().value, 5);
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (1, Some(false)), "Esc puts the value back");
    assert!(harness.get_by_role(Role::ComboBox).is_focused(), "and keeps the focus");
}

#[test]
fn up_and_down_on_the_closed_box_step_the_value_and_alt_down_enter_and_space_open_it() {
    let mut harness = blend();
    harness.state_mut().value = 2;
    harness.run();
    focus_box(&mut harness);
    let steps =
        [(Key::ArrowDown, 4), (Key::ArrowDown, 5), (Key::ArrowDown, 5), (Key::ArrowUp, 4), (Key::ArrowUp, 2), (Key::ArrowUp, 1), (Key::ArrowUp, 0)];
    for (key, expected) in steps {
        harness.key_press(key);
        harness.run();
        assert_eq!((harness.state().value, expanded(&harness)), (expected, Some(false)), "{key:?} steps over the disabled option");
    }
    harness.key_press(Key::ArrowUp);
    harness.run();
    let events = std::mem::take(&mut harness.state_mut().events);
    let sets: Vec<_> = events.iter().map(|e| if let EditEvent::Set { value, .. } = e { Some(*value) } else { None }).collect();
    assert_eq!(sets, [Some(4), Some(5), Some(4), Some(2), Some(1), Some(0)], "a Set a step, none past the ends");
    for key in [Key::Home, Key::End] {
        harness.key_press(key);
        harness.run();
        assert_eq!((harness.state().value, expanded(&harness)), (0, Some(false)), "{key:?} on the closed box does nothing");
    }
    for (modifiers, key) in [(Modifiers::ALT, Key::ArrowDown), (Modifiers::NONE, Key::Enter), (Modifiers::NONE, Key::Space)] {
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        assert_eq!((expanded(&harness), active(&harness).as_deref()), (Some(true), Some("Normal")), "{key:?} opens on the chosen option");
        harness.key_press(Key::Escape);
        harness.run();
    }
}

/// Blend modes in a panel that scrolls, the select `wheel_steps` or not, at the top.
fn wheel_panel(wheel_steps: bool) -> Harness<'static, Host> {
    harness().build_ui_state(
        move |ui, host: &mut Host| {
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                let mut select = Select::new("blend", "Blend mode", &mut host.value).wheel_steps(wheel_steps);
                for (i, mode) in MODES.into_iter().enumerate() {
                    select = select.option(i, mode).option_enabled(i != 3);
                }
                if let Some(event) = select.show(ui).event {
                    host.events.push(event);
                }
                ui.add_space(1000.0);
            });
        },
        Host::default(),
    )
}

fn wheel(lines: f32) -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, lines),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn the_wheel_over_the_closed_box_steps_the_value_only_when_asked_and_then_the_panel_stays() {
    let mut harness = wheel_panel(true);
    harness.run();
    let field = harness.get_by_role(Role::ComboBox).rect();
    harness.hover_at(field.center());
    harness.run();
    let turns = [(-1.0, 1), (-2.0, 4), (-3.0, 5), (1.0, 4), (0.5, 4), (0.5, 2)];
    for (lines, expected) in turns {
        harness.event(wheel(lines));
        harness.run();
        assert_eq!((harness.state().value, expanded(&harness)), (expected, Some(false)), "{lines} lines");
    }
    assert_eq!(harness.get_by_role(Role::ComboBox).rect(), field, "the panel doesn't scroll");
    let events = &harness.state().events;
    assert!(events.iter().all(|e| matches!(e, EditEvent::Set { .. })) && events.len() == 5, "{events:?}");

    let mut harness = wheel_panel(false);
    harness.run();
    let field = harness.get_by_role(Role::ComboBox).rect();
    harness.hover_at(field.center());
    harness.run();
    harness.event(wheel(-1.0));
    harness.run();
    assert_eq!(harness.state().value, 0, "off by default");
    assert!(harness.get_by_role(Role::ComboBox).rect().top() < field.top(), "the panel scrolls");
}

#[test]
fn a_panel_scroll_that_passes_over_a_select_with_wheel_steps_goes_on_scrolling() {
    let mut harness = harness().build_ui_state(
        |ui, host: &mut Host| {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                ui.add_space(400.0);
                let mut select = Select::new("blend", "Blend mode", &mut host.value).wheel_steps(true);
                for (i, mode) in MODES.into_iter().enumerate() {
                    select = select.option(i, mode);
                }
                host.events.extend(select.show(ui).event);
                ui.add_space(1000.0);
            });
        },
        Host::default(),
    );
    harness.run();
    let start = harness.get_by_role(Role::ComboBox).rect();
    let pointer = egui::pos2(start.center().x, 150.0);
    assert!(!start.contains(pointer));
    harness.hover_at(pointer);
    harness.run();
    let mut passed_over = false;
    for _ in 0..40 {
        harness.event(wheel(-0.5));
        harness.step();
        passed_over |= harness.get_by_role(Role::ComboBox).rect().contains(pointer);
    }
    harness.run();
    assert!(passed_over, "the select went under the pointer");
    assert!(harness.get_by_role(Role::ComboBox).rect().bottom() < pointer.y, "the panel went on scrolling past it");
    assert_eq!((harness.state().value, harness.state().events.len()), (0, 0));
}

#[test]
fn the_open_list_is_one_gesture_that_commits_the_choice_or_cancels() {
    let mut harness = blend();
    harness.run();
    focus_box(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run();
    for _ in 0..2 {
        harness.key_press(Key::ArrowDown);
        harness.run();
    }
    assert_eq!(harness.state().value, 2, "the host shows the active option");
    harness.key_press(Key::Escape);
    harness.run();
    let events = std::mem::take(&mut harness.state_mut().events);
    assert!(
        matches!(
            events[..],
            [
                EditEvent::Begin { .. },
                EditEvent::Update { value: 1, .. },
                EditEvent::Update { value: 2, .. },
                EditEvent::Cancel { reason: CancelReason::Escape, .. }
            ]
        ),
        "{events:?}"
    );
    assert_eq!(harness.state().value, 0, "a cancel puts the value back");

    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.hover_at(option(&harness, "Lighten").rect().center());
    harness.run();
    assert_eq!(harness.state().value, 0, "the pointer doesn't move the value");
    option(&harness, "Lighten").click();
    harness.run();
    let events = std::mem::take(&mut harness.state_mut().events);
    assert!(matches!(events[..], [EditEvent::Begin { .. }, EditEvent::Commit { value: 5, changed: true, .. }]), "{events:?}");
    assert_eq!(harness.state().value, 5);
}

#[test]
fn the_box_is_a_combo_box_and_the_options_are_listbox_options_with_the_chosen_one_selected() {
    let mut harness = harness().build_ui_state(
        |ui, value: &mut usize| {
            let _ = Select::new("resampling", "Resampling", value)
                .option(0, "Automatic")
                .separator()
                .group("Smooth")
                .option(1, "Bicubic")
                .option(2, "Bilinear")
                .group("Sharp")
                .option(3, "Nearest neighbour")
                .option_enabled(false)
                .show(ui);
        },
        1,
    );
    harness.run();
    a11y::assert_labelled(&harness);
    let node = harness.get_by_role(Role::ComboBox).accesskit_node();
    assert_eq!((node.label().as_deref(), node.value().as_deref(), node.data().is_expanded()), (Some("Resampling"), Some("Bicubic"), Some(false)));
    assert_eq!(node.data().has_popup(), Some(accesskit::HasPopup::Listbox));
    assert!(node.data().controls().is_empty(), "a closed box controls nothing");
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    a11y::assert_labelled(&harness);
    assert_eq!(harness.query_all_by_role(Role::ListBox).count(), 1);
    let list = harness.get_by_role(Role::ListBox).accesskit_node().locate().0;
    assert_eq!(harness.get_by_role(Role::ComboBox).accesskit_node().data().controls(), [list], "the open box controls the list");
    let groups: Vec<_> = harness.query_all_by_role(Role::Group).filter_map(|n| n.accesskit_node().label()).collect();
    assert_eq!(groups, ["Smooth", "Sharp"]);
    let states: Vec<_> = harness
        .query_all_by_role(Role::ListBoxOption)
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.accesskit_node().is_selected(), n.accesskit_node().is_disabled()))
        .collect();
    let expected = [("Automatic", false, false), ("Bicubic", true, false), ("Bilinear", false, false), ("Nearest neighbour", false, true)]
        .map(|(label, selected, disabled)| (label.to_owned(), Some(selected), disabled));
    assert_eq!(states, expected);
    assert_eq!(active(&harness).as_deref(), Some("Bicubic"), "the list opens on the chosen option");
}

struct Fonts {
    value: usize,
    names: Vec<String>,
}

fn fonts() -> Harness<'static, Fonts> {
    let names: Vec<String> = (0..60).map(|i| if i % 10 == 7 { format!("Bodoni {i}") } else { format!("Garamond {i}") }).collect();
    harness().build_ui_state(
        |ui, fonts: &mut Fonts| {
            let names = &fonts.names;
            let _ = Select::new("font", "Font", &mut fonts.value)
                .rows(names.len(), |i| i, |i| names[i].as_str())
                .searchable("Search fonts", "Clear")
                .show(ui);
            let _ = ui.button("Next");
        },
        Fonts { value: 30, names },
    )
}

/// The open list's rect: the one popup area.
fn list<State>(harness: &Harness<'_, State>) -> egui::Rect {
    harness
        .ctx
        .memory(|m| m.layer_ids().filter(|l| l.order == egui::Order::Foreground).find_map(|l| m.area_rect(l.id)))
        .unwrap_or(egui::Rect::NOTHING)
}

fn list_height<State>(harness: &Harness<'_, State>) -> f32 {
    list(harness).height()
}

#[test]
fn a_search_filters_the_options_and_the_list_takes_its_height_back() {
    let mut harness = fonts();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let focused = harness.query_by(|n| n.is_focused()).map(|n| (n.accesskit_node().role(), n.accesskit_node().label()));
    // AccessKit puts the focus of the search field on its active descendant.
    assert_eq!(focused, Some((Role::ListBoxOption, Some("Garamond 30".to_owned()))), "the search field takes the focus on the chosen row");
    assert!(options(&harness).contains(&"Garamond 30".to_owned()), "the list opens scrolled to the chosen row");
    let full = list_height(&harness);
    harness.event(Event::Text("bod".into()));
    harness.run();
    assert_eq!(options(&harness), ["Bodoni 7", "Bodoni 17", "Bodoni 27", "Bodoni 37", "Bodoni 47", "Bodoni 57"]);
    assert!(list_height(&harness) < full, "the list shrinks to the matches");
    assert_eq!(active(&harness).as_deref(), Some("Bodoni 7"), "the first match is active");
    assert_eq!(harness.state().value, 30, "and waits for Enter");
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().value, 17, "the arrows move the value");
    let search = harness.get_by_label("Search fonts").accesskit_node().active_descendant().and_then(|n| n.label());
    assert_eq!(search.as_deref(), Some("Bodoni 17"), "the search field names the active option");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    assert!((list_height(&harness) - full).abs() < 1.0, "cleared, the list is {} tall, not {full}", list_height(&harness));
    harness.event(Event::Text("bodoni 4".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (47, Some(false)));
    assert!(harness.get_by_role(Role::ComboBox).is_focused(), "the focus goes back to the box");
}

#[test]
fn right_to_left_the_list_lines_up_with_the_end_of_the_box() {
    let mut harness = harness()
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, Direction::Rtl);
        })
        .build_ui_state(
            |ui, value: &mut usize| {
                ui.with_layout(direction::column(ui), |ui| {
                    let _ = Select::new("units", "Units", value).option(0, "Pixels").option(1, "Inches").show(ui);
                });
            },
            0,
        );
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let field = harness.get_by_role(Role::ComboBox).rect();
    let list = list(&harness);
    assert!((list.right() - field.right()).abs() < 1.0, "list {list:?}, box {field:?}");
    assert!(list.top() >= field.bottom() - 1.0, "below the box");
}

#[test]
fn esc_right_after_a_click_opens_the_list_closes_it_and_keeps_the_focus() {
    let mut harness = blend();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    assert_eq!(expanded(&harness), Some(true));
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(expanded(&harness), Some(false));
    assert!(harness.get_by_role(Role::ComboBox).is_focused());
}

#[test]
fn closing_the_list_without_a_choice_reports_why_and_puts_the_value_back() {
    let mut harness = blend();
    harness.run();
    let field = harness.get_by_role(Role::ComboBox).rect();
    let outside = field.left_bottom() + egui::vec2(0.0, 400.0);
    let cancelled = |harness: &mut Harness<'_, Host>| match std::mem::take(&mut harness.state_mut().events).last() {
        Some(EditEvent::Cancel { reason, .. }) => Some(*reason),
        _ => None,
    };
    let previewing = |harness: &mut Harness<'_, Host>| {
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        assert_eq!(harness.state().value, 1, "the host shows the active option");
    };

    previewing(&mut harness);
    harness.event(press(outside, true));
    harness.event(press(outside, false));
    harness.run();
    assert_eq!(
        (cancelled(&mut harness), harness.state().value, expanded(&harness)),
        (Some(CancelReason::Dismissed), 0, Some(false)),
        "a click outside"
    );

    previewing(&mut harness);
    harness.event(press(field.center(), true));
    harness.run();
    harness.event(press(field.center(), false));
    harness.run();
    assert_eq!((cancelled(&mut harness), harness.state().value), (Some(CancelReason::Dismissed), 0), "a press on the open box");

    previewing(&mut harness);
    harness.state_mut().disabled = true;
    harness.run();
    assert_eq!((cancelled(&mut harness), harness.state().value, expanded(&harness)), (Some(CancelReason::Disabled), 0, Some(false)));
    harness.state_mut().disabled = false;
    harness.run();

    previewing(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(
        (cancelled(&mut harness), harness.state().value, expanded(&harness)),
        (Some(CancelReason::UndoDuringGesture), 0, Some(false)),
        "an undo ends the list's gesture and closes the list"
    );
}

#[test]
fn the_pointer_over_the_open_list_highlights_a_row_and_the_keys_go_on_from_it() {
    let mut harness = blend();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let selected = |harness: &Harness<'_, Host>, label: &str| option(harness, label).accesskit_node().data().is_selected();
    for label in ["Screen", "Lighten"] {
        harness.hover_at(option(&harness, label).rect().center());
        harness.run();
        assert_eq!(active(&harness).as_deref(), Some(label));
    }
    assert_eq!(harness.state().value, 0);
    assert!(matches!(harness.state().events[..], [EditEvent::Begin { .. }]), "{:?}", harness.state().events);
    assert_eq!((selected(&harness, "Normal"), selected(&harness, "Lighten")), (Some(true), Some(false)), "the value's row stays selected");

    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!((harness.state().value, active(&harness).as_deref()), (4, Some("Darken")), "the key moves on from the hovered row");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().value, 0);

    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.state_mut().events.clear();
    harness.hover_at(option(&harness, "Lighten").rect().center());
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let events = &harness.state().events;
    assert!(matches!(events[..], [EditEvent::Commit { value: 5, changed: true, .. }]), "{events:?}");
}

#[test]
fn a_select_drawn_again_after_a_gap_starts_closed_without_its_gesture() {
    let mut harness = blend();
    harness.run();
    focus_box(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run();
    let first = match harness.state().events[..] {
        [EditEvent::Begin { id }] => id,
        ref events => panic!("{events:?}"),
    };
    harness.state_mut().hidden = true;
    harness.run();
    harness.state_mut().hidden = false;
    harness.state_mut().events.clear();
    harness.run();
    assert_eq!(expanded(&harness), Some(false));
    assert!(options(&harness).is_empty());
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().events);
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    assert!(matches!(harness.state().events[..], [EditEvent::Begin { id }] if id > first), "a new gesture: {:?}", harness.state().events);
}

#[test]
fn options_that_go_while_the_list_is_open_leave_nothing_to_choose() {
    let mut harness = harness().build_ui_state(
        |ui, (value, count): &mut (usize, usize)| {
            let mut select = Select::new("modes", "Mode", value);
            for (i, mode) in MODES.into_iter().enumerate().take(*count) {
                select = select.option(i, mode);
            }
            let _ = select.show(ui);
        },
        (0, MODES.len()),
    );
    harness.run();
    focus_box(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.key_press(Key::End);
    harness.run();
    assert_eq!((active(&harness).as_deref(), harness.state().0), (Some("Lighten"), 5));
    harness.state_mut().1 = 2;
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!((harness.state().0, expanded(&harness)), (5, Some(true)), "nothing is chosen");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!((harness.state().0, expanded(&harness)), (0, Some(false)), "Esc puts the value back");
}

#[test]
fn an_empty_list_takes_the_keys_and_changes_nothing() {
    let mut search = fonts();
    search.run();
    search.get_by_role(Role::ComboBox).click();
    search.run();
    search.event(Event::Text("zzz".into()));
    search.run();
    assert!(options(&search).is_empty());
    for key in [Key::PageUp, Key::PageDown, Key::Home, Key::End, Key::ArrowDown, Key::ArrowUp] {
        search.key_press(key);
        search.run();
    }
    search.event(Event::Text("q".into()));
    search.run();
    assert_eq!((search.state().value, expanded(&search)), (30, Some(true)));

    let mut none = harness().build_ui_state(
        |ui, value: &mut usize| {
            let _ = Select::new("none", "Nothing", value).show(ui);
        },
        0,
    );
    none.run();
    focus_box(&mut none);
    none.event(Event::Text("a".into()));
    none.run();
    none.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    for key in [Key::ArrowDown, Key::End, Key::PageUp] {
        none.key_press(key);
        none.run();
    }
    none.event(Event::Text("a".into()));
    none.run();
    assert_eq!((*none.state(), expanded(&none), active(&none)), (0, Some(true), None));
}

#[test]
fn an_input_method_keeps_its_enter_and_esc_from_the_search_field_s_list() {
    let mut harness = fonts();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.event(Event::Text("bod".into()));
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("Bodoni 7"));
    let ime = |text: &str| Event::Ime(egui::ImeEvent::Preedit { text: text.to_owned(), active_range_chars: Some(0..text.chars().count()) });
    let key = |key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE };
    harness.event(ime("o"));
    harness.run();
    // The platform delivers the commit with the Enter that confirmed it, and the cancel with its Esc.
    harness.input_mut().events.extend([Event::Ime(egui::ImeEvent::Commit("o".to_owned())), key(Key::Enter)]);
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (30, Some(true)), "the confirming Enter chooses nothing");
    harness.event(ime("x"));
    harness.run();
    harness.input_mut().events.extend([Event::Ime(egui::ImeEvent::Preedit { text: String::new(), active_range_chars: None }), key(Key::Escape)]);
    harness.run();
    assert_eq!(expanded(&harness), Some(true), "the cancelling Esc closes nothing");
    assert_eq!(harness.get_by_label("Search fonts").accesskit_node().value().as_deref(), Some("bodo"));
}

#[test]
fn tab_from_the_search_field_chooses_and_moves_the_focus_on() {
    let mut harness = fonts();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.event(Event::Text("bodoni 4".into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (47, Some(false)));
    assert!(harness.get_by_label("Next").is_focused(), "the widget after the select has the focus");
}

#[test]
fn tab_twice_from_a_search_with_no_matches_closes_the_list() {
    let mut harness = fonts();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.event(Event::Text("zzz".into()));
    harness.run();
    for _ in 0..2 {
        harness.key_press(Key::Tab);
        harness.run();
    }
    assert_eq!((harness.state().value, expanded(&harness)), (30, Some(false)));
}

#[test]
fn a_closed_select_of_rows_reads_a_few_rows_a_frame() {
    use std::cell::Cell;
    let mut harness = harness().build_ui_state(
        |ui, (value, calls): &mut (usize, Cell<usize>)| {
            let calls = &*calls;
            let count = || calls.set(calls.get() + 1);
            let value_of = |i| {
                count();
                i
            };
            let text_of = |i| {
                count();
                format!("Row {i}")
            };
            let _ = Select::new("many", "Many", value).rows(10_000, value_of, text_of).show(ui);
        },
        (9_000, Cell::new(0)),
    );
    harness.run();
    for _ in 0..3 {
        harness.state().1.set(0);
        harness.step();
        assert!(harness.state().1.get() <= 2, "{} calls in a frame", harness.state().1.get());
    }
}

/// The resolved builtin theme `id` in dark mode, if there is one.
fn dark(id: &str) -> Option<Base> {
    use craft_theme::{AppDefaults, ModeChoice, Size, SuiteChoice, SystemAppearance, ThemePair, builtins};
    let theme = builtins().iter().find(|t| t.id.as_str() == id)?.id.clone();
    let defaults = AppDefaults { theme: ThemePair::same(theme), mode: ModeChoice::Dark, size: Size::Compact };
    Some(Base::resolve(&SuiteChoice::default().with_defaults(&defaults), SystemAppearance::default(), &defaults, None, &mut |id| {
        builtins().iter().find(|t| t.id == *id).cloned()
    }))
}

#[test]
fn the_keyboard_draws_the_focus_ring_on_the_active_row_and_the_pointer_only_the_fill() {
    let base = dark("craft.artcraft").unwrap();
    let focus = base.shown(craft_theme::Mode::Dark).resolved.tokens.color.line.focus.to_egui();
    let mut harness = Harness::builder().with_size([480.0, 640.0]).build_ui_state(
        move |ui, host: &mut Host| {
            install(ui.ctx(), &base);
            let mut select = Select::new("blend", "Blend mode", &mut host.value);
            for (i, mode) in MODES.into_iter().enumerate() {
                select = select.option(i, mode);
            }
            let _ = select.show(ui);
        },
        Host::default(),
    );
    let rings =
        |harness: &Harness<'_, Host>| a11y::focus_rings(harness).map_or(0, |(_, rings)| rings.iter().filter(|r| r.stroke.color == focus).count());
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    assert_eq!((active(&harness).as_deref(), rings(&harness)), (Some("Normal"), 0), "a list opened by the pointer");
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("Multiply"));
    a11y::assert_focus_ring(&harness, focus);
    harness.hover_at(option(&harness, "Overlay").rect().center());
    harness.run();
    assert_eq!((active(&harness).as_deref(), rings(&harness)), (Some("Overlay"), 0), "the pointer");
}

#[test]
fn a_search_s_first_match_is_active_while_the_check_mark_stays_on_the_value() {
    let mut harness = fonts();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let selected = |harness: &Harness<'_, Fonts>| -> Vec<String> {
        harness
            .query_all_by_role(Role::ListBoxOption)
            .filter(|n| n.accesskit_node().is_selected() == Some(true))
            .filter_map(|n| n.accesskit_node().label())
            .collect()
    };
    harness.event(Event::Text("amond 3".into()));
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("Garamond 3"), "the first match is active");
    assert_eq!((harness.state().value, selected(&harness)), (30, vec!["Garamond 30".to_owned()]), "the value's row keeps the check mark");
    assert_eq!(harness.get_by_role(Role::ComboBox).accesskit_node().value().as_deref(), Some("Garamond 30"));
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!((harness.state().value, selected(&harness)), (31, vec!["Garamond 31".to_owned()]), "the arrows move the value and the mark");
    assert_eq!(active(&harness).as_deref(), Some("Garamond 31"));
}

/// A select with the options of `texts` and the new options, with the hover output.
#[derive(Default)]
struct Preview {
    value: usize,
    highlighted: Option<usize>,
    events: Vec<EditEvent<usize>>,
    texts: Vec<&'static str>,
    fit: bool,
    size: Option<ControlSize>,
    rows: bool,
    icons: bool,
}

fn preview(texts: &[&'static str], fit: bool) -> Harness<'static, Preview> {
    harness().build_ui_state(
        |ui, host: &mut Preview| {
            let mut select = Select::new("preview", "Mode", &mut host.value).fit(host.fit).highlighted(&mut host.highlighted);
            select = select.size(host.size.unwrap_or(ControlSize::Md));
            if host.rows {
                let texts = host.texts.clone();
                select = select.rows(texts.len(), |i| i, move |i| texts[i]);
            } else {
                for (i, text) in host.texts.iter().enumerate() {
                    select = if host.icons && i == 0 { select.option_icon(i, Icon::Check, *text) } else { select.option(i, *text) };
                    if i == 1 {
                        select = select.option_tooltip("Blends dark over light");
                    }
                }
            }
            if let Some(event) = select.show(ui).event {
                host.events.push(event);
            }
        },
        Preview { texts: texts.to_vec(), fit, ..Default::default() },
    )
}

fn box_width<State>(harness: &Harness<'_, State>) -> f32 {
    harness.get_by_role(Role::ComboBox).rect().width()
}

#[test]
fn fit_is_as_wide_as_the_widest_option_in_every_size_class() {
    for size in [ControlSize::Sm, ControlSize::Md] {
        let width = |texts: &[&'static str], fit: bool, rows: bool| {
            let mut harness = preview(texts, fit);
            harness.state_mut().size = Some(size);
            harness.state_mut().rows = rows;
            harness.run();
            box_width(&harness)
        };
        let field = width(&["A"], false, false);
        let long = width(&["Linear Light"], true, false);
        assert_eq!(width(&["A", "Linear Light", "Hue"], true, false), long, "{size:?}: the widest option decides");
        assert!(long > width(&["A"], true, false), "{size:?}");
        assert_ne!(long, field);
        assert_eq!(width(&["A", "Linear Light"], true, true), field, "{size:?}: rows ignore fit");
    }
}

#[test]
fn the_fitted_width_follows_a_change_of_icons_with_the_texts_unchanged() {
    let mut harness = preview(&["A", "Linear Light"], true);
    harness.run();
    let plain = box_width(&harness);
    harness.state_mut().icons = true;
    harness.run();
    assert!(box_width(&harness) > plain, "room for the icon");
    harness.state_mut().icons = false;
    harness.run();
    assert_eq!(box_width(&harness), plain);
}

#[test]
fn highlighted_follows_the_pointer_over_rows_and_clears_when_the_list_closes_or_the_pointer_leaves() {
    let mut harness = preview(&["Normal", "Multiply", "Screen"], false);
    harness.run();
    assert_eq!(harness.state().highlighted, None);
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    for (label, index) in [("Screen", 2), ("Multiply", 1)] {
        harness.hover_at(option(&harness, label).rect().center());
        harness.run();
        assert_eq!(harness.state().highlighted, Some(index), "{label}");
    }
    harness.hover_at(Pos2::new(470.0, 630.0));
    harness.run();
    assert_eq!(harness.state().highlighted, None, "the pointer left the rows");
    harness.hover_at(option(&harness, "Screen").rect().center());
    harness.run();
    assert_eq!(harness.state().highlighted, Some(2));
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!((expanded(&harness), harness.state().highlighted), (Some(false), None), "closed");
}

#[test]
fn a_highlight_does_not_change_the_value_or_send_an_update() {
    let mut harness = preview(&["Normal", "Multiply", "Screen"], false);
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.hover_at(option(&harness, "Screen").rect().center());
    harness.run();
    assert_eq!(harness.state().highlighted, Some(2));
    assert_eq!(harness.state().value, 0);
    assert!(matches!(harness.state().events[..], [EditEvent::Begin { .. }]), "{:?}", harness.state().events);
}

#[test]
fn option_tooltip_shows_after_the_delay_and_names_the_row() {
    let mut harness = preview(&["Normal", "Multiply", "Screen"], false);
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let tips = |harness: &Harness<'_, Preview>| {
        harness
            .query_all_by(|n| n.label().as_deref() == Some("Blends dark over light") || n.value().as_deref() == Some("Blends dark over light"))
            .count()
    };
    assert_eq!(tips(&harness), 0);
    harness.hover_at(option(&harness, "Multiply").rect().center());
    harness.step();
    assert_eq!(tips(&harness), 0, "not before the delay");
    harness.run_steps(8);
    assert_eq!(tips(&harness), 1, "the tooltip shows");
    assert_eq!(option(&harness, "Multiply").accesskit_node().data().description(), Some("Blends dark over light"));
    assert_eq!(option(&harness, "Screen").accesskit_node().data().description(), None);
}

#[test]
fn a_secondary_press_in_the_search_field_keeps_the_list_and_opens_no_edit_menu() {
    let mut harness = fonts();
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let at = harness.get_by_label("Search fonts").rect().center();
    let press = |pressed| Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    harness.event(Event::PointerMoved(at));
    harness.run();
    harness.event(press(true));
    harness.run();
    harness.event(press(false));
    harness.run();
    assert!(harness.query_by_label("Cut").is_none(), "the search field has no edit menu");
    assert!(!options(&harness).is_empty(), "the list stays open");
}

/// `row_content` draws the option's own value in a box of the asked size in every row of the open
/// list and, for the chosen value, in the closed box, before the text.
#[test]
fn row_content_draws_each_options_value_in_the_list_and_the_chosen_one_in_the_box() {
    use std::cell::RefCell;
    type State = (usize, RefCell<Vec<(usize, egui::Rect)>>);
    let mut harness = harness::<State>().build_ui_state(
        |ui, (value, drawn): &mut State| {
            let drawn = &*drawn;
            let _ = Select::new("strip", "Strip", value)
                .option(0, "Warm")
                .option(1, "Cool")
                .option(2, "Mono")
                .row_content(vec2(30.0, 12.0), |_, rect, value| drawn.borrow_mut().push((*value, rect)))
                .show(ui);
        },
        (1, RefCell::default()),
    );
    harness.run();
    let field = harness.get_by_role(Role::ComboBox).rect();
    let closed = harness.state().1.take();
    let (value, rect) = *closed.last().expect("drawn");
    assert!(closed.iter().all(|c| *c == (value, rect)), "{closed:?}");
    assert_eq!((value, rect.size()), (1, vec2(30.0, 12.0)));
    assert!(field.contains_rect(rect) && rect.center().y == field.center().y.round());

    harness.event(press(field.center(), true));
    harness.run();
    harness.run();
    let mut open = harness.state().1.take();
    open.retain(|(_, r)| !field.contains_rect(*r));
    open.sort_by_key(|(v, _)| *v);
    open.dedup_by_key(|(v, _)| *v);
    assert_eq!(open.iter().map(|(v, _)| *v).collect::<Vec<_>>(), [0, 1, 2], "{open:?}");
    for (value, rect) in &open {
        let row = option(&harness, MODES_STRIP[*value]).rect();
        assert!(row.contains_rect(*rect), "{row:?} {rect:?}");
        assert_eq!(rect.size(), vec2(30.0, 12.0));
        assert_eq!(rect.left(), open[0].1.left(), "the boxes line up");
    }
}

const MODES_STRIP: [&str; 3] = ["Warm", "Cool", "Mono"];

#[derive(Default)]
struct Gear {
    value: usize,
    picked: Option<usize>,
    events: Vec<EditEvent<usize>>,
}

/// A select of blend modes, optionally searchable, with a gear menu of one entry ("Import"),
/// between the buttons "Before" and "After".
fn gear(searchable: bool) -> Harness<'static, Gear> {
    harness().build_ui_state(
        move |ui, host: &mut Gear| {
            let entries: Vec<MenuEntry<usize>> = vec![MenuItem::new("Import", 7).into()];
            let _ = ui.button("Before");
            let mut select = Select::new("blend", "Blend mode", &mut host.value).header_menu("Preset options", &entries, &mut host.picked);
            for (i, mode) in MODES.into_iter().enumerate() {
                select = select.option(i, mode);
            }
            if searchable {
                select = select.searchable("Search modes", "Clear");
            }
            host.events.extend(select.show(ui).event);
            let _ = ui.button("After");
        },
        Gear::default(),
    )
}

fn gear_button<'h, State>(harness: &'h Harness<'_, State>) -> egui_kittest::Node<'h> {
    harness.get_by_role_and_label(Role::Button, "Preset options")
}

fn menu_items<State>(harness: &Harness<'_, State>) -> Vec<String> {
    harness.query_all_by_role(Role::MenuItem).filter_map(|n| n.accesskit_node().label()).collect()
}

#[test]
fn the_gear_is_a_button_named_by_its_label_with_a_menu_popup() {
    for searchable in [false, true] {
        let mut harness = gear(searchable);
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        let data = gear_button(&harness).accesskit_node().data().clone();
        assert_eq!(data.has_popup(), Some(accesskit::HasPopup::Menu));
        assert_eq!(data.is_expanded(), Some(false));
    }
}

#[test]
fn choosing_a_gear_menu_entry_reports_it_and_closes_the_menu_and_the_list() {
    for searchable in [false, true] {
        let mut harness = gear(searchable);
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        gear_button(&harness).click();
        harness.run();
        assert_eq!(menu_items(&harness), ["Import"], "the gear opens its menu");
        assert_eq!(expanded(&harness), Some(true));
        harness.get_by_role(Role::MenuItem).click();
        harness.run();
        harness.run();
        assert_eq!(harness.state().picked, Some(7));
        assert_eq!(harness.state().value, 0, "the value is kept");
        assert_eq!(commits(&harness), [false], "one commit, without a change: {:?}", harness.state().events);
        assert!(menu_items(&harness).is_empty(), "the menu closed");
        assert_eq!(expanded(&harness), Some(false), "the list closed");
    }
}

#[test]
fn esc_in_the_gear_menu_closes_only_the_menu() {
    for searchable in [false, true] {
        let mut harness = gear(searchable);
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        gear_button(&harness).click();
        harness.run();
        assert_eq!(menu_items(&harness).len(), 1);
        harness.key_press(Key::Escape);
        harness.run();
        assert!(menu_items(&harness).is_empty(), "the menu closed");
        assert_eq!(expanded(&harness), Some(true), "the list stays open");
        harness.key_press(Key::Escape);
        harness.run();
        assert_eq!(expanded(&harness), Some(false), "a second Esc closes the list");
    }
}

#[test]
fn tab_reaches_the_gear_and_back_while_the_arrows_step_the_options_past_it() {
    for searchable in [false, true] {
        let mut harness = gear(searchable);
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        assert_eq!(active(&harness).as_deref(), Some("Multiply"));
        harness.key_press(Key::ArrowDown);
        harness.run();
        assert_eq!(active(&harness).as_deref(), Some("Screen"), "the gear is not an option");
        harness.key_press(Key::Tab);
        harness.run();
        assert!(gear_button(&harness).is_focused(), "Tab moves to the gear");
        assert_eq!((expanded(&harness), harness.state().value), (Some(true), 2), "the list stays open with the keys' value");
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(menu_items(&harness), ["Import"], "Enter on the gear opens its menu");
        harness.key_press(Key::Enter);
        harness.run();
        harness.run();
        assert_eq!(harness.state().picked, Some(7), "Enter chooses the entry");
        assert_eq!(expanded(&harness), Some(false));
    }
}

#[test]
fn shift_tab_from_the_gear_returns_to_the_search_field() {
    let mut harness = gear(true);
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(gear_button(&harness).is_focused());
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    harness.event(Event::Text("scr".into()));
    harness.run();
    assert_eq!(harness.get_by_label("Search modes").accesskit_node().value().as_deref(), Some("scr"), "typing goes to the search field");
    assert_eq!(expanded(&harness), Some(true));
}

#[test]
fn tab_from_the_gear_chooses_the_active_option_and_leaves_the_control() {
    for searchable in [false, true] {
        let mut harness = gear(searchable);
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::Tab);
        harness.run();
        assert!(gear_button(&harness).is_focused());
        harness.key_press(Key::Tab);
        harness.run();
        harness.run();
        assert_eq!(expanded(&harness), Some(false), "the list closed");
        assert_eq!(harness.state().value, 1, "the active option is chosen");
        assert!(harness.get_by_label("After").is_focused(), "the focus moved to the next widget");
    }
}

#[test]
fn shift_tab_from_the_box_or_the_search_field_chooses_the_active_option_and_leaves_backwards() {
    for searchable in [false, true] {
        let mut harness = gear(searchable);
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
        harness.run();
        harness.run();
        assert_eq!(expanded(&harness), Some(false), "the list closed");
        assert_eq!(harness.state().value, 1, "the active option is chosen");
        assert!(harness.get_by_label("Before").is_focused(), "the focus moved to the previous widget");
    }
}

#[test]
fn a_gear_entry_after_an_arrow_step_keeps_the_stepped_value() {
    for searchable in [false, true] {
        let mut harness = gear(searchable);
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        gear_button(&harness).click();
        harness.run();
        harness.get_by_role(Role::MenuItem).click();
        harness.run();
        harness.run();
        assert_eq!(harness.state().picked, Some(7));
        assert_eq!(harness.state().value, 2, "the stepped value is kept");
        assert_eq!(commits(&harness), [true], "one commit with the change, no cancel: {:?}", harness.state().events);
        assert!(!harness.state().events.iter().any(|e| matches!(e, EditEvent::Cancel { .. })), "{:?}", harness.state().events);
        assert_eq!(expanded(&harness), Some(false));
    }
}

#[test]
fn a_gear_entry_pressed_and_released_in_separate_frames_keeps_the_stepped_value() {
    for searchable in [false, true] {
        let mut harness = gear(searchable);
        harness.run();
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        gear_button(&harness).click();
        harness.run();
        let pos = harness.get_by_role(Role::MenuItem).rect().center();
        let mut time = 10.0;
        let mut button = |harness: &mut Harness<'_, Gear>, pressed| {
            time += 0.2;
            harness.input_mut().time = Some(time);
            harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
            harness.run();
        };
        harness.event(Event::PointerMoved(pos));
        harness.run();
        button(&mut harness, true);
        button(&mut harness, false);
        harness.run();
        assert_eq!(harness.state().picked, Some(7));
        assert_eq!(harness.state().value, 2, "the stepped value is kept");
        assert_eq!(commits(&harness), [true], "{:?}", harness.state().events);
        assert_eq!(expanded(&harness), Some(false));
    }
}

/// The `changed` flags of the commits the gear harness reported.
fn commits(harness: &Harness<'_, Gear>) -> Vec<bool> {
    harness.state().events.iter().filter_map(|e| if let EditEvent::Commit { changed, .. } = e { Some(*changed) } else { None }).collect()
}

#[test]
fn tab_with_no_matching_option_leaves_the_control_and_closes_the_list() {
    let mut harness = gear(true);
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.event(Event::Text("zzzz".into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(gear_button(&harness).is_focused());
    harness.key_press(Key::Tab);
    harness.run();
    harness.run();
    assert_eq!(expanded(&harness), Some(false), "the list closed");
    assert_eq!(harness.state().value, 0, "the value is kept");
    assert!(harness.get_by_label("After").is_focused(), "the focus moved to the next widget");
}

#[test]
fn shift_tab_from_the_search_field_with_no_match_leaves_backwards_and_closes_the_list() {
    let mut harness = gear(true);
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.event(Event::Text("zzzz".into()));
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    harness.run();
    assert_eq!(expanded(&harness), Some(false), "the list closed");
    assert_eq!(harness.state().value, 0, "the value is kept");
    assert!(harness.get_by_label("Before").is_focused(), "the focus moved to the previous widget");
}

/// A list that opens in a modal after a press elsewhere in the modal is above it: egui's modal
/// area is raised by that press in the pass the list opens in, and the list stays in front of it.
#[test]
fn a_list_opened_after_a_press_in_its_modal_is_in_front_of_the_modal() {
    let mut harness = harness().build_ui_state(
        |ui, host: &mut Host| {
            egui::Modal::new(egui::Id::unique("host-modal")).show(ui.ctx(), |ui| {
                ui.set_width(200.0);
                ui.label("Paper");
                ui.add(egui::Button::new("Other"));
                let mut select = Select::new("paper", "Paper", &mut host.value);
                for (i, mode) in MODES.into_iter().enumerate() {
                    select = select.option(i, mode);
                }
                if let Some(event) = select.show(ui).event {
                    host.events.push(event);
                }
            });
        },
        Host::default(),
    );
    harness.run();
    let click = |harness: &mut Harness<'_, Host>, pos: Pos2| {
        harness.hover_at(pos);
        harness.event(press(pos, true));
        harness.run();
        harness.event(press(pos, false));
        harness.run();
    };
    let field = harness.get_by_role(Role::ComboBox).rect().center();
    click(&mut harness, field);
    assert_eq!(expanded(&harness), Some(true));
    let pick = option(&harness, "Screen").rect().center();
    click(&mut harness, pick);
    assert_eq!(harness.state().value, 2);
    let other = harness.get_by(|n| n.label().as_deref() == Some("Other")).rect().center();
    click(&mut harness, other);

    click(&mut harness, field);
    assert_eq!(expanded(&harness), Some(true), "the list opened");
    let row = option(&harness, "Multiply").rect().center();
    let layer = harness.ctx.layer_id_at(row).expect("a layer under the row");
    assert!(layer.id != egui::Id::unique("host-modal"), "the row is under the modal: {layer:?}");
    click(&mut harness, row);
    assert_eq!(harness.state().value, 1, "the click picked the row");
}
