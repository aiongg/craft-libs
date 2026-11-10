//! A scrolling body: the wheel and the thumb move it in the frame that carries them, and its content size and
//! scroll offset follow the content.

use super::kit::Slab;
use craft_behavior::ColumnWidth;
use craft_ui::buttons::Button;
use craft_ui::contract::{ContainerProps, Props as _, Scroll, Show as _, ShowContainer as _, Sizing};
use craft_ui::layout_components::{Grid, Stack};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::testing::layout::{Block, in_region};
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Id, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const LINE: Vec2 = Vec2::new(100.0, 20.0);
/// Positions that should coincide, and a looser bound for positions that follow a drag.
const SAME: f32 = 0.5;
const NEAR: f32 = 1.0;
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

static COLUMNS: [ColumnWidth; 3] = [ColumnWidth::Fixed(FIRST_COLUMN), ColumnWidth::Content, ColumnWidth::fill(1.0)];

#[trial]
fn a_grid_body_places_children_by_column() {
    let rects = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    frames.idle(5, |ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, Vec2::new(400.0, 300.0)), |ui| {
            let sizes = [
                Vec2::new(30.0, 10.0),
                Vec2::new(40.0, 14.0),
                Vec2::new(20.0, 10.0),
                Vec2::new(30.0, 10.0),
                Vec2::new(WIDEST_CHILD, 10.0),
                Vec2::new(20.0, 10.0),
            ];
            Grid::new(&COLUMNS).id_salt("grid").show(ui, |ui, _| {
                rects.borrow_mut().clear();
                for size in sizes {
                    rects.borrow_mut().push(Block::new(size).show(ui).rect);
                }
            });
        });
    });
    let rects = rects.borrow();
    assert!(
        (rects[0].min.x - rects[3].min.x).abs() < SAME && (rects[1].min.x - rects[4].min.x).abs() < SAME,
        "children of one column share an x: {rects:?}"
    );
    assert!(rects[1].min.x >= rects[0].min.x + FIRST_COLUMN, "the first column is 100 points wide: {rects:?}");
    assert!(rects[2].min.x >= rects[1].min.x + WIDEST_CHILD, "the second column is as wide as its widest child: {rects:?}");
    assert!(rects[3].min.y >= rects[1].max.y, "the second row starts under the tallest child of the first: {rects:?}");
}

#[trial]
fn a_content_sized_scrolling_stack_is_as_tall_as_its_room_and_scrolls() {
    let (rect, extent) = (Cell::new(Rect::NOTHING), Cell::new(0.0));
    let mut frames = Frames::new();
    frames.idle(5, |ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let out = Stack::new().scroll(Scroll::vertical()).id_salt("stack").show(ui, |ui, _| {
                for _ in 0..40 {
                    Block::new(LINE).show(ui);
                }
                extent.set(ui.min_rect().height());
            });
            rect.set(out.output.rect);
        });
    });
    assert!(extent.get() > VIEW.y, "the content is taller than the room: {}", extent.get());
    assert!((rect.get().height() - VIEW.y).abs() < NEAR, "a content-sized scrolling body takes the room's height, not zero: {:?}", rect.get());
}

