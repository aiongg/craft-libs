//! A canvas under a modal dialog (15.5, 8.3): an armed sampler lets primary presses through, the view stays usable,
//! and everything else stays blocked. The dialog's body arms the canvas the way a dialog does, through `BodyCx`.

use super::kit::{Leaf, Opened, Opener, PlainStyle, Seen};
use crate::body::kit::Slab;
use crate::contract::kit::{Probe, Seen as ProbeSeen};
use crate::windows_kit::Windows;
use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CanvasView, Sense, WheelTaker};
use craft_ui::canvas::{Canvas, Momentary, NavigationOptions, NoTool, ToolHost};
use craft_ui::contract::{
    Common, Component, ContainerProps, Cx, MeasureCx, Mount, Overlay, OverlayUi, Overlays as _, Props as _, Role, Sampler as _, Show as _,
    ShowContainer as _, Sizing, Trigger,
};
use craft_ui::style::StyleCx;
use craft_ui::testing::focus::{is_focused, key_release};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{middle, moved, press, release, secondary, wheel};
use craft_ui::testing::wheel::{focus, register, take};
use craft_ui::testing::{Frames, trial};
use egui::{Event, Id, Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};

const OPENER: Rect = Rect { min: Pos2::new(600.0, 100.0), max: Pos2::new(680.0, 120.0) };
const CANVAS: Rect = Rect { min: Pos2::new(20.0, 20.0), max: Pos2::new(320.0, 260.0) };
/// A canvas under the whole centred dialog.
const WIDE: Rect = Rect { min: Pos2::new(0.0, 0.0), max: Pos2::new(1200.0, 900.0) };

/// What the dialog's body asks of the canvas.
#[derive(Clone, Copy, PartialEq)]
enum Arm {
    Nothing,
    View,
    Sampler,
}

/// A button that opens a modal dialog whose body arms the canvas, holds a focusable leaf, a menu and a nested modal.
#[derive(Component)]
#[component(overlay)]
struct Dialog<'a> {
    #[common]
    common: Common,
    scene: &'a Scene,
    canvas: Id,
}

