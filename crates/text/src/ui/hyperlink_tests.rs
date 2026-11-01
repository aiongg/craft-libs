//! The hyperlink dialog.

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::hyperlink::*;

struct App {
    host: DialogHost,
    answers: Vec<HyperlinkAnswer>,
}

fn open(dialog: HyperlinkDialog) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(dialog);
    let mut harness =
        HarnessBuilder::default().with_size(vec2(900.0, 900.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(answer)) = app.host.take::<HyperlinkDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        );
    harness.run();
    harness
}

fn type_into(harness: &mut Harness<'_, App>, field: &str, text: &str) {
    harness.get_by_role_and_label(Role::TextInput, field).focus();
    harness.run();
    harness.event(Event::Text(text.into()));
    harness.run();
}

fn ok(harness: &mut Harness<'_, App>) {
    harness.get_by_label("OK").click();
    harness.run();
}

fn answer(harness: &Harness<'_, App>) -> Option<HyperlinkAnswer> {
    harness.state().answers.last().cloned()
}

fn set(target: LinkTarget) -> HyperlinkAnswer {
    HyperlinkAnswer::Set { link: Hyperlink { target, tooltip: String::new(), highlight_click: false }, text: None }
}

// Over CurrentLink::None the address starts "https://"; typing an address and OK returns Set with it.
#[test]
fn new_links_start_at_https_and_set() {
    let mut harness = open(HyperlinkDialog::new(CurrentLink::None, Some("word".into())));
    assert_eq!(harness.get_by_role_and_label(Role::TextInput, "Address:").accesskit_node().value(), Some("https://".to_owned()));
    type_into(&mut harness, "Address:", "https://example.org");
    ok(&mut harness);
    assert_eq!(answer(&harness), Some(set(LinkTarget::Url("https://example.org".into()))));
}

// Over Mixed, OK returns Keep; typing an address then OK returns Set.
#[test]
fn mixed_links_are_kept_until_a_new_target() {
    let mut harness = open(HyperlinkDialog::new(CurrentLink::Mixed, None));
    ok(&mut harness);
    assert_eq!(answer(&harness), Some(HyperlinkAnswer::Keep));

    let mut harness = open(HyperlinkDialog::new(CurrentLink::Mixed, None));
    type_into(&mut harness, "Address:", "https://a.example");
    ok(&mut harness);
    assert_eq!(answer(&harness), Some(set(LinkTarget::Url("https://a.example".into()))));
}

// Choosing E-mail Address returns LinkTarget::Email; Place in This Document returns the picked place's index.
#[test]
fn email_and_place_targets() {
    let mut harness = open(HyperlinkDialog::new(CurrentLink::None, None));
    harness.get_by_role_and_label(Role::RadioButton, "E-mail Address").click();
    harness.run();
    type_into(&mut harness, "E-mail address:", "a@b.example");
    ok(&mut harness);
    assert_eq!(answer(&harness), Some(set(LinkTarget::Email("a@b.example".into()))));

    let places = vec!["".to_owned(), "1. Title".to_owned(), "2. Plan".to_owned()];
    let mut harness = open(HyperlinkDialog::new(CurrentLink::None, None).places(places));
    harness.get_by_role_and_label(Role::RadioButton, "Place in This Document").click();
    harness.run();
    harness.get_by_label("Top of the Document");
    harness.get_by_label("2. Plan").click();
    harness.run();
    ok(&mut harness);
    assert_eq!(answer(&harness), Some(set(LinkTarget::Place(2))));
}
