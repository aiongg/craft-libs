//! A container's sizes come from this pass's measuring run: it measures its contents once per frame, before its real
//! run, and the containers inside it reuse what that run recorded.

use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::display::Text;
use craft_ui::layout_components::{Row, Stack};
use craft_ui::sections::Section;
use craft_ui::testing::layout::Block;
use craft_ui::testing::{Fixture, Frames, measuring_runs, trial};
use egui::{Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SLACK: f32 = 0.5;

/// A section in a content-sized stack takes the width of its text in the frame the text grows, without a second pass.
#[trial]
fn a_content_section_takes_its_new_width_in_the_same_pass() {
    let text = RefCell::new("Short");
    let seen = Cell::new((Rect::NOTHING, Rect::NOTHING));
    let show = |ui: &mut Ui| {
        Stack::new().show(ui, |ui, _| {
            let mut text_rect = Rect::NOTHING;
            let section = Section::fixed("S").id_salt("section").show(ui, |ui, _| text_rect = Text::new(*text.borrow()).show(ui).rect);
            seen.set((section.output.response.rect, text_rect));
        });
    };
    let mut frames = Frames::new();
    frames.idle(3, show);
    let (section, short) = seen.get();
    let pad = section.width() - short.width();
    *text.borrow_mut() = "A much longer line of text than before";
    let passes = frames.frame(vec![], show);
    let (section, long) = seen.get();
    assert!(long.width() > short.width() + 10.0, "the text grew: {short:?} -> {long:?}");
    assert!(
        (section.width() - (long.width() + pad)).abs() <= SLACK,
        "the section is the text's width plus padding {pad}: {section:?}, text {long:?}"
    );
    assert_eq!(passes, 1, "the new width comes from this pass's measuring run");
}

/// Six nested content-sized stacks: each one's contents run in exactly one measuring run per frame.
#[trial]
fn nested_containers_measure_once_per_frame() {
    fn nest(ui: &mut Ui, depth: usize) {
        if depth == 0 {
            Text::new("Inside").show(ui);
            return;
        }
        Stack::new().show(ui, |ui, _| nest(ui, depth - 1));
    }
    let mut frames = Frames::new();
    frames.idle(3, |ui| nest(ui, 6));
    let passes = frames.frame(vec![], |ui| nest(ui, 6));
    let runs = measuring_runs(&frames.ctx);
    assert_eq!(passes, 1);
    assert!(runs.len() >= 6, "every stack measured: {runs:?}");
    assert!(runs.iter().all(|(_, count)| *count == 1), "one measuring run per body: {runs:?}");
}

/// A filling stack inside a filling stack shown at a rect: the inner body runs in its own measuring run and its real
/// run, never in its parent's measuring run.
#[trial]
fn a_filling_root_is_not_run_in_its_parents_measure() {
    let calls = Cell::new(0);
    let show = |ui: &mut Ui| {
        let at = Rect::from_min_size(ui.cursor().min, Vec2::new(300.0, 200.0));
        Stack::new().fill().show_at(ui, at, |ui, _| {
            Stack::new().fill().show(ui, |ui, _| {
                calls.set(calls.get() + 1);
                Text::new("Root").show(ui);
            });
        });
    };
    let mut frames = Frames::new();
    frames.idle(3, show);
    calls.set(0);
    let passes = frames.frame(vec![], show);
    assert_eq!(passes, 1);
    assert_eq!(calls.get(), 2, "one call from the real run and one from its own measuring run");
}

/// The rects of a wrapping row's lead and of the row nested after it.
fn wrap_rects(fixture: &mut Fixture, seen: &Rc<Cell<(Rect, Rect)>>, width: f32) -> (Rect, Rect) {
    fixture.room(Vec2::new(width, 300.0)).settle();
    seen.get()
}

fn wrapping_fixture() -> (Fixture, Rc<Cell<(Rect, Rect)>>) {
    let seen = Rc::new(Cell::new((Rect::NOTHING, Rect::NOTHING)));
    let out = Rc::clone(&seen);
    let fixture = Fixture::new(move |ui| {
        Stack::new().fill().show(ui, |ui, _| {
            Stack::new().show(ui, |ui, _| {
                Row::new().wrap(true).show(ui, |ui, _| {
                    let lead = Block::new(Vec2::new(200.0, 24.0)).show(ui).rect;
                    let nested = Row::new().show(ui, |ui, _| {
                        Text::new("to").show(ui);
                        Block::new(Vec2::new(192.0, 24.0)).show(ui);
                    });
                    out.set((lead, nested.output.rect));
                });
            });
        });
    });
    (fixture, seen)
}

/// A fixture widened from 400 to 700 lays its wrapping row out as a fresh 700 fixture does: no size from an earlier
/// pass keeps the narrow layout.
#[trial]
fn a_row_widened_live_lays_out_as_a_fresh_one() {
    let (mut live, live_seen) = wrapping_fixture();
    wrap_rects(&mut live, &live_seen, 400.0);
    let widened = wrap_rects(&mut live, &live_seen, 700.0);
    let (mut fresh, fresh_seen) = wrapping_fixture();
    let fresh = wrap_rects(&mut fresh, &fresh_seen, 700.0);
    assert!((fresh.1.top() - fresh.0.top()).abs() <= SLACK, "a fresh 700 room fits the nested row beside the lead: {fresh:?}");
    let near = |a: Rect, b: Rect| (a.min - b.min).length() <= SLACK && (a.max - b.max).length() <= SLACK;
    assert!(near(widened.0, fresh.0) && near(widened.1, fresh.1), "widened {widened:?}, fresh {fresh:?}");
}

/// Six nested content-sized containers of three kinds: in a settled frame each one's contents run once in a measuring
/// run (an enclosing one's, reused by the rest) and once in the real run.
#[trial]
fn a_six_deep_nest_runs_each_container_once_measuring_and_once_real() {
    fn nest(ui: &mut Ui, depth: usize, calls: &RefCell<Vec<u32>>) {
        if depth == 0 {
            Text::new("Inside").show(ui);
            return;
        }
        calls.borrow_mut()[depth - 1] += 1;
        let inner = |ui: &mut Ui| nest(ui, depth - 1, calls);
        match depth % 3 {
            0 => {
                let _ = Stack::new().show(ui, |ui, _| inner(ui));
            }
            1 => {
                let _ = Row::new().show(ui, |ui, _| inner(ui));
            }
            _ => {
                let _ = Section::fixed("S").id_salt(("nest", depth)).show(ui, |ui, _| inner(ui));
            }
        }
    }
    let calls = RefCell::new(vec![0; 6]);
    let mut frames = Frames::new();
    frames.idle(3, |ui| nest(ui, 6, &calls));
    calls.borrow_mut().fill(0);
    let passes = frames.frame(vec![], |ui| nest(ui, 6, &calls));
    let runs = measuring_runs(&frames.ctx);
    assert_eq!(passes, 1);
    assert!(runs.len() >= 6 && runs.iter().all(|(_, count)| *count == 1), "one measuring run per container: {runs:?}");
    // The outermost closure runs outside any container; every other one runs in one measuring run and one real run.
    assert_eq!(calls.borrow()[..5], [2; 5], "each container's contents run twice: {calls:?}");
}