impl Component for Dialog<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: DialogStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(80.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("popup");
        popup.trigger(Trigger::Manual, cx);
        if self.scene.open.get() {
            popup.open(cx);
        }
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("popup");
        let scene = self.scene;
        let canvas = self.canvas;
        let shown = ui.show(popup, Overlay::modal(), |ui, body| {
            match scene.arm.get() {
                Arm::Nothing => {}
                Arm::View => body.keep_view_open(canvas),
                Arm::Sampler => body.arm_sampler(canvas),
            }
            let leaf = Leaf::new(&scene.leaf, Vec2::new(80.0, 30.0)).id_salt("armer").show(ui);
            scene.armer_at.set(leaf.rect);
            scene.armer_id.set(leaf.id);
            Opener::new(&scene.menu_shown, &scene.menu_content, Trigger::Manual, Overlay::menu).open(scene.menu.get()).id_salt("menu").show(ui);
            Opener::new(&scene.nested_shown, &scene.nested_content, Trigger::Manual, Overlay::modal)
                .open(scene.nested.get())
                .id_salt("nested")
                .show(ui);
        });
        if shown.is_some() {
            self.scene.dialog.shown.set(true);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The scene: a canvas probe, a second root behind it and the dialog's opener.
struct Scene {
    area: Rect,
    canvas: RefCell<ProbeSeen>,
    behind: RefCell<ProbeSeen>,
    leaf: RefCell<Seen>,
    wheels: Cell<u32>,
    armer_at: Cell<Rect>,
    armer_id: Cell<Id>,
    open: Cell<bool>,
    arm: Cell<Arm>,
    menu: Cell<bool>,
    nested: Cell<bool>,
    dialog: Opened,
    menu_shown: Opened,
    menu_content: RefCell<Seen>,
    nested_shown: Opened,
    nested_content: RefCell<Seen>,
}

impl Scene {
    fn new(arm: Arm) -> Self {
        Self::over(arm, CANVAS)
    }

    fn over(arm: Arm, area: Rect) -> Self {
        Scene {
            area,
            canvas: RefCell::default(),
            behind: RefCell::default(),
            leaf: RefCell::default(),
            wheels: Cell::new(0),
            armer_at: Cell::new(Rect::NOTHING),
            armer_id: Cell::new(Id::NULL),
            open: Cell::new(false),
            arm: Cell::new(arm),
            menu: Cell::new(false),
            nested: Cell::new(false),
            dialog: Opened::default(),
            menu_shown: Opened::default(),
            menu_content: RefCell::default(),
            nested_shown: Opened::default(),
            nested_content: RefCell::default(),
        }
    }

    fn show(&self, ui: &mut Ui) {
        self.dialog.shown.set(false);
        self.menu_shown.shown.set(false);
        self.nested_shown.shown.set(false);
        let canvas = self.canvas.borrow().id.unwrap_or(Id::NULL);
        Probe::new(&self.behind).size(self.area.size()).sense(Sense::CLICK | Sense::DRAG).id_salt("behind").show_at(ui, self.area);
        Probe::new(&self.canvas).size(self.area.size()).sense(Sense::CLICK | Sense::DRAG).id_salt("canvas").show_at(ui, self.area);
        register(ui.ctx(), WheelTaker::new(canvas, self.area).vertical(true));
        if take(ui.ctx(), canvas).is_some() {
            self.wheels.set(self.wheels.get() + 1);
        }
        Dialog { common: Common::new(), scene: self, canvas }.id_salt("dialog").show_at(ui, OPENER);
    }

    /// A frame with `events`; the scene's flags are read as they stand.
    fn frame(&self, frames: &mut Frames, events: Vec<Event>) {
        frames.frame(events, &mut |ui: &mut Ui| self.show(ui));
    }

    fn settle(&self, frames: &mut Frames, count: usize) {
        frames.idle(count, &mut |ui: &mut Ui| self.show(ui));
    }

    /// Opens the dialog and settles it.
    fn open_dialog(&self, frames: &mut Frames) {
        self.settle(frames, 3);
        self.open.set(true);
        self.frame(frames, vec![]);
        self.open.set(false);
        self.settle(frames, 30);
        assert!(self.dialog.shown.get(), "the dialog shows");
    }

    fn forget_canvas(&self) {
        self.canvas.borrow_mut().press.clear();
        self.behind.borrow_mut().press.clear();
        self.leaf.borrow_mut().presses = 0;
        self.leaf.borrow_mut().activations = 0;
    }

    fn pressed_canvas(&self) -> bool {
        !self.canvas.borrow().press.is_empty()
    }

    fn pressed_behind(&self) -> bool {
        !self.behind.borrow().press.is_empty()
    }

    fn dragged_canvas(&self) -> bool {
        self.canvas.borrow().press.iter().any(|phase| phase.is_drag())
    }

    /// A primary press at `at`, a move by `by` and the release.
    fn drag(&self, frames: &mut Frames, at: Pos2, by: Vec2) {
        self.frame(frames, vec![moved(at)]);
        self.frame(frames, vec![press(at)]);
        self.frame(frames, vec![moved(at + by / 2.0)]);
        self.frame(frames, vec![moved(at + by)]);
        self.frame(frames, vec![release(at + by)]);
        self.settle(frames, 2);
    }

    /// A click at `at`.
    fn click(&self, frames: &mut Frames, at: Pos2) {
        self.frame(frames, vec![moved(at)]);
        self.frame(frames, vec![press(at)]);
        self.frame(frames, vec![release(at)]);
        self.settle(frames, 2);
    }
}

fn space(pressed: bool) -> Event {
    if pressed { key_press(Key::Space, Modifiers::NONE) } else { key_release(Key::Space, Modifiers::NONE) }
}

/// A press and drag reach the canvas under a dialog only while the dialog arms it; with no arm the modal blocks the canvas.
#[trial]
fn an_armed_sampler_takes_the_press_and_an_unarmed_one_gets_none() {
    for (arm, armed) in [(Arm::Sampler, true), (Arm::Nothing, false)] {
        let mut frames = Frames::new();
        let scene = Scene::new(arm);
        scene.open_dialog(&mut frames);
        scene.drag(&mut frames, CANVAS.center(), Vec2::new(40.0, 10.0));
        assert_eq!(scene.pressed_canvas(), armed, "the canvas gets the press only when the dialog arms it");
        assert_eq!(scene.dragged_canvas(), armed, "and keeps the drag");
        assert!(scene.dialog.shown.get(), "a press on the canvas does not close the dialog");
    }
}

/// A press that began on the dialog stays the dialog's when it moves over the canvas.
#[trial]
fn a_press_that_began_on_the_dialog_never_reaches_the_canvas() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::Sampler);
    scene.open_dialog(&mut frames);
    let from = scene.armer_at.get().center();
    let to = CANVAS.center();
    scene.drag(&mut frames, from, to - from);
    assert!(!scene.pressed_canvas(), "the dialog's press never becomes the canvas's");
}

/// A popup that absorbs outside presses (a menu) over the dialog takes the press on the canvas as its outside press.
#[trial]
fn a_press_on_the_canvas_closes_a_popup_over_the_dialog_and_does_not_reach_the_canvas() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::Sampler);
    scene.open_dialog(&mut frames);
    scene.menu.set(true);
    scene.settle(&mut frames, 30);
    assert!(scene.menu_shown.shown.get(), "the menu is open over the dialog");
    scene.menu.set(false);
    scene.drag(&mut frames, CANVAS.center(), Vec2::new(40.0, 10.0));
    assert!(!scene.pressed_canvas(), "the popup took the press");
    assert!(!scene.menu_shown.shown.get(), "the press closed the popup");
    assert!(scene.dialog.shown.get(), "the dialog stays open");
}

