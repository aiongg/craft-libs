//! Tests of the Brush Settings section list (task BR-BS4).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::theme::{Base, install};
use egui::accesskit::{Role, Toggled};
use egui::{Key, Modifiers, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{BrushSection, BrushSectionEvent, BrushSectionList};

type Event = BrushSectionEvent<&'static str>;

struct Host {
    sections: Vec<BrushSection<&'static str>>,
    selected: Option<&'static str>,
    link: bool,
    events: Vec<Event>,
}

impl Host {
    fn new() -> Self {
        Host {
            sections: vec![
                BrushSection::pane("tip", "Brush Tip Shape"),
                BrushSection::pane("dyn", "Shape Dynamics").enabled(false).locked(false),
                BrushSection::pane("scatter", "Scattering").enabled(true).locked(true),
                BrushSection::switch("noise", "Noise", false),
                BrushSection::switch("wet", "Wet Edges", true),
            ],
            selected: Some("tip"),
            link: false,
            events: Vec::new(),
        }
    }

    fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

// The host only follows `Select`; the other changes are read from the events.
fn show(ui: &mut Ui, host: &mut Host) {
    let out = BrushSectionList::new("sections", &host.sections).selected(host.selected.as_ref()).link(host.link).show(ui);
    for event in out.events {
        if let BrushSectionEvent::Select(key) = &event {
            host.selected = Some(*key);
        }
        host.events.push(event);
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([320.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn named<'h>(harness: &'h Harness<'_, Host>, role: Role, label: &'static str) -> Node<'h> {
    harness.get_by(move |n| n.role() == role && n.label().as_deref() == Some(label))
}

/// Lets the double-click delay pass, so the next click is a single click.
fn pause(harness: &mut Harness<'_, Host>) {
    harness.run_steps(30);
}

fn key(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

// A click on an off pane row's name reports `Select` then `Enable { on: true }`; on an on row only `Select`.
#[test]
fn a_click_on_a_pane_row_selects_and_enables_it() {
    let mut harness = harness(Host::new());
    named(&harness, Role::Button, "Shape Dynamics").click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::Select("dyn"), Event::Enable { key: "dyn", on: true }]);
    pause(&mut harness);
    named(&harness, Role::Button, "Scattering").click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::Select("scatter")]);
}

// A click on the enable box reports `Enable` with the flipped state and no `Select`.
#[test]
fn the_enable_box_toggles_without_selecting() {
    let mut harness = harness(Host::new());
    named(&harness, Role::CheckBox, "Enable Shape Dynamics").click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::Enable { key: "dyn", on: true }]);
    pause(&mut harness);
    named(&harness, Role::CheckBox, "Enable Scattering").click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::Enable { key: "scatter", on: false }]);
}

// A click on a switch row reports `Switch` with the flipped state and never `Select`.
#[test]
fn switch_rows_only_toggle() {
    let mut harness = harness(Host::new());
    named(&harness, Role::ListBoxOption, "Noise").click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::Switch { key: "noise", on: true }]);
    pause(&mut harness);
    named(&harness, Role::ListBoxOption, "Wet Edges").click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::Switch { key: "wet", on: false }]);
}

// A click on the lock reports `Lock` with the flipped state only; the lock shows on unlocked rows too.
#[test]
fn the_lock_toggles_without_selecting() {
    let mut harness = harness(Host::new());
    let lock = named(&harness, Role::Button, "Lock: Shape Dynamics");
    assert_eq!(lock.accesskit_node().toggled(), Some(Toggled::False), "an open lock is drawn and named");
    lock.click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::Lock { key: "dyn", on: true }]);
    pause(&mut harness);
    named(&harness, Role::Button, "Lock: Scattering").click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::Lock { key: "scatter", on: false }]);
}

// Focused list: Down onto a pane row reports `Select`; Space toggles its box (`Enable`) or a switch
// (`Switch`); Shift+Space its lock; Enter on an off pane row reports `Select` and `Enable`.
#[test]
fn section_keys_move_toggle_and_lock() {
    let mut harness = harness(Host::new());
    key(&mut harness, Modifiers::NONE, Key::Tab);
    assert!(harness.get_by_role(Role::ListBox).accesskit_node().active_descendant().is_some(), "Tab reaches the list");
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(harness.state_mut().take_events(), [Event::Select("dyn")]);
    key(&mut harness, Modifiers::NONE, Key::Space);
    assert_eq!(harness.state_mut().take_events(), [Event::Enable { key: "dyn", on: true }]);
    key(&mut harness, Modifiers::SHIFT, Key::Space);
    assert_eq!(harness.state_mut().take_events(), [Event::Lock { key: "dyn", on: true }]);
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state_mut().take_events(), [Event::Select("dyn"), Event::Enable { key: "dyn", on: true }]);
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(harness.state_mut().take_events(), [Event::Select("scatter")], "the lead moves onto a switch without selecting it");
    key(&mut harness, Modifiers::NONE, Key::Space);
    assert_eq!(harness.state_mut().take_events(), [Event::Switch { key: "noise", on: true }]);
    key(&mut harness, Modifiers::NONE, Key::Home);
    assert_eq!(harness.state_mut().take_events(), [Event::Select("tip")]);
}

// With `link(true)` the "Brushes" row shows first and a click reports `OpenLink`; without it, no row.
#[test]
fn the_link_row_reports_open_link() {
    let mut host = Host::new();
    assert!(harness(Host::new()).query_by_label("Brushes").is_none());
    host.link = true;
    let mut harness = harness(host);
    let link = named(&harness, Role::Button, "Brushes");
    let first = named(&harness, Role::Button, "Brush Tip Shape");
    assert!(link.rect().top() < first.rect().top(), "the link row comes first");
    link.click();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [Event::OpenLink]);
}

// AccessKit: pane rows are Buttons with `selected`, boxes CheckBoxes "Enable {name}", locks
// Buttons "Lock: {name}" with a state, switch rows one CheckBox each.
#[test]
fn section_rows_have_their_accessible_roles_and_names() {
    let harness = harness(Host::new());
    let list = harness.get_by_role(Role::ListBox);
    assert_eq!(list.accesskit_node().label().as_deref(), Some("Brush settings sections"));
    assert_eq!(named(&harness, Role::Button, "Brush Tip Shape").accesskit_node().is_selected(), Some(true));
    assert_eq!(named(&harness, Role::Button, "Shape Dynamics").accesskit_node().is_selected(), Some(false));
    assert_eq!(named(&harness, Role::CheckBox, "Enable Scattering").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(named(&harness, Role::CheckBox, "Enable Shape Dynamics").accesskit_node().toggled(), Some(Toggled::False));
    assert_eq!(named(&harness, Role::Button, "Lock: Scattering").accesskit_node().toggled(), Some(Toggled::True));
    assert!(harness.query_by_label("Enable Brush Tip Shape").is_none(), "no box, no node");
    assert!(harness.query_by_label("Lock: Brush Tip Shape").is_none(), "no lock, no node");
    for (name, on) in [("Noise", false), ("Wet Edges", true)] {
        let boxes = harness.query_all_by_role_and_label(Role::CheckBox, name).count();
        assert_eq!(boxes, 1, "{name} is one check box");
        let toggled = named(&harness, Role::CheckBox, name).accesskit_node().toggled();
        assert_eq!(toggled, Some(if on { Toggled::True } else { Toggled::False }));
    }
}