/// Children of a scrolling body that run only in view (`Lazy`): one far below the view does not run and keeps the
/// body's content size; scrolled into view, it runs again. A thumb drag that jumps several views shows children that
/// ran in that frame. Guards the out-of-view rule, its one store, and that the decision sees the painted offset.
#[trial]
fn an_out_of_view_child_does_not_run_until_it_scrolls_in() {
    const CHILDREN: usize = 30;
    let runs: Vec<Cell<usize>> = (0..CHILDREN).map(|_| Cell::new(0)).collect();
    let seen = Seen::default();
    // The rects of the children the last pass ran for real.
    let shown = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        shown.borrow_mut().clear();
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            Slab::new(ContainerProps::new().scroll(Scroll::vertical())).id_salt("panel").show(ui, |ui, cx| {
                seen.body.set(Some(cx.rect));
                seen.first_top.set(ui.next_widget_position().y);
                for (at, count) in runs.iter().enumerate() {
                    craft_ui::contract::Lazy::new(at).show(ui, |ui| {
                        count.set(count.get() + 1);
                        let rect = Block::new(Vec2::new(LINE.x, HUNDRED)).show(ui).rect;
                        if !ui.is_sizing_pass() {
                            shown.borrow_mut().push(rect);
                        }
                    });
                }
                seen.content.set(ui.min_rect().height());
            });
        });
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    let full = seen.content.get();
    let last = CHILDREN - 1;
    runs.iter().for_each(|count| count.set(0));
    frames.frame(vec![], &mut show);
    assert!(runs[0].get() > 0, "the child in view runs");
    assert_eq!(runs[last].get(), 0, "the child far below the view does not run");
    assert!((seen.content.get() - full).abs() <= SAME, "the content keeps its size: {} then {full}", seen.content.get());
    let view = seen.body.get().expect("the body ran");
    for _ in 0..CHILDREN {
        wheel_at(&mut frames, view.center(), -BIG_STEP, &mut show);
    }
    runs.iter().for_each(|count| count.set(0));
    frames.frame(vec![], &mut show);
    assert!(runs[last].get() > 0, "scrolled to the end, the last child runs");
    assert_eq!(runs[0].get(), 0, "and the first, far above the view, does not");
    assert!((seen.content.get() - full).abs() <= SAME, "the content keeps its size at the end");

    // Back at the top, a thumb drag of 60 points moves the content by several views in one frame.
    for _ in 0..CHILDREN {
        wheel_at(&mut frames, view.center(), BIG_STEP, &mut show);
    }
    let top0 = seen.first_top.get();
    let thumb = Pos2::new(view.max.x - 2.0, view.min.y + 4.0);
    frames.frame(vec![pointer::moved(thumb)], &mut show);
    frames.frame(vec![pointer::press(thumb)], &mut show);
    frames.frame(vec![pointer::moved(thumb + Vec2::new(0.0, 4.0))], &mut show);
    frames.frame(vec![pointer::moved(thumb + Vec2::new(0.0, 8.0))], &mut show);
    let before = top0 - seen.first_top.get();
    frames.frame(vec![pointer::moved(thumb + Vec2::new(0.0, 60.0))], &mut show);
    let jumped = top0 - seen.first_top.get();
    assert!(jumped - before > 3.0 * view.height(), "the drag jumped several views: {before} -> {jumped}");
    let mut spans: Vec<(f32, f32)> = shown.borrow().iter().map(|rect| (rect.min.y, rect.max.y)).collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut covered = view.min.y;
    for (top, bottom) in spans {
        if top <= covered + GAPS {
            covered = covered.max(bottom);
        }
    }
    assert!(covered >= view.max.y, "the children in view after the jump ran in that frame: covered to {covered}, view {view:?}");
    frames.frame(vec![pointer::release(thumb + Vec2::new(0.0, 60.0))], &mut show);
}

/// A focus request for a control in a child far out of view runs that child, keeps the focus there, and the body
/// scrolls the control into view: the child's stops from its last run name it.
#[trial]
fn a_focus_request_into_an_out_of_view_child_runs_it_and_scrolls_it_in() {
    const CHILDREN: usize = 30;
    let runs: Vec<Cell<usize>> = (0..CHILDREN).map(|_| Cell::new(0)).collect();
    let buttons: Vec<Cell<(Id, Rect)>> = (0..CHILDREN).map(|_| Cell::new((Id::NULL, Rect::NOTHING))).collect();
    let view = Cell::new(Rect::NOTHING);
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            Slab::new(ContainerProps::new().scroll(Scroll::vertical())).id_salt("panel").show(ui, |ui, cx| {
                view.set(cx.rect);
                for (at, (count, button)) in runs.iter().zip(&buttons).enumerate() {
                    craft_ui::contract::Lazy::new(at).show(ui, |ui| {
                        count.set(count.get() + 1);
                        let out = Button::new(format!("Child {at}")).show(ui);
                        if !ui.is_sizing_pass() {
                            button.set((out.id, out.rect));
                        }
                        Block::new(Vec2::new(LINE.x, HUNDRED)).show(ui);
                    });
                }
            });
        });
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    let last = CHILDREN - 1;
    runs.iter().for_each(|count| count.set(0));
    frames.frame(vec![], &mut show);
    assert_eq!(runs[last].get(), 0, "the child far below the view does not run");
    let target = buttons[last].get().0;
    request(&frames.ctx, FocusTarget::Id(target));
    frames.idle(3, &mut show);
    assert!(runs[last].get() > 0, "the requested child runs");
    assert_eq!(frames.ctx.memory(|memory| memory.focused()), Some(target), "the focus stays on the requested control");
    let (_, rect) = buttons[last].get();
    assert!(view.get().contains_rect(rect), "the body scrolled the control into view: {rect:?} in {:?}", view.get());
}