/// The button filter: a secondary press on the armed canvas is blocked, a middle press under a view-only reach passes.
#[trial]
fn the_canvas_filters_presses_by_button() {
    // Only the arm's own button passes: a secondary press is blocked under Sample, and a middle press (the pan) reaches
    // the canvas under a view-only reach.
    for (arm, secondary_button, passes) in [(Arm::Sampler, true, false), (Arm::View, false, true)] {
        let mut frames = Frames::new();
        let scene = Scene::new(arm);
        scene.open_dialog(&mut frames);
        let at = CANVAS.center();
        let button = |down| if secondary_button { secondary(at, down) } else { middle(at, down) };
        scene.frame(&mut frames, vec![moved(at)]);
        scene.frame(&mut frames, vec![button(true)]);
        scene.frame(&mut frames, vec![button(false)]);
        scene.settle(&mut frames, 2);
        assert_eq!(scene.pressed_canvas(), passes);
    }
}

/// A dialog in its own viewport: a press on the armed canvas in the main viewport reaches it, and the main viewport's
/// key focus stays where it was.
#[trial]
fn a_dialog_in_its_own_viewport_lets_an_armed_canvas_in_the_main_viewport_take_the_press() {
    let windows = Windows::new();
    let canvas = RefCell::new(ProbeSeen::default());
    let field = RefCell::new(Seen::default());
    let frame = |root: Vec<Event>| {
        windows.frame(root, vec![], |ui| {
            let id = canvas.borrow().id.unwrap_or(Id::NULL);
            Probe::new(&canvas).size(CANVAS.size()).sense(Sense::CLICK | Sense::DRAG).id_salt("canvas").show_at(ui, CANVAS);
            Leaf::new(&field, Vec2::new(80.0, 30.0))
                .id_salt("field")
                .show_at(ui, Rect::from_min_size(Pos2::new(400.0, 300.0), Vec2::new(80.0, 30.0)));
            let props = ContainerProps::new().mount(Mount::viewport(Overlay::modal())).sizing(Sizing::Content);
            Slab::new(props).label("Sample").id_salt("dialog").show(ui, |ui, body| {
                body.arm_sampler(id);
                Leaf::new(&RefCell::new(Seen::default()), Vec2::new(80.0, 30.0)).id_salt("armer").show(ui);
            });
        });
    };
    for _ in 0..5 {
        frame(vec![]);
    }
    let held = field.borrow().id.expect("the field shows");
    focus(&windows.ctx, held);
    frame(vec![]);
    let at = CANVAS.center();
    frame(vec![moved(at)]);
    frame(vec![press(at)]);
    frame(vec![moved(at + Vec2::new(30.0, 0.0))]);
    frame(vec![release(at + Vec2::new(30.0, 0.0))]);
    frame(vec![]);
    let seen = canvas.borrow();
    assert!(!seen.press.is_empty(), "the press reached the canvas in the main viewport");
    assert!(seen.press.iter().any(|phase| phase.is_drag()), "and the drag stayed its own");
    assert!(is_focused(&windows.ctx, held), "the press moved no key focus");
}

/// A dialog over the canvas keeps its own presses, under either reach: the press on its button is the dialog's, the
/// canvas's own corner still reaches the canvas under Sample, and a second root behind the canvas never gets one.
#[trial]
fn a_dialog_over_the_canvas_keeps_its_presses_and_a_root_behind_the_canvas_gets_none() {
    for arm in [Arm::View, Arm::Sampler] {
        let mut frames = Frames::new();
        let scene = Scene::over(arm, WIDE);
        scene.open_dialog(&mut frames);
        let button = scene.armer_at.get().center();
        assert!(WIDE.contains(button), "the dialog lies over the canvas");
        scene.click(&mut frames, button);
        assert!(scene.leaf.borrow().presses > 0, "the dialog's own button got the press");
        assert!(!scene.pressed_canvas(), "the canvas did not");
        scene.forget_canvas();
        let corner = Pos2::new(30.0, 30.0);
        scene.drag(&mut frames, corner, Vec2::new(20.0, 5.0));
        assert_eq!(scene.pressed_canvas(), arm == Arm::Sampler, "the canvas takes the press away from the dialog only when armed");
        assert!(!scene.pressed_behind(), "a root behind the canvas never gets the press");
        assert!(is_focused(&frames.ctx, scene.armer_id.get()), "the dialog keeps key focus");
    }
}

