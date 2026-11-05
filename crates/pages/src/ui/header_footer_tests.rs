//! Header and footer zones.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::direction::Direction;
use craft_ui::theme::{Base, install};
use egui::{Color32, Event, Modifiers, PointerButton, Rect, pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::header_footer::*;
use super::style::HeaderFooterStyle;
use super::view::PlacedPage;

const INSET: f32 = 3.0;

fn style() -> HeaderFooterStyle {
    let stroke = egui::Stroke::new(1.0, Color32::GRAY);
    HeaderFooterStyle { inset: INSET, hint: stroke, editing: stroke, neighbour: stroke, paper: Color32::WHITE, ink: Color32::BLACK }
}

const PAGE: Rect = Rect { min: pos2(50.0, 50.0), max: pos2(662.0, 842.0) };

fn placed() -> PlacedPage {
    PlacedPage { page: 0, rect: PAGE, scale: 1.0, band: (0.0, PAGE.height()) }
}

fn bands(_page: usize) -> ZoneBands {
    ZoneBands { top: 54.0, bottom: 54.0, sides: [50.4, 50.4] }
}

// The header band of a Letter page at 100 % with 0.7 in sides gives three equal rects inset by the style's 3 pt, top and bottom.
#[test]
fn sections_split_the_band_in_thirds() {
    let pages = [placed()];
    let zones = HeaderFooterZones::new("hf", &pages, &bands, style());
    let rects: Vec<Rect> =
        (0..3).map(|section| zones.rect(&Direction::Ltr, ZoneAt { page: 0, zone: Zone { footer: false, section } }).expect("placed")).collect();
    let third = (612.0 - 100.8) / 3.0;
    for (k, rect) in rects.iter().enumerate() {
        assert!((rect.width() - third).abs() < 0.01 && (rect.min.x - (50.0 + 50.4 + third * k as f32)).abs() < 0.01, "{rect:?}");
        assert_eq!((rect.min.y, rect.max.y), (50.0 + INSET, 104.0 - INSET));
    }
    let footer = zones.rect(&Direction::Ltr, ZoneAt { page: 0, zone: Zone { footer: true, section: 2 } }).expect("placed");
    assert_eq!((footer.min.y, footer.max.y), (788.0 + INSET, 842.0 - INSET));
    assert_eq!(zones.rect(&Direction::Ltr, ZoneAt { page: 3, zone: Zone { footer: false, section: 0 } }), None);
}

#[derive(Default)]
struct App {
    begun: Vec<ZoneAt>,
    ended: usize,
    editing: Option<ZoneAt>,
}

fn open(editing: Option<ZoneAt>) -> Harness<'static, App> {
    let mut harness =
        HarnessBuilder::default().with_size(vec2(800.0, 900.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let pages = [placed()];
                let response = HeaderFooterZones::new("hf", &pages, &bands, style()).editing(app.editing).show(ui);
                app.begun.extend(response.begin);
                app.ended += usize::from(response.end);
            },
            App { editing, ..App::default() },
        );
    harness.run();
    harness
}

// Hovering an empty footer section shows "Add footer"; a click returns begin with that page and section; a click on the page body while editing returns end.
#[test]
fn hover_hints_and_click_begins() {
    let mut harness = open(None);
    assert_eq!(harness.get_all_by_label("Add header").count(), 3);
    let footers: Vec<_> = harness.get_all_by_label("Add footer").collect();
    assert_eq!(footers.len(), 3);
    footers[1].click();
    harness.run();
    assert_eq!(harness.state().begun, [ZoneAt { page: 0, zone: Zone { footer: true, section: 1 } }]);

    let mut harness = open(Some(ZoneAt { page: 0, zone: Zone { footer: false, section: 1 } }));
    let body = pos2(300.0, 400.0);
    harness.event(Event::PointerMoved(body));
    harness.event(Event::PointerButton { pos: body, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.event(Event::PointerButton { pos: body, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();
    assert_eq!(harness.state().ended, 1);
    assert!(harness.state().begun.is_empty());
}

// Page 1 is First with Different First Page, else Odd; page 4 is Even with Different Odd & Even.
#[test]
fn page_kinds_follow_first_and_odd_even() {
    assert_eq!(HeaderFooterPage::of(1, true, true), HeaderFooterPage::First);
    assert_eq!(HeaderFooterPage::of(1, false, true), HeaderFooterPage::Odd);
    assert_eq!(HeaderFooterPage::of(4, true, true), HeaderFooterPage::Even);
    assert_eq!(HeaderFooterPage::of(4, true, false), HeaderFooterPage::Odd);
    assert_eq!(HeaderFooterPage::of(3, true, true), HeaderFooterPage::Odd);
}

// In a right-to-left interface section 0 is the rightmost third.
#[test]
fn sections_mirror_in_right_to_left() {
    let pages = [placed()];
    let zones = HeaderFooterZones::new("hf", &pages, &bands, style());
    let x = |direction: Direction, section| zones.rect(&direction, ZoneAt { page: 0, zone: Zone { footer: false, section } }).expect("placed").min.x;
    assert!(x(Direction::Ltr, 0) < x(Direction::Ltr, 2));
    assert_eq!(x(Direction::Rtl, 0), x(Direction::Ltr, 2));
    assert_eq!(x(Direction::Rtl, 1), x(Direction::Ltr, 1));
    assert_eq!(x(Direction::Rtl, 2), x(Direction::Ltr, 0));
}
