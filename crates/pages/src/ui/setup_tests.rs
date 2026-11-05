//! Page Setup and the Layout menus.
#![allow(clippy::expect_used, reason = "tests")]

use std::sync::Arc;

use craft_host::{AppInfo, Host};
use craft_i18n::{Embedded, I18n};
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::units::LengthUnit;
use craft_ui::menu::{MenuEntry, PressMenu};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::setup::*;
use crate::setup::{Margins, Orientation, PageSetup};

#[derive(Default)]
struct App {
    host: DialogHost,
    answers: Vec<PageSetup>,
}

fn open(dialog: PageSetupDialog) -> Harness<'static, App> {
    let mut app = App::default();
    app.host.open(dialog);
    let mut harness =
        HarnessBuilder::default().with_size(vec2(1000.0, 1000.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(answer)) = app.host.take::<PageSetupDialog>() {
                    app.answers.push(answer);
                }
            },
            app,
        );
    harness.run();
    harness
}

fn edited(harness: &Harness<'_, App>) -> PageSetup {
    harness.state().host.get::<PageSetupDialog>().expect("open").setup().clone()
}

fn type_into(harness: &mut Harness<'_, App>, field: &str, text: &str, key: Key) {
    harness.get_all_by_role_and_label(Role::SpinButton, field).next().expect("the field is shown").focus();
    harness.run();
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(key);
    harness.run();
}

// Typing 1.5 in Top (inches) and pressing Enter returns margins.top 108.
#[test]
fn ok_returns_the_edited_setup() {
    let mut harness = open(PageSetupDialog::new(PageSetup::default()).unit(LengthUnit::Inches));
    type_into(&mut harness, "Top", "1.5", Key::Enter);
    let answers = &harness.state().answers;
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].margins.top, 108.0);
    assert_eq!(answers[0].margins.bottom, 72.0, "an untouched margin keeps its value");
}

// Left and right of 4.5 in on Letter (4 in each fit exactly, as in Word) keep the dialog open with the message under the fields.
#[test]
fn margins_that_dont_fit_refuse_ok() {
    let mut harness = open(PageSetupDialog::new(PageSetup::default()).unit(LengthUnit::Inches));
    type_into(&mut harness, "Left", "4.5", Key::Tab);
    type_into(&mut harness, "Right", "4.5", Key::Enter);
    assert!(harness.state().answers.is_empty());
    assert!(harness.state().host.get::<PageSetupDialog>().is_some(), "the dialog stays open");
    assert!(harness.query_all_by_label(&PageSetupLabels::default().margins_dont_fit).next().is_some(), "the message shows");
    type_into(&mut harness, "Right", "1", Key::Enter);
    assert_eq!(harness.state().answers.len(), 1);
}

// Picking A4 sets 595.28 × 841.89; Landscape swaps them; typing a width larger than the height selects Landscape and Custom size stays chosen while the numbers match no name.
#[test]
fn paper_list_and_orientation_follow_each_other() {
    let mut harness = open(PageSetupDialog::new(PageSetup::default()).unit(LengthUnit::Inches));
    harness.get_by_role_and_label(Role::ComboBox, "Paper size").click();
    harness.run();
    harness.get_by_label("A4").click();
    harness.run();
    let setup = edited(&harness);
    assert_eq!((setup.width, setup.height), (595.28, 841.89));

    harness.get_by_label("Landscape").click();
    harness.run();
    let setup = edited(&harness);
    assert_eq!((setup.width, setup.height, setup.orientation), (841.89, 595.28, Orientation::Landscape));

    harness.get_by_label("Portrait").click();
    harness.run();
    type_into(&mut harness, "Width", "12", Key::Tab);
    let setup = edited(&harness);
    assert_eq!((setup.width, setup.orientation), (864.0, Orientation::Landscape));
    assert_eq!(harness.get_by_role_and_label(Role::ComboBox, "Paper size").value().as_deref(), Some("Custom size"));
}

// Over Moderate margins the Moderate item is checked; Normal reads "Normal  1" all" in inches; Custom Margins… returns PageSetup.
#[test]
fn margins_menu_checks_the_matching_preset() {
    let setup = PageSetup { margins: Margins { top: 72.0, bottom: 72.0, left: 54.0, right: 54.0 }, ..PageSetup::default() };
    let entries = PageLayoutMenus::new(&Context::default(), &setup, LengthUnit::Inches).margins();
    let items: Vec<_> = entries.iter().filter_map(MenuEntry::item).collect();
    let checked: Vec<_> = items.iter().filter(|item| item.item_kind().checked() == Some(true)).map(|item| item.label()).collect();
    assert_eq!(checked, ["Moderate"]);
    assert!(items.iter().any(|item| item.label() == "Normal  1\" all"));
    assert!(items.iter().any(|item| item.label() == "Narrow  0.5\" all"));
    let custom = items.last().expect("the last item");
    assert_eq!((custom.label(), custom.command()), ("Custom Margins…", Some(&PageLayoutChoice::PageSetup)));
    let metric = PageLayoutMenus::new(&Context::default(), &setup, LengthUnit::Centimetres).margins();
    assert!(metric.iter().filter_map(MenuEntry::item).any(|item| item.label() == "Normal  2.54 cm all"));
}

