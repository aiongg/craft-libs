//! The Show areas, the app's tabs and the state snapshot automation reads.

use craft_keymap::Platform;
use craft_menu::shortcuts::{Area, DialogTab, Outcome, ShortcutsDialog};
use craft_ui::contract::Show as _;
use craft_ui::display::Text;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

use super::fixture::{keymap, options};

/// Area::photoshop puts File > Save in Application Menus, a panel-scoped command in Panel Menus and a tool in Tools.
#[trial]
fn photoshop_areas_split_menus_panels_and_tools() {
    let areas = Area::photoshop();
    let labels: Vec<_> = areas.iter().map(|a| a.label).collect();
    assert_eq!(labels, ["Application Menus", "Panel Menus", "Tools"]);
    let km = keymap(Platform::Mac);
    let holding = |id: &str| -> Vec<&str> {
        let b = km.catalog().iter().find(|b| b.command.id == id).unwrap_or_else(|| panic!("{id}"));
        areas.iter().filter(|a| (a.rows)(b)).map(|a| a.label).collect()
    };
    assert_eq!(holding("file.save"), ["Application Menus"]);
    assert_eq!(holding("timeline.loop"), ["Panel Menus"]);
    assert_eq!(holding("tool.marquee"), ["Tools"]);
}

/// A second tab draws its body when chosen, Ctrl+Tab moves between tabs, and a body returning true reports Changed.
#[trial]
fn app_tabs_switch_and_report_changes() {
    struct Host {
        km: craft_keymap::Keymap,
        state: craft_menu::shortcuts::DialogState,
        options: craft_menu::shortcuts::Options,
        outcomes: Vec<Outcome>,
    }
    let host = Rc::new(RefCell::new(Host { km: keymap(Platform::Windows), state: Default::default(), options: options(), outcomes: Vec::new() }));
    let pass = host.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut guard = pass.borrow_mut();
        let Host { km, state, options, outcomes } = &mut *guard;
        let mut tabs = [DialogTab::new("menus", "Menus", |ui| {
            Text::new("Menu colours").show(ui);
            craft_ui::contract::clicked(&craft_ui::buttons::Button::new("Change colour").show(ui))
        })];
        outcomes.extend(ShortcutsDialog::new(km).options(options).tabs(&mut tabs).show(ui, state).outcome);
    });
    fixture.room(egui::vec2(1000.0, 700.0));
    fixture.settle();
    let tab = |host: &Rc<RefCell<Host>>| host.borrow().state.snapshot(&host.borrow().km).map(|s| s.tab);
    assert!(fixture.by_label("Menu colours").is_none(), "Keyboard Shortcuts is the first tab");
    assert_eq!(tab(&host), Some("shortcuts".to_string()));

    let menus = fixture.by_label("Menus").unwrap_or_else(|| panic!("the Menus tab"));
    fixture.click(&menus);
    fixture.idle(3);
    assert!(fixture.by_label("Menu colours").is_some());
    assert_eq!(tab(&host), Some("menus".to_string()));

    fixture.key(Key::Tab, Modifiers::CTRL);
    fixture.idle(3);
    assert!(fixture.by_label("Menu colours").is_none(), "Ctrl+Tab wraps to Keyboard Shortcuts");
    fixture.key(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT);
    fixture.idle(3);
    assert!(fixture.by_label("Menu colours").is_some(), "Ctrl+Shift+Tab goes back");

    let change = fixture.by_label("Change colour").unwrap_or_else(|| panic!("the tab's button"));
    fixture.click(&change);
    fixture.idle(3);
    assert!(host.borrow().outcomes.iter().any(|o| matches!(o, Outcome::Changed)));
}
