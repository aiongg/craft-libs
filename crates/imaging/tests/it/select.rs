//! The selection tools on the tool host: each test drives one tool through a `ToolFixture` with injected pointer input
//! and a fake `SelectionProbe`, then reads the events the tool reports.

use craft_behavior::geometry::{Constraint, SelectionOperation};
use craft_behavior::{CancelReason, EditEvent, PixelSize, PointerButton, PointerSample};
use craft_imaging::ui::select::{
    DragKind, EdgeSearch, EdgeTracer, LassoTool, MagneticLassoTool, MarqueeKind, MarqueeTool, PolygonLassoTool, SelectEvent, SelectPhase,
    SelectToolPass, SelectionProbe, SelectionShape,
};
use craft_keymap::Mods;
use craft_ui::canvas::{CanvasTool, ToolAccess, ToolContext, ToolHost, ToolInput, ToolKey, ToolPath, ToolPointer};
use craft_ui::primitives::Draw;
use craft_ui::testing::{ToolFixture, input, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect as ScreenRect, pos2};
use kurbo::{Point, Rect, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SIZE: PixelSize = PixelSize::new(400, 300);
const NONE: Mods = Mods::NONE;

/// A document with an optional rectangle selection and floating piece.
#[derive(Default)]
struct Fake {
    selection: Option<Rect>,
    piece: Option<Rect>,
    document: u64,
}

impl Fake {
    fn empty() -> Self {
        Fake { document: 1, ..Fake::default() }
    }

    fn selected(selection: Rect) -> Self {
        Fake { selection: Some(selection), ..Fake::empty() }
    }
}

impl SelectionProbe for Fake {
    fn document(&self) -> u64 {
        self.document
    }

    fn size(&self) -> PixelSize {
        SIZE
    }

    fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    fn inside_selection(&self, at: Point) -> bool {
        self.selection.is_some_and(|rect| rect.contains(at))
    }

    fn floating(&self) -> bool {
        self.piece.is_some()
    }

    fn on_floating(&self, at: Point) -> bool {
        self.piece.is_some_and(|rect| rect.contains(at))
    }
}

/// What the tests read from every tool.
trait Reports {
    fn events(&mut self) -> Vec<SelectEvent>;
    fn phase(&self) -> SelectPhase;
    fn reset(&mut self);
}

macro_rules! reports {
    ($tool:ty $(, $param:ident: $bound:path)?) => {
        impl$(<$param: $bound>)? Reports for $tool {
            fn events(&mut self) -> Vec<SelectEvent> {
                self.take_events()
            }

            fn phase(&self) -> SelectPhase {
                self.phase()
            }

            fn reset(&mut self) {
                self.reset();
            }
        }
    };
}

reports!(MarqueeTool);
reports!(LassoTool);
reports!(PolygonLassoTool);
reports!(MagneticLassoTool<T>, T: EdgeTracer);

/// A tool with its probe, owning both so a fixture can hold them: every call builds the pass the app builds.
struct Owned<T> {
    tool: RefCell<T>,
    probe: Fake,
}

impl<T: Reports> Owned<T> {
    fn new(tool: T, probe: Fake) -> Self {
        Owned { tool: RefCell::new(tool), probe }
    }

    fn tool(&mut self) -> &mut T {
        self.tool.get_mut()
    }

    fn events(&mut self) -> Vec<SelectEvent> {
        self.tool().events()
    }

    fn phase(&self) -> SelectPhase {
        self.tool.borrow().phase()
    }
}

impl<T: Reports + 'static> CanvasTool for Owned<T>
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        SelectToolPass::new(self.tool.get_mut(), &self.probe).each_pass(cx);
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> craft_ui::canvas::ToolAction {
        SelectToolPass::new(self.tool.get_mut(), &self.probe).press(pointer, cx)
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        SelectToolPass::new(self.tool.get_mut(), &self.probe).drag(pointer, cx);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        SelectToolPass::new(self.tool.get_mut(), &self.probe).release(pointer, cx);
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        SelectToolPass::new(self.tool.get_mut(), &self.probe).hover(pointer, cx);
    }

    fn cancel(&mut self, reason: CancelReason, cx: &mut ToolContext<'_>) {
        SelectToolPass::new(self.tool.get_mut(), &self.probe).cancel(reason, cx);
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        SelectToolPass::new(self.tool.get_mut(), &self.probe).key(key, cx)
    }

    fn paint(&self, draw: &Draw<'_>, cx: &ToolContext<'_>) {
        SelectToolPass::new(&mut *self.tool.borrow_mut(), &self.probe).paint(draw, cx);
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> craft_behavior::Cursor {
        SelectToolPass::new(&mut *self.tool.borrow_mut(), &self.probe).cursor(cx)
    }

    fn takes(&self, button: PointerButton, mods: Mods) -> bool {
        SelectToolPass::new(&mut *self.tool.borrow_mut(), &self.probe).takes(button, mods)
    }

    fn claims(&self) -> craft_ui::canvas::ToolClaims {
        SelectToolPass::new(&mut *self.tool.borrow_mut(), &self.probe).claims()
    }

    fn access(&self, cx: &ToolContext<'_>) -> Option<ToolAccess> {
        SelectToolPass::new(&mut *self.tool.borrow_mut(), &self.probe).access(cx)
    }

    fn command_form(&self) -> ToolPath {
        SelectToolPass::new(&mut *self.tool.borrow_mut(), &self.probe).command_form()
    }
}

type Rig<T> = ToolFixture<Owned<T>>;

/// A canvas whose document point and screen point coincide.
fn rig<T: Reports + 'static>(label: &'static str, tool: T, probe: Fake) -> Rig<T>
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    let view = |room: ScreenRect| {
        let centre = room.center();
        craft_behavior::CanvasView::new(room, Point::new(f64::from(centre.x), f64::from(centre.y)), 1.0)
    };
    ToolFixture::with_view(label, Owned::new(tool, probe), view, |_, _| {})
}

