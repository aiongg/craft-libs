//! The body: edits, Apply, OK, Cancel, search, reset, keys, layout and AccessKit.

use egui::accesskit::Role;
use egui::{Context, Key, Modifiers, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::stories::FakeHost;
use super::{Closed, PrefValue, PreferencesBody, PreferencesHost as _};
use crate::direction::{self, Direction};
use crate::theme::{Base, install};

struct State {
    host: FakeHost,
    closed: Option<Closed>,
    picked: Option<String>,
    id: egui::Id,
}

fn builder(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([760.0, 480.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

fn open_with(dir: Direction, host: FakeHost) -> Harness<'static, State> {
    builder(dir).build_ui_state(
        |ui: &mut Ui, s: &mut State| {
            s.id = PreferencesBody::id(ui, "t");
            let out = PreferencesBody::new("t").show(ui, &mut s.host);
            if out.closed.is_some() {
                s.closed = out.closed;
            }
            if out.pick_color.is_some() {
                s.picked = out.pick_color;
            }
        },
        State { host, closed: None, picked: None, id: egui::Id::NULL },
    )
}

fn open() -> Harness<'static, State> {
    open_with(Direction::Ltr, FakeHost::new())
}

fn type_search(h: &mut Harness<'static, State>, text: &str) {
    h.get_by_role(Role::SearchInput).focus();
    h.run();
    h.get_by_role(Role::SearchInput).type_text(text);
    h.run();
}

#[test]
fn editing_a_row_sets_the_draft_and_does_not_apply() {
    let mut h = open();
    h.get_by_label("Save automatically").click();
    h.run();
    assert_eq!(h.state().host.logged("set general.autosave"), 1);
    assert_eq!(h.state().host.logged("apply"), 0);
    assert_eq!(h.state().host.draft.get("general.autosave"), Some(&PrefValue::Bool(false)));
}

#[test]
fn apply_is_disabled_until_the_draft_differs_then_applies_once_and_stays_open() {
    let mut h = open();
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    h.get_by_label("Save automatically").click();
    h.run();
    h.get_by_label("Apply").click();
    h.run();
    assert_eq!(h.state().host.logged("apply"), 1);
    assert_eq!(h.state().closed, None);
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
}

#[test]
fn ok_applies_then_closes_and_a_failed_apply_shows_the_message_and_stays() {
    let mut h = open();
    h.get_by_label("OK").click();
    h.run();
    assert_eq!((h.state().host.logged("apply"), h.state().closed), (1, Some(Closed::Ok)));

    let mut host = FakeHost::new();
    host.fail = Some("Name is required".into());
    let mut h = open_with(Direction::Ltr, host);
    h.get_by_label("OK").click();
    h.run();
    assert_eq!(h.state().closed, None);
    assert!(h.get_all_by_label_contains("Name is required").count() >= 1);
}

#[test]
fn cancel_the_close_request_and_esc_call_cancel_once() {
    let mut h = open();
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!((h.state().host.logged("cancel"), h.state().closed), (1, Some(Closed::Cancel)));

    let mut h = open();
    h.key_press(Key::Escape);
    h.run();
    assert_eq!((h.state().host.logged("cancel"), h.state().closed), (1, Some(Closed::Cancel)));
}

#[test]
fn esc_clears_the_search_then_leaves_it_and_only_then_cancels() {
    let mut h = open();
    type_search(&mut h, "tools");
    h.key_press(Key::Escape);
    h.run();
    assert!(PreferencesBody::state(&h.ctx, h.state().id).search.is_empty());
    assert_eq!(h.state().closed, None);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().closed, None);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().closed, Some(Closed::Cancel));
}

#[test]
fn enter_is_ok_and_the_search_field_keeps_its_enter() {
    let mut h = open();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().closed, Some(Closed::Ok));

    let mut h = open();
    type_search(&mut h, "tools");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().closed, None);
}

#[test]
fn the_language_row_changes_the_next_frames_texts() {
    let mut h = open();
    h.get_by_label("Cancel");
    h.state_mut().host.set("general.language", PrefValue::Choice("de".into()));
    h.run();
    h.get_by_label("Abbrechen");
    assert_eq!(h.state().host.texts().title, "Einstellungen");
}

#[test]
fn reset_section_resets_the_selected_page_and_is_absent_while_searching() {
    let mut h = open();
    h.get_by_label("Reset Section").click();
    h.run();
    assert_eq!(h.state().host.logged("reset general"), 1);
    type_search(&mut h, "language");
    assert!(h.query_by_label("Reset Section").is_none());
}

