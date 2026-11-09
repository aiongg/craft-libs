//! `TextRun` layout and `Draw::text` under both directions.

use craft_i18n::Direction;
use craft_theme::{Contrast, Mode};
use craft_ui::primitives::{ChordText, HAlign, Span, TextRun, Wrap};
use craft_ui::style::Ink;
use craft_ui::testing::style::{paint_frame, tokens};
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Shape, pos2};

const SLACK: f32 = 1.0;
const RECT: Rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(210.0, 40.0));

fn text_x(shapes: &[Shape]) -> Option<f32> {
    shapes.iter().find_map(|s| match s {
        Shape::Text(text) => Some(text.pos.x),
        _ => None,
    })
}

#[trial]
fn text_starts_at_the_reading_start_and_a_chord_stays_left_to_right() {
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let chord = ChordText::new("Ctrl+K");
    let paint = |direction, chord_run: bool| {
        let mut frames = Frames::new();
        paint_frame(&mut frames, &tokens, direction, Ink::OnPanel, |draw| {
            if chord_run {
                draw.text(RECT, &chord.run());
            } else {
                draw.text(RECT, &TextRun::new("Open"));
            }
        })
        .0
    };
    let ltr = text_x(&paint(Direction::Ltr, false));
    let rtl = text_x(&paint(Direction::Rtl, false));
    assert_eq!(ltr, Some(RECT.left()));
    assert!(rtl.is_some_and(|x| x > RECT.center().x), "under RTL the run hangs from the right edge");
    assert_eq!(text_x(&paint(Direction::Rtl, true)), Some(RECT.left()), "a chord is its own left-to-right run");
}

#[trial]
fn spans_style_a_run_without_splitting_its_text() {
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let mut frames = Frames::new();
    paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| {
        let run = TextRun::spans(vec![Span::new("Save "), Span::new("Ctrl+S").shortcut()]);
        let layout = draw.text(RECT, &run);
        assert_eq!(layout.galley.job.text, "Save Ctrl+S");
        assert_eq!(layout.galley.job.sections.len(), 2, "the shortcut has its own format");
        assert!(!layout.elided);
    });
}

/// The first and last row's start and end, from the layout's left edge.
fn row_edges(direction: Direction, align: HAlign) -> [(f32, f32); 2] {
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let mut frames = Frames::new();
    let mut edges = [(0.0, 0.0); 2];
    paint_frame(&mut frames, &tokens, direction, Ink::OnPanel, |draw| {
        let layout = draw.text(RECT, &TextRun::new("A wide first line\nshort").align(align));
        for (edge, placed) in edges.iter_mut().zip(&layout.galley.rows) {
            *edge = (placed.pos.x, placed.pos.x + placed.row.size.x);
        }
    });
    edges
}

#[trial]
fn lines_of_a_run_align_at_the_start_the_centre_or_the_end_and_mirror() {
    let close = |a: f32, b: f32| (a - b).abs() <= SLACK;
    let [wide, short] = row_edges(Direction::Ltr, HAlign::Center);
    assert!(close((wide.0 + wide.1) / 2.0, (short.0 + short.1) / 2.0) && short.0 > wide.0, "centred lines share a centre");
    let [wide, short] = row_edges(Direction::Ltr, HAlign::End);
    assert!(close(wide.1, short.1) && short.0 > wide.0, "end-aligned lines share their right edge in LTR");
    let [wide, short] = row_edges(Direction::Rtl, HAlign::End);
    assert!(close(wide.0, short.0) && short.1 < wide.1, "end-aligned lines share their left edge in RTL");
    let [wide, short] = row_edges(Direction::Ltr, HAlign::Start);
    assert!(close(wide.0, short.0), "start-aligned lines share their left edge in LTR");
}

#[trial]
fn elided_text_records_its_full_text_and_a_path_keeps_its_ends() {
    {
        let tokens = tokens(Mode::Light, Contrast::Standard);
        let mut frames = Frames::new();
        let full = "A label far too long for the room it is given";
        let narrow = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(60.0, 20.0));
        let (_, recorded) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| {
            let layout = draw.text(narrow, &TextRun::new(full).elide());
            assert!(layout.elided);
            assert_eq!(layout.full, full);
        });
        assert_eq!(recorded.elided.as_deref(), Some(full));
        assert!(recorded.baseline.is_some_and(|b| b > 0.0));
        let (_, fits) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| {
            assert!(!draw.text(RECT, &TextRun::new("Short").elide()).elided);
        });
        assert_eq!(fits.elided, None, "text that fits records no elision");
    }
    {
        let tokens = tokens(Mode::Light, Contrast::Standard);
        let mut frames = Frames::new();
        let path = "/home/user/projects/a-long-project-name/exports/final-document.txt";
        let narrow = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(200.0, 20.0));
        paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| {
            let layout = draw.text(narrow, &TextRun::new(path).wrap(Wrap::ElidePath));
            let shown = layout.galley.job.text.clone();
            assert!(layout.elided && layout.galley.size().x <= narrow.width() + SLACK, "the run fits its room");
            assert!(shown.contains('\u{2026}'), "the cut is marked");
            assert!(shown.ends_with("document.txt"), "the file name stays: {shown}");
            assert!(shown.starts_with('/'), "the start stays too: {shown}");
        });
    }
}
