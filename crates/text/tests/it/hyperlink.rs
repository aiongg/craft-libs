//! The hyperlink dialog: which link OK answers for each target and each kind of current link.

use craft_text::ui::hyperlink::{CurrentLink, Hyperlink, HyperlinkAnswer, HyperlinkDialog, LinkTarget};
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    answers: Rc<RefCell<Vec<HyperlinkAnswer>>>,
}

impl Rig {
    fn new(dialog: HyperlinkDialog) -> Rig {
        let mut host = DialogHost::new();
        host.open(dialog);
        let host = Rc::new(RefCell::new(host));
        let answers = Rc::new(RefCell::new(Vec::new()));
        let sink = answers.clone();
        let mut fixture = Fixture::new(move |ui| {
            host.borrow_mut().run(ui);
            if let Some(DialogResult::Ok(answer)) = host.borrow_mut().take::<HyperlinkDialog>() {
                sink.borrow_mut().push(answer);
            }
        });
        fixture.settle();
        Rig { fixture, answers }
    }

    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("`{label}` is shown"));
        self.fixture.click(&target);
    }

    fn type_into(&mut self, field: &str, text: &str) {
        self.click(field);
        self.fixture.key(Key::A, Modifiers::COMMAND);
        self.fixture.type_text(text);
    }

    fn ok(&mut self) -> Option<HyperlinkAnswer> {
        self.click("OK");
        self.answers.borrow().last().cloned()
    }
}

fn set(target: LinkTarget) -> HyperlinkAnswer {
    HyperlinkAnswer::set(Hyperlink::new(target), None)
}

/// A new link starts at `https://`; a typed address and OK answer Set with it.
#[trial]
fn new_links_start_at_https_and_set() {
    let mut rig = Rig::new(HyperlinkDialog::new(CurrentLink::None, Some("word".to_owned())));
    rig.type_into("Address:", "https://example.org");
    assert_eq!(rig.ok(), Some(set(LinkTarget::Url("https://example.org".to_owned()))));
}

/// Over a mixed selection OK answers Keep; a typed address then answers Set.
#[trial]
fn mixed_links_are_kept_until_a_new_target() {
    let mut rig = Rig::new(HyperlinkDialog::new(CurrentLink::Mixed, None));
    assert_eq!(rig.ok(), Some(HyperlinkAnswer::Keep));

    let mut rig = Rig::new(HyperlinkDialog::new(CurrentLink::Mixed, None));
    rig.type_into("Address:", "https://a.example");
    assert_eq!(rig.ok(), Some(set(LinkTarget::Url("https://a.example".to_owned()))));
}

/// E-mail Address answers an email target; Place in This Document answers the picked place's index in `places`
/// (Top of the Document is not one of them).
#[trial]
fn email_and_place_targets() {
    let mut rig = Rig::new(HyperlinkDialog::new(CurrentLink::None, None));
    rig.click("E-mail Address");
    rig.type_into("E-mail address:", "a@b.example");
    assert_eq!(rig.ok(), Some(set(LinkTarget::email("a@b.example", None))));

    let places = vec!["1. Title".to_owned(), "2. Plan".to_owned()];
    let mut rig = Rig::new(HyperlinkDialog::new(CurrentLink::None, None).places(places));
    rig.click("Place in This Document");
    assert!(rig.fixture.by_label("Top of the Document").is_some());
    rig.click("2. Plan");
    assert_eq!(rig.ok(), Some(set(LinkTarget::Place(1))));
    {
        let mut rig = Rig::new(HyperlinkDialog::new(CurrentLink::None, None));
        rig.click("E-mail Address");
        rig.type_into("E-mail address:", "a@b.example");
        rig.type_into("Subject:", "Hello");
        assert_eq!(rig.ok(), Some(set(LinkTarget::email("a@b.example", Some("Hello".to_owned())))));
    }
}

/// Top of the Document is its own target, and an edited Top link reopens with that row picked.
#[trial]
fn the_top_of_the_document_is_its_own_target() {
    let places = vec!["1. Title".to_owned()];
    let mut rig = Rig::new(HyperlinkDialog::new(CurrentLink::None, None).places(places.clone()));
    rig.click("Place in This Document");
    rig.click("Top of the Document");
    assert_eq!(rig.ok(), Some(set(LinkTarget::Top)));

    let current = CurrentLink::One(Hyperlink::new(LinkTarget::Top));
    let mut rig = Rig::new(HyperlinkDialog::new(current, None).places(places));
    assert_eq!(rig.ok(), Some(set(LinkTarget::Top)));
}

/// Editing an e-mail link opens on its address (Word's Insert Hyperlink, E-mail Address): typing replaces it with no
/// click first.
#[trial]
fn an_email_link_opens_on_its_address() {
    let current = CurrentLink::One(Hyperlink::new(LinkTarget::email("a@b.example", None)));
    let mut rig = Rig::new(HyperlinkDialog::new(current, None));
    rig.fixture.key(Key::A, Modifiers::COMMAND);
    rig.fixture.type_text("c@d.example");
    assert_eq!(rig.ok(), Some(set(LinkTarget::email("c@d.example", None))));
}
