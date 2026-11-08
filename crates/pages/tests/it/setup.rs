//! The page setup dialog, the margin rules, the column bar, the header and footer zones, the page range codec and the
//! frame marks.

use craft_behavior::Codec as _;
use craft_behavior::kurbo::Point;
use craft_pages::range::{PageRange, PageRangeError, Subset};
use craft_pages::setup::{Margins, PageSetup};
use craft_pages::ui::fixtures::{INK, PAPER, demo_placed};
use craft_pages::ui::{
    ColumnBar, HeaderFooterPage, HeaderFooterZones, HiddenMark, MarginEdge, MarginHandles, MarginRules, PageEdges, PageRangeCodec, PageRangeOptions,
    PageSetupDialog, Zone, ZoneAt, ZoneBands, clamp_margin, dragged_margin, margin_range,
};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Vec2, pos2, vec2};
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

/// The share a screen reader sets the first divider to, and the margin it sets the left edge to, points.
const SHARE: f64 = 0.4;
const SET_MARGIN: f64 = 100.0;

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

/// A column bar over four equal columns, and the widths it keeps.
fn column_rig() -> (Fixture, Rc<RefCell<Vec<f64>>>) {
    let widths = Rc::new(RefCell::new(vec![EQUAL; COLUMNS]));
    let seen = Rc::clone(&widths);
    let mut fixture = Fixture::new(move |ui| {
        let mut current = seen.borrow().clone();
        ColumnBar::new("Columns", Bind::bound(&mut current)).show(ui);
        *seen.borrow_mut() = current;
    });
    fixture.settle();
    (fixture, widths)
}

#[trial]
fn a_screen_reader_sets_a_divider_by_percent() {
    let (mut fixture, widths) = column_rig();
    let divider = fixture.by_label("Column 1 width").expect("the first divider");
    fixture.access(AccessActionKind::SetValue(SHARE * 100.0), &divider);
    let widths = widths.borrow();
    let total: f64 = widths.iter().sum();
    assert!(close(widths[0] / total, SHARE), "the first column holds 40 percent: {widths:?}");
    assert!(close(total, EQUAL * COLUMNS as f64), "the total stays: {widths:?}");
}

#[trial]
fn a_screen_reader_steps_a_divider() {
    let (mut fixture, widths) = column_rig();
    let divider = fixture.by_label("Column 1 width").expect("the first divider");
    fixture.access(AccessActionKind::Increment, &divider);
    assert!(widths.borrow()[0] > EQUAL, "Increment widens the column before the divider: {:?}", widths.borrow());
    fixture.access(AccessActionKind::Decrement, &divider);
    fixture.access(AccessActionKind::Decrement, &divider);
    assert!(widths.borrow()[0] < EQUAL, "Decrement narrows it: {:?}", widths.borrow());
}

/// The margin handles of one Letter page at the start of a room, and the edges they edit.
fn margin_rig() -> (Fixture, Rc<RefCell<PageEdges>>) {
    let edges = Rc::new(RefCell::new(PageEdges::new(Margins::uniform(72.0))));
    let seen = Rc::clone(&edges);
    let mut fixture = Fixture::new(move |ui| {
        let page = demo_placed(1, pos2(40.0, 40.0)).remove(0);
        let mut current = *seen.borrow();
        let room = ui.available_rect_before_wrap();
        MarginHandles::new("Margins", &page, Bind::bound(&mut current)).show_at(ui, room);
        *seen.borrow_mut() = current;
    });
    fixture.room(vec2(800.0, 900.0));
    fixture.settle();
    (fixture, edges)
}

#[trial]
fn a_screen_reader_sets_a_margin_inside_its_range() {
    let (mut fixture, edges) = margin_rig();
    let left = fixture.by_label("Left Margin").expect("the left edge");
    fixture.access(AccessActionKind::SetValue(SET_MARGIN), &left);
    assert!(close(edges.borrow().margins.left, SET_MARGIN), "{:?}", edges.borrow());
    fixture.access(AccessActionKind::SetValue(f64::MAX), &left);
    let furthest = *margin_range(MarginEdge::Left, &MarginRules::new(LETTER, *edges.borrow())).end();
    assert_eq!(edges.borrow().margins.left, furthest, "the text keeps its least width: {:?}", edges.borrow());
    assert!(furthest < LETTER.0, "the range ends inside the page");
}

#[trial]
fn a_screen_reader_steps_a_margin() {
    let (mut fixture, edges) = margin_rig();
    let left = fixture.by_label("Left Margin").expect("the left edge");
    let before = edges.borrow().margins.left;
    fixture.access(AccessActionKind::Increment, &left);
    assert!(edges.borrow().margins.left > before, "Increment moves the edge toward the text: {:?}", edges.borrow());
}