fn at(point: (f64, f64)) -> Pos2 {
    pos2(point.0 as f32, point.1 as f32)
}

fn inject<T: Reports + 'static>(rig: &mut Rig<T>, input: ToolInput)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    let (ctx, id) = (rig.fixture().ctx().clone(), rig.canvas_id());
    ToolHost::inject(&ctx, id, input);
    rig.fixture().frame(Vec::new());
}

fn down<T: Reports + 'static>(rig: &mut Rig<T>, point: (f64, f64), mods: Mods)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    inject(rig, ToolInput::down(PointerButton::Primary, at(point)).mods(mods));
}

fn drag_to<T: Reports + 'static>(rig: &mut Rig<T>, point: (f64, f64), mods: Mods)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    inject(rig, ToolInput::moved(at(point)).mods(mods));
}

fn up<T: Reports + 'static>(rig: &mut Rig<T>, point: (f64, f64), mods: Mods)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    inject(rig, ToolInput::up(at(point)).mods(mods));
}

fn click<T: Reports + 'static>(rig: &mut Rig<T>, point: (f64, f64), mods: Mods)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    down(rig, point, mods);
    up(rig, point, mods);
}

/// A press, a drag and a release; the events.
fn stroke(rig: &mut Rig<MarqueeTool>, from: (f64, f64), to: (f64, f64), at_press: Mods, later: Mods) -> Vec<SelectEvent> {
    down(rig, from, at_press);
    drag_to(rig, to, later);
    up(rig, to, later);
    rig.tool_mut().events()
}

/// Holds `mods` from now on; one pass shows them to the tool.
fn hold<T: Reports + 'static>(rig: &mut Rig<T>, mods: Modifiers)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    rig.fixture().hold(mods).frame(Vec::new());
}

/// Gives the canvas the keyboard with a click that the tool then forgets.
fn focus<T: Reports + 'static>(rig: &mut Rig<T>)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    let corner = rig.canvas().rect.left_top() + egui::vec2(2.0, 2.0);
    rig.press_at(corner).release_at(corner);
    let mut tool = rig.tool_mut();
    tool.tool().reset();
    tool.events();
    drop(tool);
    // One pass drops the node the click's outline left in the tree.
    rig.fixture().frame(Vec::new());
}

fn press_key<T: Reports + 'static>(rig: &mut Rig<T>, key: Key, mods: Modifiers)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    rig.fixture().key(key, mods);
}

fn commit(rect: (f64, f64, f64, f64), operation: SelectionOperation) -> SelectEvent {
    SelectEvent::commit(SelectionShape::Rectangle(Rect::new(rect.0, rect.1, rect.2, rect.3)), operation)
}

fn polygon(points: &[(f64, f64)]) -> SelectionShape {
    SelectionShape::Polygon(points.iter().map(|&(x, y)| Point::new(x, y)).collect())
}

/// "begin", "update 1,0", "commit 2,0", "cancel"; other events as `Debug`.
fn tags(events: &[SelectEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| match event {
            SelectEvent::DragSelection(EditEvent::Begin { .. }) => "begin".to_owned(),
            SelectEvent::DragSelection(EditEvent::Update { value, .. }) => format!("update {},{}", value.delta[0], value.delta[1]),
            SelectEvent::DragSelection(EditEvent::Commit { after, .. }) => format!("commit {},{}", after.delta[0], after.delta[1]),
            SelectEvent::DragSelection(EditEvent::Cancel { .. }) => "cancel".to_owned(),
            other => format!("{other:?}"),
        })
        .collect()
}

/// The value of the tool's accessible node named "Selection outline", `None` while it has none.
fn outline_value<T: Reports + 'static>(rig: &mut Rig<T>) -> Option<String>
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    let node = rig.fixture().by_label("Selection outline")?;
    rig.fixture().tree().value_of(node.node).map(str::to_owned)
}

/// Cancels the gesture the tool holds, as the app does on a tool switch.
fn cancel_with<T: Reports + 'static>(rig: &mut Rig<T>, reason: CancelReason)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    let (ctx, id) = (rig.fixture().ctx().clone(), rig.canvas_id());
    let _ = ToolHost::cancel(&ctx, id, &mut *rig.tool_mut(), reason);
    rig.fixture().frame(Vec::new());
}

// ---- the shared tool row ----

#[trial]
fn each_selection_tool_meets_the_tool_row() {
    rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), Fake::empty()).check();
    rig("Lasso", LassoTool::new(), Fake::empty()).check();
    rig("Polygonal Lasso", PolygonLassoTool::new(), Fake::empty()).check();
    rig("Magnetic Lasso", MagneticLassoTool::new(Circle { log: Rc::default() }), Fake::empty()).check();
}

// ---- the marquees ----

