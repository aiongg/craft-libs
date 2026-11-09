//! A scrolling body: the wheel and the thumb move it in the frame that carries them, and its content size and
//! scroll offset follow the content.

use super::kit::Slab;
use craft_ui::contract::{ContainerProps, Props as _, Scroll, Show as _, ShowContainer as _, Sizing};
use craft_ui::layout_components::Stack;
use craft_ui::testing::layout::{Block, in_region};
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::Cell;

const LINE: Vec2 = Vec2::new(100.0, 20.0);
/// Positions that should coincide, and a looser bound for positions that follow a drag.
const SAME: f32 = 0.5;
const NEAR: f32 = 1.0;
/// Wheel distances, and the least a thumb drag of 60 points moves a body of 40 lines beyond them.
const BIG_STEP: f32 = 200.0;
const HUNDRED: f32 = 100.0;
const THUMB_GAIN: f32 = 150.0;
/// What the gaps between lines add to a body's height, at most.
const GAPS: f32 = 20.0;
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
