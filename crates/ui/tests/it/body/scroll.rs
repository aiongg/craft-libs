//! A scrolling body: the wheel and the thumb move it in the frame that carries them, and its content size and
//! scroll offset follow the content.

use super::kit::{Regions, Slab};
use craft_behavior::ColumnWidth;
use craft_ui::contract::{ContainerProps, FooterRegion, HeaderRegion, Props as _, Scroll, Show as _, ShowContainer as _, Sizing};
use craft_ui::layout_components::{Grid, Stack};
use craft_ui::testing::layout::{Block, in_region};
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const LINE: Vec2 = Vec2::new(100.0, 20.0);
/// Positions that should coincide, and a looser bound for positions that follow a drag.
const SAME: f32 = 0.5;
const NEAR: f32 = 1.0;
/// Wheel distances, and the least a thumb drag of 60 points moves a body of 40 lines beyond them.
const STEP: f32 = 60.0;
const BIG_STEP: f32 = 200.0;
const HUNDRED: f32 = 100.0;
const THUMB_GAIN: f32 = 150.0;
/// What the gaps between lines add to a body's height, at most.
const GAPS: f32 = 20.0;
const FIRST_COLUMN: f32 = 100.0;
const WIDEST_CHILD: f32 = 60.0;
const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);

/// What a frame of `scrolling_body` saw.
#[derive(Default)]
struct Seen {
    first_top: Cell<f32>,
    /// The height of the content, gaps included.
    content: Cell<f32>,
    body: Cell<Option<Rect>>,
}

/// A scrolling panel of `lines` lines at `ORIGIN`.
fn scrolling_body(ui: &mut Ui, lines: usize, seen: &Seen) {
    let region = Rect::from_min_size(ORIGIN, VIEW);
    in_region(ui, region, |ui| {
        let props = ContainerProps::new().scroll(Scroll::vertical());
        let out = Slab::new(props).id_salt("panel").show(ui, |ui, cx| {
            seen.body.set(Some(cx.rect));
            for line in 0..lines {
                if line == 0 {
                    seen.first_top.set(ui.next_widget_position().y);
                }
                Block::new(LINE).show(ui);
            }
            seen.content.set(ui.min_rect().height());
        });
        seen.body.set(seen.body.get().or(Some(out.output.rect)));
    });
}

fn wheel_at(frames: &mut Frames, at: Pos2, dy: f32, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![pointer::moved(at)], &mut *show);
    frames.frame(vec![pointer::wheel(dy)], &mut *show);
}

/// Skeleton: `frames_pointer::panel_body_and_list_share_the_wheel_rule`, the body half. The list half needs the
/// item step (F6).
#[trial]
fn panel_body_and_list_share_the_wheel_rule() {
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| scrolling_body(ui, 40, &seen);
    frames.idle(3, &mut show);
    let (top0, centre) = (seen.first_top.get(), seen.body.get().expect("the body ran").center());

    wheel_at(&mut frames, centre, -STEP, &mut show);
    assert!((top0 - seen.first_top.get() - STEP).abs() < SAME, "the body moved by the wheel's 60 points: {top0} -> {}", seen.first_top.get());

    for _ in 0..3 {
        wheel_at(&mut frames, centre, -100_000.0, &mut show);
    }
    let moved = top0 - seen.first_top.get();
    let end = seen.content.get() - seen.body.get().expect("the body ran").height();
    assert!(end > STEP && (moved - end).abs() < NEAR, "the body scrolled to its end, not past it: {moved} of {end}");
}

/// Skeleton: `frames_pointer::wheel_and_thumb_scroll_show_new_rows_in_the_same_frame`, the body half. The list
/// half needs the item step (F6).
#[trial]
fn wheel_and_thumb_scroll_show_new_rows_in_the_same_frame() {
    let seen = Seen::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| scrolling_body(ui, 40, &seen);
    frames.idle(3, &mut show);
    let view = seen.body.get().expect("the body ran");
    let top0 = seen.first_top.get();

    wheel_at(&mut frames, view.center(), -BIG_STEP, &mut show);
    let after_wheel = top0 - seen.first_top.get();
    assert!((after_wheel - BIG_STEP).abs() < SAME, "the frame that carries the wheel shows the lines it scrolled to: {after_wheel}");

    // The thumb of a 200-point view over 800 points of content sits a third of the way down the track.
    let track_x = view.max.x - 2.0;
    let thumb = Pos2::new(track_x, view.min.y + view.height() * 0.375);
    pointer::drag(&mut frames, thumb, Vec2::new(0.0, 60.0), &mut show);
    let after_thumb = top0 - seen.first_top.get();
    assert!(after_thumb > after_wheel + THUMB_GAIN, "a thumb drag of 60 points moved the content: {after_wheel} -> {after_thumb}");
}

/// What `body_with_lines` reported.
fn body_with_lines(frames: &mut Frames, lines: usize, scroll: bool) -> (Rect, f32) {
    let (rect, top) = (Cell::new(Rect::NOTHING), Cell::new(0.0));
    frames.idle(3, |ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let props = if scroll { ContainerProps::new().scroll(Scroll::vertical()) } else { ContainerProps::new().sizing(Sizing::Content) };
            let out = Slab::new(props).id_salt("panel").show(ui, |ui, _| {
                for line in 0..lines {
                    if line == 0 {
                        top.set(ui.next_widget_position().y);
                    }
                    Block::new(LINE).show(ui);
                }
            });
            rect.set(out.output.rect);
        });
    });
    (rect.get(), top.get())
}

