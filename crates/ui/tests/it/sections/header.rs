//! Section headers: the whole row toggles a disclosure, the trailing region takes its own clicks, an inline header
//! shares a line with its body, and an elided title shows whole in a tooltip.

use craft_ui::choice::{Checkbox, Segmented};
use craft_ui::contract::tooltip::DELAY;
use craft_ui::contract::{Bind, Entry, Props as _, Show as _, ShowContainer as _};
use craft_ui::runtime::overlay::{OpenFilter, any_open};
use craft_ui::sections::{GroupBox, HeaderAction, Section, TitleRow};
use craft_ui::testing::pointer::{click, moved};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui};
use std::cell::Cell;

const AREA: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 110.0) };
/// Half the height of a section header, where its parts are centred.
const MID: f32 = 14.0;

/// A click on the title text of a section header toggles it; a click on its menu action only reports the action.
#[trial]
fn clicking_a_sections_title_toggles_it_and_a_click_on_its_menu_does_not() {
    let mut frames = Frames::new();
    let (mut open, action) = (true, Cell::new(None));
    let show = |ui: &mut Ui, open: &mut bool| {
        let shown = Section::new("Options").open(Bind::bound(open)).actions([HeaderAction::Menu]).id_salt("section").show_at(ui, AREA, |_, _| {});
        action.set(action.get().or(shown.output.action));
    };
    frames.idle(3, |ui| show(ui, &mut open));
    let title = Pos2::new(AREA.left() + 3.0 * MID, AREA.top() + MID);
    click(&mut frames, title, &mut |ui| show(ui, &mut open));
    assert!(!open, "a click on the title text closes the section");
    click(&mut frames, title, &mut |ui| show(ui, &mut open));
    assert!(open, "a second click opens it again");
    let menu = Pos2::new(AREA.right() - MID, AREA.top() + MID);
    click(&mut frames, menu, &mut |ui| show(ui, &mut open));
    assert!(open, "a click on the menu action leaves the section as it is");
    assert_eq!(action.get(), Some(HeaderAction::Menu), "the menu action is reported");
}

/// A control in the trailing region takes its own clicks: the section keeps its state.
#[trial]
fn a_control_in_the_trailing_region_takes_its_clicks_without_toggling() {
    let mut frames = Frames::new();
    let (mut open, mut view) = (true, 1u8);
    let entries = [Entry::new(1u8, "List"), Entry::new(2, "Grid")];
    let show = |ui: &mut Ui, open: &mut bool, view: &mut u8| {
        Section::new("Options")
            .open(Bind::bound(open))
            .controls(|ui, _| {
                Segmented::new("View", Bind::bound(view), &entries).show(ui);
            })
            .id_salt("section")
            .show_at(ui, AREA, |_, _| {});
    };
    frames.idle(4, |ui| show(ui, &mut open, &mut view));
    let tree = frames.tree();
    let grid = tree
        .keys()
        .into_iter()
        .find(|key| tree.name_of(*key) == Some("Grid"))
        .and_then(|key| tree.bounds_of(key))
        .expect("the Grid segment is a node");
    click(&mut frames, grid.center(), &mut |ui| show(ui, &mut open, &mut view));
    assert_eq!(view, 2, "the click reaches the control");
    assert!(open, "the click does not toggle the section");
}

/// An inline header and its body share one line: the body starts at the top of the row and the row is no taller
/// than its taller part.
#[trial]
fn an_inline_header_and_its_body_share_one_line() {
    let mut frames = Frames::new();
    let (mut apply, row, body) = (true, Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
    let mut show = |ui: &mut Ui| {
        let shown = GroupBox::apply("Opacity", Bind::bound(&mut apply)).id_salt("apply").show(ui, |ui, _| {
            body.set(Checkbox::new(Bind::given(false)).label("Body").show(ui).response.rect);
        });
        row.set(shown.output.response.rect);
    };
    frames.idle(5, &mut show);
    let (row, body) = (row.get(), body.get());
    let (half, line) = (body.height() / 2.0, body.height() * 1.5);
    assert!(body.top() - row.top() < half, "the body starts on the header's line, not below it");
    assert!(row.height() < line, "the row is one line high");
    assert!(body.left() > row.left(), "the body sits after the header");
}

