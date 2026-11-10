//! `Menu` and `MenuItem`: the mnemonic of each row of a level, what running a row does to the menu, the one open
//! submenu, value menus and the panel rows.

mod bar;

mod native;

mod openers;

mod pick_focus;

use craft_behavior::IconId;
use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Entry, Role, Show as _, ShowContainer as _, Trigger};
use craft_ui::layout_components::Stack;
use craft_ui::menu::{GroupItem, Menu, MenuItem, PanelMenu, PanelRow};
use craft_ui::primitives::IconSpec;
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Pos2};
use std::cell::Cell;
use std::rc::Rc;

const ROWS: [&str; 3] = ["&Open", "Save", "Save As"];

fn rows(ui: &mut egui::Ui, ran: &Cell<Option<&'static str>>) {
    for label in ROWS {
        if MenuItem::new(label).show(ui).ran {
            ran.set(Some(label));
        }
    }
}

fn menu_shown(fixture: &Fixture) -> bool {
    !fixture.by_role(Role::MENU).is_empty()
}

/// A menu opened by a press of its opener, with no binding and no row focused: the container reads the typed letter.
#[trial]
fn a_typed_letter_runs_a_row_of_a_menu_opened_by_the_pointer() {
    let ran = Rc::new(Cell::new(None));
    let shown_ran = ran.clone();
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("File").show(ui);
        Menu::new("File").opened_by(&opener, Trigger::Press).show(ui, |ui, _| rows(ui, &shown_ran));
    });
    fixture.settle();
    let opener = fixture.by_label("File").expect("the opener has a node");
    fixture.click(&opener);
    fixture.idle(4);
    assert!(menu_shown(&fixture), "a press of the opener opens the menu");
    fixture.type_text("a");
    fixture.idle(4);
    assert_eq!(ran.get(), Some("Save As"));
    assert!(!menu_shown(&fixture), "running a row closes the menu");
}

/// The names of the menus on screen, sorted.
fn menus(fixture: &Fixture) -> Vec<String> {
    let tree = fixture.tree();
    let mut names: Vec<String> = fixture.by_role(Role::MENU).iter().filter_map(|menu| tree.name_of(menu.node)).map(str::to_owned).collect();
    names.sort();
    names
}

/// The pointer rests on a row for longer than the submenu delay.
fn rest_on(fixture: &mut Fixture, row: &str) {
    let at: Pos2 =
        fixture.by_role(Role::MENU_ITEM).iter().find(|target| fixture.tree().name_of(target.node) == Some(row)).expect("the row shows").center();
    fixture.frame(vec![Event::PointerMoved(at)]);
    fixture.advance(0.5);
    fixture.idle(6);
}

#[trial]
fn resting_on_a_second_submenu_row_closes_the_first_submenu() {
    let submenu = |label: &str, inner: &str| {
        PanelRow::Submenu(label.to_owned().into(), vec![PanelRow::Item(Entry::new(GroupItem::MoveTo(0), inner.to_owned()))])
    };
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Panel").show(ui);
        Menu::new("Panel").opened_by(&opener, Trigger::Press).show(ui, |ui, _| {
            PanelMenu::new(vec![submenu("Alpha", "Alpha One"), submenu("Beta", "Beta One")], Vec::new()).show(ui);
        });
    });
    fixture.settle();
    let opener = fixture.by_label("Panel").expect("the opener has a node");
    fixture.click(&opener);
    fixture.idle(4);
    rest_on(&mut fixture, "Alpha");
    assert_eq!(menus(&fixture), ["Alpha", "Panel"], "the submenu of the row the pointer rests on is open");
    rest_on(&mut fixture, "Beta");
    assert_eq!(menus(&fixture), ["Beta", "Panel"], "only the second submenu is open");
}

fn zoom(checked: Option<u8>) -> Vec<Entry<u8>> {
    let entry = |value: u8, label: &str| Entry::new(value, label.to_owned()).checked(checked == Some(value));
    vec![entry(0, "Fit in Window"), entry(1, "Fill Screen"), Entry::separator(), entry(2, "100%")]
}

fn value_menu(checked: Option<u8>) -> Fixture {
    let entries = zoom(checked);
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Zoom").show(ui);
        let mut open = true;
        Menu::new("Zoom").values().open(Bind::bound(&mut open)).opened_by(&opener, Trigger::Click).pick(ui, &entries);
    });
    fixture.settle();
    fixture
}

#[trial]
fn a_value_menu_opens_with_the_lead_on_the_checked_entry() {
    let fixture = value_menu(Some(2));
    let focused = fixture.focused().and_then(|id| fixture.node_of(id)).expect("a row holds the focus");
    let checked = fixture.by_label("100%").expect("the checked entry shows");
    assert_eq!(focused.rect, checked.rect);
}

