//! The annotation tool.

use craft_ui::behavior::SelectionPick;
use craft_ui::canvas::{CanvasId, CanvasView, ToolHost, ToolInput};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::theme::{Base, install};
use egui::{Context, Event, Key, Modifiers, RawInput, Sense, pos2, vec2};
use kurbo::{Point, Rect, Vec2};

use super::annotate::{AnnotateTool, AnnotationToolbar};
use crate::annotate::{Annotation, AnnotationEdit, AnnotationId, AnnotationKind, AnnotationTool};

/// A tool on a canvas fed by the host's automation input, one pass per call.
struct Rig {
    ctx: Context,
    tool: AnnotateTool,
    view: CanvasView,
    id: Option<CanvasId>,
    focus: bool,
    time: f64,
}

impl Rig {
    fn new(tool: AnnotateTool) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let view = CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
        let mut rig = Rig { ctx, tool, view, id: None, focus: false, time: 0.0 };
        rig.pass(Vec::new());
        rig
    }

    fn pass(&mut self, events: Vec<Event>) {
        self.time += 0.05;
        let input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))),
            time: Some(self.time),
            events,
            focused: true,
            ..Default::default()
        };
        let Rig { ctx, tool, view, id, focus, .. } = self;
        ctx.run_ui(input, |ui| {
            let canvas_id = CanvasId::new(ui, "annotations");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
            if std::mem::take(focus) {
                canvas.request_focus();
            }
            *id = Some(ToolHost::new(canvas_id).show(ui, view, &canvas, tool).id);
        })
        .drop_without_applying_deltas();
    }

    fn input(&mut self, input: ToolInput) {
        ToolHost::inject(&self.ctx, self.id.expect("a pass ran"), input);
        self.pass(Vec::new());
    }

    fn down(&mut self, at: Point, modifiers: Modifiers) {
        self.input(ToolInput::Down { button: egui::PointerButton::Primary, doc: at, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
    }

    fn drag_to(&mut self, at: Point, modifiers: Modifiers) {
        self.input(ToolInput::Move { doc: at, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
    }

    fn up(&mut self, at: Point, modifiers: Modifiers) {
        self.input(ToolInput::Up { doc: at, modifiers });
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.focus = true;
        self.pass(Vec::new());
        self.pass(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    }
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

fn values(tool: &mut AnnotateTool) -> Vec<AnnotationEdit> {
    tool.take_edits().iter().filter_map(|e| e.value().cloned()).collect()
}

// a drag with the Rectangle tool reports Create with the rect.
#[test]
fn drawing_a_rectangle_creates_it() {
    let mut rig = Rig::new(AnnotateTool::new(AnnotationTool::Draw(AnnotationKind::Rectangle), 3));
    rig.down(Point::new(50.0, 50.0), Modifiers::NONE);
    rig.drag_to(Point::new(150.0, 120.0), Modifiers::NONE);
    rig.up(Point::new(150.0, 120.0), Modifiers::NONE);
    let edits = values(&mut rig.tool);
    let [AnnotationEdit::Create(created)] = &edits[..] else { panic!("{edits:?}") };
    assert_eq!((created.kind, created.page, created.rect), (AnnotationKind::Rectangle, 3, Rect::new(50.0, 50.0, 150.0, 120.0)));

    rig.down(Point::new(10.0, 10.0), Modifiers::NONE);
    rig.drag_to(Point::new(90.0, 40.0), Modifiers::SHIFT);
    rig.up(Point::new(90.0, 40.0), Modifiers::SHIFT);
    let edits = values(&mut rig.tool);
    let [AnnotationEdit::Create(square)] = &edits[..] else { panic!("{edits:?}") };
    assert_eq!(square.rect, Rect::new(10.0, 10.0, 90.0, 90.0));
}

// a drag reports a Move gesture; Esc cancels.
#[test]
fn dragging_a_selected_annotation_moves_it() {
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    tool.annotations = vec![annotation(7, AnnotationKind::Rectangle, Rect::new(100.0, 100.0, 200.0, 180.0))];
    let mut rig = Rig::new(tool);
    rig.down(Point::new(150.0, 140.0), Modifiers::NONE);
    rig.drag_to(Point::new(180.0, 150.0), Modifiers::NONE);
    rig.up(Point::new(180.0, 150.0), Modifiers::NONE);
    let edits = rig.tool.take_edits();
    let EditEvent::Set { value: AnnotationEdit::Selection(change), .. } = &edits[0] else { panic!("{edits:?}") };
    assert_eq!((change.pick, change.row, &change.selection.items[..]), (SelectionPick::Replace, Some(AnnotationId(7)), &[AnnotationId(7)][..]));
    assert!(matches!(&edits[1], EditEvent::Begin { .. }), "{edits:?}");
    let Some(EditEvent::Commit { value, changed: true, .. }) = edits.last() else { panic!("{edits:?}") };
    assert_eq!(value, &AnnotationEdit::Move { ids: vec![AnnotationId(7)], delta: Vec2::new(30.0, 10.0) });

    rig.down(Point::new(150.0, 140.0), Modifiers::NONE);
    rig.drag_to(Point::new(170.0, 140.0), Modifiers::NONE);
    rig.key(Key::Escape, Modifiers::NONE);
    rig.up(Point::new(170.0, 140.0), Modifiers::NONE);
    let edits = rig.tool.take_edits();
    assert!(matches!(edits.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{edits:?}");
    assert!(!edits.iter().any(|e| matches!(e, EditEvent::Commit { .. })), "{edits:?}");
}

// the Eraser reports Erase for the strokes it crosses.
#[test]
fn eraser_reports_crossed_strokes() {
    let mut ink = annotation(4, AnnotationKind::Ink, Rect::new(40.0, 0.0, 300.0, 200.0));
    ink.strokes = vec![vec![Point::new(50.0, 0.0), Point::new(50.0, 100.0)], vec![Point::new(300.0, 0.0), Point::new(300.0, 100.0)]];
    let mut tool = AnnotateTool::new(AnnotationTool::Eraser, 0);
    tool.annotations = vec![ink];
    let mut rig = Rig::new(tool);
    rig.down(Point::new(0.0, 50.0), Modifiers::NONE);
    rig.drag_to(Point::new(120.0, 50.0), Modifiers::NONE);
    rig.up(Point::new(120.0, 50.0), Modifiers::NONE);
    assert_eq!(values(&mut rig.tool), vec![AnnotationEdit::Erase { id: AnnotationId(4), strokes: vec![0] }]);
}

// Delete reports Delete, arrows nudge.
#[test]
fn delete_and_arrows_edit_the_selection() {
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    tool.annotations = vec![annotation(1, AnnotationKind::Rectangle, Rect::new(10.0, 10.0, 60.0, 60.0))];
    tool.selected = vec![AnnotationId(1)];
    let mut rig = Rig::new(tool);
    let ids = vec![AnnotationId(1)];
    rig.key(Key::ArrowRight, Modifiers::NONE);
    rig.key(Key::ArrowDown, Modifiers::SHIFT);
    rig.key(Key::Delete, Modifiers::NONE);
    assert_eq!(
        values(&mut rig.tool),
        vec![
            AnnotationEdit::Move { ids: ids.clone(), delta: Vec2::new(1.0, 0.0) },
            AnnotationEdit::Move { ids: ids.clone(), delta: Vec2::new(0.0, 10.0) },
            AnnotationEdit::Delete(ids),
        ]
    );
}

// The Eraser reports each annotation once, even when its strokes are crossed with another
// annotation's in between: an app removing strokes by index would otherwise delete the wrong ones.
#[test]
fn eraser_reports_one_erase_per_annotation_crossing_a_b_a() {
    let ink = |id: u64, xs: &[f64]| {
        let mut a = annotation(id, AnnotationKind::Ink, Rect::new(0.0, 0.0, 300.0, 200.0));
        a.strokes = xs.iter().map(|&x| vec![Point::new(x, 0.0), Point::new(x, 100.0)]).collect();
        a
    };
    let mut tool = AnnotateTool::new(AnnotationTool::Eraser, 0);
    tool.annotations = vec![ink(1, &[50.0, 250.0]), ink(2, &[150.0])];
    let mut rig = Rig::new(tool);
    rig.down(Point::new(0.0, 50.0), Modifiers::NONE);
    for x in [80.0, 180.0, 280.0] {
        rig.drag_to(Point::new(x, 50.0), Modifiers::NONE);
    }
    rig.up(Point::new(280.0, 50.0), Modifiers::NONE);
    assert_eq!(
        values(&mut rig.tool),
        vec![AnnotationEdit::Erase { id: AnnotationId(1), strokes: vec![0, 1] }, AnnotationEdit::Erase { id: AnnotationId(2), strokes: vec![0] },]
    );
}

// A double-click on a note reports Open, once, after the selection its first click made.
#[test]
fn double_click_on_a_note_opens_it() {
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    tool.annotations = vec![annotation(9, AnnotationKind::Note, Rect::new(100.0, 100.0, 100.0, 100.0))];
    let mut rig = Rig::new(tool);
    let at = Point::new(104.0, 104.0);
    for _ in 0..2 {
        rig.down(at, Modifiers::NONE);
        rig.up(at, Modifiers::NONE);
    }
    let edits = values(&mut rig.tool);
    assert!(matches!(&edits[0], AnnotationEdit::Selection(_)), "{edits:?}");
    assert_eq!(edits[1..], [AnnotationEdit::Open(AnnotationId(9))], "{edits:?}");
}

// A note is created with an icon-sized rect at the click, in the tool's colour and width.
#[test]
fn a_new_note_takes_an_icon_rect_and_the_tools_style() {
    let mut tool = AnnotateTool::new(AnnotationTool::Draw(AnnotationKind::Note), 0);
    tool.color = [1, 2, 3];
    tool.width = 5.0;
    let mut rig = Rig::new(tool);
    rig.down(Point::new(60.0, 70.0), Modifiers::NONE);
    let edits = values(&mut rig.tool);
    let [AnnotationEdit::Create(note)] = &edits[..] else { panic!("{edits:?}") };
    assert_eq!((note.rect.x0, note.rect.y0), (60.0, 70.0));
    assert!(note.rect.width() > 0.0 && note.rect.width() == note.rect.height(), "{:?}", note.rect);
    assert_eq!((note.color, note.stroke_width), ([1, 2, 3], 5.0));
}

// Backspace deletes as Delete does.
#[test]
fn backspace_deletes_the_selection() {
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    tool.annotations = vec![annotation(1, AnnotationKind::Rectangle, Rect::new(10.0, 10.0, 60.0, 60.0))];
    tool.selected = vec![AnnotationId(1)];
    let mut rig = Rig::new(tool);
    rig.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(values(&mut rig.tool), vec![AnnotationEdit::Delete(vec![AnnotationId(1)])]);
}

// The toolbar reports a flyout per group, and opening one by its id shows it.
#[test]
fn the_toolbar_exposes_its_flyouts() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut tool = AnnotateTool::new(AnnotationTool::Select, 0);
    let run = |tool: &mut AnnotateTool| {
        let mut flyouts = Vec::new();
        ctx.run_ui(RawInput { screen_rect: Some(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 200.0))), ..Default::default() }, |ui| {
            flyouts = AnnotationToolbar::new("annotate", tool).show(ui).flyouts;
        })
        .drop_without_applying_deltas();
        flyouts
    };
    let flyouts = run(&mut tool);
    assert_eq!(flyouts.len(), 3);
    let (_, draw) = flyouts[2];
    assert!(!egui::Popup::is_id_open(&ctx, draw));
    egui::Popup::open_id(&ctx, draw);
    run(&mut tool);
    assert!(egui::Popup::is_id_open(&ctx, draw));
    assert_eq!(tool.tool, AnnotationTool::Select);
}
