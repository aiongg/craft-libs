use egui::accesskit::{self, Role};
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::edit::{self, CancelReason, EditEvent};
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
fn blend(preview: bool) -> Harness<'static, Host> {
    harness().build_ui_state(
        move |ui, host: &mut Host| {
            if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
                edit::cancel_all(ui.ctx(), CancelReason::UndoDuringGesture);
            }
            if host.hidden {
                return;
            }
            let mut select = Select::new("blend", "Blend mode", &mut host.value).preview(preview).enabled(!host.disabled);
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

#[test]
fn esc_right_after_a_click_opens_the_list_closes_it_and_keeps_the_focus() {
    let mut harness = blend(false);
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
fn closing_a_preview_without_a_choice_reports_why_and_puts_the_value_back() {
    let mut harness = blend(true);
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
        harness.hover_at(option(harness, "Lighten").rect().center());
        harness.run();
        assert_eq!(harness.state().value, 5, "the host shows the preview");
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
        "an undo ends the preview and closes the list"
    );
}

#[test]
fn a_select_drawn_again_after_a_gap_starts_closed_without_its_gesture() {
    let mut harness = blend(true);
    harness.run();
    focus_box(&mut harness);
    harness.key_press(Key::ArrowDown);
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
    harness.key_press(Key::End);
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("Lighten"));
    harness.state_mut().1 = 2;
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!((harness.state().0, expanded(&harness)), (0, Some(true)), "nothing is chosen");
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
    for key in [Key::ArrowDown, Key::End, Key::PageUp] {
        none.key_press(key);
        none.run();
    }
    none.event(Event::Text("a".into()));
    none.run();
    assert_eq!((*none.state(), expanded(&none), active(&none)), (0, Some(true), None));
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