/// Regression: a level reserves the mark column for every row once any row has a check, so the labels share one left
/// edge, in the first frame the menu opens (not only after a later pass has seen the checked row).
#[trial]
fn plain_rows_align_with_a_checked_row_in_the_first_frame_of_a_menu() {
    let mut fixture = Fixture::new(|ui| {
        let opener = Button::new("View").show(ui);
        let mut open = true;
        Menu::new("View").open(Bind::bound(&mut open)).opened_by(&opener, Trigger::Click).show(ui, |ui, _| {
            MenuItem::new("New").show(ui);
            MenuItem::new("Open").show(ui);
            MenuItem::new("Show Rulers").check(true).show(ui);
        });
    });
    let x = |fixture: &Fixture, label: &str| fixture.texts().iter().find(|text| text.text.replace('&', "") == label).map(|text| text.rect.left());
    let mut frames = 0;
    while x(&fixture, "Show Rulers").is_none() && frames < 8 {
        fixture.frame(vec![]);
        frames += 1;
    }
    let (new, rulers) = (x(&fixture, "New").expect("New is shown"), x(&fixture, "Show Rulers").expect("Show Rulers is shown"));
    assert_eq!(new, rulers, "the labels share one left edge in the first frame the menu shows");
}

/// Regression: an inline menu inside a content-sized container shows by the record its container's measuring run
/// left, so its own body runs only for real; the rows before the first iconed or checked row still take the columns,
/// in every frame (a tab strip's overflow menu checks only the shown tab).
#[trial]
fn rows_of_an_inline_menu_shown_from_its_container_record_take_the_columns_of_later_rows() {
    let mut fixture = Fixture::new(|ui| {
        Stack::new().show(ui, |ui, _| {
            Menu::inline("Tabs").show(ui, |ui, _| {
                MenuItem::new("Layers").show(ui);
                MenuItem::new("History").icon(IconSpec::svg(IconId::new("history"))).show(ui);
                MenuItem::new("Swatches").check(true).show(ui);
            });
        });
    });
    let at = |fixture: &Fixture, label: &str| fixture.texts().iter().find(|text| text.text.replace('&', "") == label).map(|text| text.rect.left());
    let mut frames = 0;
    while at(&fixture, "Swatches").is_none() && frames < 8 {
        fixture.frame(vec![]);
        frames += 1;
    }
    let x = |fixture: &Fixture, label: &str| at(fixture, label).expect("the row is shown");
    for frame in 0..3 {
        assert_eq!(x(&fixture, "Layers"), x(&fixture, "Swatches"), "frame {frame}: a plain row takes the mark and icon columns");
        assert_eq!(x(&fixture, "History"), x(&fixture, "Swatches"), "frame {frame}: an iconed row takes the mark column");
        fixture.frame(vec![]);
    }
}

/// Whether the latest frame painted a fill over the row labelled `label` (the active row's fill).
fn row_filled(fixture: &Fixture, label: &str) -> bool {
    let row = fixture.by_label(label).expect("the row shows").rect;
    // The popup's own surface contains every row; a row's fill is no taller than the row plus its rounding.
    fixture.shape_bounds().iter().any(|shape| shape.contains_rect(row) && shape.height() < row.height() * 1.5)
}

/// A menu opened by the pointer paints no active row; the first arrow key makes the row it reaches active.
#[trial]
fn a_menu_opened_by_the_pointer_shows_no_active_row_until_a_key_moves_it() {
    let mut fixture = Fixture::new(|ui| {
        let opener = Button::new("File").show(ui);
        Menu::new("File").opened_by(&opener, Trigger::Press).show(ui, |ui, _| {
            for label in ["Open", "Save", "Close"] {
                MenuItem::new(label).show(ui);
            }
        });
    });
    fixture.settle();
    let opener = fixture.by_label("File").expect("the opener has a node");
    fixture.click(&opener);
    fixture.idle(4);
    assert!(menu_shown(&fixture), "a press of the opener opens the menu");
    let resting = ["Open", "Save", "Close"].map(|label| row_filled(&fixture, label));
    assert_eq!(resting, [false; 3], "no row is active at rest");
    fixture.key(egui::Key::ArrowDown, egui::Modifiers::NONE);
    fixture.idle(4);
    let after = ["Open", "Save", "Close"].map(|label| row_filled(&fixture, label));
    assert_eq!(after.iter().filter(|&&on| on).count(), 1, "the arrow key makes one row active: {after:?}");
}
