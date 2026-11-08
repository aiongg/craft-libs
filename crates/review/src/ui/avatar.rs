//! `Avatar`: a person's round face, the initials of their name on their colour.

use craft_behavior::{Metrics, Sense};
use craft_theme::Rgba;
use craft_theme::contrast::ensure_contrast;
use craft_ui::contract::{Common, Component, Cx, MeasureCx, NoValue, Outcome, PaintUi, Props as _, Role, clicked};
use craft_ui::fonts::FontRef;
use craft_ui::primitives::{DocRun, DocText, Draw, PaintShape, StrokeWidth, TextRun};
use craft_ui::style::{ComponentStyle, DataColor, FocusLook, StyleColor, StyleCx, TypeRole};
use egui::{Color32, Pos2, Rect, Response, Vec2};

use crate::collab::initials;

#[cfg(feature = "screen-testing")]
mod stories;

/// The size and contrast rule of the initials, shared by every painter of a face.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AvatarFace {
    /// The font size of the initials.
    pub size: f32,
    /// The line height of the initials.
    pub line: f32,
    /// The least contrast ratio of the initials on the fill.
    pub contrast: f64,
    /// The ink the initials start from, before the contrast rule adjusts it.
    pub ink: [u8; 3],
}

impl AvatarFace {
    /// The face the style context gives: caption type, the text contrast of the theme.
    pub fn of(cx: &StyleCx<'_>) -> Self {
        let caption = cx.tokens.typography.get(TypeRole::Caption);
        AvatarFace {
            size: caption.size,
            line: caption.line_height,
            contrast: craft_theme::contrast::text_contrast_min(cx.contrast()),
            ink: rgb8(craft_theme::Srgba::from(cx.tokens.color.text.on_strong)),
        }
    }
}

/// The ink that reads on `fill`: `ink` where the contrast rule keeps it, else the nearest that it does.
pub(super) fn ink_on(fill: [u8; 3], ink: [u8; 3], contrast: f64) -> Color32 {
    let [r, g, b] = fill;
    let [ir, ig, ib] = ink;
    let ink = ensure_contrast(Rgba::rgb(ir, ig, ib), Rgba::rgb(r, g, b), contrast);
    let [r, g, b, _] = ink.0;
    Color32::from_rgb(r, g, b)
}

/// The 8-bit channels of a theme colour, the form `paint_avatar` takes a fill in.
pub(super) fn rgb8(color: craft_theme::Srgba) -> [u8; 3] {
    let [r, g, b, _] = Color32::from(color).to_array();
    [r, g, b]
}

/// Paints a round face filling `rect`: `fill` with the initials of `name` centred on it.
pub fn paint_avatar(draw: &Draw<'_>, rect: Rect, name: &str, fill: [u8; 3], face: &AvatarFace) {
    let [r, g, b] = fill;
    let circle = PaintShape::Circle(rect.center(), rect.width().min(rect.height()) / 2.0);
    draw.data_fill(&circle, DataColor::new(Color32::from_rgb(r, g, b)));
    paint_initials(draw, rect, name, DataColor::new(ink_on(fill, face.ink, face.contrast)), face);
}

fn paint_initials(draw: &Draw<'_>, rect: Rect, name: &str, ink: DataColor, face: &AvatarFace) {
    let text = initials(name);
    let runs = [DocRun::new(text, FontRef::proportional(), face.size, ink)];
    // The face size is the caption role's, so the caption run measures the initials exactly.
    let width = draw.layout(&TextRun::new(runs[0].text.as_ref()).role(TypeRole::Caption), f32::INFINITY).size().x;
    let at = Rect::from_min_size(
        Pos2::new(rect.center().x - width / 2.0, rect.center().y - face.line / 2.0),
        Vec2::new(width.min(rect.width()), face.line),
    );
    draw.doc(at, &DocText::new(&runs));
}

/// The style of an avatar.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct AvatarStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The fill of an avatar without a colour of its own.
    accent: StyleColor,
    /// The initials on the accent fill.
    on_accent: StyleColor,
    /// The ring of a selected or hovered avatar.
    ring: StyleColor,
    face_size: f32,
    face_line: f32,
    contrast: f64,
    ink: [u8; 3],
    side: f32,
}

/// What an avatar returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use]
pub struct AvatarOutput {
    /// The avatar's response.
    pub response: Response,
    /// The avatar was pressed by pointer or key.
    pub clicked: bool,
}

impl Outcome for AvatarOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A round well at the control height with the initials of a person's name, named by the name. `fill` gives it the
/// person's colour, else it takes the accent; `selected` draws a ring round it. A pressable avatar is a button.
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
pub struct Avatar {
    #[common]
    common: Common,
    /// The person's colour.
    #[prop]
    fill: Option<[u8; 3]>,
    /// The avatar is a button that reports a press.
    #[prop]
    pressable: bool,
}

impl Avatar {
    /// An avatar of the person `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Avatar { common: Common::new(), fill: None, pressable: false }.label(name.into())
    }
}

impl Component for Avatar {
    const ROLE: Role = Role::BUTTON;
    const ALT_ROLES: &'static [Role] = &[Role::IMAGE];
    const PARTS: bool = false;
    const OVERLAYS: bool = false;
    type Style = AvatarStyle;
    type Output = AvatarOutput;

    fn style(cx: &StyleCx<'_>, _: AvatarStyleKey) -> AvatarStyle {
        let colors = &cx.tokens.color;
        let control = cx.control();
        let face = AvatarFace::of(cx);
        AvatarStyle {
            metrics: Metrics::new().min_size(Vec2::splat(control.height)),
            focus: cx.focus_look(),
            accent: cx.color(colors.accent.solid),
            on_accent: cx.color(colors.text.on_strong),
            ring: cx.color(colors.accent.solid),
            face_size: face.size,
            face_line: face.line,
            contrast: face.contrast,
            ink: face.ink,
            side: control.height,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.style.side)
    }

    fn sense(&self) -> Sense {
        if self.pressable { Sense::CLICK | Sense::FOCUS } else { Sense::HOVER }
    }

    fn access(&self, access: &mut craft_ui::contract::Access, _: &Cx<'_, Self>) {
        if !self.pressable {
            access.role(Role::IMAGE);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let side = cx.content_rect.width().min(cx.content_rect.height());
        let face = Rect::from_center_size(cx.content_rect.center(), Vec2::splat(side));
        let face_look = AvatarFace { size: style.face_size, line: style.face_line, contrast: style.contrast, ink: style.ink };
        let draw = ui.draw();
        match self.fill {
            Some(fill) => paint_avatar(&draw, face, cx.resolved.label, fill, &face_look),
            None => {
                draw.fill(&PaintShape::Circle(face.center(), side / 2.0), style.accent);
                paint_initials(&draw, face, cx.resolved.label, DataColor::new(style.on_accent.egui()), &face_look);
            }
        }
        if cx.resolved.selected == Some(true) || (self.pressable && cx.response.hovered()) {
            draw.stroke(&PaintShape::Circle(face.center(), side / 2.0), StrokeWidth::Medium, style.ring);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> AvatarOutput {
        AvatarOutput { clicked: clicked(&cx.response), response: cx.response.clone() }
    }
}
