//! [`Surface`]: semantic surfaces, and the only reader of the theme's edge style.

use craft_theme::Edge;
use egui::emath::GuiRounding as _;
use egui::epaint::{Mesh, RectShape};
use egui::{Color32, CornerRadius, InnerResponse, Painter, Pos2, Rect, Shadow, Shape, Stroke, StrokeKind, Ui, Vec2, vec2};

use super::Corner;
use crate::canvas::story::NoArgs;
use crate::inspect::{self, RecordKind};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// What a surface is, which sets every part of its look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SurfaceKind {
    /// The window behind everything: `surface.background`, no edge.
    Background,
    /// Docked panels and bars: `surface.panel`, no edge.
    Panel,
    /// Raised controls and cards: `surface.raised`, outlined or bevelled out.
    Raised,
    /// Fields and wells: `surface.sunken`, outlined with `line.control_edge` or bevelled in.
    Sunken,
    /// A raised surface while pressed: `state.pressed`, outlined with `line.control_edge` or
    /// bevelled in.
    Pressed,
    /// Popups and menus: `popup.fill` and `popup.border` (outlined in every edge style) with the
    /// popup shadow.
    Popup,
    /// Dialogs, which are popup frames in PhotoCraft: the popup look (`popup.fill`, `popup.border`
    /// in every edge style, the popup shadow and radius).
    Dialog,
    /// A knob that rides on a track (slider and switch thumbs): `surface.knob` with the knob
    /// shadow, outlined with `line.knob_edge` or bevelled out. Controls give it their own corners.
    Knob,
}

impl SurfaceKind {
    /// Every kind, in declaration order.
    pub const ALL: &'static [SurfaceKind] = &[
        SurfaceKind::Background,
        SurfaceKind::Panel,
        SurfaceKind::Raised,
        SurfaceKind::Sunken,
        SurfaceKind::Pressed,
        SurfaceKind::Popup,
        SurfaceKind::Dialog,
        SurfaceKind::Knob,
    ];

    /// The name in story args and dumps, e.g. `"popup"`.
    pub const fn name(self) -> &'static str {
        match self {
            SurfaceKind::Background => "background",
            SurfaceKind::Panel => "panel",
            SurfaceKind::Raised => "raised",
            SurfaceKind::Sunken => "sunken",
            SurfaceKind::Pressed => "pressed",
            SurfaceKind::Popup => "popup",
            SurfaceKind::Dialog => "dialog",
            SurfaceKind::Knob => "knob",
        }
    }

    /// The corners: `None` for the square `Background`.
    pub const fn corner(self) -> Option<Corner> {
        match self {
            SurfaceKind::Background => None,
            SurfaceKind::Panel => Some(Corner::Panel),
            SurfaceKind::Raised | SurfaceKind::Sunken | SurfaceKind::Pressed | SurfaceKind::Knob => Some(Corner::Control),
            SurfaceKind::Popup | SurfaceKind::Dialog => Some(Corner::Popup),
        }
    }
}

/// A surface's edge, resolved from the theme's edge style.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum SurfaceEdge {
    /// Nothing around the fill.
    None,
    /// An outline inside the rect.
    Border(Stroke),
    /// A two-tone bevel inside the rect: `top_left` on the top and left sides, `bottom_right` on
    /// the bottom and right, meeting on the diagonals of the top-right and bottom-left corners.
    /// `width` is in points; it is drawn rounded to whole physical pixels, at least one.
    Bevel { top_left: Color32, bottom_right: Color32, width: f32 },
}

/// Everything a surface draws, from [`Surface::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SurfaceStyle {
    pub fill: Color32,
    pub radius: CornerRadius,
    pub edge: SurfaceEdge,
    /// Drawn under the fill; `None` where the elevation casts none.
    pub shadow: Option<Shadow>,
}

impl SurfaceStyle {
    /// The shapes of the surface over `rect`, snapped to the pixel grid: shadow, fill, edge.
    /// A bevel is whole physical pixels wide.
    pub fn shapes(&self, rect: Rect, pixels_per_point: f32) -> Vec<Shape> {
        let rect = rect.round_to_pixels(pixels_per_point);
        let mut out = Vec::with_capacity(3);
        if let Some(shadow) = self.shadow {
            out.push(Shape::Rect(shadow.as_shape(rect, self.radius)));
        }
        out.push(Shape::Rect(RectShape::filled(rect, self.radius, self.fill)));
        match self.edge {
            SurfaceEdge::None => {}
            SurfaceEdge::Border(stroke) => out.push(Shape::Rect(RectShape::stroke(rect, self.radius, stroke, StrokeKind::Inside))),
            SurfaceEdge::Bevel { top_left, bottom_right, width } => {
                let width = (width * pixels_per_point).round().max(1.0) / pixels_per_point;
                out.push(Shape::mesh(bevel(rect, self.radius, width, top_left, bottom_right)));
            }
        }
        out
    }
}

