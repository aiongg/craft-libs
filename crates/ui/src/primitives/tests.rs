use std::path::{Path, PathBuf};

use craft_theme::{Appearance, Contrast, Edge, Mode, Size, TextSize, TypeRole, builtins, resolve};
use egui::emath::GuiRounding as _;
use egui::epaint::RectShape;
use egui::{Color32, Context, CornerRadius, RawInput, Rect, Shape, Stroke, TextureId, Ui, ViewportId, pos2, vec2};

use super::*;
use crate::direction::{self, Direction};
use crate::icons::{self, IconSize};
use crate::theme::{FontFamilies, ToEgui, TokensOf, UiTokens};

fn tokens(id: &str, mode: Mode) -> UiTokens {
    tokens_at(id, mode, Contrast::Standard)
}

fn high_contrast_tokens(id: &str, mode: Mode) -> UiTokens {
    tokens_at(id, mode, Contrast::High)
}

fn tokens_at(id: &str, mode: Mode, contrast: Contrast) -> UiTokens {
    let theme = builtins().iter().find(|t| t.id.as_str() == id).unwrap();
    let appearance = Appearance { mode, contrast, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
    UiTokens::new(resolve(theme, appearance, None).tokens, appearance, mode, FontFamilies::default())
}

/// Every shape of one pass at `pixels_per_point`, nested lists flattened.
fn shapes(ctx: &Context, pixels_per_point: f32, mut add: impl FnMut(&mut Ui)) -> Vec<Shape> {
    fn flatten(shape: Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.into_iter().for_each(|s| flatten(s, out)),
            shape => out.push(shape),
        }
    }
    let mut input = RawInput::default();
    input.viewports.entry(ViewportId::ROOT).or_default().native_pixels_per_point = Some(pixels_per_point);
    let mut output = ctx.run_ui(input, |ui| add(ui));
    output.textures_delta.clear();
    let mut out = Vec::new();
    for clipped in output.shapes {
        flatten(clipped.shape, &mut out);
    }
    out
}

#[test]
fn surface_style_per_kind_flat_and_bevelled() {
    use SurfaceKind as K;
    let t = tokens("craft.graphite", Mode::Dark);
    assert_eq!(t.shape.edge, Edge::Flat);
    let c = &t.color;
    let border = |color: craft_theme::Rgba| SurfaceEdge::Border(Stroke::new(1.0, color.to_egui()));
    let expected = [
        (K::Background, c.surface.background, CornerRadius::ZERO, SurfaceEdge::None, None),
        (K::Panel, c.surface.panel, t.radius.panel, SurfaceEdge::None, None),
        (K::Raised, c.surface.raised, t.radius.control, border(c.line.border), None),
        (K::Sunken, c.surface.sunken, t.radius.control, border(c.line.control_edge), None),
        (K::Pressed, c.state.pressed, t.radius.control, border(c.line.control_edge), None),
        (K::Popup, t.component.popup.fill, t.radius.popup, border(t.component.popup.border), Some(t.shadow.popup)),
        (K::Dialog, c.surface.raised, t.radius.window, border(c.line.border), Some(t.shadow.dialog)),
        (K::Knob, c.surface.knob, t.radius.control, border(c.line.knob_edge), Some(t.shadow.knob)),
    ];
    for (kind, fill, radius, edge, shadow) in expected {
        let style = Surface::style(&t, kind);
        assert_eq!((style.fill, style.radius, style.edge, style.shadow), (fill.to_egui(), radius, edge, shadow), "{kind:?}");
    }

    let classic = tokens("craft.classic", Mode::Light);
    let Edge::Bevel { light, dark, width } = classic.shape.edge else { panic!("Classic bevels: {:?}", classic.shape.edge) };
    let (light, dark) = (light.to_egui(), dark.to_egui());
    assert_ne!(dark, classic.color.surface.panel.to_egui(), "the shaded edge shows on the panel");
    for &kind in K::ALL {
        let want = match kind {
            K::Background | K::Panel => SurfaceEdge::None,
            K::Raised | K::Knob => SurfaceEdge::Bevel { top_left: light, bottom_right: dark, width },
            K::Popup => SurfaceEdge::Border(Stroke::new(1.0, classic.component.popup.border.to_egui())),
            K::Dialog => SurfaceEdge::Border(Stroke::new(1.0, classic.color.line.border.to_egui())),
            K::Sunken | K::Pressed => SurfaceEdge::Bevel { top_left: dark, bottom_right: light, width },
        };
        assert_eq!(Surface::style(&classic, kind).edge, want, "{kind:?}");
    }
    for kind in [K::Popup, K::Dialog, K::Knob] {
        assert_eq!(Surface::style(&classic, kind).shadow, None, "{kind:?} casts no shadow in Classic");
    }

    // A bevel draws no outline: a fill and a mesh inside the rect, lit and shaded halves.
    let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(40.0, 20.0));
    let drawn = Surface::style(&classic, K::Raised).shapes(rect, 1.0);
    let [Shape::Rect(RectShape { stroke, .. }), Shape::Mesh(mesh)] = &drawn[..] else { panic!("{drawn:?}") };
    assert_eq!(*stroke, Stroke::NONE);
    assert!(mesh.vertices.iter().all(|v| rect.contains(v.pos)), "{:?}", mesh.vertices);
    assert!(mesh.vertices.iter().any(|v| v.color == light && v.pos == rect.min), "lit top left");
    assert!(mesh.vertices.iter().any(|v| v.color == dark && v.pos == rect.max), "shaded bottom right");
}

