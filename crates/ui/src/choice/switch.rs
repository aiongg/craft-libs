//! `Switch`: an on/off setting that applies at once, a track with a thumb that slides to the on end. Its text is the
//! label step's, as the check box's is.

use crate::contract::{Animate as _, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, PaintUi, Pointer as _, Role, Styled as _, spec};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::style::{ComponentStyle, Flags, FocusLook, StyleColor, StyleCx};
use craft_behavior::{Flow, Metrics, Sense};
use craft_theme::{Stateful, States, variant::Tone};
use egui::{CornerRadius, Rect, Vec2};

/// The track's width as a multiple of its height.
const TRACK_ASPECT: f32 = 1.8;
/// Where the thumb rests while the value is mixed: the middle of its travel.
const MIXED_AT: f32 = 0.5;

/// The style of a switch.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SwitchStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The track fill.
    track: Stateful<StyleColor>,
    /// The track edge.
    edge: Stateful<StyleColor>,
    /// The thumb.
    thumb: Stateful<StyleColor>,
    radius: CornerRadius,
    width: f32,
    height: f32,
    /// Between the track's edge and the thumb.
    inset: f32,
}

/// A switch over a bool; a `Mixed` binding rests the thumb in the middle and a click turns it on.
#[derive(crate::contract::Component)]
pub struct Switch<'a> {
    #[common]
    common: Common,
    value: Bind<'a, bool>,
}

impl<'a> Switch<'a> {
    /// A switch of `value`.
    pub fn new(value: Bind<'a, bool>) -> Self {
        Switch { common: Common::new(), value }
    }
}

impl Component for Switch<'_> {
    const ROLE: Role = Role::SWITCH;
    const OVERLAYS: bool = false;
    type Style = SwitchStyle;
    type Output = EditResponse<bool>;

    fn style(cx: &StyleCx<'_>, _: SwitchStyleKey) -> SwitchStyle {
        let control = cx.control();
        let (colors, accent) = (&cx.tokens.color, &cx.tokens.color.accent);
        let solid = |rest, hover, pressed| States::new(cx.color(rest), cx.color(hover), cx.color(pressed), cx.color(colors.state.disabled_fill));
        let on = solid(accent.strong, accent.strong_hover, accent.strong_pressed);
        let (well, knob, on_knob) = (cx.color(colors.surface.sunken), cx.color(colors.surface.knob), cx.color(colors.text.on_strong));
        let track = Stateful::new(well).on(on.clone()).when(Flags::INVALID, well);
        let edge = Stateful::new(cx.color(colors.line.control_edge)).on(on).when(Flags::INVALID, cx.tone(Tone::Bad));
        let thumb = Stateful::new(knob).on(States::new(on_knob, on_knob, on_knob, knob));
        SwitchStyle {
            metrics: Metrics::new().min_size(Vec2::splat(control.height)),
            focus: crate::controls::focus_look(cx),
            track,
            edge,
            thumb,
            radius: CornerRadius::same((control.icon / 2.0).round() as u8),
            width: control.icon * TRACK_ASPECT,
            height: control.icon,
            inset: cx.tokens.space.gap.tight,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(cx.style.width, cx.style.height)
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
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let on = cx.value().copied().unwrap_or(false);
        let at = if cx.mixed { MIXED_AT } else { cx.anim_bool("on", on, spec::CHECK) };
        let track = Rect::from_center_size(cx.rect.center(), Vec2::new(style.width, style.height));
        let draw = ui.draw();
        let shape = PaintShape::Rect(track, style.radius);
        draw.fill(&shape, cx.resolve(&style.track));
        draw.stroke(&shape, StrokeWidth::Thin, cx.resolve(&style.edge));
        let slot = Flow::new(track, cx.flow.is_rtl()).span(at * (style.width - style.height), style.height);
        let thumb = PaintShape::Circle(slot.center(), (style.height - style.inset * 2.0) / 2.0);
        draw.fill(&thumb, cx.resolve(&style.thumb));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<bool> {
        cx.edit_response()
    }
}
