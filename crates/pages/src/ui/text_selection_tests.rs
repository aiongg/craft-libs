//! The Select Text tool in a `ToolHost` over a mock page text.

use std::ops::Range;

use craft_ui::canvas::{CanvasId, CanvasView, ToolHost};
use craft_ui::direction::Direction;
use craft_ui::theme::{Base, install};
use egui::{CursorIcon, Event, Key, Modifiers, PointerButton, Pos2, Sense, pos2, vec2};
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::{Point, Rect};

use super::text_selection::*;
use super::view::PlacedPage;

/// Three lines of seven glyphs, 10 wide and 12 high, lines 20 apart, page point = screen point.
const LINES: [&str; 3] = ["one two", "six ten", "ace dog"];

struct Mock;

impl Mock {
    fn chars() -> Vec<char> {
        LINES.iter().flat_map(|line| line.chars()).collect()
    }

    fn box_of(glyph: usize) -> Rect {
        let (line, column) = ((glyph / 7) as f64, (glyph % 7) as f64);
        Rect::new(column * 10.0, line * 20.0, column * 10.0 + 10.0, line * 20.0 + 12.0)
    }

    fn center(glyph: usize) -> Pos2 {
        let c = Self::box_of(glyph).center();
        pos2(c.x as f32, c.y as f32)
    }

    fn joined(glyphs: impl Iterator<Item = usize>) -> String {
        let chars = Self::chars();
        let mut out = String::new();
        let mut line = None;
        for glyph in glyphs {
            if line.is_some_and(|l| l != glyph / 7) {
                out.push('\n');
            }
            line = Some(glyph / 7);
            out.push(chars[glyph]);
        }
        out
    }
}

impl PageTextSource for Mock {
    fn glyph_count(&self) -> usize {
        21
    }

    /// Nearest within 4 points, as a text layer takes a press between glyphs.
    fn glyph_at(&self, p: Point) -> Option<usize> {
        (0..21).find(|glyph| Self::box_of(*glyph).inflate(4.0, 4.0).contains(p))
    }

    fn glyph_box_at(&self, p: Point) -> Option<usize> {
        (0..21).find(|glyph| Self::box_of(*glyph).contains(p))
    }

    fn line_rects(&self, glyphs: Range<usize>) -> Vec<Rect> {
        (0..3)
            .filter_map(|line| {
                let own: Vec<usize> = glyphs.clone().filter(|g| g / 7 == line).collect();
                own.first().zip(own.last()).map(|(a, b)| Self::box_of(*a).union(Self::box_of(*b)))
            })
            .collect()
    }

    fn glyphs_in(&self, rect: Rect) -> Vec<usize> {
        (0..21).filter(|glyph| rect.contains_rect(Self::box_of(*glyph))).collect()
    }

    fn word_at(&self, glyph: usize) -> Option<Range<usize>> {
        let chars = Self::chars();
        if chars[glyph] == ' ' {
            return None;
        }
        let line = glyph / 7;
        let start = (line * 7..=glyph).rev().take_while(|g| chars[*g] != ' ').last()?;
        let end = (glyph..line * 7 + 7).take_while(|g| chars[*g] != ' ').last()? + 1;
        Some(start..end)
    }

    fn line_at(&self, glyph: usize) -> Option<Range<usize>> {
        Some(glyph / 7 * 7..glyph / 7 * 7 + 7)
    }

    fn text_of(&self, glyphs: Range<usize>) -> String {
        Mock::joined(glyphs)
    }

    fn column_text(&self, glyphs: &[usize]) -> String {
        Mock::joined(glyphs.iter().copied())
    }
}

struct Texts;

impl PageTexts for Texts {
    fn page(&self, page: usize) -> Option<&dyn PageTextSource> {
        (page == 0).then_some(&Mock)
    }
}

struct Rig {
    view: CanvasView,
    pages: Vec<PlacedPage>,
    state: TextSelectState,
    changed: bool,
}

fn rig() -> Harness<'static, Rig> {
    rig_in(Direction::Ltr)
}

fn rig_in(direction: Direction) -> Harness<'static, Rig> {
    let rect = egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
    let rig = Rig {
        view: CanvasView::new(rect, Point::new(100.0, 50.0), 1.0),
        pages: vec![PlacedPage { page: 0, rect, scale: 1.0, band: (0.0, 100.0) }],
        state: TextSelectState::default(),
        changed: false,
    };
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(vec2(200.0, 100.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            craft_ui::direction::apply(ctx, direction);
        })
        .build_ui_state(
            |ui, rig: &mut Rig| {
                let canvas_id = CanvasId::new(ui, "canvas");
                let canvas = canvas_id.interact(ui, rig.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                let mut tool = TextSelectTool::new(&rig.pages, &Texts, &mut rig.state);
                let _ = ToolHost::new(canvas_id).show(ui, &rig.view, &canvas, &mut tool);
                rig.changed |= tool.changed();
            },
            rig,
        )
}

fn button(at: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers }
}