/// The bevel's inner outline is whole physical pixels inside the rect, at least one, at any scale.
#[test]
fn a_bevel_is_whole_physical_pixels_wide() {
    let classic = tokens("craft.classic", Mode::Light);
    let mut style = Surface::style(&classic, SurfaceKind::Raised);
    style.radius = CornerRadius::ZERO;
    let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(40.0, 20.0));
    for (width, ppp, pixels) in [(1.0, 1.0, 1.0), (1.0, 1.5, 2.0), (1.0, 1.25, 1.0), (0.4, 1.0, 1.0), (2.0, 1.5, 3.0)] {
        let SurfaceEdge::Bevel { top_left, bottom_right, .. } = style.edge else { panic!("{:?}", style.edge) };
        style.edge = SurfaceEdge::Bevel { top_left, bottom_right, width };
        let drawn = style.shapes(rect, ppp);
        let Some(Shape::Mesh(mesh)) = drawn.last() else { panic!("{drawn:?}") };
        let left = rect.round_to_pixels(ppp).left();
        let inner = mesh.vertices.iter().map(|v| v.pos.x).filter(|&x| x > left).fold(f32::INFINITY, f32::min);
        assert!(((inner - left) * ppp - pixels).abs() < 1e-3, "{width} pt at {ppp}x: {} px", (inner - left) * ppp);
    }
}

/// State fills replace the rest fill: fully hovered is `state.hover` whatever the rest fill, and
/// pressed is `state.pressed`; at rest the element keeps its own fill.
#[test]
fn state_fills_replace_the_rest_fill_with_the_hover_easing() {
    let t = tokens("craft.graphite", Mode::Light);
    let style = StateLayer::style(&t);
    assert_eq!(style.easing, t.motion.hover.easing);
    let raised = Surface::style(&t, SurfaceKind::Raised).fill;
    for (surface, rest) in [(None, Color32::TRANSPARENT), (Some(SurfaceKind::Raised), raised)] {
        let fills = StateLayer::fills(&t, surface);
        assert_eq!((fills.rest, fills.disabled), (rest, rest), "{surface:?}");
        assert_eq!(state_layer::mix(&fills, 0.0, 0.0), rest);
        assert_eq!(state_layer::mix(&fills, 1.0, 0.0), style.hover);
        assert_eq!(state_layer::mix(&fills, 1.0, 1.0), style.pressed);
        assert_eq!(state_layer::mix(&fills, 0.0, 1.0), style.pressed);
    }
    let bare = StateLayer::fills(&t, None);
    assert_eq!(state_layer::mix(&bare, 0.5, 0.0), style.hover.gamma_multiply(0.5), "a bare element fades in");
}

