//! A canvas under a modal dialog (15.5, 8.3): an armed sampler lets primary presses through, the view stays usable,
//! and everything else stays blocked.

use super::kit::{Opened, Opener, PlainStyle, Seen};
use crate::body::kit::Slab;
use crate::contract::kit::{Probe, Seen as ProbeSeen};
use crate::windows_kit::Windows;
use craft_behavior::Sense;
use craft_ui::contract::{
    Common, Component, ContainerProps, Cx, MeasureCx, Mount, Overlay, OverlayUi, Overlays as _, Props as _, Role, Show as _, ShowContainer as _,
    Sizing, Trigger,
};
use craft_ui::style::StyleCx;
use craft_ui::testing::focus::is_focused;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{middle, moved, press, release, secondary};
use craft_ui::testing::{Frames, trial};
use egui::{Id, Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};

const OPENER: Rect = Rect { min: Pos2::new(600.0, 100.0), max: Pos2::new(680.0, 120.0) };
const CANVAS: Rect = Rect { min: Pos2::new(20.0, 20.0), max: Pos2::new(320.0, 260.0) };

/// What the dialog's control asks of the canvas.
#[derive(Clone, Copy, PartialEq)]
enum Arm {
    Nothing,
    View,
    Sampler,
}

/// The control in the dialog: it holds key focus and registers the canvas in every pass.
#[derive(Component)]
struct Armer<'a> {
    #[common]
    common: Common,
    canvas: Id,
    arm: Arm,
    at: &'a Cell<Rect>,
    id: &'a Cell<Id>,
}

impl Component for Armer<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ArmerStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(80.0, 30.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.at.set(cx.rect);
        self.id.set(cx.id);
        match self.arm {
            Arm::Nothing => {}
            Arm::View => cx.keep_view_open(self.canvas),
            Arm::Sampler => cx.arm_sampler(self.canvas),
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A button that opens a modal dialog holding the armer and, while `menu` is set, a menu.
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
        let shown = ui.show(popup, Overlay::modal(), |ui, _| {
            Armer { common: Common::new(), canvas, arm: scene.arm.get(), at: &scene.armer_at, id: &scene.armer_id }.id_salt("armer").show(ui);
            Opener::new(&scene.menu_shown, &scene.menu_content, Trigger::Manual, Overlay::menu).open(scene.menu.get()).id_salt("menu").show(ui);
        });
        if shown.is_some() {
            self.scene.dialog.shown.set(true);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The scene: a canvas probe and the dialog's opener.
struct Scene {
    canvas: RefCell<ProbeSeen>,
    armer_at: Cell<Rect>,
    armer_id: Cell<Id>,
    open: Cell<bool>,
    arm: Cell<Arm>,
    menu: Cell<bool>,
    dialog: Opened,
    menu_shown: Opened,
    menu_content: RefCell<Seen>,
}

impl Scene {
    fn new(arm: Arm) -> Self {
        Scene {
            canvas: RefCell::default(),
            armer_at: Cell::new(Rect::NOTHING),
            armer_id: Cell::new(Id::NULL),
            open: Cell::new(false),
            arm: Cell::new(arm),
            menu: Cell::new(false),
            dialog: Opened::default(),
            menu_shown: Opened::default(),
            menu_content: RefCell::default(),
        }
    }

    fn show(&self, ui: &mut Ui) {
        self.dialog.shown.set(false);
        self.menu_shown.shown.set(false);
        let canvas = self.canvas.borrow().id.unwrap_or(Id::NULL);
        Probe::new(&self.canvas).size(CANVAS.size()).sense(Sense::CLICK | Sense::DRAG).id_salt("canvas").show_at(ui, CANVAS);
        Dialog { common: Common::new(), scene: self, canvas }.id_salt("dialog").show_at(ui, OPENER);
    }

    /// A frame with `events`; the scene's flags are read as they stand.
    fn frame(&self, frames: &mut Frames, events: Vec<egui::Event>) {
        frames.frame(events, &mut |ui: &mut Ui| self.show(ui));
    }

    /// Opens the dialog and settles it.
    fn open_dialog(&self, frames: &mut Frames) {
        frames.idle(3, &mut |ui: &mut Ui| self.show(ui));
        self.open.set(true);
        self.frame(frames, vec![]);
        self.open.set(false);
        frames.idle(30, &mut |ui: &mut Ui| self.show(ui));
        assert!(self.dialog.shown.get(), "the dialog shows");
    }

    fn forget_canvas(&self) {
        let mut seen = self.canvas.borrow_mut();
        seen.press.clear();
    }

    fn pressed_canvas(&self) -> bool {
        !self.canvas.borrow().press.is_empty()
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
        frames.idle(2, &mut |ui: &mut Ui| self.show(ui));
    }
}

/// With a sampler armed, a press and a drag on the canvas under the dialog reach it, the dialog keeps key focus and
/// Escape closes it; once it is closed the arm lapses, so a dialog that no longer arms blocks the canvas again.
#[trial]
fn an_armed_sampler_passes_a_press_and_a_drag_to_the_canvas_and_lapses_with_the_dialog() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::Sampler);
    scene.open_dialog(&mut frames);
    scene.drag(&mut frames, CANVAS.center(), Vec2::new(40.0, 10.0));
    assert!(scene.pressed_canvas(), "the press reached the canvas");
    assert!(scene.dragged_canvas(), "the drag stayed the canvas's");
    assert!(scene.dialog.shown.get(), "a press on the canvas does not close the dialog");
    assert!(is_focused(&frames.ctx, scene.armer_id.get()), "the dialog keeps key focus");
    scene.frame(&mut frames, vec![key_press(Key::Escape, Modifiers::NONE)]);
    frames.idle(30, &mut |ui: &mut Ui| scene.show(ui));
    assert!(!scene.dialog.shown.get(), "Escape closes the dialog");
    // A dialog that opens later and does not arm gets no press through.
    scene.arm.set(Arm::Nothing);
    scene.open_dialog(&mut frames);
    scene.forget_canvas();
    scene.drag(&mut frames, CANVAS.center(), Vec2::new(40.0, 10.0));
    assert!(!scene.pressed_canvas(), "the old arm has lapsed");
}

/// Without an arm the modal rule holds: nothing reaches the canvas.
#[trial]
fn without_an_arm_a_press_on_the_canvas_reaches_nothing() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::Nothing);
    scene.open_dialog(&mut frames);
    scene.drag(&mut frames, CANVAS.center(), Vec2::new(40.0, 10.0));
    assert!(!scene.pressed_canvas(), "the modal blocks the canvas");
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
    frames.idle(30, &mut |ui: &mut Ui| scene.show(ui));
    assert!(scene.menu_shown.shown.get(), "the menu is open over the dialog");
    scene.menu.set(false);
    scene.drag(&mut frames, CANVAS.center(), Vec2::new(40.0, 10.0));
    assert!(!scene.pressed_canvas(), "the popup took the press");
    assert!(!scene.menu_shown.shown.get(), "the press closed the popup");
    assert!(scene.dialog.shown.get(), "the dialog stays open");
}

