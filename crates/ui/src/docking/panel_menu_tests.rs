//! The panel-menu host in a harness.

use egui::{Id, Key, vec2};
use egui_kittest::{Harness, kittest::Queryable};

use super::*;
use crate::menu::{MenuEntry, MenuItem};

/// A start dock with one column holding `[2, 3]` (3 selected) above `[4]`.
fn docked() -> Layout<u32> {
    Layout {
        root: Some(Node::Dock { side: Side::Start, columns: vec![Column::new(vec![Group::new(vec![2, 3], 1), Group::new(vec![4], 0)], 240.0)] }),
        floating: Vec::new(),
    }
}

#[derive(Default)]
struct Host {
    locked: bool,
    /// Calls of `panel_menu_entries`.
    asked: usize,
}

impl DockContent<u32> for Host {
    fn body(&mut self, ui: &mut egui::Ui, _: &u32) {
        ui.label("Body");
    }
    fn logical_label(&self, panel: &u32) -> String {
        format!("Panel {panel}")
    }
    fn permissions(&self, _: &u32) -> Permissions {
        if self.locked { Permissions::PROTECTED } else { Permissions::default() }
    }
    fn action_label(&mut self, _: &egui::Ui, logical: &str, _: &egui::FontId, _: egui::Color32) -> egui::WidgetText {
        format!("«{logical}»").into()
    }
    fn panel_menu_entries(&mut self, panel: &u32) -> Vec<MenuEntry<String>> {
        self.asked += 1;
        vec![MenuItem::new(format!("Host Row {panel}"), "host.cmd".to_owned()).into()]
    }
}

struct State {
    host: Host,
    events: Vec<DockEvent<u32>>,
}

fn harness(locked: bool) -> Harness<'static, State> {
    let mut h = Harness::builder().with_size(vec2(800.0, 600.0)).build_ui_state(
        |ui, state: &mut State| {
            let mut style = DockStyle::from_ui(ui);
            style.panel_menu_button = true;
            let out = DockArea::new(Id::unique("panel-menu-area")).show_content(ui, &docked(), &style, &mut state.host);
            state.events.extend(out.events);
        },
        State { host: Host { locked, asked: 0 }, events: Vec::new() },
    );
    h.run_steps(2);
    h
}

fn open_first_menu(h: &mut Harness<'_, State>) {
    h.query_all_by_label("Panel Menu").next().unwrap().click();
    h.run_steps(3);
}

#[test]
// The ≡ button opens a menu with the host's rows first, then a separator and Float, Move Panel To, Close.
fn panel_menu_lists_host_rows_then_group_items() {
    let mut h = harness(false);
    open_first_menu(&mut h);
    let y = |label: &str| h.get_by_label(label).rect().top();
    let rows = [y("Host Row 3"), y("«Float Panel»"), y("«Move Panel to Group»"), y("«Close Panel»"), y("«Close Tab Group»")];
    assert!(rows.windows(2).all(|pair| pair[0] < pair[1]), "{rows:?}");
}

#[test]
// Choosing a host row emits DockEvent::PanelCommand with the panel and the id and closes the menu.
fn host_row_emits_panel_command_event() {
    let mut h = harness(false);
    open_first_menu(&mut h);
    h.get_by_label("Host Row 3").click();
    h.run_steps(3);
    assert_eq!(h.state().events, vec![DockEvent::PanelCommand { panel: 3, command: "host.cmd".to_owned() }]);
    assert_eq!(h.query_all_by_label("Host Row 3").count(), 0, "the menu closed");
}

#[test]
// A locked group's menu has the host rows only.
fn locked_group_hides_dock_rows() {
    let mut h = harness(true);
    open_first_menu(&mut h);
    assert_eq!(h.query_all_by_label("Host Row 3").count(), 1);
    for label in ["«Float Panel»", "«Move Panel to Group»", "«Close Panel»", "«Close Tab Group»", "«Float Tab Group»"] {
        assert_eq!(h.query_all_by_label(label).count(), 0, "{label}");
    }
}

#[test]
// Escape closes the menu and returns focus to the ≡ button.
fn escape_returns_focus_to_menu_button() {
    let mut h = harness(false);
    h.query_all_by_label("Panel Menu").next().unwrap().focus();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert_eq!(h.query_all_by_label("Host Row 3").count(), 1, "Enter opens the menu");
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert_eq!(h.query_all_by_label("Host Row 3").count(), 0, "Escape closes it");
    assert!(h.query_all_by_label("Panel Menu").next().unwrap().is_focused());
}

#[test]
// The host is asked for its entries only while a menu opens or is open, not on every pass.
fn host_entries_are_asked_only_for_an_open_menu() {
    let mut h = harness(false);
    h.run_steps(3);
    assert_eq!(h.state().host.asked, 0, "closed menus ask nothing");
    open_first_menu(&mut h);
    assert!(h.state().host.asked > 0);
}

#[test]
// Dock rows show the host's shaped action text, as the closure path does.
fn dock_rows_use_the_hosts_action_text() {
    let mut h = harness(false);
    open_first_menu(&mut h);
    assert_eq!(h.query_all_by_label("«Float Panel»").count(), 1);
    assert_eq!(h.query_all_by_label("Float Panel").count(), 0);
}
