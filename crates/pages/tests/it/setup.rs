//! The page setup dialog, the margin rules, the column bar, the header and footer zones, the page range codec and the
//! frame marks.

use craft_behavior::Codec as _;
use craft_behavior::kurbo::Point;
use craft_pages::range::PageRangeError;
use craft_pages::setup::{Margins, PageSetup};
use craft_pages::ui::{
    ColumnBar, HeaderFooterPage, HiddenMark, MarginEdge, MarginHandles, MarginRules, PageEdges, PageRangeCodec, PageSetupDialog, Zone, ZoneBands,
    clamp_margin, dragged_margin, margin_range,
};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// A column's width at the start of the column tests, and the number of columns.
const EQUAL: f64 = 1.0;
const COLUMNS: usize = 4;

/// A drag far past the end of the bar, points.
const FAR: f32 = 10_000.0;

/// A hundredth of the whole bar, the arrow key's step.
const KEY_STEP: f64 = 0.01;

/// 1.5 inches in points.
const TOP: f64 = 1.5 * 72.0;

/// Letter, in points.
const LETTER: (f64, f64) = (612.0, 792.0);

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn rules() -> MarginRules {
    MarginRules::new(LETTER, PageEdges::new(Margins::uniform(72.0)))
}

#[trial]
fn dragged_margins_snap_and_leave_room() {
    let left = |at: f64, rules: &MarginRules| dragged_margin(MarginEdge::Left, at, rules).points;
    assert_eq!(left(100.0, &rules()), 99.0, "snapped to 4.5");
    assert_eq!(left(600.0, &rules()), 468.0, "leaves the 72 points of text between the margins");
    assert_eq!(left(100.0, &rules().snap(0.0)), 100.0, "Alt drags freely");
    assert_eq!(left(-30.0, &rules()), 0.0, "never negative");
}

#[trial]
fn mirrored_even_pages_swap_the_edges() {
    let swapped = dragged_margin(MarginEdge::Left, 100.0, &rules().mirrored_even(true));
    assert_eq!(swapped.edge, MarginEdge::Right, "the visible left edge edits the stored right margin");
    assert_eq!(swapped.points, 99.0, "the pointer is reflected across the page: 612 - 512 = 100, snapped");
}

#[trial]
fn header_and_footer_distances_bound_the_margins() {
    let edges = PageEdges::new(Margins::uniform(72.0)).header_footer(30.0, 40.0);
    let rules = MarginRules::new(LETTER, edges);
    assert_eq!(*margin_range(MarginEdge::Top, &rules).start(), 30.0, "the top margin stays below the header");
    assert_eq!(*margin_range(MarginEdge::Bottom, &rules).start(), 40.0, "the bottom margin stays above the footer");
    assert_eq!(*margin_range(MarginEdge::Header, &rules).end(), 72.0, "the header stays within the top margin");
    assert_eq!(clamp_margin(MarginEdge::Top, 10.0, &rules).points, 30.0, "kept at the header's distance");
    assert_eq!(clamp_margin(MarginEdge::Left, f64::NAN, &rules).points, 72.0, "a value that is not a number leaves the edge");
}

#[trial]
fn the_ruler_bands_follow_the_visible_edges() {
    let bands = MarginHandles::spans(&rules(), Point::new(10.0, 20.0));
    assert_eq!(bands.horizontal, vec![10.0..82.0, 550.0..622.0]);
    assert_eq!(bands.vertical, vec![20.0..92.0, 740.0..812.0]);
}

/// A page setup dialog over `setup`, with the setup it edits and the setup OK answered.
fn dialog(setup: PageSetup) -> (Fixture, Rc<RefCell<PageSetup>>, Rc<RefCell<Option<PageSetup>>>) {
    let edited = Rc::new(RefCell::new(setup));
    let answered = Rc::new(RefCell::new(None));
    let (seen_edited, seen_answered) = (Rc::clone(&edited), Rc::clone(&answered));
    let mut fixture = Fixture::new(move |ui| {
        let mut draft = seen_edited.borrow().clone();
        let shown = PageSetupDialog::new(Bind::bound(&mut draft)).opened(Bind::given(true)).show(ui);
        *seen_edited.borrow_mut() = draft;
        if shown.accepted.is_some() {
            *seen_answered.borrow_mut() = shown.accepted;
        }
    });
    fixture.settle();
    (fixture, edited, answered)
}

#[trial]
fn ok_answers_the_edited_setup() {
    let (mut fixture, edited, answered) = dialog(PageSetup::default());
    let top = fixture.by_label("Top").expect("the Top field");
    fixture.click(&top);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text("1.5");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(close(edited.borrow().margins.top, TOP), "1.5 inches typed in Top: {:?}", edited.borrow().margins);
    let ok = fixture.by_label("OK").expect("the OK button");
    fixture.click(&ok);
    assert_eq!(answered.borrow().as_ref().map(|setup| setup.margins.top), Some(TOP));
}

#[trial]
fn margins_that_do_not_fit_refuse_ok() {
    let mut setup = PageSetup::default();
    setup.margins.left = 324.0;
    setup.margins.right = 324.0;
    let (mut fixture, _, answered) = dialog(setup);
    let ok = fixture.by_label("OK").expect("the OK button");
    fixture.click(&ok);
    assert_eq!(*answered.borrow(), None, "the margins leave no room for text");
}

