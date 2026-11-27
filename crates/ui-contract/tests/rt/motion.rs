//! Count trials of motion on node memory (`Animate`): an animating node builds once per frame while
//! its motion is owed and not at all once it settles, and its siblings never build.

use std::cell::Cell;

use craft_behavior::IconId;
use craft_ui_contract::contract::*;
use craft_ui_contract::primitives::PaintShape;
use craft_ui_contract::runtime::windows::window_root;
use craft_ui_contract::style::{StyleColor, StyleCx, UiTokens, rgba_color};
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Rect, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

use crate::skeleton::{IconButton, Stack};

/// The knob's height and the width of its fill.
const SIZE: f32 = 24.0;
/// One frame at 60 Hz.
const FRAME: f64 = 1.0 / 60.0;
/// More frames than any theme's check motion lasts.
const MOST_FRAMES: usize = 120;

thread_local! {
    /// Runs of the knob's hooks on this test's thread.
    static KNOB_RUNS: Cell<u32> = const { Cell::new(0) };
}

fn knob_runs() -> u32 {
    KNOB_RUNS.with(Cell::get)
}

/// A fill that slides to the knob's end while `on`, over the check motion, driven from paint.
#[derive(Component)]
pub struct Knob {
    #[common]
    common: Common,
    on: bool,
}

impl Knob {
    fn new(on: bool) -> Knob {
        Knob { common: Common::new(), on }
    }

    fn show(self, ui: &mut Ui<'_, '_>) -> Response {
        Show::show(self, ui)
    }
}

impl Component for Knob {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: KnobStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(SIZE * 2.0, SIZE)
    }

    fn update(&mut self, _: &mut Cx<'_, Self>) {
        KNOB_RUNS.with(|runs| runs.set(runs.get() + 1));
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let at = cx.anim_bool("on", self.on, spec::CHECK);
        let fill = ui.env.tokens.theme().color.state.hover;
        let rect = Rect::from_min_size(cx.rect.min + Vec2::new(at * SIZE, 0.0), Vec2::splat(SIZE));
        ui.draw().fill(&PaintShape::Rect(rect, UiTokens::corner(SIZE / 4.0)), StyleColor(rgba_color(fill)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A "Toggle" button over a stack of the knob and a sibling button; a click on Toggle flips the
/// knob's prop.
struct App {
    on: bool,
    toggle: Rect,
}

fn app_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    let toggle = IconButton::new(IconId::new("plus"), "Toggle").show(ui);
    app.toggle = toggle.rect;
    if toggle.clicked() {
        app.on = !app.on;
    }
    let on = app.on;
    Stack::new().show(ui, |ui| {
        Knob::new(on).show(ui);
        IconButton::new(IconId::new("brush"), "Sibling").show(ui);
    });
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    rig.step_with(&mut |_, cx| window_root(cx, |ui| app_ui(ui, app)))
}

fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Input) -> StepReport {
    let main = rig.main;
    rig.inject(main, input);
    step(rig, app)
}

/// Steps a frame later each time until a step runs nothing.
fn settle(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
    for _ in 0..MOST_FRAMES {
        rig.driver.platform.advance(FRAME);
        if !step(rig, app).ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

#[test]
fn an_animating_node_builds_once_per_frame_while_owed_and_never_after_it_settles() {
    let mut app = App { on: false, toggle: Rect::NOTHING };
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let start = knob_runs();
    assert!(step(&mut rig, &mut app).ran);
    let first = knob_runs();
    assert_eq!(first, start + 1, "the knob's first run shows its target at once and owes nothing");
    settle(&mut rig, &mut app);
    assert_eq!(knob_runs(), first, "a knob at rest builds no more");

    let at = app.toggle.center();
    input(&mut rig, &mut app, Input::mouse_move(at));
    input(&mut rig, &mut app, Input::mouse_down(at));
    settle(&mut rig, &mut app);
    let before = knob_runs();
    let up = input(&mut rig, &mut app, Input::mouse_up(at));
    assert!(app.on);
    assert_eq!(knob_runs(), before + 1, "the changed prop runs the knob, which starts its slide");
    assert_eq!(up.frame.runs, 2, "Toggle and the knob run; the stack and the sibling are kept: {up:?}");

    // The slide: one knob run per frame and no other hook, until the frame that finds it settled.
    let mut owed = 0;
    let mut settled = false;
    for _ in 0..MOST_FRAMES {
        rig.driver.platform.advance(FRAME);
        let runs_before = knob_runs();
        let frame = step(&mut rig, &mut app);
        let knob = knob_runs() - runs_before;
        if !settled && knob == 1 {
            owed += 1;
            assert_eq!(frame.frame.runs, 1, "only the knob runs while it slides: {frame:?}");
            continue;
        }
        settled = true;
        assert_eq!((knob, frame.frame.runs), (0, 0), "nothing runs once the slide has settled: {frame:?}");
        if !frame.ran {
            break;
        }
    }
    assert!(settled, "the slide settles");
    assert!(owed >= 2, "the slide runs over several frames, one knob run each: {owed}");
    let last = knob_runs();
    rig.driver.platform.advance(1.0);
    let later = step(&mut rig, &mut app);
    assert!(!later.ran && knob_runs() == last, "a settled knob asks for no frame: {later:?}");
}
