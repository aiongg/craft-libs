//! A container's sizes come from a measuring run: in a pass that measures, it measures its contents once, before its
//! real run, and the containers inside it reuse what that run recorded. A later pass with the same inputs takes the
//! kept measurement; the trials that count the runs of a measuring pass drop the kept measurements first
//! (`forget_measurements`).

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::display::Text;
use craft_ui::layout_components::{Row, Stack};
use craft_ui::sections::Section;
use craft_ui::testing::layout::Block;
use craft_ui::testing::{Fixture, Frames, forget_measurements, last_pass_measuring_runs, measuring_runs, trial};
use craft_ui::text::TextField;
use egui::{Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SLACK: f32 = 0.5;

/// A section in a content-sized stack takes the width of its text in the frame the text grows.
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
    assert!(long.width() > short.width() + SLACK, "the text grew: {short:?} -> {long:?}");
    assert!(
        (section.width() - (long.width() + pad)).abs() <= SLACK,
        "the section is the text's width plus padding {pad}: {section:?}, text {long:?}"
    );
    assert!(passes <= 2, "the real run finds the content changed and the frame measures again: {passes} passes");
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
    forget_measurements(&frames.ctx);
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
    forget_measurements(&frames.ctx);
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
    forget_measurements(&frames.ctx);
    calls.borrow_mut().fill(0);
    let passes = frames.frame(vec![], |ui| nest(ui, 6, &calls));
    let runs = measuring_runs(&frames.ctx);
    assert_eq!(passes, 1);
    assert!(runs.len() >= 6 && runs.iter().all(|(_, count)| *count == 1), "one measuring run per container: {runs:?}");
    // The outermost closure runs outside any container; every other one runs in one measuring run and one real run.
    assert_eq!(calls.borrow()[..5], [2; 5], "each container's contents run twice: {calls:?}");
}

/// Runs `show` until it settles, then one more frame with the kept measurements dropped: the measuring runs of that
/// frame, by body.
fn runs_of_a_settled_frame(mut show: impl FnMut(&mut Ui)) -> Vec<(egui::Id, u32)> {
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    forget_measurements(&frames.ctx);
    let passes = frames.frame(vec![], &mut show);
    assert_eq!(passes, 1);
    measuring_runs(&frames.ctx)
}

/// A content-sized section after a fill leaf in a given-width row is measured at the room it gets, so neither it nor
/// the stacks inside it measure again in the real run.
#[trial]
fn a_section_after_a_fill_leaf_measures_once() {
    let mut text = String::new();
    let runs = runs_of_a_settled_frame(|ui| {
        Row::new().fill().show(ui, |ui, _| {
            let _ = TextField::new("Name", Bind::bound(&mut text)).show(ui);
            Section::fixed("S").id_salt("section").show(ui, |ui, _| {
                Stack::new().show(ui, |ui, _| {
                    Stack::new().show(ui, |ui, _| Text::new("Inside").show(ui));
                });
            });
        });
    });
    assert!(runs.len() >= 4, "every body measured: {runs:?}");
    assert!(runs.iter().all(|(_, count)| *count == 1), "one measuring run per body: {runs:?}");
}

/// A fill-width stack sharing a given-width row with a fixed sibling is measured at its share, so it does not measure
/// again in the real run.
#[trial]
fn a_fill_width_stack_beside_a_fixed_sibling_measures_once() {
    let runs = runs_of_a_settled_frame(|ui| {
        Row::new().fill().show(ui, |ui, _| {
            Stack::new().fill_width().show(ui, |ui, _| {
                Stack::new().show(ui, |ui, _| Text::new("Inside").show(ui));
            });
            Block::new(Vec2::new(80.0, 24.0)).show(ui);
        });
    });
    assert!(runs.len() >= 3, "every body measured: {runs:?}");
    assert!(runs.iter().all(|(_, count)| *count == 1), "one measuring run per body: {runs:?}");
}