// Normal style: Shift pressed after the press squares, Alt pressed after it centres, Shift held at the press does not
// (it is Add); Fixed Ratio and Fixed Size give whole-pixel rectangles.
#[trial]
fn marquee_corners_follow_constraint_and_modifiers() {
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), Fake::empty());
    let events = stroke(&mut rig, (10.5, 10.5), (50.5, 30.5), NONE, NONE);
    assert_eq!(events, [commit((10.0, 10.0, 51.0, 31.0), SelectionOperation::New)]);

    let events = stroke(&mut rig, (10.5, 10.5), (50.5, 30.5), NONE, Mods::SHIFT);
    assert_eq!(events, [commit((10.0, 10.0, 51.0, 51.0), SelectionOperation::New)], "Shift after the press squares");

    let events = stroke(&mut rig, (10.5, 10.5), (50.5, 30.5), Mods::SHIFT, Mods::SHIFT);
    assert_eq!(events, [commit((10.0, 10.0, 51.0, 31.0), SelectionOperation::Add)], "Shift at the press is Add, not square");

    let events = stroke(&mut rig, (100.0, 100.0), (110.0, 120.0), NONE, Mods::ALT);
    assert_eq!(events, [commit((90.0, 80.0, 110.0, 120.0), SelectionOperation::New)], "Alt after the press centres");

    rig.tool_mut().tool().constraint(Constraint::Ratio(2.0, 1.0));
    let events = stroke(&mut rig, (10.0, 10.0), (50.0, 12.0), NONE, NONE);
    assert_eq!(events, [commit((10.0, 10.0, 50.0, 30.0), SelectionOperation::New)]);

    rig.tool_mut().tool().constraint(Constraint::Size(10.0, 10.0));
    let events = stroke(&mut rig, (20.0, 20.0), (40.4, 60.4), NONE, NONE);
    assert_eq!(events, [commit((40.0, 60.0, 50.0, 70.0), SelectionOperation::New)], "the box follows the pointer");
}

// Single Row and Single Column select the one-pixel line under the pointer across the whole document, follow the pointer
// until the release, and stay on the canvas past its edge; a press inside the selection draws the line.
#[trial]
fn marquee_single_row_and_column_follow_the_pointer() {
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::SingleRow), Fake::empty());
    down(&mut rig, (50.0, 40.7), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::marquee(Some(Rect::new(0.0, 40.0, 400.0, 41.0))), "a press selects");
    drag_to(&mut rig, (60.0, 90.2), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::marquee(Some(Rect::new(0.0, 90.0, 400.0, 91.0))));
    drag_to(&mut rig, (60.0, 500.0), NONE);
    up(&mut rig, (60.0, 500.0), NONE);
    assert_eq!(rig.tool_mut().events(), [commit((0.0, 299.0, 400.0, 300.0), SelectionOperation::New)]);

    let mut column = self::rig("Marquee", MarqueeTool::new(MarqueeKind::SingleColumn), Fake::empty());
    assert_eq!(stroke(&mut column, (10.5, 20.0), (450.0, 20.0), NONE, NONE), [commit((399.0, 0.0, 400.0, 300.0), SelectionOperation::New)]);

    let selection = Rect::new(100.0, 100.0, 200.0, 200.0);
    for (kind, line) in [(MarqueeKind::SingleRow, (0.0, 150.0, 400.0, 151.0)), (MarqueeKind::SingleColumn, (150.0, 0.0, 151.0, 300.0))] {
        let mut inside = self::rig("Marquee", MarqueeTool::new(kind), Fake::selected(selection));
        down(&mut inside, (150.0, 150.0), NONE);
        assert_eq!(inside.tool().phase(), SelectPhase::marquee(Some(Rect::new(line.0, line.1, line.2, line.3))), "{kind:?}");
        up(&mut inside, (150.0, 150.0), NONE);
        assert_eq!(inside.tool_mut().events(), [commit(line, SelectionOperation::New)], "{kind:?}");
    }
}

// A release without a drag reports `Deselect`; with Fixed Size the click commits the box at the pointer.
#[trial]
fn marquee_click_deselects_and_fixed_size_click_places() {
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), Fake::empty());
    click(&mut rig, (30.0, 30.0), NONE);
    assert_eq!(rig.tool_mut().events(), [SelectEvent::Deselect]);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);

    rig.tool_mut().tool().constraint(Constraint::Size(10.0, 10.0));
    click(&mut rig, (30.2, 30.2), NONE);
    assert_eq!(rig.tool_mut().events(), [commit((30.0, 30.0, 40.0, 40.0), SelectionOperation::New)]);
}

// A plain press inside the selection with the bar on New drags the outline; with Shift, Alt or a bar on Add it draws a
// marquee instead.
#[trial]
fn marquee_press_inside_selection_drags_the_outline_only_for_new() {
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    down(&mut rig, (150.0, 150.0), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::Dragging(DragKind::Outline));
    drag_to(&mut rig, (160.0, 155.0), NONE);
    up(&mut rig, (160.0, 155.0), NONE);
    assert_eq!(tags(&rig.tool_mut().events()), ["begin", "update 10,5", "commit 10,5"]);

    let events = stroke(&mut rig, (150.0, 150.0), (190.0, 190.0), Mods::SHIFT, Mods::SHIFT);
    assert_eq!(events, [commit((150.0, 150.0, 190.0, 190.0), SelectionOperation::Add)]);
    let events = stroke(&mut rig, (150.0, 150.0), (190.0, 190.0), Mods::ALT, Mods::ALT);
    assert_eq!(events, [commit((150.0, 150.0, 190.0, 190.0), SelectionOperation::Subtract)]);

    rig.tool_mut().tool().operation(SelectionOperation::Add);
    let events = stroke(&mut rig, (150.0, 150.0), (190.0, 190.0), NONE, NONE);
    assert_eq!(events, [commit((150.0, 150.0, 190.0, 190.0), SelectionOperation::Add)]);
}

