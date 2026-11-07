//! `Checkbox`: a box with a check mark, a dash while the value is mixed. Its text is the label step's (a `Field`'s
//! or `PaintUi::labelled`'s); the control draws only the box and its label names it.

use crate::contract::{Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, PaintUi, Pointer as _, Role, Styled as _};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::style::{ComponentStyle, Flags, FocusLook, StyleColor, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Mixed, Sense};
use craft_theme::{Stateful, States, variant::Tone};
use egui::{CornerRadius, Pos2, Rect, Vec2};

/// The check mark's corner and tip, as shares of the box's side from its top-left.
const TICK: [(f32, f32); 3] = [(0.22, 0.52), (0.42, 0.72), (0.78, 0.30)];
/// The mixed dash's half-length as a share of the box's side.
const DASH: f32 = 0.25;

/// The style of a check box.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CheckboxStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The box fill.
    fill: Stateful<StyleColor>,
    /// The box edge.
    edge: Stateful<StyleColor>,
    /// The check mark and the dash.
    mark: StyleColor,
    radius: CornerRadius,
    side: f32,
}

/// A check box over a bool; a `Mixed` binding shows the mixed state and a click makes it checked.
#[derive(crate::contract::Component)]
pub struct Checkbox<'a> {
    #[common]
    common: Common,
    value: Bind<'a, bool>,
    /// The value `flags` reads: the pass's shown value once `update` ran.
    shown: Option<Mixed<bool>>,
}

impl<'a> Checkbox<'a> {
    /// A check box of `value`.
    pub fn new(value: Bind<'a, bool>) -> Self {
        Checkbox { common: Common::new(), value, shown: None }
    }

    fn held(&self) -> Option<Mixed<bool>> {
        self.shown.clone().or_else(|| {
            self.value.current().map(|held| match held {
                Mixed::Same(value) => Mixed::Same(*value),
                Mixed::Mixed => Mixed::Mixed,
            })
        })
    }
}

impl Component for Checkbox<'_> {
    const ROLE: Role = Role::CHECK_BOX;
    type Style = CheckboxStyle;
    type Output = EditResponse<bool>;

    fn style(cx: &StyleCx<'_>, _: CheckboxStyleKey) -> CheckboxStyle {
        let control = cx.control();
        let (colors, accent) = (&cx.tokens.color, &cx.tokens.color.accent);
        let (well, solid) =
            (cx.color(colors.surface.sunken), States::new(accent.strong, accent.strong_hover, accent.strong_pressed, colors.state.disabled_fill));
        let on = |states: States<craft_theme::Rgba>| {
            States::new(cx.color(states.rest), cx.color(states.hover), cx.color(states.pressed), cx.color(states.disabled))
        };
        let fill = Stateful::new(well).on(on(solid)).when(Flags::INVALID, well);
        let edge = Stateful::new(cx.color(colors.line.control_edge)).on(on(solid)).when(Flags::INVALID, cx.tone(Tone::Bad));
        CheckboxStyle {
            metrics: Metrics::new().min_size(Vec2::splat(control.height)),
            focus: super::focus_look(cx),
            fill,
            edge,
            mark: cx.color(colors.text.on_strong),
            radius: cx.surface(SurfaceKind::Well).radius,
            side: control.icon,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.icon())
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn bind(&mut self) -> Option<BindMut<'_, bool>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.activation().is_some() {
            let on = cx.value().copied().unwrap_or(false);
            cx.set(!on);
        }
        self.shown = Some(cx.value().map_or(Mixed::Mixed, |value| Mixed::Same(*value)));
    }

    fn flags(&self) -> Flags {
        match self.held() {
            Some(Mixed::Same(true)) => Flags::CHECKED,
            Some(Mixed::Mixed) => Flags::MIXED,
            _ => Flags::empty(),
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let square = Rect::from_center_size(cx.rect.center(), Vec2::splat(cx.style.side));
        let draw = ui.draw();
        let shape = PaintShape::Rect(square, cx.style.radius);
        draw.fill(&shape, cx.resolve(&cx.style.fill));
        draw.stroke(&shape, StrokeWidth::Thin, cx.resolve(&cx.style.edge));
        let at = |(x, y): (f32, f32)| -> Pos2 { square.min + Vec2::new(x, y) * cx.style.side };
        if cx.mixed {
            let c = square.center();
            let half = Vec2::new(cx.style.side * DASH, 0.0);
            draw.stroke(&PaintShape::Path(vec![c - half, c + half]), StrokeWidth::Medium, cx.style.mark);
        } else if cx.value().copied().unwrap_or(false) {
            draw.stroke(&PaintShape::Path(TICK.map(at).to_vec()), StrokeWidth::Medium, cx.style.mark);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<bool> {
        cx.edit_response()
    }
}