/// A fill-width stack of wrapping text beside a fixed block is measured at its share of the row: it is as high as its
/// text wraps there, and what follows the row starts below all of it. The row sweeps its children twice while
/// measuring and once in the real run; every body is measured once.
#[trial]
fn a_fill_width_stack_is_measured_at_its_share() {
    const WORDS: &str = "A long line of words that wraps when the stack is narrower than the whole room it could have taken here";
    let seen = Cell::new([Rect::NOTHING; 4]);
    let sweeps = Cell::new(0);
    let runs = runs_of_a_settled_frame(|ui| {
        sweeps.set(0);
        Stack::new().fill().show(ui, |ui, _| {
            let mut text = Rect::NOTHING;
            let row = Row::new().fill_width().show(ui, |ui, _| {
                sweeps.set(sweeps.get() + 1);
                let stack = Stack::new().fill_width().show(ui, |ui, _| {
                    craft_ui::testing::layout::wrapped_text(ui, WORDS);
                    text = ui.min_rect();
                });
                let block = Block::new(Vec2::new(600.0, 24.0)).show(ui);
                (stack.output.rect, block.rect)
            });
            let below = Block::new(Vec2::new(100.0, 24.0)).show(ui).rect;
            if let Some((stack, _)) = row.inner {
                seen.set([row.output.rect, stack, text, below]);
            }
        });
    });
    let [row, stack, text, below] = seen.get();
    let dump = format!("row {row:?}, stack {stack:?}, text {text:?}, below {below:?}");
    assert!(row.width() - stack.width() > stack.width(), "the stack takes what the block leaves: {dump}");
    assert!(text.height() > 0.0 && stack.expand(SLACK).contains_rect(text), "the stack holds its wrapped text: {dump}");
    assert!(row.expand(SLACK).contains_rect(stack) && below.top() >= row.bottom() - SLACK, "the row below starts after the stack: {dump}");
    assert_eq!(sweeps.get(), 3, "two measuring sweeps and the real run");
    assert!(runs.iter().all(|(_, count)| *count == 1), "one measuring run per body: {runs:?}");
}

/// A scrolling stack whose content overflows it: its measuring run lays the content out beside the bar, as its real
/// run does, so the fill-width grid rows and the panels in them measure once per frame, not again in the real run.
#[trial]
fn panels_in_an_overflowing_scroll_stack_measure_once() {
    use craft_behavior::ColumnWidth;
    use craft_ui::contract::{Scroll, Sizing};
    use craft_ui::layout_components::Grid;
    use craft_ui::sections::Panel;
    const TWO: &[ColumnWidth] = &[ColumnWidth::fill(1.0); 2];
    let runs = runs_of_a_settled_frame(|ui| {
        Stack::new().fill().scroll(Scroll::vertical()).show(ui, |ui, _| {
            for row in 0..8 {
                Grid::new(TWO).fill_width().show(ui, |ui, _| {
                    for at in 0..2 {
                        Panel::new(format!("Card {row} {at}")).sizing(Sizing::Fill).show(ui, |ui, _| {
                            Block::new(Vec2::new(100.0, 80.0)).show(ui);
                        });
                    }
                });
            }
        });
    });
    assert!(runs.len() >= 25, "the stack, every row and every panel measured: {runs:?}");
    assert!(runs.iter().all(|(_, count)| *count == 1), "one measuring run per body: {runs:?}");
}