// Cmd inside the selection drags pixels, Cmd+Alt a copy; Cmd outside it is the Move tool's, so the host starts no
// gesture.
#[trial]
fn marquee_cmd_press_cuts_and_cmd_alt_copies() {
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    for (mods, copy) in [(Mods::CMD, false), (Mods { cmd: true, alt: true, ..NONE }, true)] {
        down(&mut rig, (150.0, 150.0), mods);
        assert_eq!(rig.tool().phase(), SelectPhase::Dragging(DragKind::pixels(copy)));
        up(&mut rig, (150.0, 150.0), mods);
        rig.tool_mut().events();
    }
    rig.forget();
    down(&mut rig, (20.0, 20.0), Mods::CMD);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
    assert!(rig.gestures().is_empty(), "a Cmd press outside the selection starts no gesture: {:?}", rig.gestures());
}

// With a floating piece a plain press on it drags pixels; one in the margin the cut left draws.
#[trial]
fn marquee_floating_piece_moves_with_a_plain_drag() {
    let probe = Fake { piece: Some(Rect::new(300.0, 50.0, 360.0, 110.0)), ..Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)) };
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), probe);
    down(&mut rig, (330.0, 80.0), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::Dragging(DragKind::pixels(false)));
    up(&mut rig, (330.0, 80.0), NONE);
    rig.tool_mut().events();

    let events = stroke(&mut rig, (150.0, 150.0), (180.0, 180.0), NONE, NONE);
    assert_eq!(events, [commit((150.0, 150.0, 180.0, 180.0), SelectionOperation::New)], "the margin draws");
}

// A selection drag reports Begin, whole-pixel Updates (only when the rounded delta changes) and Commit; a cancel reports
// Cancel and nothing else; a release without travel commits a zero delta.
#[trial]
fn marquee_selection_drag_reports_whole_pixel_deltas_and_cancel() {
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    down(&mut rig, (150.0, 150.0), NONE);
    drag_to(&mut rig, (150.2, 150.2), NONE);
    drag_to(&mut rig, (150.6, 150.0), NONE);
    drag_to(&mut rig, (150.7, 150.0), NONE);
    up(&mut rig, (152.0, 150.0), NONE);
    assert_eq!(tags(&rig.tool_mut().events()), ["begin", "update 1,0", "commit 2,0"]);

    click(&mut rig, (150.0, 150.0), NONE);
    assert_eq!(tags(&rig.tool_mut().events()), ["begin", "commit 0,0"]);

    down(&mut rig, (150.0, 150.0), NONE);
    drag_to(&mut rig, (170.0, 150.0), NONE);
    cancel_with(&mut rig, CancelReason::Removed);
    let events = rig.tool_mut().events();
    assert_eq!(tags(&events), ["begin", "update 20,0", "cancel"]);
    assert!(matches!(events.last(), Some(SelectEvent::DragSelection(EditEvent::Cancel { reason: CancelReason::Removed, .. }))), "{events:?}");
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
}

// Space held during the drag moves the marquee without resizing it.
#[trial]
fn marquee_space_moves_the_marquee() {
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), Fake::empty());
    down(&mut rig, (10.0, 10.0), NONE);
    drag_to(&mut rig, (30.0, 30.0), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::marquee(Some(Rect::new(10.0, 10.0, 30.0, 30.0))));
    rig.fixture().frame(vec![input::key_press(Key::Space, Modifiers::NONE)]);
    drag_to(&mut rig, (60.0, 50.0), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::marquee(Some(Rect::new(40.0, 30.0, 60.0, 50.0))), "same size, moved by the pointer");
    up(&mut rig, (60.0, 50.0), NONE);
    assert_eq!(rig.tool_mut().events(), [commit((40.0, 30.0, 60.0, 50.0), SelectionOperation::New)]);
}

// While a marquee is dragged the tool is one accessible object named "Selection outline" valued with its pixel size.
#[trial]
fn marquee_being_dragged_is_one_accessible_object() {
    let mut rig = rig("Marquee", MarqueeTool::new(MarqueeKind::Rectangle), Fake::empty());
    assert_eq!(outline_value(&mut rig), None);
    down(&mut rig, (10.0, 10.0), NONE);
    drag_to(&mut rig, (40.0, 30.0), NONE);
    assert_eq!(outline_value(&mut rig).as_deref(), Some("30 px × 20 px"));
    up(&mut rig, (40.0, 30.0), NONE);
    assert_eq!(outline_value(&mut rig), None);
}

// ---- the Lasso ----

// A freehand drag commits one `Polygon` with the operation latched at the press (Shift is Add), and nothing more.
#[trial]
fn lasso_commits_a_closed_outline_with_the_latched_operation() {
    let mut rig = rig("Lasso", LassoTool::new(), Fake::empty());
    down(&mut rig, (50.0, 50.0), Mods::SHIFT);
    drag_to(&mut rig, (150.0, 50.0), NONE);
    drag_to(&mut rig, (150.0, 150.0), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(3));
    up(&mut rig, (150.0, 150.0), NONE);
    assert_eq!(rig.tool_mut().events(), [SelectEvent::commit(polygon(&[(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)]), SelectionOperation::Add)]);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
}

// Between Alt clicks the tool takes Backspace (retract) and Enter (close); with no outline open it takes neither.
#[trial]
fn lasso_takes_retract_and_enter_only_while_an_outline_is_open() {
    let mut rig = rig("Lasso", LassoTool::new(), Fake::empty());
    focus(&mut rig);
    rig.forget();
    press_key(&mut rig, Key::Backspace, Modifiers::NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle, "no outline: the key is the app's");
    // Alt with nothing selected starts straight segments.
    hold(&mut rig, Modifiers::ALT);
    for point in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        click(&mut rig, point, Mods::ALT);
    }
    assert_eq!(rig.tool().phase(), SelectPhase::outline(3));
    // Between Alt clicks the outline is open with the button up, so Alt is held through the retract.
    press_key(&mut rig, Key::Backspace, Modifiers::NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(2));
    click(&mut rig, (100.0, 150.0), Mods::ALT);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(3));
    press_key(&mut rig, Key::Enter, Modifiers::NONE);
    assert_eq!(rig.tool_mut().events(), [SelectEvent::commit(polygon(&[(50.0, 50.0), (150.0, 50.0), (100.0, 150.0)]), SelectionOperation::New)]);
}

