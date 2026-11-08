//! The width a style reads for a role and string is the width of the drawn text run.

use craft_ui::contract::{Common, Component, Cx, MeasureCx, Role, Show as _};
use craft_ui::primitives::TextRun;
use craft_ui::style::{StyleCx, TypeRole};
use craft_ui::testing::{Frames, trial};
use egui::{Response, Vec2};
use std::cell::Cell;

const SAMPLE: &str = "Wide 0123 text";

thread_local! {
    static STYLE_WIDTH: Cell<f32> = const { Cell::new(0.0) };
    static RUN_WIDTH: Cell<f32> = const { Cell::new(0.0) };
}

#[derive(craft_ui::contract::Component)]
struct Reader {
    #[common]
    common: Common,
}

#[derive(Clone, Debug, Default, PartialEq, craft_ui::style::ComponentStyle)]
struct ReaderLook {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

impl Component for Reader {
    const ROLE: Role = Role::GROUP;
    type Style = ReaderLook;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: ReaderStyleKey) -> ReaderLook {
        STYLE_WIDTH.with(|width| width.set(cx.text_width(TypeRole::Label, SAMPLE)));
        ReaderLook::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let run = TextRun::new(SAMPLE).role(TypeRole::Label);
        RUN_WIDTH.with(|width| width.set(cx.layout(&run).size().x));
        Vec2::ZERO
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn the_style_width_of_a_string_is_the_width_of_its_text_run() {
    let mut frames = Frames::new();
    for _ in 0..2 {
        frames.frame(vec![], |ui| {
            Reader { common: Common::new() }.show(ui);
        });
    }
    let (style, run) = (STYLE_WIDTH.with(Cell::get), RUN_WIDTH.with(Cell::get));
    assert!(style > 0.0, "the style read a width");
    assert_eq!(style, run, "one measurement of a role's text");
}