/// A `Fill` collection under a content-sized body builds and measures its widest row once per frame: the measuring
/// run's record serves the other probes of the pass.
#[trial]
fn a_fill_collection_under_a_content_body_measures_its_widest_row_once() {
    use crate::collection::kit::Words;
    use craft_ui::contract::{Scroll, ShowCollection as _, Sizing};
    use craft_ui::list::{List, ListRow};
    use craft_ui::testing::widest_runs;
    let words = Words::flat(&["short", "a row label wider than the others", "mid"]);
    let mut show = |ui: &mut Ui| {
        Stack::new().scroll(Scroll::horizontal()).show(ui, |ui, _| {
            let _ = List::<usize>::new("Rows").sizing(Sizing::Fill).show_items(ui, &words, &mut |key, _| ListRow::new(words.words[*key]));
        });
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    frames.frame(vec![], &mut show);
    let runs = widest_runs(&frames.ctx);
    assert!(!runs.is_empty() && runs.iter().all(|(_, count)| *count == 1), "the widest row is measured once per collection: {runs:?}");
}

/// A settled frame of nested containers with unchanged inputs runs no measuring run: each takes its kept measurement.
/// The first frame measured each once.
#[trial]
fn a_settled_frame_runs_no_measuring_run() {
    fn nest(ui: &mut Ui, depth: usize) {
        if depth == 0 {
            Text::new("Inside").show(ui);
            return;
        }
        Stack::new().show(ui, |ui, _| nest(ui, depth - 1));
    }
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| nest(ui, 6));
    assert!(last_pass_measuring_runs(&frames.ctx).len() >= 6, "the first frame measures every stack");
    frames.idle(3, |ui| nest(ui, 6));
    let passes = frames.frame(vec![], |ui| nest(ui, 6));
    assert_eq!(passes, 1);
    let runs = last_pass_measuring_runs(&frames.ctx);
    assert!(runs.is_empty(), "no stack measures again: {runs:?}");
}

/// A row that shrinks its children takes its kept measurement in settled frames as any body does, whether its
/// children fit (one sweep) or shrink (a second sweep at their widths): the real run shows them under the ids the
/// measuring run gave them, so it finds each in the kept transcript.
#[trial]
fn a_settled_row_that_shrinks_its_children_runs_one_pass_and_no_measuring_run() {
    for width in [900.0, 260.0] {
        let show = |ui: &mut Ui| {
            let at = Rect::from_min_size(ui.cursor().min, Vec2::new(width, 200.0));
            Stack::new().fill().show_at(ui, at, |ui, _| {
                Row::new().show(ui, |ui, _| {
                    let mut path = String::from("/home/studio/Documents/brochure-final.pdf");
                    let _ = TextField::new("Path", Bind::bound(&mut path)).show(ui);
                    let _ = Button::new("Choose File...").show(ui);
                    let _ = Button::new("Another").show(ui);
                });
            });
        };
        let mut frames = Frames::new();
        frames.idle(3, show);
        for _ in 0..3 {
            let passes = frames.frame(vec![], show);
            assert_eq!(passes, 1, "a settled frame at width {width} runs one pass");
            let runs = last_pass_measuring_runs(&frames.ctx);
            assert!(runs.is_empty(), "no body measures again at width {width}: {runs:?}");
        }
    }
}

/// A room change measures the containers again in that frame, in one pass: the room is an input the kept
/// measurement was taken under.
#[trial]
fn a_room_change_measures_again_in_its_frame() {
    let width = Cell::new(300.0);
    let seen = Cell::new((Rect::NOTHING, Rect::NOTHING));
    let show = |ui: &mut Ui| {
        let at = Rect::from_min_size(ui.cursor().min, Vec2::new(width.get(), 200.0));
        Stack::new().fill().show_at(ui, at, |ui, _| {
            let inner = Stack::new().show(ui, |ui, _| {
                Text::new("A line of text that wraps when the room is narrow enough").show(ui);
            });
            seen.set((at, inner.output.rect));
        });
    };
    let mut frames = Frames::new();
    frames.idle(4, show);
    width.set(120.0);
    let passes = frames.frame(vec![], show);
    let (at, inner) = seen.get();
    assert_eq!(passes, 1, "the new room is known before the run");
    assert!(!last_pass_measuring_runs(&frames.ctx).is_empty(), "the stacks measure at the new room");
    assert!(inner.width() <= at.width() + SLACK, "the inner stack fits the narrower room: {inner:?} in {at:?}");
    // A fresh measure under new inputs is not motion: the next frame takes what it found.
    let passes = frames.frame(vec![], show);
    assert_eq!(passes, 1, "the frame after");
    let runs = last_pass_measuring_runs(&frames.ctx);
    assert!(runs.is_empty(), "the frame after the room change measures nothing again: {runs:?}");
}