/// A semantic surface: `Surface::new(kind).show(ui, |ui| ..)` draws it behind the contents, and
/// [`Surface::paint`] draws it over a given rect.
///
/// Components ask for a kind, never for a fill or a border. With the theme's
/// [`Edge::Bevel`], raised and knob surfaces are lit on the top and left, sunken and pressed ones
/// on the bottom and right, and none has a border; popups and dialogs keep their one point border
/// (PhotoCraft Classic draws windows flat). The bevel is lighting, so it does not mirror in a
/// right-to-left interface.
#[derive(Clone, Copy, Debug)]
pub struct Surface {
    kind: SurfaceKind,
}

impl Surface {
    pub fn new(kind: SurfaceKind) -> Self {
        Self { kind }
    }

    /// The look of `kind` in `tokens`.
    pub fn style(tokens: &UiTokens, kind: SurfaceKind) -> SurfaceStyle {
        use SurfaceKind as K;
        let c = &tokens.color;
        let fill = match kind {
            K::Background => c.surface.background,
            K::Panel => c.surface.panel,
            K::Raised => c.surface.raised,
            K::Popup | K::Dialog => tokens.component.popup.fill,
            K::Sunken => c.surface.sunken,
            K::Pressed => c.state.pressed,
            K::Knob => c.surface.knob,
        };
        let radius = kind.corner().map_or(CornerRadius::ZERO, |corner| corner.radius(tokens));
        let shadow = match kind {
            K::Popup | K::Dialog => Some(tokens.shadow.popup),
            K::Knob => Some(tokens.shadow.knob),
            _ => None,
        }
        .filter(|shadow| shadow.color.a() > 0);
        // Container and control outlines are one point wide.
        let width = craft_theme::Stroke::W1.width(1.0);
        let edge = match (tokens.shape.edge, kind) {
            (_, K::Background | K::Panel) => SurfaceEdge::None,
            (_, K::Popup | K::Dialog) => SurfaceEdge::Border(Stroke::new(width, tokens.component.popup.border.to_egui())),
            (Edge::Flat, K::Raised) => SurfaceEdge::Border(Stroke::new(width, c.line.border.to_egui())),
            (Edge::Flat, K::Sunken | K::Pressed) => SurfaceEdge::Border(Stroke::new(width, c.line.control_edge.to_egui())),
            (Edge::Flat, K::Knob) => SurfaceEdge::Border(Stroke::new(width, c.line.knob_edge.to_egui())),
            (Edge::Bevel { light, dark, width }, K::Raised | K::Knob) => {
                SurfaceEdge::Bevel { top_left: light.to_egui(), bottom_right: dark.to_egui(), width }
            }
            (Edge::Bevel { light, dark, width }, K::Sunken | K::Pressed) => {
                SurfaceEdge::Bevel { top_left: dark.to_egui(), bottom_right: light.to_egui(), width }
            }
        };
        SurfaceStyle { fill: fill.to_egui(), radius, edge, shadow }
    }

    /// Draws the surface over `rect`.
    pub fn paint(painter: &Painter, rect: Rect, kind: SurfaceKind, tokens: &UiTokens) {
        painter.extend(Surface::style(tokens, kind).shapes(rect, painter.pixels_per_point()));
    }

    /// Shows `add_contents` on the surface, which takes the contents' size. Pad with
    /// [`Stack::pad`](super::Stack::pad).
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let tokens = Tokens::of(ui);
        let slot = ui.painter().add(Shape::Noop);
        let inner = ui.scope(add_contents);
        let shapes = Surface::style(&tokens, self.kind).shapes(inner.response.rect, ui.pixels_per_point());
        ui.painter().set(slot, Shape::Vec(shapes));
        let rect = inner.response.rect;
        inspect::record(ui, inner.response.id, RecordKind::Surface(self.kind), rect, rect);
        inner
    }
}

