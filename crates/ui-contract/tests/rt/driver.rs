//! Count trials of the driver's own reads: the tracked hover point (`CxCore::hover_point`) reruns only
//! the node that reads it.

use craft_behavior::IconId;
use craft_ui_contract::contract::*;
use craft_ui_contract::runtime::windows::window_root;
use craft_ui_contract::style::StyleCx;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Pos2, Rect, Sense, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

use crate::skeleton::IconButton;

/// A pad that reads where the pointer is over it, as a collection's hover lead does.
#[derive(Component)]
struct HoverPad {
    #[common]
    common: Common,
}

impl Component for HoverPad {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HoverPadStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(120.0, 60.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let _ = cx.hover_point();
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[derive(Default)]
struct App {
    pad: Rect,
    button: Rect,
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    rig.step_with(&mut |_, cx| {
        window_root(cx, |ui| {
            app.button = IconButton::new(IconId::new("plus"), "Other").show(ui).rect;
            app.pad = Show::show(HoverPad { common: Common::new() }, ui).rect;
        })
    })
}

fn settled(app: &mut App) -> Rig<RecordingRenderer> {
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    for _ in 0..5 {
        if !step(&mut rig, app).ran {
            return rig;
        }
    }
    panic!("the rig did not settle");
}

fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Input) -> StepReport {
    let main = rig.main;
    rig.inject(main, input);
    step(rig, app)
}

#[test]
fn a_pointer_move_reruns_only_the_node_that_reads_its_hover_point() {
    let mut app = App::default();
    let mut rig = settled(&mut app);
    let entered = input(&mut rig, &mut app, Input::mouse_move(app.pad.center()));
    assert_eq!(entered.frame.runs, 1, "entering the pad runs the pad alone: {entered:?}");
    let within = input(&mut rig, &mut app, Input::mouse_move(app.pad.center() + Vec2::new(5.0, 3.0)));
    assert_eq!(within.frame.runs, 1, "a move over the pad runs the pad again, and nothing else: {within:?}");
    let left = input(&mut rig, &mut app, Input::mouse_move(app.button.center()));
    assert_eq!(left.frame.runs, 1, "leaving the pad runs it once; the button only repaints: {left:?}");
    let over_button = input(&mut rig, &mut app, Input::mouse_move(app.button.center() + Vec2::new(1.0, 1.0)));
    assert_eq!(over_button.frame.runs, 0, "a move away from the pad runs nothing: {over_button:?}");
    let away = input(&mut rig, &mut app, Input::mouse_move(Pos2::new(390.0, 290.0)));
    assert_eq!(away.frame.runs, 0);
}