/// The widest label of a label column becomes short: in that frame the controls move to the narrower column. The
/// column's kept width confirms itself from the labels the real run shows, not from the width it gave them.
#[trial]
fn a_label_column_narrows_in_the_frame_its_widest_label_shrinks() {
    use crate::label::group::Group;
    use crate::label::kit::{Dial, Field, Seen};
    use craft_ui::contract::SideLabel;
    let long = Cell::new("Hue and saturation");
    let (a_seen, b_seen) = (RefCell::new(Seen::default()), RefCell::new(Seen::default()));
    let (mut a, mut b) = (1.0, 1.0);
    let mut show = |ui: &mut Ui| {
        let at = Rect::from_min_size(ui.cursor().min, Vec2::new(300.0, 150.0));
        Group::column().id_salt("column").show_at(ui, at, |ui, _| {
            Field::new("Size", SideLabel::new()).id_salt("short").show(ui, |ui, _| {
                Dial::new(&mut a, &a_seen).show(ui);
            });
            Field::new(long.get(), SideLabel::new()).id_salt("long").show(ui, |ui, _| {
                Dial::new(&mut b, &b_seen).show(ui);
            });
        });
    };
    let mut frames = Frames::new();
    frames.idle(4, &mut show);
    let wide = a_seen.borrow().rect.min.x;
    long.set("Hue");
    frames.frame(vec![], &mut show);
    let (a_start, b_start) = (a_seen.borrow().rect.min.x, b_seen.borrow().rect.min.x);
    assert!(a_start < wide - SLACK, "the column narrows in the same frame: {wide} -> {a_start}");
    assert!((a_start - b_start).abs() <= SLACK, "both controls start in the one column: {a_start}, {b_start}");
}

/// A value the app changes every frame, with no change of size, runs one pass a frame and no measuring run.
#[trial]
fn a_value_change_without_a_size_change_runs_one_pass_and_no_measuring_run() {
    use craft_ui::value_drag::Slider;
    let value = Cell::new(0.1_f64);
    let show = |ui: &mut Ui| {
        Stack::new().show(ui, |ui, _| {
            Text::new("Opacity").show(ui);
            let _ = Slider::new(Bind::given(value.get()), 0.0..=1.0).show(ui);
        });
    };
    let mut frames = Frames::new();
    frames.idle(4, show);
    for step in 0..3 {
        value.set(0.2 + 0.2 * f64::from(step));
        let passes = frames.frame(vec![], show);
        assert_eq!(passes, 1, "frame {step}");
        let runs = last_pass_measuring_runs(&frames.ctx);
        assert!(runs.is_empty(), "frame {step}: no body measures again: {runs:?}");
    }
}

/// The widest cell of a content-sized grid narrows: in that frame the second column moves to the narrower first one.
/// The check asks each cell at the room its measuring run gave it, not at the track width the kept records give.
#[trial]
fn a_grid_narrows_in_the_frame_its_widest_cell_shrinks() {
    use craft_behavior::ColumnWidth;
    use craft_ui::layout_components::Grid;
    static COLUMNS: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::Content];
    let first = Cell::new("A much longer first column label");
    let second = Cell::new(Rect::NOTHING);
    let show = |ui: &mut Ui| {
        Stack::new().show(ui, |ui, _| {
            Grid::new(&COLUMNS).show(ui, |ui, _| {
                Text::new(first.get()).show(ui);
                second.set(Text::new("Second").show(ui).rect);
                Text::new("Short").show(ui);
                Text::new("Again").show(ui);
            });
        });
    };
    let mut frames = Frames::new();
    frames.idle(4, show);
    let wide = second.get().min.x;
    first.set("Tiny");
    frames.frame(vec![], show);
    let narrow = second.get().min.x;
    assert!(narrow < wide - SLACK, "the second column moves left in the same frame: {wide} -> {narrow}");
    frames.idle(3, show);
    assert!((second.get().min.x - narrow).abs() <= SLACK, "and stays there: {narrow} -> {}", second.get().min.x);
}