// Releasing Alt with the button up closes the outline, and a double click closes it.
#[trial]
fn lasso_alt_release_and_double_click_close() {
    let mut rig = rig("Lasso", LassoTool::new(), Fake::empty());
    hold(&mut rig, Modifiers::ALT);
    for point in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        click(&mut rig, point, Mods::ALT);
    }
    assert!(rig.tool_mut().events().is_empty());
    hold(&mut rig, Modifiers::NONE);
    let expected = polygon(&[(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)]);
    assert_eq!(rig.tool_mut().events(), [SelectEvent::commit(expected, SelectionOperation::New)]);

    hold(&mut rig, Modifiers::ALT);
    for point in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        click(&mut rig, point, Mods::ALT);
    }
    rig.double_press_at(at((100.0, 150.0)));
    let events = rig.tool_mut().events();
    assert_eq!(events.len(), 1, "{events:?}");
    assert!(matches!(&events[0], SelectEvent::Commit { shape: SelectionShape::Polygon(points), .. } if points.len() >= 3), "{events:?}");
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
}

// A plain press inside the selection drags the outline instead of drawing.
#[trial]
fn lasso_press_inside_drags_the_selection() {
    let mut rig = rig("Lasso", LassoTool::new(), Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    down(&mut rig, (150.0, 150.0), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::Dragging(DragKind::Outline));
    drag_to(&mut rig, (160.0, 150.0), NONE);
    up(&mut rig, (160.0, 150.0), NONE);
    assert_eq!(tags(&rig.tool_mut().events()), ["begin", "update 10,0", "commit 10,0"]);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
}

// An open outline is dropped without an event by `reset` and by a change of the probe's document.
#[trial]
fn lasso_is_dropped_on_reset_or_document_change() {
    let mut rig = rig("Lasso", LassoTool::new(), Fake::empty());
    hold(&mut rig, Modifiers::ALT);
    click(&mut rig, (50.0, 50.0), Mods::ALT);
    click(&mut rig, (150.0, 50.0), Mods::ALT);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(2));
    rig.tool_mut().tool().reset();
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
    click(&mut rig, (50.0, 50.0), Mods::ALT);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(1));
    rig.tool_mut().probe.document = 2;
    rig.fixture().frame(Vec::new());
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
    assert!(rig.tool_mut().events().is_empty());
}

// While a freehand outline is open the tool is one accessible object with its point count; after the commit it has none.
#[trial]
fn lasso_outline_being_drawn_is_one_accessible_object() {
    let mut rig = rig("Lasso", LassoTool::new(), Fake::empty());
    assert_eq!(outline_value(&mut rig), None);
    down(&mut rig, (50.0, 50.0), NONE);
    drag_to(&mut rig, (150.0, 50.0), NONE);
    drag_to(&mut rig, (150.0, 150.0), NONE);
    assert_eq!(outline_value(&mut rig).as_deref(), Some("3 points"));
    up(&mut rig, (150.0, 150.0), NONE);
    assert_eq!(outline_value(&mut rig), None);
}

// A press and release that draw fewer than 3 points (a click, or a two-point drag) deselect.
#[trial]
fn lasso_click_deselects() {
    let mut rig = rig("Lasso", LassoTool::new(), Fake::empty());
    click(&mut rig, (50.0, 50.0), NONE);
    assert_eq!(rig.tool_mut().events(), [SelectEvent::Deselect]);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
    down(&mut rig, (50.0, 50.0), NONE);
    drag_to(&mut rig, (90.0, 60.0), NONE);
    up(&mut rig, (90.0, 60.0), NONE);
    assert_eq!(rig.tool_mut().events(), [SelectEvent::Deselect], "two points are no outline");
}

// ---- the Polygonal Lasso ----

// Shift turns a segment to 45 degree steps from the last vertex, not for the first vertex and not with Alt.
#[trial]
fn polygon_shift_constrains_to_45_degrees() {
    let mut rig = rig("Polygonal Lasso", PolygonLassoTool::new(), Fake::empty());
    focus(&mut rig);
    hold(&mut rig, Modifiers::SHIFT);
    click(&mut rig, (50.0, 50.0), Mods::SHIFT);
    // 11 degrees from the last vertex rounds to 0, keeping the distance.
    click(&mut rig, (150.0, 70.0), Mods::SHIFT);
    hold(&mut rig, Modifiers::SHIFT | Modifiers::ALT);
    click(&mut rig, (140.0, 170.0), Mods { shift: true, alt: true, ..NONE });
    hold(&mut rig, Modifiers::NONE);
    press_key(&mut rig, Key::Enter, Modifiers::NONE);
    let events = rig.tool_mut().events();
    let [SelectEvent::Commit { shape: SelectionShape::Polygon(points), operation, .. }] = events.as_slice() else { panic!("one commit: {events:?}") };
    assert_eq!(*operation, SelectionOperation::Add);
    let turned = Point::new(50.0 + 100.0_f64.hypot(20.0), 50.0);
    assert_eq!(points.len(), 3);
    assert_eq!(points[0], Point::new(50.0, 50.0), "the first vertex is never turned");
    assert!((points[1] - turned).hypot() < 1e-9, "{:?}", points[1]);
    assert_eq!(points[2], Point::new(140.0, 170.0), "Shift with Alt keeps the point");
}