/// The bevel inside `rect`: two strips between the outline and the outline inset by `width`,
/// split at 45° on the top-right and bottom-left corners. Both outlines are sampled at the same
/// angles, so square corners get mitred joins and round ones follow the radius.
fn bevel(rect: Rect, radius: CornerRadius, width: f32, top_left: Color32, bottom_right: Color32) -> Mesh {
    let width = width.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    let inner = rect.shrink(width);
    let max_r = rect.width().min(rect.height()) / 2.0;
    // Each corner as (outer centre, outer radius, inner centre, inner radius, start angle), from
    // its outline corners, the direction into the rect and its radius. Angles run clockwise with
    // y down: 180° points left.
    let corner = |point: Pos2, inner_point: Pos2, into: Vec2, radius: u8, start: f32| {
        let outer_r = f32::from(radius).min(max_r);
        let inner_r = (outer_r - width).max(0.0);
        (point + into * outer_r, outer_r, inner_point + into * inner_r, inner_r, start)
    };
    let corners = [
        corner(rect.left_top(), inner.left_top(), vec2(1.0, 1.0), radius.nw, 180.0),
        corner(rect.right_top(), inner.right_top(), vec2(-1.0, 1.0), radius.ne, 270.0),
        corner(rect.right_bottom(), inner.right_bottom(), vec2(-1.0, -1.0), radius.se, 0.0),
        corner(rect.left_bottom(), inner.left_bottom(), vec2(1.0, -1.0), radius.sw, 90.0),
    ];
    let at = |centre: Pos2, r: f32, degrees: f32| centre + r * vec2(degrees.to_radians().cos(), degrees.to_radians().sin());
    // The arc of corner `i` from `from` to `to` degrees past its start, on both outlines.
    let arc = |i: usize, from: f32, to: f32, out: &mut Vec<(Pos2, Pos2)>| {
        let (oc, or, ic, ir, start) = corners[i];
        let steps = ((or * (to - from) / 90.0 / 2.0).ceil() as usize).clamp(1, 16);
        for s in 0..=steps {
            let a = start + from + (to - from) * s as f32 / steps as f32;
            out.push((at(oc, or, a), at(ic, ir, a)));
        }
    };
    let mut lit = Vec::new();
    arc(3, 45.0, 90.0, &mut lit);
    arc(0, 0.0, 90.0, &mut lit);
    arc(1, 0.0, 45.0, &mut lit);
    let mut shaded = Vec::new();
    arc(1, 45.0, 90.0, &mut shaded);
    arc(2, 0.0, 90.0, &mut shaded);
    arc(3, 0.0, 45.0, &mut shaded);

    let mut mesh = Mesh::default();
    for (strip, color) in [(lit, top_left), (shaded, bottom_right)] {
        let base = mesh.vertices.len() as u32;
        for (outer, inner) in &strip {
            mesh.colored_vertex(*outer, color);
            mesh.colored_vertex(*inner, color);
        }
        for i in 0..strip.len().saturating_sub(1) as u32 {
            let (o0, i0, o1, i1) = (base + 2 * i, base + 2 * i + 1, base + 2 * i + 2, base + 2 * i + 3);
            mesh.add_triangle(o0, i0, o1);
            mesh.add_triangle(i0, i1, o1);
        }
    }
    mesh
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Surface"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum KindArg {
    Background,
    Panel,
    Raised,
    Sunken,
    Pressed,
    Popup,
    Dialog,
    Knob,
}

impl KindArg {
    fn kind(self) -> SurfaceKind {
        match self {
            KindArg::Background => SurfaceKind::Background,
            KindArg::Panel => SurfaceKind::Panel,
            KindArg::Raised => SurfaceKind::Raised,
            KindArg::Sunken => SurfaceKind::Sunken,
            KindArg::Pressed => SurfaceKind::Pressed,
            KindArg::Popup => SurfaceKind::Popup,
            KindArg::Dialog => SurfaceKind::Dialog,
            KindArg::Knob => SurfaceKind::Knob,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct KindArgs {
    kind: KindArg,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/surface/kind", Level::Primitives, "Surface", "Kind", KindArgs { kind: KindArg::Raised }, show_kind),
        typed_story!("primitives/surface/surfaces", Level::Primitives, "Surface", "Surfaces", NoArgs {}, show_surfaces),
    ]
}

fn show_kind(ui: &mut Ui, args: &mut KindArgs) {
    sample(ui, args.kind.kind());
}

/// Every kind on `Background`, so a bevel shows against the window colour.
fn show_surfaces(ui: &mut Ui, _: &mut NoArgs) {
    Surface::new(SurfaceKind::Background).show(ui, |ui| {
        super::Stack::new(super::Gap::Stack).pad(super::Pad::Panel).show(ui, |ui| {
            for &kind in SurfaceKind::ALL {
                sample(ui, kind);
            }
        });
    });
}

fn sample(ui: &mut Ui, kind: SurfaceKind) {
    if kind == SurfaceKind::Knob {
        // A knob carries no text: a round one at the icon size, named beside it.
        super::Row::new(super::Gap::Inline).show(ui, |ui| {
            let tokens = Tokens::of(ui);
            let side = tokens.size.icon.md;
            let (rect, _) = ui.allocate_exact_size(vec2(side, side), egui::Sense::hover());
            let mut style = Surface::style(&tokens, kind);
            style.radius = crate::theme::corner_radius(side / 2.0);
            ui.painter().extend(style.shapes(rect, ui.pixels_per_point()));
            ui.add(super::Text::new(kind.name()));
        });
        return;
    }
    Surface::new(kind).show(ui, |ui| {
        super::Stack::new(super::Gap::Inline).pad(super::Pad::Control).show(ui, |ui| {
            ui.set_min_width(160.0);
            ui.add(super::Text::new(kind.name()));
        });
    });
}