/// A modal opened over an armed dialog blocks the canvas again.
#[trial]
fn a_nested_modal_over_an_armed_dialog_blocks_the_canvas() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::Sampler);
    scene.open_dialog(&mut frames);
    scene.nested.set(true);
    scene.settle(&mut frames, 30);
    assert!(scene.nested_shown.shown.get(), "the nested modal is open over the dialog");
    scene.drag(&mut frames, CANVAS.center(), Vec2::new(40.0, 10.0));
    assert!(!scene.pressed_canvas(), "the nested modal blocks the canvas");
}

/// Space held over the canvas turns the primary press into a pan press, under either reach, and the dialog's focused
/// button never gets the Space.
#[trial]
fn space_drag_over_the_canvas_reaches_it_and_does_not_press_the_dialogs_button() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::View);
    scene.open_dialog(&mut frames);
    scene.forget_canvas();
    let at = CANVAS.center();
    scene.frame(&mut frames, vec![moved(at)]);
    scene.frame(&mut frames, vec![space(true)]);
    scene.frame(&mut frames, vec![press(at)]);
    scene.frame(&mut frames, vec![moved(at + Vec2::new(30.0, 0.0))]);
    scene.frame(&mut frames, vec![moved(at + Vec2::new(60.0, 0.0))]);
    scene.frame(&mut frames, vec![release(at + Vec2::new(60.0, 0.0))]);
    scene.frame(&mut frames, vec![space(false)]);
    scene.settle(&mut frames, 2);
    assert!(scene.dragged_canvas(), "the Space drag is the canvas's");
    assert_eq!(scene.leaf.borrow().activations, 0, "the dialog's focused button did not take the Space");
    assert!(is_focused(&frames.ctx, scene.armer_id.get()), "the dialog keeps key focus");
}

/// Space held over a canvas under an armed dialog is the Hand: the pointer drag pans the view. The canvas is a real
/// `Canvas` under a `ToolHost` with a `Momentary`, as an app shows it, and the dialog holds the key focus.
#[trial]
fn space_drag_over_the_canvas_under_a_dialog_pans_the_view() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::View);
    let view = Cell::new(None::<CanvasView>);
    let canvas = Cell::new(Id::NULL);
    let momentary = RefCell::new(Momentary::new(NavigationOptions::default()));
    let show = |frames: &mut Frames, events: Vec<Event>| {
        frames.frame(events, &mut |ui: &mut Ui| {
            scene.dialog.shown.set(false);
            let mut current = view.get().unwrap_or_else(|| CanvasView::new(CANVAS, Point::new(500.0, 400.0), 1.0));
            current.rect = CANVAS;
            let shown = Canvas::new("Document").view(current).document(Size::new(1000.0, 800.0)).show_at(ui, CANVAS, |_, _| {});
            canvas.set(shown.output.response.id);
            let reply = ToolHost::new().momentary(&mut momentary.borrow_mut()).run(ui, &shown.output, &mut NoTool);
            view.set(Some(reply.view.unwrap_or(current)));
            Dialog { common: Common::new(), scene: &scene, canvas: canvas.get() }.id_salt("dialog").show_at(ui, OPENER);
        });
    };
    for _ in 0..3 {
        show(&mut frames, vec![]);
    }
    scene.open.set(true);
    show(&mut frames, vec![]);
    scene.open.set(false);
    for _ in 0..30 {
        show(&mut frames, vec![]);
    }
    assert!(scene.dialog.shown.get(), "the dialog shows");
    let at = CANVAS.center();
    let start = view.get().map(|view| view.center);
    show(&mut frames, vec![moved(at)]);
    show(&mut frames, vec![space(true)]);
    show(&mut frames, vec![press(at)]);
    show(&mut frames, vec![moved(at + Vec2::new(30.0, 0.0))]);
    show(&mut frames, vec![moved(at + Vec2::new(60.0, 0.0))]);
    show(&mut frames, vec![release(at + Vec2::new(60.0, 0.0))]);
    show(&mut frames, vec![space(false)]);
    show(&mut frames, vec![]);
    assert_ne!(view.get().map(|view| view.center), start, "the Hand pans the view under the dialog");
}

/// The wheel over a canvas under a dialog is blocked without a registration and passes with `keep_view_open`.
#[trial]
fn the_wheel_over_the_canvas_passes_a_dialog_only_with_a_registration() {
    for (arm, passes) in [(Arm::Nothing, false), (Arm::View, true)] {
        let mut frames = Frames::new();
        let scene = Scene::new(arm);
        scene.open_dialog(&mut frames);
        scene.wheels.set(0);
        scene.frame(&mut frames, vec![moved(CANVAS.center())]);
        scene.frame(&mut frames, vec![wheel(-30.0)]);
        scene.settle(&mut frames, 2);
        assert_eq!(scene.wheels.get() > 0, passes, "the wheel passes only with a registration");
        assert!(is_focused(&frames.ctx, scene.armer_id.get()), "the dialog keeps key focus");
    }
}