#[test]
fn search_filters_pages_and_rows_and_the_choice_returns_when_cleared() {
    let mut h = open();
    h.get_by_role_and_label(Role::ListBoxOption, "Tools").click();
    h.run();
    assert_eq!(PreferencesBody::state(&h.ctx, h.state().id).selected, "tools");
    type_search(&mut h, "language");
    assert!(h.query_by_role_and_label(Role::ListBoxOption, "Tools").is_none());
    h.get_by_role_and_label(Role::ListBoxOption, "General");
    assert_eq!(PreferencesBody::state(&h.ctx, h.state().id).selected, "tools");
    h.get_by_label("Clear search").click();
    h.run();
    h.get_by_role_and_label(Role::ListBoxOption, "Tools");
    assert_eq!(h.get_by_role_and_label(Role::ListBoxOption, "Tools").accesskit_node().is_selected(), Some(true));
}

#[test]
fn a_search_with_no_match_says_so() {
    let mut h = open();
    type_search(&mut h, "qqqq");
    assert!(h.get_all_by_label("No matching settings").count() >= 1);
}

#[test]
fn ctrl_f_focuses_the_search_field_and_arrows_change_the_page() {
    let mut h = open();
    h.key_press_modifiers(Modifiers::COMMAND, Key::F);
    h.run();
    assert!(h.get_by_role(Role::SearchInput).accesskit_node().is_focused());
    h.key_press(Key::Escape);
    h.run();
    h.get_by_role(Role::ListBox).focus();
    h.run();
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(PreferencesBody::state(&h.ctx, h.state().id).selected, "interface");
    h.key_press(Key::End);
    h.run();
    assert_eq!(PreferencesBody::state(&h.ctx, h.state().id).selected, "advanced");
}

#[test]
fn the_swatch_reports_the_row_to_pick() {
    let mut h = open();
    h.get_by_role_and_label(Role::ListBoxOption, "Interface").click();
    h.run();
    h.get_by_role_and_label(Role::ColorWell, "Accent colour:").click();
    h.run();
    assert_eq!(h.state().picked.as_deref(), Some("interface.accent"));
}

#[test]
fn a_hex_field_edit_sets_the_colour_only_when_it_parses() {
    let mut h = open();
    h.get_by_role_and_label(Role::ListBoxOption, "Interface").click();
    h.run();
    let field = h.get_by_role(Role::TextInput);
    field.focus();
    h.run();
    h.get_by_role(Role::TextInput).type_text("#ff0000");
    h.run();
    assert_eq!(h.state().host.draft.get("interface.accent"), Some(&PrefValue::Color(craft_theme::Rgba([255, 0, 0, 255]))));
}

#[test]
fn tab_visits_search_then_the_list_then_the_rows_then_the_buttons() {
    let mut h = open();
    h.get_by_role(Role::SearchInput).focus();
    h.run();
    let mut seen = Vec::new();
    for _ in 0..8 {
        h.key_press(Key::Tab);
        h.run();
        let role = h.query_by(|n| n.is_focused()).map(|n| n.accesskit_node().role());
        if let Some(role) = role {
            seen.push(role);
        }
    }
    // The page list is one stop (its focused node is the lead row); the buttons are Reset
    // Section, OK and Cancel (Apply is disabled), then the order wraps to the search field.
    let expected =
        [Role::ListBoxOption, Role::ComboBox, Role::TextInput, Role::CheckBox, Role::Button, Role::Button, Role::Button, Role::SearchInput];
    assert_eq!(seen, expected);
}

#[test]
fn the_page_list_numbers_the_listed_pages_and_the_page_is_a_named_group() {
    let h = open();
    let list = h.get_by_role_and_label(Role::ListBox, "Pages");
    assert_eq!(list.accesskit_node().size_of_set(), Some(5));
    assert_eq!(h.get_by_role_and_label(Role::ListBoxOption, "Tools").accesskit_node().position_in_set(), Some(2));
    h.get_by_role_and_label(Role::Group, "General");
    h.get_by_role(Role::SearchInput);
}

#[test]
fn the_label_column_is_as_wide_as_the_widest_label_from_the_first_frame() {
    let h = open();
    let label = h.get_by_role_and_label(Role::Label, "Workspace name:").rect();
    let control = h.get_by_role_and_label(Role::TextInput, "Workspace name:").rect();
    let language = h.get_by_role_and_label(Role::ComboBox, "Language:").rect();
    assert!((control.left() - language.left()).abs() < 0.5, "{control:?} {language:?}");
    assert!(control.left() > label.right());
}