/// A page range over twelve pages, and the range the options last reported.
fn range_rig(start: PageRange) -> (Fixture, Rc<RefCell<PageRange>>) {
    let range = Rc::new(RefCell::new(start));
    let seen = Rc::clone(&range);
    let mut fixture = Fixture::new(move |ui| {
        let current = *seen.borrow();
        if let Some(next) = PageRangeOptions::new("Page range", current, 12).show(ui).changed {
            *seen.borrow_mut() = next;
        }
    });
    fixture.settle();
    (fixture, range)
}

/// Types `text` into the number field `label`, and leaves it.
fn type_number(fixture: &mut Fixture, nth: usize, text: &str) {
    let fields = fixture.by_role(craft_ui::contract::Role::SPIN_BUTTON);
    fixture.click(&fields[nth]);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(text);
    fixture.key(Key::Enter, Modifiers::NONE);
}

#[trial]
fn from_and_to_stay_inside_the_document_and_in_order() {
    let start = PageRange { all: false, from: 3, to: Some(9), subset: Subset::All };
    let (mut fixture, range) = range_rig(start);
    type_number(&mut fixture, 1, "99");
    assert_eq!(range.borrow().to, Some(12), "a page past the end is the last page: {:?}", range.borrow());
    let (mut fixture, range) = range_rig(start);
    type_number(&mut fixture, 0, "11");
    assert_eq!((range.borrow().from, range.borrow().to), (11, Some(11)), "the end follows a start past it");
    let (mut fixture, range) = range_rig(start);
    type_number(&mut fixture, 1, "2");
    assert_eq!((range.borrow().from, range.borrow().to), (2, Some(2)), "the start follows an end before it");
}

/// The header and footer zones of one page, the zone being edited, and what the zones last reported.
fn zones_rig() -> (Fixture, Rc<RefCell<(Option<ZoneAt>, bool)>>) {
    let reported = Rc::new(RefCell::new((None, false)));
    let seen = Rc::clone(&reported);
    let mut fixture = Fixture::new(move |ui| {
        let placed = demo_placed(1, pos2(20.0, 20.0));
        let bands = |_: usize| ZoneBands::new(72.0, 72.0, [50.0, 50.0]);
        let room = ui.available_rect_before_wrap();
        let out = HeaderFooterZones::new("Zones", &placed, &bands, PAPER, INK).editing(ZoneAt::new(0, Zone::new(true, 2))).show_at(ui, room);
        let mut seen = seen.borrow_mut();
        seen.0 = out.begin.or(seen.0);
        seen.1 |= out.end;
    });
    fixture.room(vec2(700.0, 900.0));
    fixture.settle();
    (fixture, reported)
}

#[trial]
fn a_click_on_a_section_starts_editing_it_and_escape_ends_the_editing() {
    let (mut fixture, reported) = zones_rig();
    let first_header: Pos2 = pos2(20.0 + 50.0 + 20.0, 20.0 + 36.0);
    fixture.click_at(first_header);
    assert_eq!(reported.borrow().0, Some(ZoneAt::new(0, Zone::new(false, 0))), "{:?}", reported.borrow());
    assert!(!reported.borrow().1, "no end yet");
    fixture.key(Key::Escape, Modifiers::NONE);
    assert!(reported.borrow().1, "Escape on a focused section ends the editing that was under way");
}

#[trial]
fn text_the_field_rejects_refuses_ok() {
    let (mut fixture, _, answered) = dialog(PageSetup::default());
    let top = fixture.by_label("Top").expect("the Top field");
    fixture.click(&top);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text("tall");
    fixture.key(Key::Tab, Modifiers::NONE);
    let ok = fixture.by_label("OK").expect("the OK button");
    fixture.click(&ok);
    assert_eq!(*answered.borrow(), None, "a Top that is not a length keeps the dialog open");
}

#[trial]
fn custom_size_stays_chosen_while_the_numbers_match_a_paper() {
    let (mut fixture, _, _) = dialog(PageSetup::default());
    let paper = fixture.by_label("Paper size").expect("the paper list");
    fixture.click(&paper);
    let custom = fixture.by_label("Custom size").expect("the Custom size entry");
    fixture.click(&custom);
    fixture.idle(2);
    let paper = fixture.by_label("Paper size").expect("the paper list");
    assert_eq!(fixture.tree().value_of(paper.node), Some("Custom size"), "the size is still Letter's");
}
