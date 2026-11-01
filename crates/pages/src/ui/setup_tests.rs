//! Page Setup and the Layout menus.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::localized::units::LengthUnit;
use craft_ui::menu::MenuEntry;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, vec2};
use egui_kittest::kittest::Queryable as _;
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
    type_into(&mut harness, "Top:", "1.5", Key::Enter);
    let answers = &harness.state().answers;
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].margins.top, 108.0);
    assert_eq!(answers[0].margins.bottom, 72.0, "an untouched margin keeps its value");
}

// Left and right of 4.5 in on Letter (4 in each fit exactly, as in Word) keep the dialog open with the message under the fields.
#[test]
fn margins_that_dont_fit_refuse_ok() {
    let mut harness = open(PageSetupDialog::new(PageSetup::default()).unit(LengthUnit::Inches));
    type_into(&mut harness, "Left:", "4.5", Key::Tab);
    type_into(&mut harness, "Right:", "4.5", Key::Enter);
    assert!(harness.state().answers.is_empty());
    assert!(harness.state().host.get::<PageSetupDialog>().is_some(), "the dialog stays open");
    assert!(harness.query_all_by_label(&PageSetupLabels::default().margins_dont_fit).next().is_some(), "the message shows");
    type_into(&mut harness, "Right:", "1", Key::Enter);
    assert_eq!(harness.state().answers.len(), 1);
}

// Picking A4 sets 595.28 × 841.89; Landscape swaps them; typing a width larger than the height selects Landscape and Custom size stays chosen while the numbers match no name.
#[test]
fn paper_list_and_orientation_follow_each_other() {
    let mut harness = open(PageSetupDialog::new(PageSetup::default()).unit(LengthUnit::Inches));
    harness.get_by_role_and_label(Role::ComboBox, "Paper size:").click();
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
    type_into(&mut harness, "Width:", "12", Key::Tab);
    let setup = edited(&harness);
    assert_eq!((setup.width, setup.orientation), (864.0, Orientation::Landscape));
    assert_eq!(harness.get_by_role_and_label(Role::ComboBox, "Paper size:").value().as_deref(), Some("Custom size"));
}

// Over Moderate margins the Moderate item is checked; Normal reads "Normal  1" all" in inches; Custom Margins… returns PageSetup.
#[test]
fn margins_menu_checks_the_matching_preset() {
    let setup = PageSetup { margins: Margins { top: 72.0, bottom: 72.0, left: 54.0, right: 54.0 }, ..PageSetup::default() };
    let entries = PageLayoutMenus::new(&setup, LengthUnit::Inches).margins();
    let items: Vec<_> = entries.iter().filter_map(MenuEntry::item).collect();
    let checked: Vec<_> = items.iter().filter(|item| item.item_kind().checked() == Some(true)).map(|item| item.label()).collect();
    assert_eq!(checked, ["Moderate"]);
    assert!(items.iter().any(|item| item.label() == "Normal  1\" all"));
    assert!(items.iter().any(|item| item.label() == "Narrow  0.5\" all"));
    let custom = items.last().expect("the last item");
    assert_eq!((custom.label(), custom.command()), ("Custom Margins…", Some(&PageLayoutChoice::PageSetup)));
    let metric = PageLayoutMenus::new(&setup, LengthUnit::Centimetres).margins();
    assert!(metric.iter().filter_map(MenuEntry::item).any(|item| item.label() == "Normal  2.54 cm all"));
}