// A plain press inside the selection places a vertex; only Cmd drags it, and not while an outline is open.
#[trial]
fn polygon_click_driven_needs_cmd_to_drag() {
    let mut rig = rig("Polygonal Lasso", PolygonLassoTool::new(), Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    click(&mut rig, (150.0, 150.0), NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(1));
    click(&mut rig, (180.0, 120.0), Mods::CMD);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(2), "Cmd with an outline open places a vertex too");
    assert!(rig.tool_mut().events().is_empty());
    rig.tool_mut().tool().reset();
    down(&mut rig, (150.0, 150.0), Mods::CMD);
    assert_eq!(rig.tool().phase(), SelectPhase::Dragging(DragKind::pixels(false)));
    drag_to(&mut rig, (155.0, 150.0), Mods::CMD);
    up(&mut rig, (155.0, 150.0), Mods::CMD);
    assert_eq!(tags(&rig.tool_mut().events()), ["begin", "update 5,0", "commit 5,0"]);
    rig.forget();
    click(&mut rig, (300.0, 250.0), Mods::CMD);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle, "Cmd outside the selection is the Move tool's");
    assert!(rig.gestures().is_empty(), "{:?}", rig.gestures());
}

// A secondary click while an outline is open, Backspace and Delete remove the last vertex; the last one cancels with no
// event. With no outline open the secondary button starts nothing.
#[trial]
fn polygon_secondary_click_removes_the_last_vertex_then_cancels() {
    let mut rig = rig("Polygonal Lasso", PolygonLassoTool::new(), Fake::empty());
    focus(&mut rig);
    let secondary = |rig: &mut Rig<PolygonLassoTool>| {
        inject(rig, ToolInput::down(PointerButton::Secondary, at((10.0, 10.0))));
        up(rig, (10.0, 10.0), NONE);
    };
    secondary(&mut rig);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle, "nothing open");
    for point in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        click(&mut rig, point, NONE);
    }
    secondary(&mut rig);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(2));
    press_key(&mut rig, Key::Backspace, Modifiers::NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(1));
    secondary(&mut rig);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
    assert!(rig.tool_mut().events().is_empty());
}

// A new outline replaces the selection while it is open: true only for the operation New.
#[trial]
fn polygon_new_outline_replaces_the_selection() {
    let mut rig = rig("Polygonal Lasso", PolygonLassoTool::new(), Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    assert!(!rig.tool_mut().tool().replaces_selection());
    click(&mut rig, (20.0, 20.0), NONE);
    assert!(rig.tool_mut().tool().replaces_selection());
    rig.tool_mut().tool().reset();
    assert!(!rig.tool_mut().tool().replaces_selection());
    click(&mut rig, (20.0, 20.0), Mods::SHIFT);
    assert_eq!(rig.tool().phase(), SelectPhase::outline(1));
    assert!(!rig.tool_mut().tool().replaces_selection(), "Add keeps the selection visible");
}

// ---- the Magnetic Lasso ----

const CENTRE: Point = Point::new(200.0, 150.0);
const RADIUS: f64 = 80.0;

/// What the fake tracer was asked.
#[derive(Debug, Default)]
struct Log {
    snaps: Cell<usize>,
    /// The `from` of every `trace`: the fastening points, with repeats.
    froms: RefCell<Vec<Point>>,
    searches: RefCell<Vec<EdgeSearch>>,
}

impl Log {
    /// The distinct points traces started from.
    fn starts(&self) -> usize {
        let mut seen: Vec<Point> = Vec::new();
        for &p in self.froms.borrow().iter() {
            if !seen.iter().any(|q| q.distance(p) < 1e-6) {
                seen.push(p);
            }
        }
        seen.len()
    }

    fn clear(&self) {
        self.snaps.set(0);
        self.froms.borrow_mut().clear();
        self.searches.borrow_mut().clear();
    }

    fn last_width(&self) -> f64 {
        self.searches.borrow().last().map_or(f64::NAN, |search| search.width)
    }
}

/// Edges lie on a circle of radius 80 round (200, 150): `snap` finds the nearest circle point within the width, `trace`
/// walks the shorter arc.
#[derive(Clone, Debug)]
struct Circle {
    log: Rc<Log>,
}

fn on_circle(p: Point) -> bool {
    ((p - CENTRE).hypot() - RADIUS).abs() < 0.5
}

impl EdgeTracer for Circle {
    fn snap(&mut self, at: Point, search: EdgeSearch) -> Option<Point> {
        self.log.snaps.set(self.log.snaps.get() + 1);
        self.log.searches.borrow_mut().push(search);
        let d = at - CENTRE;
        let len = d.hypot();
        ((len - RADIUS).abs() <= search.width && len > 0.0).then(|| CENTRE + d * (RADIUS / len))
    }

    fn trace(&mut self, from: Point, to: Point, _guide: &[Point], search: EdgeSearch) -> Vec<Point> {
        self.log.froms.borrow_mut().push(from);
        self.log.searches.borrow_mut().push(search);
        if !on_circle(from) || !on_circle(to) {
            return vec![from, to];
        }
        let angle = |p: Point| (p.y - CENTRE.y).atan2(p.x - CENTRE.x);
        let mut delta = angle(to) - angle(from);
        if delta > std::f64::consts::PI {
            delta -= std::f64::consts::TAU;
        } else if delta < -std::f64::consts::PI {
            delta += std::f64::consts::TAU;
        }
        let n = ((delta.abs() * RADIUS / 2.0).ceil() as usize).max(1);
        let mut points: Vec<Point> = (1..n).map(|k| CENTRE + circle_offset(angle(from) + delta * k as f64 / n as f64)).collect();
        points.insert(0, from);
        points.push(to);
        points
    }
}

fn circle_offset(angle: f64) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin()) * RADIUS
}

