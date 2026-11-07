//! `Disclosure`: the open/closed toggle of a section, a title row's lead and a tree row's expander.

use crate::contract::{
    Animate as _, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, PaintUi, Pointer as _, Role, spec,
};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::style::{ComponentStyle, Ink, StyleColor, StyleCx};
use craft_behavior::{Metrics, Sense};
use craft_i18n::Direction;
use egui::{Pos2, Rect, Vec2};

/// A quarter turn: closed points to the end, open points down.
const QUARTER_TURN: f32 = std::f32::consts::FRAC_PI_2;
/// The chevron's arms, as a share of the glyph rect's side.
const ARM: f32 = 0.2;

/// The glyph of a disclosure: a chevron that turns from pointing to the end of the line (closed) to pointing down
/// (open) as `open` goes from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisclosureGlyph {
    /// The turn, 0 closed to 1 open.
    pub open: f32,
    /// The reading direction: a closed chevron points to the end of the line.
    pub direction: Direction,
}

impl DisclosureGlyph {
    /// The chevron's path in `rect`.
    pub fn path(&self, rect: Rect) -> Vec<Pos2> {
        let rtl = self.direction == Direction::Rtl;
        let side = rect.width().min(rect.height());
        let arm = side * ARM;
        let turn = self.open * QUARTER_TURN * if rtl { -1.0 } else { 1.0 };
        let (sin, cos) = turn.sin_cos();
        let flip = if rtl { -1.0 } else { 1.0 };
        let center = rect.center();
        [Vec2::new(-arm, -arm * 2.0), Vec2::new(arm, 0.0), Vec2::new(-arm, arm * 2.0)]
            .map(|point| {
                let point = Vec2::new(point.x * flip, point.y);
                center + Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
            })
            .to_vec()
    }
}

/// The style of a disclosure.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DisclosureStyle {
    #[style(metrics)]
    metrics: Metrics,
    ink: StyleColor,
}

/// A button bound to an open state, with the expanded state. A face shown by a title row or a tree row has no node.
#[derive(crate::contract::Component)]
pub struct Disclosure<'a> {
    #[common]
    common: Common,
    open: Bind<'a, bool>,
    node: bool,
}

impl<'a> Disclosure<'a> {
    /// A disclosure of `open`.
    pub fn new(open: Bind<'a, bool>) -> Self {
        Disclosure { common: Common::new(), open, node: true }
    }

    /// Whether the disclosure is its own AccessKit node; a part of another component's node turns it off.
    #[must_use]
    pub fn node(mut self, on: bool) -> Self {
        self.node = on;
        self
    }
}

impl Component for Disclosure<'_> {
    const ROLE: Role = Role::BUTTON;
    const ALT_ROLES: &'static [Role] = &[Role::None];
    type Style = DisclosureStyle;
    type Output = EditResponse<bool>;

    fn style(cx: &StyleCx<'_>, _: DisclosureStyleKey) -> DisclosureStyle {
        let side = cx.control().icon;
        DisclosureStyle { metrics: Metrics::new().min_size(Vec2::splat(side)), ink: cx.ink(Ink::OnPanel) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.icon())
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn bind(&mut self) -> Option<BindMut<'_, bool>> {
        Some(self.open.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.activation().is_some() {
            let open = cx.value().copied().unwrap_or(false);
            cx.set(!open);
        }
    }

    fn access(&self, access: &mut crate::contract::Access, cx: &Cx<'_, Self>) {
        if !self.node {
            access.role(Role::None);
        }
        access.expanded(cx.value().copied().unwrap_or(false));
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let open = cx.anim_bool("open", cx.value().copied().unwrap_or(false), spec::OPEN_RISE);
        let glyph = DisclosureGlyph { open, direction: cx.env.direction() };
        ui.draw().stroke(&PaintShape::Path(glyph.path(cx.rect)), StrokeWidth::Medium, cx.style.ink);
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<bool> {
        cx.edit_response()
    }
}
