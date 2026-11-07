//! Animated values: state easing at step 12, the motion table, and the one repaint request.

use crate::contract::kit::PlainStyle;
use craft_behavior::{MotionMode, Sense};
use craft_ui::contract::{Animate as _, Common, Component, Cx, PaintUi, Role, Show as _, Styled as _, spec};
use craft_ui::runtime::env::RootEnv;
use craft_ui::style::StyleCx;
use craft_ui::testing::pointer::moved;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Response};
use std::cell::Cell;
use std::time::Duration;

/// A component that reports the hover value its paint saw.
#[derive(Component)]
struct Hovered<'a> {
    #[common]
    common: Common,
    seen: &'a Cell<f32>,
}

impl Component for Hovered<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HoveredStyleKey) -> PlainStyle {
        PlainStyle::min(egui::vec2(40.0, 20.0))
    }

    fn sense(&self) -> Sense {
        Sense::CLICK
    }

    fn paint(&mut self, _: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        self.seen.set(cx.state().t.hover);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// State easing reaches its end value in steps, then asks for no further pass.
#[trial]
fn state_easing_reaches_its_end_value_and_stops_requesting_repaints() {
    let seen = Cell::new(0.0);
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        Hovered { common: Common::new(), seen: &seen }.show(ui);
    };
    frames.idle(3, &mut show);
    assert_eq!(frames.repaint_delay(), None, "a resting component asks for nothing");
    let rest = seen.get();
    frames.frame(vec![moved(Pos2::new(10.0, 10.0))], &mut show);
    let mut values = vec![seen.get()];
    while frames.repaint_delay() == Some(Duration::ZERO) && values.len() < 60 {
        frames.frame(Vec::new(), &mut show);
        values.push(seen.get());
    }
    let end = seen.get();
    assert!(end > rest, "the hover value rose from {rest} to its end: {values:?}");
    assert!(values.iter().any(|v| *v > rest && *v < end), "it passed through values between: {values:?}");
    assert!(values.windows(2).all(|pair| pair[0] <= pair[1]), "it rose without stepping back: {values:?}");
    assert_eq!(frames.repaint_delay(), None, "once at its end it requests no repaint");
}

/// A component that reports what `anim` returns for its target.
#[derive(Component)]
struct Mover<'a> {
    #[common]
    common: Common,
    target: f32,
    travel: bool,
    shown: &'a Cell<f32>,
}

impl Component for Mover<'_> {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: MoverStyleKey) -> PlainStyle {
        PlainStyle::min(egui::vec2(40.0, 20.0))
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let kind = if self.travel { spec::MOVE } else { spec::HOVER };
        self.shown.set(cx.anim("x", self.target, kind));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The value one frame after the target changed from 0 to 100, and whether that frame asked for another pass.
fn after_a_change(mode: MotionMode, travel: bool) -> (f32, bool) {
    let shown = Cell::new(0.0);
    let target = Cell::new(0.0);
    let mut frames = Frames::new();
    frames.root(RootEnv::default().motion(mode));
    let mut show = |ui: &mut egui::Ui| {
        Mover { common: Common::new(), target: target.get(), travel, shown: &shown }.show(ui);
    };
    frames.idle(2, &mut show);
    target.set(100.0);
    frames.frame(Vec::new(), &mut show);
    (shown.get(), frames.repaint_delay().is_some())
}

/// Travel jumps under reduced motion and fades jump when motion is off, while a fade still runs reduced.
#[trial]
fn reduced_motion_jumps_travel_and_off_jumps_everything() {
    let (running, asks) = after_a_change(MotionMode::Full, true);
    assert!(running > 0.0 && running < 100.0 && asks, "full motion travels: {running}");
    assert_eq!(after_a_change(MotionMode::Reduced, true), (100.0, false), "reduced motion snaps a travel");
    let (fade, asks) = after_a_change(MotionMode::Reduced, false);
    assert!(fade < 100.0 && asks, "reduced motion keeps its fades: {fade}");
    assert_eq!(after_a_change(MotionMode::Off, false), (100.0, false), "off snaps a fade");
}
