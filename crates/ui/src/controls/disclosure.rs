//! `Disclosure`: the open/closed toggle of a section, a title row's lead and a tree row's expander.

use crate::contract::{
    Animate as _, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, PaintUi, Pointer as _, Role, Styled as _, spec,
};
use crate::primitives::{Glyph, PaintShape, StrokeWidth};
use crate::style::{ComponentStyle, IconLook, Ink, Stateful, StyleCx, UiTokens};
use craft_behavior::{Metrics, Sense};
use craft_i18n::Direction;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};

/// A quarter turn: closed points to the end, open points down.
const QUARTER_TURN: f32 = std::f32::consts::FRAC_PI_2;
/// The chevron's arms, as a share of the glyph rect's side: it spans two fifths of the side along the arrow and a
/// fifth across.
const ARM: f32 = 0.1;
/// The submenu arrow's base and depth, as shares of the glyph rect's side.
const ARROW_BASE: f32 = 0.55;
const ARROW_DEPTH: f32 = 0.35;

/// The arrow every component shows for something that opens or unfolds: a chevron that turns from pointing to the
/// end of the line (closed) to pointing down (open) as `open` goes from 0 to 1. A dropdown's arrow is the open
/// chevron; a submenu's is the filled arrow to the end of the line. Components name these through
/// [`DisclosureGlyph::dropdown`] and [`DisclosureGlyph::submenu`] and never an icon of their own.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DisclosureGlyph {
    /// The turn, 0 closed to 1 open.
    pub open: f32,
    /// A filled arrow in place of the stroked chevron.
    pub filled: bool,
}

/// The dropdown arrow.
static DROPDOWN: DisclosureGlyph = DisclosureGlyph { open: 1.0, filled: false };
/// The submenu arrow.
static SUBMENU: DisclosureGlyph = DisclosureGlyph { open: 0.0, filled: true };

impl DisclosureGlyph {
    /// A chevron `open` of the way turned.
    pub const fn new(open: f32) -> Self {
        DisclosureGlyph { open, filled: false }
    }

    /// The icon of the arrow after a dropdown's text: a button that opens a menu, a picker's face, a field with a list.
    pub fn dropdown() -> crate::primitives::IconSpec<'static> {
        crate::primitives::IconSpec::glyph(&DROPDOWN)
    }

    /// The icon of the arrow at the end of a menu row that opens a submenu.
    pub fn submenu() -> crate::primitives::IconSpec<'static> {
        crate::primitives::IconSpec::glyph(&SUBMENU)
    }

    /// The outline in `rect`, reading in `direction`: a closed glyph points to the end of the line.
    pub fn path(&self, rect: Rect, direction: Direction) -> Vec<Pos2> {
        self.outline(rect, direction == Direction::Rtl)
    }

    fn outline(&self, rect: Rect, rtl: bool) -> Vec<Pos2> {
        let side = rect.width().min(rect.height());
        let turn = self.open * QUARTER_TURN * if rtl { -1.0 } else { 1.0 };
        let (sin, cos) = turn.sin_cos();
        let flip = if rtl { -1.0 } else { 1.0 };
        let center = rect.center();
        let points: [Vec2; 3] = if self.filled {
            let (half, depth) = (side * ARROW_BASE / 2.0, side * ARROW_DEPTH / 2.0);
            [Vec2::new(-depth, -half), Vec2::new(depth, 0.0), Vec2::new(-depth, half)]
        } else {
            let arm = side * ARM;
            [Vec2::new(-arm, -arm * 2.0), Vec2::new(arm, 0.0), Vec2::new(-arm, arm * 2.0)]
        };
        points
            .map(|point| {
                let point = Vec2::new(point.x * flip, point.y);
                center + Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
            })
            .to_vec()
    }
}

impl Glyph for DisclosureGlyph {
    fn directional(&self) -> bool {
        self.open < 1.0
    }

    fn paint(&self, painter: &Painter, tokens: &UiTokens, rect: Rect, color: Color32, mirrored: bool) {
        let points = self.outline(rect, mirrored);
        if self.filled {
            painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
        } else {
            let width = StrokeWidth::Thin.points(tokens, painter.pixels_per_point());
            painter.add(Shape::line(points, Stroke::new(width, color)));
        }
    }
}

/// The style of a disclosure.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DisclosureStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The chevron.
    #[style(icon)]
    glyph: IconLook,
}

/// A button bound to an open state, with the expanded state. A face shown by a title row or a tree row has no node.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Disclosure<'a> {
    #[common]
    common: Common,
    open: Bind<'a, bool>,
    /// Whether the disclosure is its own AccessKit node; a part of another component's node turns it off.
    #[prop]
    node: bool,
}

impl<'a> Disclosure<'a> {
    /// A disclosure of `open`.
    pub fn new(open: Bind<'a, bool>) -> Self {
        Disclosure { common: Common::new(), open, node: true }
    }
}

impl Component for Disclosure<'_> {
    const ROLE: Role = Role::BUTTON;
    const PARTS: bool = false;
    const OVERLAYS: bool = false;
    const ALT_ROLES: &'static [Role] = &[Role::None];
    type Style = DisclosureStyle;
    type Output = EditResponse<bool>;

    fn style(cx: &StyleCx<'_>, _: DisclosureStyleKey) -> DisclosureStyle {
        let side = cx.control().icon;
        DisclosureStyle { metrics: Metrics::new().min_size(Vec2::splat(side)), glyph: IconLook::new(side, Stateful::new(cx.ink(Ink::OnPanel))) }
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
        let glyph = DisclosureGlyph::new(open);
        ui.draw().stroke(&PaintShape::Path(glyph.path(cx.rect, cx.env.direction())), StrokeWidth::Thin, cx.resolve(&cx.style.glyph.color));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<bool> {
        cx.edit_response()
    }
}