/// A child of a row turns from content-sized to filling: in that frame it takes the room its sibling leaves. The
/// check holds each child's sizing mode, not only its size.
#[trial]
fn a_child_that_starts_to_fill_takes_its_share_in_that_frame() {
    let fills = Cell::new(false);
    let seen = Cell::new((Rect::NOTHING, Rect::NOTHING));
    let show = |ui: &mut Ui| {
        let at = Rect::from_min_size(ui.cursor().min, Vec2::new(400.0, 40.0));
        Row::new().fill().show_at(ui, at, |ui, _| {
            Text::new("Name").show(ui);
            let stack = if fills.get() { Stack::new().fill_width() } else { Stack::new() };
            let inner = stack.show(ui, |ui, _| {
                Text::new("Value").show(ui);
            });
            seen.set((at, inner.output.rect));
        });
    };
    let mut frames = Frames::new();
    frames.idle(4, show);
    let (_, content) = seen.get();
    fills.set(true);
    frames.frame(vec![], show);
    let (at, filled) = seen.get();
    assert!(filled.width() > content.width() + SLACK, "the child fills in the same frame: {content:?} -> {filled:?}");
    assert!((filled.max.x - at.max.x).abs() <= SLACK, "it ends at the row's end: {filled:?} in {at:?}");
}

/// A section closes: in every frame of the toggle the text below it starts under the section's rect as shown, so
/// the new height shows in its frame.
#[trial]
fn a_collapse_toggle_shows_the_new_height_in_its_frame() {
    let mut open = true;
    let seen = Cell::new((Rect::NOTHING, Rect::NOTHING));
    let show = |ui: &mut Ui, open: &mut bool| {
        Stack::new().show(ui, |ui, _| {
            let section = Section::new("Options").open(Bind::bound(open)).id_salt("section").show(ui, |ui, _| {
                Text::new("First line of the body").show(ui);
                Text::new("Second line of the body").show(ui);
            });
            let below = Text::new("Below").show(ui).rect;
            seen.set((section.output.response.rect, below));
        });
    };
    let mut frames = Frames::new();
    frames.idle(4, |ui| show(ui, &mut open));
    let (tall, _) = seen.get();
    open = false;
    for frame in 0..12 {
        frames.frame(vec![], |ui| show(ui, &mut open));
        let (section, below) = seen.get();
        assert!(below.min.y >= section.max.y - SLACK, "frame {frame}: the text starts below the section: {section:?}, {below:?}");
    }
    let (closed, _) = seen.get();
    assert!(closed.height() < tall.height() - SLACK, "the section closed: {tall:?} -> {closed:?}");
}

/// Fields that share a label column, in a stack measured before it shows (as a gallery card does): a settled frame
/// runs one pass. The real run's label reports match the measuring run's, which reports a label twice and how far each
/// row reaches past the column.
#[trial]
fn a_settled_shared_label_column_runs_one_pass_per_frame() {
    use craft_ui::contract::{LabelAlign, SideLabel};
    use craft_ui::form::{Field, FieldGroup};
    let rows = |ui: &mut Ui| {
        FieldGroup::new("Settings").show(ui, |ui, _| {
            for label in ["Size", "Hue and saturation", "Gap"] {
                Field::new(label).side(SideLabel::new().align(LabelAlign::Start)).id_salt(label).show(ui, |ui, _| {
                    craft_ui::text::NumberField::new(label, Bind::given(Some(42.0))).unit("mm").show(ui);
                });
            }
        });
    };
    // As a gallery card does: the stack measures its content before it shows it.
    let show = |ui: &mut Ui| {
        Stack::new().id_salt("card").measure_in(ui, craft_behavior::Proposal::Ideal, |ui, _| rows(ui));
        Stack::new().id_salt("card").show(ui, |ui, _| rows(ui));
    };
    let mut frames = Frames::new();
    frames.idle(4, show);
    for frame in 0..6 {
        assert_eq!(frames.frame(vec![], show), 1, "settled frame {frame}");
    }
}