#[trial]
fn a_content_sized_body_grows_and_shrinks_with_its_content() {
    let mut frames = Frames::new();
    let (three, _) = body_with_lines(&mut frames, 3, false);
    let (six, _) = body_with_lines(&mut frames, 6, false);
    let (three_again, _) = body_with_lines(&mut frames, 3, false);
    assert!(six.height() > three.height() + LINE.y + LINE.y, "six lines are taller than three: {three:?} {six:?}");
    assert!((three_again.height() - three.height()).abs() < SAME, "the body shrank back: {three:?} {three_again:?}");
    assert!((three.height() - LINE.y - LINE.y - LINE.y).abs() < GAPS, "the body hugs its lines: {three:?}");
}

#[trial]
fn a_scrolling_body_keeps_its_offset_while_its_content_changes_and_clamps_it_at_the_end() {
    let mut frames = Frames::new();
    let seen = Seen::default();
    let lines = Cell::new(40);
    let mut show = |ui: &mut Ui| scrolling_body(ui, lines.get(), &seen);
    frames.idle(3, &mut show);
    let (top0, centre) = (seen.first_top.get(), seen.body.get().expect("the body ran").center());
    wheel_at(&mut frames, centre, -HUNDRED, &mut show);
    frames.idle(2, &mut show);
    assert!((top0 - seen.first_top.get() - HUNDRED).abs() < SAME, "scrolled by 100");

    lines.set(30);
    frames.idle(3, &mut show);
    assert!((top0 - seen.first_top.get() - HUNDRED).abs() < SAME, "fewer lines keep the offset: {top0} {}", seen.first_top.get());

    lines.set(12);
    frames.idle(3, &mut show);
    let max = seen.content.get() - seen.body.get().expect("the body ran").height();
    assert!((top0 - seen.first_top.get() - max).abs() < NEAR, "content shorter than the offset clamps it to the end: {max}");
}

#[trial]
fn header_and_footer_bracket_the_body() {
    let regions = RefCell::new(Regions::default());
    let first = Cell::new(f32::NAN);
    let rect = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    frames.idle(4, |ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let props = ContainerProps::new().header(HeaderRegion::new().rule(true)).footer(FooterRegion::new());
            let out = Slab::new(props).regions(&regions).id_salt("panel").show(ui, |ui, _| {
                first.set(Block::new(LINE).show(ui).rect.top());
            });
            rect.set(out.output.rect);
        });
    });
    let (header, footer) = (regions.borrow().header.expect("the header ran"), regions.borrow().footer.expect("the footer ran"));
    assert!((header.top() - rect.get().top()).abs() < SAME, "the header sits at the top: {header:?} {:?}", rect.get());
    assert!((first.get() - header.bottom()).abs() < SAME, "the body starts under the header: {} {header:?}", first.get());
    assert!((footer.bottom() - rect.get().bottom()).abs() < SAME, "a filling container pins its footer to the bottom: {footer:?}");
}

static COLUMNS: [ColumnWidth; 3] = [ColumnWidth::Fixed(FIRST_COLUMN), ColumnWidth::Content, ColumnWidth::fill(1.0)];

#[trial]
fn a_grid_body_places_children_by_column() {
    let rects = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    frames.idle(5, |ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, Vec2::new(400.0, 300.0)), |ui| {
            let sizes = [Vec2::new(30.0, 10.0), Vec2::new(40.0, 14.0), Vec2::new(20.0, 10.0), Vec2::new(30.0, 10.0), Vec2::new(WIDEST_CHILD, 10.0), Vec2::new(20.0, 10.0)];
            rects.borrow_mut().clear();
            Grid::new(&COLUMNS).id_salt("grid").show(ui, |ui, _| {
                for size in sizes {
                    rects.borrow_mut().push(Block::new(size).show(ui).rect);
                }
            });
        });
    });
    let rects = rects.borrow();
    assert!((rects[0].min.x - rects[3].min.x).abs() < SAME && (rects[1].min.x - rects[4].min.x).abs() < SAME, "children of one column share an x: {rects:?}");
    assert!(rects[1].min.x >= rects[0].min.x + FIRST_COLUMN, "the first column is 100 points wide: {rects:?}");
    assert!(rects[2].min.x >= rects[1].min.x + WIDEST_CHILD, "the second column is as wide as its widest child: {rects:?}");
    assert!(rects[3].min.y >= rects[1].max.y, "the second row starts under the tallest child of the first: {rects:?}");
}

#[trial]
fn a_stack_shows_its_children_one_under_another() {
    let rects = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    frames.idle(3, |ui| {
        rects.borrow_mut().clear();
        Stack::new().show(ui, |ui, _| {
            for _ in 0..2 {
                rects.borrow_mut().push(Block::new(LINE).show(ui).rect);
            }
        });
    });
    let rects = rects.borrow();
    assert!(rects[0].max.y <= rects[1].min.y && (rects[0].min.x - rects[1].min.x).abs() < SAME, "one under another: {rects:?}");
}