fn with(h: &Harness<'_, Rig>, event: Event, modifiers: Modifiers) {
    if modifiers == Modifiers::NONE {
        h.event(event);
    } else {
        h.event_modifiers(event, modifiers);
    }
}

fn press(h: &mut Harness<'_, Rig>, at: Pos2, modifiers: Modifiers) {
    h.event(Event::PointerMoved(at));
    h.step();
    with(h, button(at, true, modifiers), modifiers);
    h.step();
}

fn move_to(h: &mut Harness<'_, Rig>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.step();
}

fn release(h: &mut Harness<'_, Rig>, at: Pos2, modifiers: Modifiers) {
    with(h, button(at, false, modifiers), modifiers);
    h.step();
}

fn selection(h: &Harness<'_, Rig>) -> Option<TextSelection> {
    h.state().state.selection
}

fn run(anchor: usize, head: usize) -> Option<TextSelection> {
    Some(TextSelection { page: 0, span: TextSpan::Run { anchor, head } })
}

// A press at glyph 3 and a drag to glyph 10 select Run { anchor: 3, head: 10 }, changed() is true and text_of gives the text; Esc during the drag cancels it.
#[test]
fn drag_selects_in_reading_order() {
    let mut h = rig();
    press(&mut h, Mock::center(4), Modifiers::NONE);
    move_to(&mut h, Mock::center(10));
    assert_eq!(selection(&h), run(4, 10));
    assert!(h.state().changed);
    let selected = selection(&h).expect("selected");
    assert_eq!(selected.text(&Mock), "two\nsix ");
    assert_eq!(selected.rects(&Mock).len(), 2, "one rectangle per line piece");

    release(&mut h, Mock::center(10), Modifiers::NONE);
    assert_eq!(selection(&h), run(4, 10), "the release keeps it");

    press(&mut h, Mock::center(15), Modifiers::NONE);
    move_to(&mut h, Mock::center(19));
    assert_eq!(selection(&h), run(15, 19));
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(selection(&h), run(4, 10), "Esc during the drag brings the earlier selection back");
}

// Alt-drag over a two-column mock page selects only the left column's glyphs, one line per row in its text.
#[test]
fn alt_drag_selects_a_column() {
    let mut h = rig();
    press(&mut h, pos2(0.0, 0.0), Modifiers::ALT);
    move_to(&mut h, pos2(31.0, 56.0));
    release(&mut h, pos2(31.0, 56.0), Modifiers::ALT);
    let selected = selection(&h).expect("selected");
    assert!(matches!(selected.span, TextSpan::Column { .. }));
    assert_eq!(selected.text(&Mock), "one\nsix\nace");
    assert_eq!(selected.rects(&Mock).len(), 3);
}

// ToolPointer clicks 2, 3 and 4 select the word, the line and the page.
#[test]
fn multi_clicks_select_word_line_and_page() {
    let mut h = rig();
    let at = Mock::center(5);
    press(&mut h, at, Modifiers::NONE);
    assert_eq!(selection(&h), run(5, 5));
    release(&mut h, at, Modifiers::NONE);
    press(&mut h, at, Modifiers::NONE);
    assert_eq!(selection(&h), run(4, 6), "the word");
    release(&mut h, at, Modifiers::NONE);
    press(&mut h, at, Modifiers::NONE);
    assert_eq!(selection(&h), run(0, 6), "the line");
    release(&mut h, at, Modifiers::NONE);
    press(&mut h, at, Modifiers::NONE);
    assert_eq!(selection(&h), run(0, 20), "the page");
    release(&mut h, at, Modifiers::NONE);

    // A press off any glyph clears the selection.
    press(&mut h, pos2(150.0, 90.0), Modifiers::NONE);
    assert_eq!(selection(&h), None);
}

// The I-beam shows only inside a glyph's box: a point a press would snap to (between lines) keeps the arrow.
#[test]
fn the_i_beam_needs_a_point_inside_a_glyph_box() {
    let mut h = rig();
    move_to(&mut h, Mock::center(4));
    assert_eq!(h.output().platform_output.cursor_icon, CursorIcon::Text);
    move_to(&mut h, pos2(45.0, 14.0));
    assert_eq!(h.output().platform_output.cursor_icon, CursorIcon::Default);
}

// Shift-arrows extend toward the arrow's side: right to left, Shift-Left reaches the next glyph.
#[test]
fn shift_arrows_mirror_right_to_left() {
    for (direction, key, head) in [(Direction::Ltr, Key::ArrowRight, 5), (Direction::Ltr, Key::ArrowLeft, 3), (Direction::Rtl, Key::ArrowLeft, 5), (Direction::Rtl, Key::ArrowRight, 3)] {
        let mut h = rig_in(direction);
        press(&mut h, Mock::center(4), Modifiers::NONE);
        release(&mut h, Mock::center(4), Modifiers::NONE);
        h.key_press_modifiers(Modifiers::SHIFT, key);
        h.run();
        assert_eq!(selection(&h), run(4, head), "{direction:?} {key:?}");
    }
}
