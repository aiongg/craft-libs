//! A container whose axis keeps its left-to-right order (`Props::ltr_axis`) splits the flow direction from the
//! paragraph direction: what is inside flows left to right, its text keeps the interface's paragraph direction.

use super::kit::{Panel, PlainStyle};
use craft_behavior::Sense;
use craft_i18n::Direction;
use craft_ui::contract::{Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::primitives::TextRun;
use craft_ui::runtime::env::RootEnv;
use craft_ui::style::StyleCx;
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Response, Vec2};
use std::cell::Cell;

const HEBREW: &str = "שלום";
const WIDTH: f32 = 300.0;

/// A leaf that paints a Hebrew label across its width and records the directions its env reads.
#[derive(craft_ui::contract::Component)]
struct Hebrew<'a> {
    #[common]
    common: Common,
    seen: &'a Cell<Option<(Direction, Direction, Rect)>>,
}

impl Component for Hebrew<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HebrewStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(WIDTH, 24.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.seen.set(Some((cx.env.direction(), cx.env.text_direction(), cx.rect)));
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.rect, &TextRun::new(HEBREW));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The painted Hebrew run's right edge, and the directions and rect the leaf read.
fn shown(ltr_axis: bool) -> (f32, (Direction, Direction, Rect)) {
    let mut frames = Frames::new();
    let mut root = RootEnv::new();
    root.direction = Direction::Rtl;
    frames.root(root);
    let seen = Cell::new(None);
    let mut show = |ui: &mut egui::Ui| {
        let panel = if ltr_axis { Panel::new().ltr_axis() } else { Panel::new() };
        panel.id_salt("axis").show(ui, |ui, _| {
            Hebrew { common: Common::new(), seen: &seen }.show(ui);
        });
    };
    frames.idle(3, &mut show);
    let run = frames.texts().iter().find(|run| run.text == HEBREW).map(|run| run.rect).expect("the label was painted");
    (run.max.x, seen.get().expect("the leaf updated"))
}

#[trial]
fn text_inside_an_ltr_axis_container_keeps_the_interfaces_paragraph_direction() {
    let (run_right, (flow, paragraph, rect)) = shown(true);
    assert_eq!((flow, paragraph), (Direction::Ltr, Direction::Rtl), "the flow is left to right and the text right to left");
    assert!((run_right - rect.max.x).abs() < 1.0, "the Hebrew label lies against the end edge: {run_right} in {rect:?}");
    let (_, (flow, paragraph, _)) = shown(false);
    assert_eq!((flow, paragraph), (Direction::Rtl, Direction::Rtl), "outside an ltr-axis container both follow the interface");
}
