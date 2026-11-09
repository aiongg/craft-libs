//! The annotation tool on a canvas (drags, keys, the Eraser) and the toolbar's palette.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::{CancelReason, CanvasView, EditEvent, SelectionChange};
use craft_review::annotate::{Annotation, AnnotationEdit, AnnotationId, AnnotationKind, AnnotationTool};
use craft_review::ui::{AnnotateTool, AnnotationToolbar, PALETTE};
use craft_ui::testing::{Fixture, ToolFixture, trial};
use egui::{Key, Modifiers, Pos2, Rect as ScreenRect, Vec2, vec2};
use kurbo::{Point, Rect, Vec2 as PageVec};

/// How close two page coordinates are when they went through the view and back.
const SAME: f64 = 1e-3;

fn view(room: ScreenRect) -> CanvasView {
    CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0)
}

/// `tool` on a canvas whose page (0, 0) is the canvas's top left corner at zoom 1.
fn rig(tool: AnnotateTool) -> ToolFixture<AnnotateTool> {
    ToolFixture::with_view("Document", tool, view, |_, _| {})
}

/// The screen position of page point `at`.
fn on_screen(rig: &ToolFixture<AnnotateTool>, at: Point) -> Pos2 {
    rig.canvas().rect.min + vec2(at.x as f32, at.y as f32)
}

fn annotation(id: u64, kind: AnnotationKind, rect: Rect) -> Annotation {
    Annotation {
        id: AnnotationId(id),
        page: 0,
        kind,
        rect,
        points: Vec::new(),
        strokes: Vec::new(),
        color: [255, 0, 0],
        opacity: 1.0,
        stroke_width: 1.0,
        text: String::new(),
        locked: false,
    }
}

fn ink(id: u64, xs: &[f64]) -> Annotation {
    let mut ink = annotation(id, AnnotationKind::Ink, Rect::new(0.0, 0.0, 300.0, 200.0));
    ink.strokes = xs.iter().map(|&x| vec![Point::new(x, 0.0), Point::new(x, 100.0)]).collect();
    ink
}

/// The edits' values, in order.
fn values(rig: &ToolFixture<AnnotateTool>) -> Vec<AnnotationEdit> {
    rig.tool_mut().take_edits().iter().filter_map(|edit| edit.value().cloned()).collect()
}

/// Drags the tool from page point `from` to page point `to`.
fn drag(rig: &mut ToolFixture<AnnotateTool>, from: Point, to: Point) {
    let (start, end) = (on_screen(rig, from), on_screen(rig, to));
    rig.drag_from(start, end - start);
}

/// A click on the annotation that is already selected: it gives the canvas the keyboard and changes nothing.
fn focus_on(rig: &mut ToolFixture<AnnotateTool>, at: Point) {
    let pos = on_screen(rig, at);
    rig.press_at(pos).release_at(pos);
}

fn same(a: Rect, b: Rect) -> bool {
    [(a.x0, b.x0), (a.y0, b.y0), (a.x1, b.x1), (a.y1, b.y1)].iter().all(|(x, y)| (x - y).abs() < SAME)
}

#[trial]
fn drawing_a_rectangle_creates_it() {
    let mut rig = rig(AnnotateTool::new(AnnotationTool::Draw(AnnotationKind::Rectangle), 3));
    drag(&mut rig, Point::new(50.0, 50.0), Point::new(150.0, 120.0));
    let edits = values(&rig);
    let [AnnotationEdit::Create(created)] = &edits[..] else { panic!("{edits:?}") };
    assert_eq!((created.kind, created.page), (AnnotationKind::Rectangle, 3));
    assert!(same(created.rect, Rect::new(50.0, 50.0, 150.0, 120.0)), "{:?}", created.rect);

    rig.fixture().hold(Modifiers::SHIFT);
    drag(&mut rig, Point::new(10.0, 10.0), Point::new(90.0, 40.0));
    rig.fixture().hold(Modifiers::NONE);
    let edits = values(&rig);
    let [AnnotationEdit::Create(square)] = &edits[..] else { panic!("{edits:?}") };
    assert!(same(square.rect, Rect::new(10.0, 10.0, 90.0, 90.0)), "Shift squares the rectangle: {:?}", square.rect);
}