#[trial]
fn landscape_turns_the_page() {
    let (mut fixture, edited, _) = dialog(PageSetup::default());
    let landscape = fixture.by_label("Landscape").expect("the Landscape radio");
    fixture.click(&landscape);
    let setup = edited.borrow();
    assert!(setup.width > setup.height, "{}x{}", setup.width, setup.height);
}

#[trial]
fn a_divider_pushes_the_columns_beyond_it() {
    let widths = Rc::new(RefCell::new(vec![EQUAL; COLUMNS]));
    let seen = Rc::clone(&widths);
    let mut fixture = Fixture::new(move |ui| {
        let mut current = seen.borrow().clone();
        ColumnBar::new("Columns", Bind::bound(&mut current)).show(ui);
        *seen.borrow_mut() = current;
    });
    fixture.settle();
    let divider = fixture.by_label("Column 2 width").expect("the second divider");
    fixture.drag(&divider, Vec2::new(FAR, 0.0));
    let widths = widths.borrow();
    assert!(close(widths.iter().sum::<f64>(), EQUAL * COLUMNS as f64), "the total stays: {widths:?}");
    assert!(close(widths[0], EQUAL), "the column before the divider's pair is untouched: {widths:?}");
    assert!(widths[1] > EQUAL && widths[2] < EQUAL && widths[3] < EQUAL, "the columns beyond give way: {widths:?}");
}

#[trial]
fn an_arrow_key_moves_the_focused_divider_one_step() {
    let widths = Rc::new(RefCell::new(vec![EQUAL; COLUMNS]));
    let seen = Rc::clone(&widths);
    let mut fixture = Fixture::new(move |ui| {
        let mut current = seen.borrow().clone();
        ColumnBar::new("Columns", Bind::bound(&mut current)).show(ui);
        *seen.borrow_mut() = current;
    });
    fixture.settle();
    let divider = fixture.by_label("Column 1 width").expect("the first divider");
    fixture.click(&divider);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    let widths = widths.borrow();
    assert!(close(widths[0], EQUAL + KEY_STEP * EQUAL * COLUMNS as f64), "the first column grows by a hundredth of the whole: {widths:?}");
    assert!(close(widths.iter().sum::<f64>(), EQUAL * COLUMNS as f64), "the total stays: {widths:?}");
}

#[trial]
fn the_band_splits_in_thirds_between_the_sides() {
    let bands = ZoneBands::new(72.0, 72.0, [50.4, 50.4]);
    let page = Vec2::new(612.0, 792.0);
    let third = (612.0 - 100.8) / 3.0;
    let first = bands.section(page, Zone::new(false, 0), false);
    assert!((first.x0 - 50.4).abs() < 1e-9 && (first.x1 - (50.4 + third)).abs() < 1e-9, "{first:?}");
    assert_eq!((first.y0, first.y1), (0.0, 72.0), "the header band lies from the top edge to the top margin");
    let footer = bands.section(page, Zone::new(true, 2), false);
    assert_eq!((footer.y0, footer.y1), (720.0, 792.0), "the footer band lies from the bottom margin to the bottom edge");
    assert!((footer.x1 - (612.0 - 50.4)).abs() < 1e-9, "the last section ends at the side inset: {footer:?}");
}

#[trial]
fn the_sections_mirror_right_to_left() {
    let bands = ZoneBands::new(72.0, 72.0, [50.4, 50.4]);
    let page = Vec2::new(612.0, 792.0);
    let start = bands.section(page, Zone::new(false, 0), true);
    let end = bands.section(page, Zone::new(false, 2), false);
    assert_eq!(start, end, "section 0 is the rightmost third");
}

#[trial]
fn pages_use_the_first_odd_and_even_headers() {
    assert_eq!(HeaderFooterPage::of(1, true, false), HeaderFooterPage::First);
    assert_eq!(HeaderFooterPage::of(1, false, true), HeaderFooterPage::Odd);
    assert_eq!(HeaderFooterPage::of(4, true, true), HeaderFooterPage::Even);
}

#[trial]
fn page_range_text_reads_and_writes_the_same_format() {
    let codec = PageRangeCodec::new(10);
    let start = craft_pages::ui::PageSelection::default();
    let read = codec.parse("1-3, 5", &start).expect("a selection");
    assert_eq!(read.pages, vec![0, 1, 2, 4]);
    assert_eq!(read.error, None);
    assert_eq!(codec.format(&read), "1-3, 5");
    let wrong = codec.parse("1-x", &start).expect("a selection that carries its error");
    assert_eq!(wrong.error, Some(PageRangeError::NotAPage("x".to_owned())));
    let beyond = codec.parse("12", &start).expect("a selection that carries its error");
    assert_eq!(beyond.error, Some(PageRangeError::OutOfRange { page: 12, count: 10 }));
}

#[trial]
fn characters_map_to_their_hidden_marks() {
    let marks: Vec<_> = [' ', '\t', '\u{2028}', '\u{A0}'].into_iter().map(|c| HiddenMark::of(c).map(HiddenMark::glyph)).collect();
    assert_eq!(marks, [Some("·"), Some("»"), Some("¬"), Some("°")]);
    assert_eq!(HiddenMark::of('a'), None);
}