// A field's Cancel (Esc in a field, or any other cancel) puts back the exact setup from before its Begin: a typing preview of 108 pt gives 72 again, and a width typed past the height turns the orientation back too.
#[test]
fn a_cancel_restores_the_setup_a_gesture_began_from() {
    let original = PageSetup::default();
    let mut setup = original.clone();
    let (mut draft, id) = (FieldDraft::default(), GestureId::next());
    let top = |next: &mut PageSetup, value: f64| next.margins.top = value;
    draft.apply("top", &mut setup, EditEvent::begin(id), top);
    draft.apply("top", &mut setup, EditEvent::update(id, 108.0, Modifiers::NONE), top);
    assert_eq!(setup.margins.top, 108.0, "the update previews");
    draft.apply("top", &mut setup, EditEvent::cancel(id, CancelReason::Escape), top);
    assert_eq!(setup, original);

    let width = |next: &mut PageSetup, value: f64| next.set_size(value, next.height);
    draft.apply("width", &mut setup, EditEvent::begin(id), width);
    draft.apply("width", &mut setup, EditEvent::update(id, 864.0, Modifiers::NONE), width);
    assert_eq!(setup.orientation, Orientation::Landscape, "the preview turns the page");
    draft.apply("width", &mut setup, EditEvent::cancel(id, CancelReason::Escape), width);
    assert_eq!(setup, original, "the width and the orientation are back");

    draft.apply("top", &mut setup, EditEvent::begin(id), top);
    draft.apply("top", &mut setup, EditEvent::commit(id, 36.0, true, Modifiers::NONE), top);
    draft.apply("top", &mut setup, EditEvent::cancel(id, CancelReason::Escape), top);
    assert_eq!(setup.margins.top, 36.0, "a commit ends the gesture, so a later cancel keeps it");
}

// While Top is typed in, the dialog's setup follows the text (the page preview reads it).
#[test]
fn typing_previews_in_the_dialog_setup() {
    let mut harness = open(PageSetupDialog::new(PageSetup::default()).unit(LengthUnit::Inches));
    harness.get_all_by_role_and_label(Role::SpinButton, "Top").next().expect("the field is shown").focus();
    harness.run();
    harness.event(Event::Text("1.5".into()));
    harness.run();
    assert_eq!(edited(&harness).margins.top, 108.0);
}

// A text no number can come from keeps OK disabled until it is fixed.
#[test]
fn a_rejected_text_disables_ok() {
    let mut harness = open(PageSetupDialog::new(PageSetup::default()).unit(LengthUnit::Inches));
    harness.get_all_by_role_and_label(Role::SpinButton, "Top").next().expect("the field is shown").focus();
    harness.run();
    harness.event(Event::Text("abc".into()));
    harness.run();
    assert!(harness.get_by_label("OK").accesskit_node().is_disabled());
}

static CATALOGS: &Embedded = &[("de", "@id\tpageSetup.paperA4\tA4 Papier\n@id\tpageSetup.presetNarrow\tSchmal\n", "")];

// With a German catalog the Size menu names A4 "A4 Papier" and gives its size, in German digits, as the row's detail line; the Margins menu names Narrow "Schmal".
#[test]
fn the_menus_read_words_and_digits_from_the_context() {
    let (i18n, problems) = I18n::new(CATALOGS, &["de"]);
    assert!(problems.is_empty(), "{problems:?}");
    i18n.set_format(craft_i18n::Format::from_tag("de"));
    let de = i18n.from_code("de").expect("de");
    let i18n = Arc::new(i18n);
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(600.0, 800.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            Host::builder(AppInfo { id: "test", name: "Test", version: "1.0" }).i18n(i18n.clone()).lang(de).install(ctx);
        })
        .build_ui(|ui| {
            let setup = PageSetup::default();
            let menus = PageLayoutMenus::new(ui.ctx(), &setup, LengthUnit::Inches);
            let (size, margins) = (menus.size(), menus.margins());
            let button = ui.button("Size");
            let _ = PressMenu::new(&size).show(ui, &button);
            let button = ui.button("Margins");
            let _ = PressMenu::new(&margins).show(ui, &button);
        });
    harness.run();
    harness.get_by_label("Size").click();
    harness.run();
    let detail = |harness: &Harness<'_, ()>, name: &str| harness.get_by_label(name).accesskit_node().description();
    assert_eq!(detail(&harness, "Letter").as_deref(), Some("8,5\" \u{d7} 11\""));
    assert_eq!(detail(&harness, "A4 Papier").as_deref(), Some("8,268\" \u{d7} 11,693\""));
    assert!(harness.query_by_label("A4").is_none(), "the English name is gone");
}