fn on(angle: f64) -> Point {
    CENTRE + circle_offset(angle)
}

fn magnetic() -> (Rig<MagneticLassoTool<Circle>>, Rc<Log>) {
    let log = Rc::new(Log::default());
    let mut rig = rig("Magnetic Lasso", MagneticLassoTool::new(Circle { log: Rc::clone(&log) }), Fake::empty());
    focus(&mut rig);
    log.clear();
    (rig, log)
}

fn pt(point: Point) -> (f64, f64) {
    (point.x, point.y)
}

/// A hover to `point`.
fn glide(rig: &mut Rig<MagneticLassoTool<Circle>>, point: Point, mods: Mods) {
    // The real pointer goes with the injected one, so the hover each pass reports is this point and not where the pointer was left.
    let (ctx, id) = (rig.fixture().ctx().clone(), rig.canvas_id());
    ToolHost::inject(&ctx, id, ToolInput::moved(at(pt(point))).mods(mods));
    rig.fixture().frame(vec![pointer::moved(at(pt(point)))]);
}

/// The pointer moves along the circle from angle `from` to angle `to` in small samples, as a hand follows an edge.
fn glide_along(rig: &mut Rig<MagneticLassoTool<Circle>>, from: f64, to: f64) {
    let samples = ((to - from).abs() / 0.05).ceil() as usize;
    for k in 1..=samples {
        glide(rig, on(from + (to - from) * k as f64 / samples as f64), NONE);
    }
}

fn commit_points(events: &[SelectEvent]) -> (Vec<Point>, SelectionOperation) {
    let [SelectEvent::Commit { shape: SelectionShape::Polygon(points), operation, .. }] = events else { panic!("one polygon commit: {events:?}") };
    (points.clone(), *operation)
}

// Points fasten every `spacing` along the edge at frequencies 0, 57 and 100, and a long jump is walked in at most 256
// steps of `spacing / 2`.
#[trial]
fn magnetic_fastens_every_spacing_and_walks_jumps() {
    let start = on(0.0);
    let fastened = |frequency: f32| {
        let (mut rig, log) = magnetic();
        rig.tool_mut().tool().settings_mut().frequency = frequency;
        click(&mut rig, pt(start), NONE);
        // The pointer follows 2.5 rad (200 px of arc) of the circle.
        glide_along(&mut rig, 0.0, 2.5);
        log.starts()
    };
    let (slow, normal, fast) = (fastened(0.0), fastened(57.0), fastened(100.0));
    assert!(slow < normal && normal < fast, "{slow} {normal} {fast}");
    // 200 px at 47 px: the start and four fastened points.
    assert!((4..=6).contains(&normal), "{normal}");
    let (mut rig, log) = magnetic();
    rig.tool_mut().tool().settings_mut().frequency = 100.0;
    click(&mut rig, pt(start), NONE);
    glide(&mut rig, Point::new(5000.0, 150.0), NONE);
    assert_eq!(log.snaps.get(), 1 + 256, "the start, then 256 steps");
}

// Alt-click adds a straight segment to the pointer; an Alt drag draws freehand and fastens where it stops.
#[trial]
fn magnetic_alt_click_is_straight_and_alt_drag_is_freehand() {
    let (mut rig, _) = magnetic();
    let start = on(0.0);
    click(&mut rig, pt(start), NONE);
    click(&mut rig, (250.0, 60.0), Mods::ALT);
    down(&mut rig, (220.0, 60.0), Mods::ALT);
    for point in [(210.0, 62.0), (210.1, 62.1), (200.0, 66.0)] {
        drag_to(&mut rig, point, Mods::ALT);
    }
    up(&mut rig, (200.0, 66.0), Mods::ALT);
    close_with(&mut rig, Modifiers::ALT);
    let (points, operation) = commit_points(&rig.tool_mut().events());
    assert_eq!(operation, SelectionOperation::New);
    let expected = [start, Point::new(250.0, 60.0), Point::new(220.0, 60.0), Point::new(210.0, 62.0), Point::new(200.0, 66.0)];
    assert_eq!(points, expected, "0.5 px spacing drops the near sample");

    // Alt pressed during a drag anchors the pointer and draws freehand from it.
    let (mut rig, _) = magnetic();
    down(&mut rig, pt(start), NONE);
    drag_to(&mut rig, pt(on(0.5)), NONE);
    drag_to(&mut rig, (270.0, 100.0), NONE);
    hold(&mut rig, Modifiers::ALT);
    drag_to(&mut rig, (260.0, 90.0), Mods::ALT);
    up(&mut rig, (260.0, 90.0), Mods::ALT);
    close_with(&mut rig, Modifiers::ALT);
    let (points, _) = commit_points(&rig.tool_mut().events());
    assert_eq!(points.first(), Some(&start));
    assert_eq!(points[points.len() - 2..], [Point::new(270.0, 100.0), Point::new(260.0, 90.0)]);
}

/// Presses Enter with `held`.
fn close_with(rig: &mut Rig<MagneticLassoTool<Circle>>, held: Modifiers) {
    hold(rig, held);
    rig.fixture().key(Key::Enter, held);
    hold(rig, Modifiers::NONE);
}