#[test]
fn the_body_never_draws_below_the_space_it_is_given() {
    let bottom = std::rc::Rc::new(std::cell::Cell::new(0.0_f32));
    let seen = bottom.clone();
    let h = builder(Direction::Ltr).with_size([560.0, 400.0]).build_ui_state(
        move |ui: &mut Ui, s: &mut State| {
            ui.set_max_height(300.0);
            seen.set(ui.max_rect().bottom());
            let _ = PreferencesBody::new("t").show(ui, &mut s.host);
        },
        State { host: FakeHost::new(), closed: None, picked: None, id: egui::Id::NULL },
    );
    let ok = h.get_by_label("OK").rect();
    assert!(ok.bottom() <= bottom.get() + 0.5, "{ok:?} in {}", bottom.get());
}

#[test]
fn right_to_left_moves_the_page_list_to_the_trailing_side() {
    let ltr = open();
    let rtl = open_with(Direction::Rtl, FakeHost::new());
    let l = ltr.get_by_role(Role::ListBox).rect();
    let r = rtl.get_by_role(Role::ListBox).rect();
    assert!(l.center().x < 380.0 && r.center().x > 380.0, "{l:?} {r:?}");
}

#[test]
fn esc_during_a_number_drag_cancels_the_drag_and_only_a_second_esc_cancels_the_window() {
    let mut h = open();
    h.get_by_role_and_label(Role::ListBoxOption, "Tools").click();
    h.run();
    let at = h.get_by_label("Brush size:").rect().center();
    let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    h.event(egui::Event::PointerMoved(at));
    h.event(button(true));
    h.step();
    for dx in [10.0, 20.0, 30.0] {
        h.event(egui::Event::PointerMoved(at + egui::vec2(dx, 0.0)));
        h.step();
    }
    h.key_press(Key::Escape);
    h.step();
    assert_eq!((h.state().host.logged("cancel"), h.state().closed), (0, None), "the Esc went to the drag");
    h.event(button(false));
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert_eq!((h.state().host.logged("cancel"), h.state().closed), (1, Some(Closed::Cancel)));
}

#[test]
fn two_bodies_with_one_salt_in_different_parents_keep_their_own_page() {
    struct Two {
        hosts: [FakeHost; 2],
        ids: [egui::Id; 2],
    }
    let mut h =
        Harness::<Two>::builder().with_size([760.0, 960.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, s: &mut Two| {
                for n in 0..2 {
                    ui.push_id(n, |ui| {
                        ui.allocate_ui(egui::vec2(760.0, 470.0), |ui| {
                            s.ids[n] = PreferencesBody::id(ui, "t");
                            let _ = PreferencesBody::new("t").show(ui, &mut s.hosts[n]);
                        });
                    });
                }
            },
            Two { hosts: [FakeHost::new(), FakeHost::new()], ids: [egui::Id::NULL; 2] },
        );
    h.run();
    h.get_all_by_role_and_label(Role::ListBoxOption, "Tools")
        .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .expect("the first body's Tools page")
        .click();
    h.run();
    let ids = h.state().ids;
    assert_ne!(ids[0], ids[1]);
    assert_eq!(PreferencesBody::state(&h.ctx, ids[0]).selected, "tools");
    assert_eq!(PreferencesBody::state(&h.ctx, ids[1]).selected, "");
}

#[test]
fn a_dimmed_page_in_the_list_is_described_as_unavailable_and_the_others_are_not() {
    let h = open();
    let empty = h.get_by_role_and_label(Role::ListBoxOption, "Plug-ins").accesskit_node();
    assert_eq!(empty.description().as_deref(), Some("These settings aren't available yet."));
    assert_eq!(h.get_by_role_and_label(Role::ListBoxOption, "Tools").accesskit_node().description(), None);
}

#[test]
fn a_float_row_steps_by_the_hosts_step_even_when_the_range_is_wide() {
    let mut h = open();
    h.get_by_role_and_label(Role::ListBoxOption, "Advanced").click();
    h.run();
    h.get_by_role_and_label(Role::SpinButton, "Gamma:").focus();
    h.run();
    h.key_press(Key::ArrowUp);
    h.run();
    let Some(PrefValue::Float(v)) = h.state().host.draft.get("advanced.gamma") else { panic!("no value set") };
    assert!((v - 20.1).abs() < 1e-9, "stepped by 0.1, got {v}");
}
