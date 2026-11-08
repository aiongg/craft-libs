//! A row's line clamp, leading painter, disc icon and the label column beside toggle columns.

use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, ItemSource};
use craft_i18n::Direction;
use craft_ui::contract::{ColumnKind, ColumnSpec, Props as _, Show as _, ShowCollection as _, ToggleColumn};
use craft_ui::list::{List, ListRow, ToggleCell, ToggleState};
use craft_ui::primitives::IconSpec;
use craft_ui::testing::relations::{mirrored, same_height};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Rect, Ui, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const INCLUDE: ColumnKey = ColumnKey::new("include");
const DIALOG: ColumnKey = ColumnKey::new("dialog");
const NAME: ColumnKey = ColumnKey::new("name");
/// Room for both toggle columns, the label and the optional text.
const WIDE: Vec2 = Vec2::new(520.0, 200.0);
/// Room for both toggle columns and the label only.
const NARROW: Vec2 = Vec2::new(150.0, 200.0);

struct One;

impl DisplayOrder<usize> for One {
    fn len(&self) -> usize {
        1
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for One {}

/// A fixture showing one row made by `make`.
fn rig(make: impl Fn() -> ListRow<'static> + 'static) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        List::new("Items").show_items(ui, &One, &mut |_, _| make());
    });
    fixture.settle();
    fixture
}

fn row_rect(fixture: &Fixture) -> Rect {
    fixture.by_label("Row").expect("the row has a node").rect
}

/// A description clamped to 3 lines makes a row as tall as exactly 3 lines do, cuts the text, and keeps all of it
/// for a screen reader.
#[trial]
fn a_clamped_description_is_no_taller_than_its_limit_and_stays_whole_for_a_screen_reader() {
    let exact = rig(|| ListRow::new("Row").lines("one\ntwo\nthree").line_limit(3));
    let long = "one\ntwo\nthree\nfour\nfive";
    let clamped = rig(move || ListRow::new("Row").lines(long).line_limit(3));
    let free = rig(move || ListRow::new("Row").lines(long));
    assert_eq!(row_rect(&clamped).height(), row_rect(&exact).height());
    assert!(row_rect(&free).height() > row_rect(&clamped).height());
    assert!(clamped.texts().iter().any(|text| text.elided && text.text.contains("five")), "the cut text is read through the elided record");
    assert!(!exact.texts().iter().any(|text| text.elided));
    let node = clamped.by_label("Row").unwrap();
    let described = clamped.tree().description_of(node.node).unwrap_or_default();
    assert!(described.contains("five"), "{described:?}");
}

/// The leading painter gets a rect at the reading start of the row, as high as the row, mirrored in rtl.
#[trial]
fn the_leading_painter_sits_at_the_reading_start_and_mirrors() {
    let slot_in = |direction: Direction| {
        let slot = Rc::new(Cell::new(Rect::NOTHING));
        let seen = slot.clone();
        let mut fixture = Fixture::new(move |ui| {
            let seen = seen.clone();
            List::new("Items").show_items(ui, &One, &mut |_, _| {
                let seen = seen.clone();
                ListRow::new("Row").lead_painter(move |_, rect| seen.set(rect))
            });
        });
        fixture.direction(direction);
        fixture.settle();
        (slot.get(), row_rect(&fixture))
    };
    let (ltr, ltr_row) = slot_in(Direction::Ltr);
    let (rtl, rtl_row) = slot_in(Direction::Rtl);
    assert!(ltr.is_positive() && rtl.is_positive(), "the painter ran");
    assert!(ltr.center().x < ltr_row.center().x && rtl.center().x > rtl_row.center().x);
    assert!(same_height(ltr, ltr_row), "the slot is as high as the row");
    assert!(mirrored(ltr.translate(rtl_row.min - ltr_row.min), rtl, rtl_row), "the rtl slot mirrors the ltr one");
}

fn circles(icon: Option<IconSpec<'static>>) -> usize {
    let mut frames = Frames::new();
    let mut show = move |ui: &mut Ui| {
        let icon = icon.clone();
        List::new("Items").show_items(ui, &One, &mut |_, _| {
            let row = ListRow::new("Row");
            match icon.clone() {
                Some(icon) => row.icon(icon),
                None => row,
            }
        });
    };
    frames.idle(3, &mut show);
    frames.frame_shapes(&mut show).iter().filter(|shape| matches!(shape, egui::Shape::Circle(_))).count()
}

/// A disc icon paints a circle shape.
#[trial]
fn a_disc_icon_paints_a_circle() {
    assert_eq!(circles(None), 0);
    assert!(circles(Some(IconSpec::disc())) > 0);
}

fn columns() -> Vec<ColumnSpec> {
    vec![
        ColumnSpec::new(INCLUDE, "", ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
        ColumnSpec::new(DIALOG, "", ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
        ColumnSpec::new(NAME, "", ColumnWidth::fill(1.0)),
    ]
}

/// A row with a cell in one of two toggle columns still lays its label and optional text out in the name column,
/// with no `ListRow::columns` call: the collection tells the row its toggle columns.
#[trial]
fn optional_text_paints_beside_two_toggle_columns_and_drops_first_in_a_narrow_row() {
    let painted = |room: Vec2| {
        let mut fixture = Fixture::new(|ui| {
            List::new("Items").columns(columns()).show_items(ui, &One, &mut |_, item| {
                ListRow::new("Row").cells(item.cells()).toggle(ToggleCell::check(INCLUDE, ToggleState::On, "Include")).optional("2/3 steps")
            });
        });
        fixture.room(room);
        fixture.settle();
        let texts: Vec<String> = fixture.texts().iter().map(|text| text.text.clone()).collect();
        (texts.iter().any(|text| text == "2/3 steps"), texts.iter().any(|text| text == "Row"))
    };
    assert_eq!(painted(WIDE), (true, true), "with room the optional text paints");
    let (optional, label) = painted(NARROW);
    assert!(!optional && label, "in a narrow row the optional text drops and the label stays");
}