#[test]
fn corners_are_their_radius_tokens_and_surfaces_have_theirs() {
    let t = tokens("craft.studio", Mode::Light);
    let r = &t.radius;
    assert_eq!(Corner::ALL.iter().map(|c| c.radius(&t)).collect::<Vec<_>>(), [r.control, r.button, r.popup, r.window, r.panel]);
    for &kind in SurfaceKind::ALL {
        assert_eq!(Surface::style(&t, kind).radius, kind.corner().map_or(CornerRadius::ZERO, |c| c.radius(&t)), "{kind:?}");
    }
}

/// Decision "Bevel": the edge style is read only by the `Surface` primitive, so no other source
/// file of the crate names it.
#[test]
fn only_surface_reads_the_edge_style() {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    // Split so this file doesn't match itself.
    let patterns = [concat!("shape", ".edge"), concat!("Edge::", "Bevel"), concat!("Edge::", "Flat")];
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut files = Vec::new();
    walk(root, &mut files);
    let mut found = Vec::new();
    for file in files.iter().filter(|f| **f != root.join("primitives/surface.rs") && **f != root.join("primitives/tests.rs")) {
        for (n, line) in std::fs::read_to_string(file).unwrap().lines().enumerate() {
            // Whole names only: `SurfaceEdge::Bevel` is the resolved style, not the theme's.
            let names = |p: &str| line.match_indices(p).any(|(i, _)| !line[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_'));
            if patterns.iter().any(|p| names(p)) {
                found.push(format!("{}:{}: {line}", file.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "only primitives/surface.rs may read the edge style:\n{}", found.join("\n"));
}

#[test]
fn a_divider_is_one_physical_pixel_on_the_pixel_grid() {
    for ppp in [1.0, 1.5, 2.0] {
        assert_eq!(Divider::style(&tokens("craft.artcraft", Mode::Light), ppp).stroke.width * ppp, 1.0, "{ppp}");
        let mut colour = Color32::TRANSPARENT;
        let lines: Vec<_> = shapes(&Context::default(), ppp, |ui| {
            colour = Tokens::of(ui).color.line.divider.to_egui();
            ui.add_space(3.3);
            Divider::horizontal().show(ui);
            ui.horizontal(|ui| {
                ui.add_space(7.7);
                Divider::vertical().show(ui);
            });
        })
        .into_iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke } => Some((points, stroke)),
            _ => None,
        })
        .collect();
        assert_eq!(lines.len(), 2, "{ppp}: {lines:?}");
        let centre = |v: f32| ((v * ppp).fract() - 0.5).abs() < 1e-3;
        let edge = |v: f32| (v * ppp - (v * ppp).round()).abs() < 1e-3;
        for ([a, b], stroke) in lines {
            assert_eq!((stroke.width * ppp, stroke.color), (1.0, colour), "{ppp}");
            if a.y == b.y {
                assert!(centre(a.y) && edge(a.x) && edge(b.x), "{ppp}: {a:?} {b:?}");
            } else {
                assert!(a.x == b.x && centre(a.x) && edge(a.y) && edge(b.y), "{ppp}: {a:?} {b:?}");
            }
        }
    }
}

/// The textures `Icon` drew in one pass.
fn icon_textures(ctx: &Context, icon: icons::Icon) -> Vec<TextureId> {
    shapes(ctx, 1.0, |ui| {
        ui.add(Icon::new(icon).size(IconSize::Md));
    })
    .into_iter()
    .filter_map(|s| match s {
        Shape::Mesh(mesh) if mesh.texture_id != TextureId::default() => Some(mesh.texture_id),
        _ => None,
    })
    .collect()
}

#[test]
fn icons_mirror_right_to_left_only_when_directional() {
    let ctx = Context::default();
    let ltr = (icon_textures(&ctx, icons::Icon::ChevronLeft), icon_textures(&ctx, icons::Icon::Close));
    direction::apply(&ctx, Direction::Rtl);
    let rtl = (icon_textures(&ctx, icons::Icon::ChevronLeft), icon_textures(&ctx, icons::Icon::Close));
    assert_eq!((ltr.0.len(), ltr.1.len(), rtl.0.len(), rtl.1.len()), (1, 1, 1, 1));
    assert_ne!(ltr.0, rtl.0, "chevron_left is drawn from its mirrored raster");
    assert_eq!(ltr.1, rtl.1, "close is not directional");
}

#[test]
fn content_colours_are_their_tokens() {
    let t = tokens("craft.studio", Mode::Dark);
    let c = &t.color;
    let want = [c.text.normal, c.text.muted, c.text.faint, c.text.disabled, c.accent.text, c.text.on_accent, c.text.on_status, c.text.on_selected];
    assert_eq!(Content::ALL.iter().map(|content| content.color(&t)).collect::<Vec<_>>(), want);
    assert_eq!(Content::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(), CONTENT_NAMES);
    for &content in Content::ALL {
        assert_eq!(Content::from_name(content.name()), Some(content));
        assert_eq!(Text::format(&t, TypeRole::Label, content).color, content.color(&t).to_egui());
    }
}

#[test]
fn handles_are_the_token_size_with_a_halo_outside_in_high_contrast_only() {
    let t = tokens("craft.artcraft", Mode::Dark);
    let (h, o) = (t.component.handle, &t.color.overlay);
    let drawn = Handle::shapes(&t, HandleShape::Square, pos2(20.3, 20.6), 1.0);
    let [Shape::Rect(body), Shape::Rect(outline)] = &drawn[..] else { panic!("{drawn:?}") };
    assert_eq!(body.rect.size(), vec2(h.size, h.size));
    assert_eq!(body.rect.min, body.rect.min.round(), "on the pixel grid");
    assert_eq!(body.fill, o.handle_fill.to_egui());
    assert_eq!(outline.stroke, Stroke::new(h.stroke, o.handle_stroke.to_egui()));
    assert_eq!(Handle::style(&t, HandleShape::Rotate).fill, o.handle_stroke.to_egui());
    assert_ne!(Handle::style(&t, HandleShape::Rotate).cursor, Handle::style(&t, HandleShape::Round).cursor);
    assert_eq!(Handle::style(&t, HandleShape::Square).hit, h.hit);

    let high = high_contrast_tokens("craft.artcraft", Mode::Dark);
    let drawn = Handle::shapes(&high, HandleShape::Square, pos2(20.3, 20.6), 1.0);
    let [Shape::Rect(halo), Shape::Rect(body), Shape::Rect(_)] = &drawn[..] else { panic!("{drawn:?}") };
    assert_eq!(halo.rect, body.rect.expand(h.halo));
    assert_eq!(halo.fill, high.color.overlay.halo.to_egui());
}

/// A selected anchor is the handle filled with its outline colour, the same size and halo.
#[test]
fn a_selected_handle_is_filled_with_its_outline_colour() {
    let t = tokens("craft.artcraft", Mode::Dark);
    let o = &t.color.overlay;
    for shape in [HandleShape::Square, HandleShape::Round] {
        let (plain, selected) = (Handle::style(&t, shape), Handle::selected_style(&t, shape));
        assert_eq!(selected.fill, o.handle_stroke.to_egui());
        assert_eq!(HandleStyle { fill: plain.fill, ..selected }, plain);
    }
}
