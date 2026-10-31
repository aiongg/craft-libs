//! The pasteboard colour choice in a kittest harness: its rows and the picker request.

use craft_theme::Srgba;
use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{ChosenColor, ColorChoice, NamedColor};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

struct Host {
    value: ChosenColor,
    custom: Option<Srgba>,
    picks: usize,
    events: Vec<EditEvent<ChosenColor>>,
}

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 480.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

fn pasteboard() -> Harness<'static, Host> {
    let host = Host { value: ChosenColor::Named("default".into()), custom: Some(Srgba::rgb(1.0, 0.0, 0.0)), picks: 0, events: Vec::new() };
    let mut harness = harness().build_ui_state(
        |ui, host: &mut Host| {
            let named = [
                NamedColor::new("default", "Default", None),
                NamedColor::new("black", "Black", Some(Srgba::rgb(0.0, 0.0, 0.0))),
                NamedColor::new("darkGray", "Dark Gray", Some(Srgba::rgb(0.25, 0.25, 0.25))),
            ];
            let shown = ColorChoice::new("pasteboard", "Pasteboard", &host.value).named(named).custom(host.custom).show(ui);
            host.picks += usize::from(shown.pick_custom);
            if let Some(event) = shown.edit.event {
                if let EditEvent::Set { value, .. } = &event {
                    host.value = value.clone();
                }
                host.events.push(event);
            }
        },
        host,
    );
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness
}

fn options(harness: &Harness<'_, Host>) -> Vec<String> {
    harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().label()).collect()
}

/// The rows are the named colours, Custom when there is one, then Select Custom Color….
#[test]
fn rows_follow_the_pasteboard_menu() {
    let harness = pasteboard();
    assert_eq!(options(&harness), ["Default", "Black", "Dark Gray", "Custom", "Select Custom Color…"]);
}

/// Select Custom Color… asks for the picker and leaves the choice unchanged.
#[test]
fn select_custom_asks_for_the_picker() {
    let mut harness = pasteboard();
    harness.get_by_label("Select Custom Color…").click();
    harness.run();
    assert_eq!(harness.state().picks, 1);
    assert_eq!(harness.state().value, ChosenColor::Named("default".into()));
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().events);

    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by_label("Black").click();
    harness.run();
    assert_eq!(harness.state().value, ChosenColor::Named("black".into()));
}
