//! The tab of a dock group's strip: a panel's title that selects the panel and starts a panel drag.

use super::{PanelDrag, PanelKey};
use crate::contract::{Common, Component, Cx, DragPreview, MeasureCx, NoValue, Outcome, PaintUi, Pointer as _, Props as _, Role};
use crate::primitives::{PaintShape, StrokeWidth, TextRun};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, Flags, StyleColor, StyleCx};
use craft_behavior::{GroupId, Metrics, PressPhase, Sense};
use craft_theme::variant::Tone;
use egui::{Response, Vec2};

/// The style of a panel tab.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
pub(super) struct TabStyle {
    #[style(metrics)]
    metrics: Metrics,
    mark: StyleColor,
}

/// What a tab reports: its response and whether it was chosen this pass.
pub(super) struct TabOut {
    response: Response,
    pub(super) chosen: bool,
}

impl Outcome for TabOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// One panel's tab.
// F8-MERGE: tabs (`TabStrip` replaces the row of these tabs and carries the panel drag)
#[derive(crate::contract::Component)]
#[component(sample = PanelTab::sample)]
pub(super) struct PanelTab {
    #[common]
    common: Common,
    group: GroupId,
    panel: PanelKey,
    title: Label,
    active: bool,
}

impl PanelTab {
    #[cfg(feature = "testing")]
    fn sample() -> Self {
        PanelTab::new(GroupId::new(0), PanelKey::new(0), Label::from("Layers"), true)
    }

    pub(super) fn new(group: GroupId, panel: PanelKey, title: Label, active: bool) -> Self {
        PanelTab { common: Common::new(), group, panel, title: title.clone(), active }.label(title)
    }
}

impl Component for PanelTab {
    const ROLE: Role = Role::BUTTON;
    const OVERLAYS: bool = false;
    type Style = TabStyle;
    type Output = TabOut;

    fn style(cx: &StyleCx<'_>, _: PanelTabStyleKey) -> TabStyle {
        TabStyle { metrics: cx.control().metrics(), mark: cx.tone(Tone::Accent) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(cx.resolved.label)).galley.size()
    }

    fn sense(&self) -> Sense {
        Sense::PRESS | Sense::FOCUS
    }

    /// A drag past the threshold carries the panel, also when it reorders within its own strip.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if matches!(cx.press().phase, PressPhase::DragStart { .. }) {
            cx.start_drag(PanelDrag::new(self.panel, self.group), DragPreview::Badge(self.title.clone()));
        }
    }

    fn flags(&self) -> Flags {
        if self.active { Flags::SELECTED } else { Flags::empty() }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let draw = ui.draw();
        draw.text(cx.content_rect, &TextRun::new(cx.resolved.label));
        if self.active {
            draw.stroke(&PaintShape::Path(vec![cx.rect.left_bottom(), cx.rect.right_bottom()]), StrokeWidth::Medium, cx.style.mark);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> TabOut {
        TabOut { response: cx.response.clone(), chosen: cx.activation().is_some() }
    }
}
