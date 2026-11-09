//! `ComboBox` filtering: typed text lists the options that start with it first, then those that contain it, and a
//! text that matches none closes the list and commits as typed.

use craft_ui::contract::{Bind, Entry, Role, Show as _};
use craft_ui::pickers::ComboBox;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

const FAMILIES: [&str; 4] = ["Charter", "Arial", "Garamond", "Verdana"];

struct Rig {
    fixture: Fixture,
    text: Rc<RefCell<String>>,
}

fn rig() -> Rig {
    rig_over(|| FAMILIES.iter().map(|family| Entry::new((*family).to_owned(), *family)).collect())
}

/// A focused combo box over the options `make` builds each frame.
fn rig_over(make: impl Fn() -> Vec<Entry<String>> + 'static) -> Rig {
    let text = Rc::new(RefCell::new(String::new()));
    let shown = text.clone();
    let mut fixture = Fixture::new(move |ui| {
        let options = make();
        let mut text = shown.borrow_mut();
        ComboBox::new("Font", Bind::bound(&mut *text), &options).show(ui);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    Rig { fixture, text }
}

impl Rig {
    /// The names of the option rows the list shows, sorted.
    fn listed(&self) -> Vec<String> {
        let tree = self.fixture.tree();
        let mut names: Vec<String> = tree.with_role(Role::LIST_BOX_OPTION).iter().filter_map(|key| tree.name_of(*key)).map(str::to_owned).collect();
        names.sort_unstable();
        names
    }

    fn type_text(&mut self, text: &str) {
        self.fixture.type_text(text);
        self.fixture.idle(4);
    }

    fn press(&mut self, key: Key) {
        self.fixture.key(key, Modifiers::NONE);
        self.fixture.idle(4);
    }
}

/// "ar" lists the options that contain it, and Enter picks the one that starts with it although it comes later in
/// the options. The filter reads the labels the rows show: "sys" lists the option labelled "Arial (system)".
#[trial]
fn typed_text_lists_the_matches_by_their_labels_and_enter_picks_the_one_that_starts_with_it() {
    let mut rig = rig();
    rig.type_text("ar");
    assert_eq!(rig.listed(), ["Arial", "Charter", "Garamond"]);
    rig.press(Key::Enter);
    assert_eq!(*rig.text.borrow(), "Arial");

    let mut rig = rig_over(|| vec![Entry::new("Arial".to_owned(), "Arial (system)"), Entry::new("Charter".to_owned(), "Charter")]);
    rig.type_text("sys");
    assert_eq!(rig.listed(), ["Arial (system)"]);
}

/// Text that matches no option shows no list, and Enter keeps the text as typed.
#[trial]
fn text_that_matches_nothing_closes_the_list_and_commits_as_typed() {
    let mut rig = rig();
    rig.type_text("ar");
    rig.type_text("z");
    assert!(rig.listed().is_empty(), "no option contains `arz`");
    rig.press(Key::Enter);
    assert_eq!(*rig.text.borrow(), "arz");
}