#[trial]
fn dragging_a_selected_annotation_moves_it() {
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    tool.annotations = vec![annotation(7, AnnotationKind::Rectangle, Rect::new(100.0, 100.0, 200.0, 180.0))];
    let mut rig = rig(tool);
    drag(&mut rig, Point::new(150.0, 140.0), Point::new(180.0, 150.0));
    let edits = rig.tool_mut().take_edits();
    let Some(EditEvent::Set { after: AnnotationEdit::Selection(SelectionChange::Set(ids)), .. }) = edits.first() else { panic!("{edits:?}") };
    assert_eq!(ids, &[AnnotationId(7)]);
    assert!(matches!(&edits[1], EditEvent::Begin { .. }), "{edits:?}");
    let Some(EditEvent::Commit { after: AnnotationEdit::Move { ids, delta }, .. }) = edits.last() else { panic!("{edits:?}") };
    assert_eq!(ids, &[AnnotationId(7)]);
    assert!((delta.x - 30.0).abs() < SAME && (delta.y - 10.0).abs() < SAME, "{delta:?}");

    let from = on_screen(&rig, Point::new(150.0, 140.0));
    rig.drag_and_escape_from(from, Vec2::new(60.0, 0.0));
    let edits = rig.tool_mut().take_edits();
    assert!(matches!(edits.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{edits:?}");
    assert!(!edits.iter().any(|edit| matches!(edit, EditEvent::Commit { .. })), "{edits:?}");
}

#[trial]
fn delete_and_arrows_edit_the_selection() {
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    tool.annotations = vec![annotation(1, AnnotationKind::Rectangle, Rect::new(10.0, 10.0, 60.0, 60.0))];
    tool.selected = vec![AnnotationId(1)];
    let mut rig = rig(tool);
    focus_on(&mut rig, Point::new(35.0, 35.0));
    rig.fixture().key(Key::ArrowRight, Modifiers::NONE);
    rig.fixture().key(Key::ArrowDown, Modifiers::SHIFT);
    rig.fixture().key(Key::Delete, Modifiers::NONE);
    let ids = vec![AnnotationId(1)];
    assert_eq!(
        values(&rig),
        vec![
            AnnotationEdit::Move { ids: ids.clone(), delta: PageVec::new(1.0, 0.0) },
            AnnotationEdit::Move { ids: ids.clone(), delta: PageVec::new(0.0, 10.0) },
            AnnotationEdit::Delete(ids),
        ]
    );
}

#[trial]
fn backspace_deletes_the_selection() {
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    tool.annotations = vec![annotation(1, AnnotationKind::Rectangle, Rect::new(10.0, 10.0, 60.0, 60.0))];
    tool.selected = vec![AnnotationId(1)];
    let mut rig = rig(tool);
    focus_on(&mut rig, Point::new(35.0, 35.0));
    rig.fixture().key(Key::Backspace, Modifiers::NONE);
    assert_eq!(values(&rig), vec![AnnotationEdit::Delete(vec![AnnotationId(1)])]);
}

/// The Eraser reports each annotation once, even when its strokes are crossed with another annotation's in
/// between: an app removing strokes by index would otherwise delete the wrong ones.
#[trial]
fn eraser_reports_one_erase_per_annotation_crossing_a_b_a() {
    let mut tool = AnnotateTool::new(AnnotationTool::Eraser, 0);
    // Annotation 4 is only partly crossed: only its first stroke is reported.
    tool.annotations = vec![ink(1, &[50.0, 250.0]), ink(2, &[150.0]), ink(4, &[270.0, 300.0])];
    let mut rig = rig(tool);
    drag(&mut rig, Point::new(10.0, 50.0), Point::new(280.0, 50.0));
    assert_eq!(
        values(&rig),
        vec![
            AnnotationEdit::Erase { id: AnnotationId(1), strokes: vec![0, 1] },
            AnnotationEdit::Erase { id: AnnotationId(2), strokes: vec![0] },
            AnnotationEdit::Erase { id: AnnotationId(4), strokes: vec![0] },
        ]
    );
}

/// A double click on a note reports `Open`, once, after the selection its first click made.
#[trial]
fn double_click_on_a_note_opens_it() {
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    tool.annotations = vec![annotation(9, AnnotationKind::Note, Rect::new(100.0, 100.0, 100.0, 100.0))];
    let mut rig = rig(tool);
    let pos = on_screen(&rig, Point::new(104.0, 104.0));
    rig.double_press_at(pos);
    let edits = values(&rig);
    assert!(matches!(&edits[0], AnnotationEdit::Selection(SelectionChange::Set(ids)) if ids == &[AnnotationId(9)]), "{edits:?}");
    assert_eq!(edits[1..], [AnnotationEdit::Open(AnnotationId(9))], "{edits:?}");
}

/// A note is created in the tool's colour and width.
#[trial]
fn a_new_note_takes_the_tools_style() {
    let mut tool = AnnotateTool::new(AnnotationTool::Draw(AnnotationKind::Note), 0);
    tool.color = [1, 2, 3];
    tool.width = 5.0;
    let mut rig = rig(tool);
    let pos = on_screen(&rig, Point::new(60.0, 70.0));
    rig.press_at(pos).release_at(pos);
    let edits = values(&rig);
    let [AnnotationEdit::Create(note)] = &edits[..] else { panic!("{edits:?}") };
    assert_eq!((note.color, note.stroke_width), ([1, 2, 3], 5.0));
}

/// Picks the Red swatch of the palette in a toolbar over a tool of `kind`, and returns the tool.
fn pick_red(kind: AnnotationTool) -> AnnotateTool {
    let tool = Rc::new(RefCell::new(AnnotateTool::new(kind, 0)));
    let shown = Rc::clone(&tool);
    let mut fixture = Fixture::new(move |ui| {
        AnnotationToolbar::new(&mut shown.borrow_mut()).show(ui);
    });
    fixture.settle();
    let well = fixture.by_label("Color").expect("the toolbar has a colour well");
    fixture.click(&well);
    fixture.idle(3);
    let red = fixture.by_label("Red").expect("the palette lists Red");
    fixture.click(&red);
    fixture.idle(3);
    drop(fixture);
    Rc::try_unwrap(tool).ok().expect("the fixture released the tool").into_inner()
}

/// A palette swatch sets the highlight colour under Highlight and the line colour under every other drawing tool.
#[trial]
fn picking_a_palette_swatch_sets_the_colour_of_the_tool() {
    let red = PALETTE[2];
    let highlight = pick_red(AnnotationTool::Draw(AnnotationKind::Highlight));
    let untouched = AnnotateTool::new(AnnotationTool::Select, 0);
    assert_eq!((highlight.highlight_color, highlight.color), (red, untouched.color), "Highlight takes its own colour");
    let rectangle = pick_red(AnnotationTool::Draw(AnnotationKind::Rectangle));
    assert_eq!((rectangle.color, rectangle.highlight_color), (red, untouched.highlight_color), "other tools take the line colour");
}