// Backspace removes fastening points back to the first; one more cancels with no event.
#[trial]
fn magnetic_removes_the_last_point_and_cancels() {
    let (mut rig, log) = magnetic();
    let start = on(0.0);
    click(&mut rig, pt(start), NONE);
    click(&mut rig, pt(on(1.0)), NONE);
    click(&mut rig, pt(on(2.0)), NONE);
    let last_start = || log.froms.borrow().last().copied();
    // Each Backspace traces from the fastening point before: the pointer stays where it was.
    press_key(&mut rig, Key::Backspace, Modifiers::NONE);
    // The press went through an f32 screen position: 1e-6 is below its rounding at this size.
    assert!(last_start().is_some_and(|p| p.distance(on(1.0)) < 1e-4));
    assert!(matches!(rig.tool().phase(), SelectPhase::Outline { .. }));
    press_key(&mut rig, Key::Backspace, Modifiers::NONE);
    assert!(last_start().is_some_and(|p| p.distance(start) < 1e-6), "back to the first point");
    assert!(matches!(rig.tool().phase(), SelectPhase::Outline { .. }), "the first point stays");
    press_key(&mut rig, Key::Backspace, Modifiers::NONE);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
    assert!(rig.tool_mut().events().is_empty());
}

// `[` and `]` change the width by 1 within 1..=256 while an outline is open.
#[trial]
fn magnetic_bracket_keys_change_the_width_within_range() {
    let (mut rig, _) = magnetic();
    click(&mut rig, pt(on(0.0)), NONE);
    press_key(&mut rig, Key::OpenBracket, Modifiers::NONE);
    assert_eq!(rig.tool_mut().tool().settings().width, 9.0);
    press_key(&mut rig, Key::CloseBracket, Modifiers::NONE);
    press_key(&mut rig, Key::CloseBracket, Modifiers::NONE);
    assert_eq!(rig.tool_mut().tool().settings().width, 11.0);
    rig.tool_mut().tool().settings_mut().width = 1.0;
    press_key(&mut rig, Key::OpenBracket, Modifiers::NONE);
    assert_eq!(rig.tool_mut().tool().settings().width, 1.0);
    rig.tool_mut().tool().settings_mut().width = 256.0;
    press_key(&mut rig, Key::CloseBracket, Modifiers::NONE);
    assert_eq!(rig.tool_mut().tool().settings().width, 256.0);
}

// Enter closes along the edges (the tracer's path), Alt+Enter with a straight segment, a click on the first point closes
// too; each commits one `Polygon` with the latched operation.
#[trial]
fn magnetic_closes_along_edges_or_straight() {
    let start = on(0.0);
    let (mut rig, _) = magnetic();
    rig.tool_mut().tool().operation(SelectionOperation::Add);
    click(&mut rig, pt(start), NONE);
    glide_along(&mut rig, 0.0, 1.5);
    click(&mut rig, pt(on(1.5)), NONE);
    close_with(&mut rig, Modifiers::NONE);
    let (points, operation) = commit_points(&rig.tool_mut().events());
    assert_eq!(operation, SelectionOperation::Add);
    assert!(points.len() > 6 && points.iter().all(|&p| on_circle(p)), "{points:?}");
    assert_ne!(points.last(), points.first(), "the outline does not repeat its start");
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);

    let (mut rig, _) = magnetic();
    click(&mut rig, pt(start), NONE);
    click(&mut rig, (250.0, 60.0), Mods::ALT);
    click(&mut rig, (150.0, 60.0), Mods::ALT);
    close_with(&mut rig, Modifiers::ALT);
    assert_eq!(commit_points(&rig.tool_mut().events()).0, [start, Point::new(250.0, 60.0), Point::new(150.0, 60.0)]);

    let (mut rig, _) = magnetic();
    click(&mut rig, pt(start), NONE);
    glide_along(&mut rig, 0.0, 1.5);
    click(&mut rig, pt(on(1.5)), NONE);
    glide_along(&mut rig, 1.5, 3.0);
    click(&mut rig, pt(on(3.0)), NONE);
    assert!(rig.tool_mut().events().is_empty());
    click(&mut rig, pt(start + Vec2::new(1.0, 1.0)), NONE);
    let (points, _) = commit_points(&rig.tool_mut().events());
    assert!(points.len() > 6);
    assert_eq!(rig.tool().phase(), SelectPhase::Idle);
}

// With `pressure_width` on, a pen sample at pressure 0.5 halves the searched width (minus one); a mouse sample never
// narrows it.
#[trial]
fn magnetic_stylus_pressure_narrows_the_width() {
    let (mut rig, log) = magnetic();
    rig.tool_mut().tool().settings_mut().pressure_width = true;
    click(&mut rig, pt(on(0.0)), NONE);
    glide(&mut rig, on(0.2), NONE);
    assert_eq!(log.last_width(), 10.0, "a mouse");
    let mut pen = PointerSample::new(0.5);
    pen.tilt = [10.0, 0.0];
    inject(&mut rig, ToolInput::moved(at(pt(on(0.4)))).sample(pen));
    assert_eq!(log.last_width(), 5.5, "1 + 9 × 0.5");
    rig.tool_mut().tool().settings_mut().pressure_width = false;
    inject(&mut rig, ToolInput::moved(at(pt(on(0.6)))).sample(pen));
    assert_eq!(log.last_width(), 10.0, "the option is off");
}

// While the outline is open the tool is one accessible object with its point count (fastening points and the live trace).
#[trial]
fn magnetic_outline_being_drawn_is_one_accessible_object() {
    let (mut rig, _) = magnetic();
    assert_eq!(outline_value(&mut rig), None);
    click(&mut rig, pt(on(0.0)), NONE);
    glide(&mut rig, on(0.3), NONE);
    let SelectPhase::Outline { points, .. } = rig.tool().phase() else { panic!("open") };
    assert_eq!(outline_value(&mut rig), Some(format!("{points} points")));
    rig.tool_mut().tool().reset();
    rig.fixture().frame(Vec::new());
    assert_eq!(outline_value(&mut rig), None);
}
