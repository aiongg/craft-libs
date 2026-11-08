//! Access letters: a Mac menu has none, the other platforms do.

use craft_host::target::Os;
use craft_ui::contract::Show as _;
use craft_ui::menu::MenuItem;
use craft_ui::menu::bar::{MenuBar, MenuTitle};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    ran: Rc<RefCell<Vec<&'static str>>>,
}

fn rig(os: Os) -> Rig {
    let ran = Rc::new(RefCell::new(Vec::new()));
    let log = ran.clone();
    let mut fixture = Fixture::new(move |ui| {
        let log = log.clone();
        let titles = vec![MenuTitle::new("&File", move |ui| {
            MenuItem::new("&New").show(ui);
            if MenuItem::new("&Save").show(ui).ran {
                log.borrow_mut().push("Save");
            }
        })];
        MenuBar::new("Main menu", titles).show(ui);
    });
    fixture.os(os).settle();
    Rig { fixture, ran }
}

/// With a menu open, the typed letter of a row runs it off a Mac; on a Mac the bar's title reports no access key and
/// the letter runs nothing.
#[trial]
fn typed_letters_and_access_keys_exist_off_a_mac_only() {
    for (os, lettered) in [(Os::Linux, true), (Os::Mac, false)] {
        let mut rig = rig(os);
        let file = rig.fixture.by_label("File").expect("the File title");
        assert_eq!(rig.fixture.tree().access_key_of(file.node), lettered.then_some('F'), "{os:?}");
        rig.fixture.click(&file);
        rig.fixture.settle();
        rig.fixture.type_text("s");
        rig.fixture.settle();
        assert_eq!(*rig.ran.borrow(), if lettered { vec!["Save"] } else { Vec::new() }, "{os:?}");
    }
}
