//! A key activation carries the modifiers held with the key, as a pointer activation carries the ones held with
//! the click: Alt+Enter on a button can add beside, the way Alt+click does.

use craft_behavior::{Metrics, Sense};
use craft_ui::contract::{Activation, Common, Component, Cx, MeasureCx, Pointer as _, Role};
use craft_ui::style::{ComponentStyle, StyleCx};
use egui::{Response, Vec2};
use std::cell::Cell;

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct AdderStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A button that records the activation the pipeline gave it.
#[derive(Component)]
pub struct Adder<'a> {
    #[common]
    common: Common,
    seen: &'a Cell<Option<Activation>>,
}

impl Component for Adder<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = AdderStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: AdderStyleKey) -> AdderStyle {
        AdderStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(activation) = cx.activation() {
            self.seen.set(Some(activation));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
