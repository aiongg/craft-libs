//! A measured list: rows as high as their wrapped lines and bodies at the width the list gives
//! them, with exact spans for the rows out of view.

use egui::{Rect, Ui};
use egui_kittest::Harness;

use super::*;
use crate::behavior::{RowInfo, Selection};
use crate::theme::{Base, install};

const LIST_HEIGHT: f32 = 160.0;
const TEXT: &str = "Reworked the header grid, moved the logo into the shared library and fixed the baseline of the second column";

struct Host {
    rows: Vec<RowInfo<usize>>,
    selection: Selection<usize>,
    /// How many lines each row asks for; 0 for none.
    lines: Vec<usize>,
    reveal: Option<usize>,
    drawn: Vec<(usize, Rect)>,
    list: Rect,
}

fn show(ui: &mut Ui, host: &mut Host) {
    let Host { rows, selection, lines, reveal, drawn, list } = host;
    let mut builder = List::new("measured", "Versions", rows.as_slice()).selection(selection).max_height(LIST_HEIGHT).measured();
    if let Some(key) = reveal.take() {
        builder = builder.reveal(key);
    }
    let out = builder.show(ui, |_, &key| {
        let row = ListRow::new(key, format!("Row {key}"));
        match lines[key] {
            0 => row,
            n => row.detail("Today").lines(TEXT, n),
        }
    });
    *drawn = out.rows.clone();
    *list = out.response.rect;
}

fn harness(width: f32, lines: Vec<usize>) -> Harness<'static, Host> {
    let count = lines.len();
    let host = Host {
        rows: (0..count).map(RowInfo::leaf).collect(),
        selection: Selection::only(0),
        lines,
        reveal: None,
        drawn: Vec::new(),
        list: Rect::NOTHING,
    };
    let mut harness = Harness::<Host>::builder()
        .with_size([width, 400.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn heights(host: &Host) -> Vec<f32> {
    host.drawn.iter().map(|(_, rect)| rect.height()).collect()
}

// Rows with wrapped lines are taller than rows without, and a narrower list wraps to more lines
// so its rows are taller: each row has the height its own text needs at the list's width.
#[test]
fn rows_are_as_high_as_their_lines_at_the_lists_width() {
    let three = |width| heights(harness(width, vec![0, 3, 1]).state());
    let wide = three(640.0);
    let narrow = three(220.0);
    assert!(wide[1] > wide[0], "wrapped lines add height");
    assert!(wide[2] <= wide[1]);
    assert_eq!(wide[0], narrow[0], "a row without lines keeps its height");
    assert!(narrow[1] > wide[1], "the narrow list wraps to more lines");
    assert_eq!(narrow[2], wide[2], "one line is one line");
}

// The scroll bar takes width the rows are measured without: a list too tall for its height
// measures at the width less the bar, in the same pass, so the first pass has the final heights.
#[test]
fn a_scroll_bar_narrows_the_rows_before_they_are_measured() {
    let mut fits = harness(300.0, vec![3; 1]);
    let mut overflows = harness(300.0, vec![3; 12]);
    fits.run();
    overflows.run();
    let (one, many) = (fits.state().drawn[0].1, overflows.state().drawn[0].1);
    assert!(many.width() < one.width(), "the bar takes width");
    // The same row, drawn at the width it was measured at, on the first frame already.
    let first = Harness::<Host>::builder().with_size([300.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        show,
        Host {
            rows: (0..12).map(RowInfo::leaf).collect(),
            selection: Selection::only(0),
            lines: vec![3; 12],
            reveal: None,
            drawn: Vec::new(),
            list: Rect::NOTHING,
        },
    );
    let mut first = first;
    first.step();
    let settled = overflows.state().drawn[0].1.height();
    assert_eq!(first.state().drawn[0].1.height(), settled);
}

// Revealing a row far down puts it at the bottom edge: the rows above it, out of view, were
// measured at their own heights, or its edge would land elsewhere.
#[test]
fn scrolling_past_measured_rows_reveals_the_right_row() {
    let lines: Vec<usize> = (0..40).map(|at| at % 4).collect();
    let mut harness = harness(300.0, lines);
    harness.state_mut().reveal = Some(30);
    harness.run();
    let host = harness.state();
    let (_, rect) = host.drawn.iter().find(|(key, _)| *key == 30).expect("row 30 is drawn");
    assert!((rect.bottom() - host.list.bottom()).abs() < 1.0, "row 30 ends at the list's bottom, not {} off", rect.bottom() - host.list.bottom());
    assert!(host.drawn.iter().all(|(key, _)| *key >= 25), "the rows far above are out of view");
}
