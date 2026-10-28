use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::edit::{CancelReason, EditEvent};
use crate::testing::a11y;
use crate::theme::{Base, install};

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

const MODES: [&str; 6] = ["Normal", "Multiply", "Screen", "Overlay", "Darken", "Lighten"];

/// A host's value and every event it got.
#[derive(Default)]
struct Host {
    value: usize,
    events: Vec<EditEvent<usize>>,
}

/// Blend modes 0 to 5, with "Overlay" (3) disabled, a separator after "Normal".
fn blend(preview: bool) -> Harness<'static, Host> {
    harness().build_ui_state(
        move |ui, host: &mut Host| {
            let mut select = Select::new("blend", "Blend mode", &mut host.value).preview(preview);
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

#[test]
fn a_press_dragged_onto_an_option_chooses_it_on_release() {
    let mut harness = blend(false);
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
    harness.event(press(screen, false));
    harness.run();
    assert_eq!(harness.state().value, 2);
    assert!(matches!(harness.state().events[..], [EditEvent::Set { value: 2, .. }]), "{:?}", harness.state().events);
    assert_eq!(expanded(&harness), Some(false));
    assert!(options(&harness).is_empty(), "the list closed");
}

#[test]
fn a_click_opens_the_list_until_a_second_click_and_a_release_on_the_box_keeps_it_open() {
    let mut harness = blend(false);
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
fn keys_move_the_active_option_over_enabled_ones_and_enter_chooses_it() {
    let mut harness = blend(false);
    harness.state_mut().value = 2;
    harness.run();
    focus_box(&mut harness);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!((expanded(&harness), active(&harness).as_deref()), (Some(true), Some("Screen")), "Down opens on the chosen option");
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
    }
    assert_eq!(harness.state().value, 2, "moving chooses nothing");
    harness.event(Event::Text("m".into()));
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("Multiply"), "type-ahead");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (1, Some(false)));
    assert!(harness.get_by_role(Role::ComboBox).is_focused());

    harness.event(Event::Text("l".into()));
    harness.run();
    assert_eq!((expanded(&harness), active(&harness).as_deref()), (Some(true), Some("Lighten")), "typing on the closed box opens it");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!((harness.state().value, expanded(&harness)), (1, Some(false)), "Esc closes without a change");
    assert!(harness.get_by_role(Role::ComboBox).is_focused(), "and keeps the focus");
}

#[test]
fn a_preview_reports_the_active_option_and_commits_the_choice_or_cancels() {
    let mut harness = blend(true);
    harness.run();
    focus_box(&mut harness);
    harness.key_press(Key::ArrowDown);
    harness.run();
    for _ in 0..2 {
        harness.key_press(Key::ArrowDown);
        harness.run();
    }
    assert_eq!(harness.state().value, 2, "the host shows the preview");
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
    assert_eq!(harness.state().value, 5, "the pointer previews too");
    option(&harness, "Lighten").click();
    harness.run();
    let events = std::mem::take(&mut harness.state_mut().events);
    assert!(
        matches!(events[..], [EditEvent::Begin { .. }, EditEvent::Update { value: 5, .. }, EditEvent::Commit { value: 5, changed: true, .. }]),
        "{events:?}"
    );
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
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    a11y::assert_labelled(&harness);
    assert_eq!(harness.query_all_by_role(Role::ListBox).count(), 1);
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
            let names = fonts.names.clone();
            let _ = Select::new("font", "Font", &mut fonts.value)
                .rows(names.len(), move |i| (i, names[i].clone()))
                .searchable("Search fonts", "Clear")
                .show(ui);
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
    harness.key_press(Key::ArrowDown);
    harness.run();
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
