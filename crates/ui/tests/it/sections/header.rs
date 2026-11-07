//! Section headers: the whole row toggles a disclosure, the trailing region takes its own clicks, an inline header
//! shares a line with its body, and an elided title shows whole in a tooltip.

use craft_ui::choice::Checkbox;
use craft_ui::contract::tooltip::DELAY;
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::runtime::overlay::{OpenFilter, any_open};
use craft_ui::sections::{ApplyRow, HeaderAction, Section, TitleRow};
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
    let (mut open, mut pinned) = (true, false);
    let show = |ui: &mut Ui, open: &mut bool, pinned: &mut bool| {
        Section::new("Options")
            .open(Bind::bound(open))
            .controls(|ui, _| {
                Checkbox::new(Bind::bound(pinned)).label("Pin").show(ui);
            })
            .id_salt("section")
            .show_at(ui, AREA, |_, _| {});
    };
    frames.idle(4, |ui| show(ui, &mut open, &mut pinned));
    // The control sits at the end edge, so a point just inside it hits the control.
    let control = Pos2::new(AREA.right() - MID, AREA.top() + MID);
    click(&mut frames, control, &mut |ui| show(ui, &mut open, &mut pinned));
    assert!(pinned, "the click reaches the control");
    assert!(open, "the click does not toggle the section");
}

/// An inline header and its body share one line: the body starts at the top of the row and the row is no taller
/// than its taller part.
#[trial]
fn an_inline_header_and_its_body_share_one_line() {
    let mut frames = Frames::new();
    let (mut apply, row, body) = (true, Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
    let mut show = |ui: &mut Ui| {
        let shown = ApplyRow::new("Opacity", Bind::bound(&mut apply)).id_salt("apply").show_at(ui, AREA, |ui, _| {
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

/// A title that does not fit shows whole in a tooltip; one that fits shows none.
#[trial]
fn an_elided_titles_tooltip_is_the_full_title() {
    const LONG: &str = "A title far too long for the narrow row it sits in";
    let tooltip_after_rest = |title: &'static str, width: f32| {
        let mut frames = Frames::new();
        let area = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(width, 30.0));
        let mut show = |ui: &mut Ui| {
            TitleRow::new(title).id_salt("row").show_at(ui, area);
        };
        frames.idle(3, &mut show);
        let named = |frames: &mut Frames| {
            let tree = frames.tree();
            tree.keys().into_iter().filter(|key| tree.name_of(*key) == Some(title)).count()
        };
        let before = named(&mut frames);
        frames.frame(vec![moved(Pos2::new(area.left() + 20.0, area.center().y))], &mut show);
        frames.advance(DELAY);
        frames.idle(4, &mut show);
        (any_open(&frames.ctx, OpenFilter::Any), named(&mut frames) > before)
    };
    const NARROW: f32 = 120.0;
    const WIDE: f32 = 300.0;
    assert_eq!(tooltip_after_rest(LONG, NARROW), (true, true), "an elided title opens a tooltip that names the full title");
    assert!(!tooltip_after_rest("Options", WIDE).0, "a title that fits opens none");
}