/// Only the primary press passes: a secondary press on the armed canvas is blocked.
#[trial]
fn a_secondary_press_on_the_armed_canvas_is_blocked() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::Sampler);
    scene.open_dialog(&mut frames);
    let at = CANVAS.center();
    scene.frame(&mut frames, vec![moved(at)]);
    scene.frame(&mut frames, vec![secondary(at, true)]);
    scene.frame(&mut frames, vec![secondary(at, false)]);
    frames.idle(2, &mut |ui: &mut Ui| scene.show(ui));
    assert!(!scene.pressed_canvas(), "a secondary press stays blocked");
}

/// View navigation under a dialog that only keeps the view open: a middle press (the pan) reaches the canvas and a
/// primary press does not.
#[trial]
fn a_view_kept_open_takes_the_middle_press_and_not_the_primary_one() {
    let mut frames = Frames::new();
    let scene = Scene::new(Arm::View);
    scene.open_dialog(&mut frames);
    let at = CANVAS.center();
    scene.drag(&mut frames, at, Vec2::new(40.0, 10.0));
    assert!(!scene.pressed_canvas(), "a primary press stays blocked while only the view is open");
    scene.frame(&mut frames, vec![moved(at)]);
    scene.frame(&mut frames, vec![middle(at, true)]);
    scene.frame(&mut frames, vec![middle(at, false)]);
    frames.idle(2, &mut |ui: &mut Ui| scene.show(ui));
    assert!(scene.pressed_canvas(), "the middle press reached the canvas");
}

/// A dialog in its own viewport: a press on the armed canvas in the main viewport reaches it.
#[trial]
fn a_dialog_in_its_own_viewport_lets_an_armed_canvas_in_the_main_viewport_take_the_press() {
    let windows = Windows::new();
    let canvas = RefCell::new(ProbeSeen::default());
    let at_armer = Cell::new(Rect::NOTHING);
    let armer_id = Cell::new(Id::NULL);
    let arm = Cell::new(Arm::Sampler);
    let frame = |root: Vec<egui::Event>| {
        windows.frame(root, vec![], |ui| {
            let id = canvas.borrow().id.unwrap_or(Id::NULL);
            Probe::new(&canvas).size(CANVAS.size()).sense(Sense::CLICK | Sense::DRAG).id_salt("canvas").show_at(ui, CANVAS);
            let props = ContainerProps::new().mount(Mount::viewport(Overlay::modal())).sizing(Sizing::Content);
            Slab::new(props).label("Sample").id_salt("dialog").show(ui, |ui, _| {
                Armer { common: Common::new(), canvas: id, arm: arm.get(), at: &at_armer, id: &armer_id }.id_salt("armer").show(ui);
            });
        });
    };
    for _ in 0..5 {
        frame(vec![]);
    }
    let at = CANVAS.center();
    frame(vec![moved(at)]);
    frame(vec![press(at)]);
    frame(vec![moved(at + Vec2::new(30.0, 0.0))]);
    frame(vec![release(at + Vec2::new(30.0, 0.0))]);
    frame(vec![]);
    let seen = canvas.borrow();
    assert!(!seen.press.is_empty(), "the press reached the canvas in the main viewport");
    assert!(seen.press.iter().any(|phase| phase.is_drag()), "and the drag stayed its own");
}
