//! Access letters: a Mac menu has none, the other platforms do.

use craft_host::target::Os;
use craft_ui::buttons::Button;
use craft_ui::contract::{Show as _, ShowContainer as _, Trigger};
use craft_ui::menu::bar::{MenuBar, MenuTitle};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

const PLATFORMS: [(Os, bool); 2] = [(Os::Linux, true), (Os::Mac, false)];

/// A menu bar title reports its access key off a Mac and none on a Mac. The window draws its own frame, so the bar is in the window on both.
#[trial]
fn a_bar_title_has_an_access_key_off_a_mac_only() {
    for (os, lettered) in PLATFORMS {
        let mut fixture = Fixture::new(|ui| {
            let titles = vec![MenuTitle::new("&File", |ui| {
                MenuItem::new("&Save").show(ui);
            })];
            MenuBar::new("Main menu", titles).show(ui);
        });
        fixture.os(os).chrome(craft_host::target::WindowChrome::Integrated).settle();
        let file = fixture.by_label("File").expect("the File title");
        assert_eq!(fixture.tree().access_key_of(file.node), lettered.then_some('F'), "{os:?}");
    }
}

/// With a menu open, the typed letter of a row runs it off a Mac; on a Mac it runs nothing.
#[trial]
fn a_typed_letter_runs_a_menu_row_off_a_mac_only() {
    for (os, lettered) in PLATFORMS {
        let ran = Rc::new(Cell::new(false));
        let shown = ran.clone();
        let mut fixture = Fixture::new(move |ui| {
            let opener = Button::new("File").show(ui);
            Menu::new("File").opened_by(&opener, Trigger::Press).show(ui, |ui, _| {
                if MenuItem::new("&Save").show(ui).ran {
                    shown.set(true);
                }
            });
        });
        fixture.os(os).settle();
        let opener = fixture.by_label("File").expect("the opener has a node");
        fixture.click(&opener);
        fixture.idle(4);
        fixture.type_text("s");
        fixture.idle(4);
        assert_eq!(ran.get(), lettered, "{os:?}");
    }
}

/// A Mac window that draws its own frame carries the menu bar itself, the only bar: F10 focuses it. The bar has no
/// access letters there, so Alt and a letter open nothing.
#[trial]
fn f10_focuses_the_in_window_bar_on_a_mac() {
    let bar = Rc::new(Cell::new(None));
    let shown = bar.clone();
    let mut fixture = Fixture::new(move |ui| {
        let titles = vec![MenuTitle::new("&File", |ui| {
            MenuItem::new("&Save").show(ui);
        })];
        shown.set(Some(MenuBar::new("Main menu", titles).show(ui).response.id));
    });
    fixture.os(Os::Mac).chrome(craft_host::target::WindowChrome::Integrated).settle();
    assert_ne!(fixture.focused(), bar.get(), "the bar starts unfocused");
    fixture.key(Key::F10, Modifiers::NONE).settle();
    assert_eq!(fixture.focused(), bar.get(), "F10 takes focus for the bar");
}
