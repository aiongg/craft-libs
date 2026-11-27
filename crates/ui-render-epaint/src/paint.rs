//! A display list to epaint primitives, in the node's local coordinates (plus a sub-pixel shift,
//! see `lib.rs`), with every clip and opacity pushed inside the list folded in.

use std::sync::Arc;

use craft_theme::Rgba;
use craft_ui_rt::{Clip, CornerRadii, DisplayList, Fill, ImageKey, Item, Mesh as DisplayMesh, StrokeAlign, SurfaceKey, TextHighlight};
use emath::{Pos2, Rangef, Rect as ERect, Vec2};
use epaint::text::cursor::{CCursor, LayoutCursor};
use epaint::text::{CharIndex, Galley};
use epaint::{
    CircleShape, ClippedPrimitive, ClippedShape, Color32, CornerRadius, Mesh, PathStroke, Primitive, RectShape, Shape, Stroke, StrokeKind,
    Tessellator, TextShape, TextureId,
};
use kurbo::{BezPath, PathEl};

use crate::clip::{clip_to_outline, rounded_outline};
use crate::concave;
use crate::text::{EpaintText, Shaped};

/// The font atlas is texture 0. An image is the odd id `2k + 1`, its alpha-only variant the even
/// id `2k + 2`.
pub(crate) const FONT_TEXTURE: TextureId = TextureId::Managed(0);

pub(crate) fn image_texture(key: ImageKey) -> TextureId {
    TextureId::Managed(key.0.wrapping_mul(2).wrapping_add(1))
}

/// The image with its colour replaced by white and its alpha kept, for [`Item::ImageMask`].
pub(crate) fn mask_texture(key: ImageKey) -> TextureId {
    TextureId::Managed(key.0.wrapping_mul(2).wrapping_add(2))
}

/// What one tessellated piece draws.
pub(crate) enum Piece {
    Mesh(Mesh),
    /// The app-drawn surface `key` inside `rect` (local coordinates plus the shift).
    Surface {
        key: SurfaceKey,
        rect: ERect,
    },
}

/// A kept piece: its scissor in local coordinates (plus the shift) and the bounds that decide
/// whether it reaches into a damage rect.
pub(crate) struct Prim {
    pub clip_rect: ERect,
    pub bounds: ERect,
    pub piece: Piece,
}

/// The result of tessellating one display list.
#[derive(Default)]
pub(crate) struct Tessellated {
    pub prims: Vec<Prim>,
    /// Images drawn as masks, which need their alpha-only texture.
    pub masks: Vec<ImageKey>,
}

fn pos(p: Pos2, shift: Vec2) -> Pos2 {
    p + shift
}

fn rect(r: ERect, shift: Vec2) -> ERect {
    r.translate(shift)
}

fn color(c: Rgba, opacity: f32) -> Color32 {
    let [r, g, b, a] = c.0;
    let c = Color32::from_rgba_unmultiplied(r, g, b, a);
    if opacity < 1.0 { c.gamma_multiply(opacity) } else { c }
}

fn solid(c: Rgba) -> bool {
    c.0[3] != 0
}

fn stroke(s: Option<craft_ui_rt::Stroke>, opacity: f32) -> Stroke {
    s.map_or(Stroke::NONE, |s| Stroke::new(s.width, color(s.color, opacity)))
}

fn corners(r: CornerRadii) -> CornerRadius {
    let u = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    CornerRadius { nw: u(r.top_left), ne: u(r.top_right), se: u(r.bottom_right), sw: u(r.bottom_left) }
}

/// The subpaths of `path` as points: flattened curves, and whether each subpath was closed.
fn subpaths(path: &BezPath, shift: Vec2, tolerance: f64) -> Vec<(Vec<Pos2>, bool)> {
    let mut out: Vec<(Vec<Pos2>, bool)> = Vec::new();
    let at = |p: kurbo::Point| pos(Pos2::new(p.x as f32, p.y as f32), shift);
    kurbo::flatten(path.iter(), tolerance, |el| match el {
        PathEl::MoveTo(p) => out.push((vec![at(p)], false)),
        PathEl::LineTo(p) => {
            if let Some((points, _)) = out.last_mut() {
                points.push(at(p));
            }
        }
        PathEl::ClosePath => {
            if let Some((_, closed)) = out.last_mut() {
                *closed = true;
            }
        }
        PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
    });
    out
}

/// A stroke along `points`: solid, or cut into dashes.
fn stroke_shapes(points: Vec<Pos2>, closed: bool, s: craft_ui_rt::Stroke, opacity: f32, out: &mut Vec<Shape>) {
    let st = Stroke::new(s.width, color(s.color, opacity));
    match s.dash {
        Some(d) if d.on > 0.0 && d.off >= 0.0 => {
            let mut points = points;
            if closed && let Some(&first) = points.first() {
                points.push(first);
            }
            out.extend(Shape::dashed_line_with_offset(&points, st, &[d.on], &[d.off], d.offset));
        }
        _ if closed => out.push(Shape::closed_line(points, PathStroke::from(st))),
        _ => out.push(Shape::line(points, PathStroke::from(st))),
    }
}

/// The stroke of `rect` at its alignment, as a polygon: inside the edge, on it or outside it.
fn aligned_outline(r: ERect, radii: CornerRadii, s: &craft_ui_rt::Stroke, scale: f32) -> Vec<Pos2> {
    let grow = match s.align {
        StrokeAlign::Inside => -s.width / 2.0,
        StrokeAlign::Middle => 0.0,
        StrokeAlign::Outside => s.width / 2.0,
    };
    let radius = |v: f32| (v + grow).max(0.0);
    let radii = CornerRadii {
        top_left: radius(radii.top_left),
        top_right: radius(radii.top_right),
        bottom_right: radius(radii.bottom_right),
        bottom_left: radius(radii.bottom_left),
    };
    rounded_outline(r.expand(grow), radii, scale)
}

/// Everything `tessellate` needs besides the list.
pub(crate) struct Env<'a> {
    pub tessellator: &'a mut Tessellator,
    pub text: &'a mut EpaintText,
    pub scale: f32,
}

/// The clip state while walking a list: the rect scissor, and the rounded outlines inside it.
struct Clips {
    rects: Vec<ERect>,
    rounded: Vec<Option<Vec<Pos2>>>,
    rect: ERect,
    outlines: Vec<Vec<Pos2>>,
}

impl Clips {
    fn push(&mut self, c: &Clip, shift: Vec2, scale: f32) {
        self.rects.push(self.rect);
        self.rect = self.rect.intersect(rect(c.rect, shift));
        if c.radii.is_zero() {
            self.rounded.push(None);
        } else {
            let outline = rounded_outline(rect(c.rect, shift), c.radii, scale);
            self.outlines.push(outline.clone());
            self.rounded.push(Some(outline));
        }
    }

    fn pop(&mut self) {
        self.rect = self.rects.pop().unwrap_or(ERect::EVERYTHING);
        if let Some(Some(_)) = self.rounded.pop() {
            self.outlines.pop();
        }
    }
}

/// Tessellates `list` shifted by `shift` points. Meshes keep the font atlas' texture for text and
/// shapes, and an image's texture for images.
pub(crate) fn tessellate(list: &DisplayList, shift: Vec2, env: &mut Env<'_>) -> Tessellated {
    let mut out = Tessellated::default();
    let mut clips = Clips { rects: Vec::new(), rounded: Vec::new(), rect: ERect::EVERYTHING, outlines: Vec::new() };
    let mut opacities: Vec<f32> = Vec::new();
    let mut opacity = 1.0f32;
    let scale = env.scale;
    let tolerance = 0.1 / f64::from(scale.max(1.0));
    // Meshes tessellated since the last flush: epaint joins consecutive shapes with the same clip
    // rect and texture into one mesh, so `raw` keeps its pieces between shapes. Where the rounded
    // outlines change (a clip pushed or popped) or a surface comes, it is flushed first.
    let mut raw: Vec<ClippedPrimitive> = Vec::new();
    let mut shapes: Vec<Shape> = Vec::new();
    for item in &list.items {
        shapes.clear();
        match item {
            Item::PushClip(c) => {
                flush(&mut raw, &clips.outlines, &mut out.prims);
                clips.push(c, shift, scale);
            }
            Item::PopClip => {
                flush(&mut raw, &clips.outlines, &mut out.prims);
                clips.pop();
            }
            Item::PushOpacity { opacity: f, .. } => {
                opacities.push(opacity);
                opacity *= f;
            }
            Item::PopOpacity => opacity = opacities.pop().unwrap_or(1.0),
            Item::Rect { rect: r, radii, fill, stroke: s } => {
                let r = rect(*r, shift);
                match s {
                    Some(s) if s.dash.is_some() => {
                        if solid(*fill) {
                            shapes.push(Shape::Rect(RectShape::filled(r, corners(*radii), color(*fill, opacity))));
                        }
                        stroke_shapes(aligned_outline(r, *radii, s, scale), true, *s, opacity, &mut shapes);
                    }
                    _ => {
                        let kind = match s.map_or(StrokeAlign::Inside, |s| s.align) {
                            StrokeAlign::Inside => StrokeKind::Inside,
                            StrokeAlign::Middle => StrokeKind::Middle,
                            StrokeAlign::Outside => StrokeKind::Outside,
                        };
                        shapes.push(Shape::Rect(RectShape::new(r, corners(*radii), color(*fill, opacity), stroke(*s, opacity), kind)));
                    }
                }
            }
            Item::Circle { center, radius, fill, stroke: s } => {
                let center = pos(*center, shift);
                if solid(*fill) || s.is_none() {
                    shapes.push(Shape::Circle(CircleShape { center, radius: *radius, fill: color(*fill, opacity), stroke: Stroke::NONE }));
                }
                if let Some(s) = s {
                    let grow = match s.align {
                        StrokeAlign::Inside => -s.width / 2.0,
                        StrokeAlign::Middle => 0.0,
                        StrokeAlign::Outside => s.width / 2.0,
                    };
                    let r = (radius + grow).max(0.0);
                    if s.dash.is_some() {
                        let n = ((r * scale).sqrt() * 8.0).ceil().clamp(16.0, 256.0) as u32;
                        let points = (0..n).map(|i| {
                            let a = std::f32::consts::TAU * i as f32 / n as f32;
                            center + Vec2::new(r * a.cos(), r * a.sin())
                        });
                        stroke_shapes(points.collect(), true, *s, opacity, &mut shapes);
                    } else {
                        shapes.push(Shape::Circle(CircleShape { center, radius: r, fill: Color32::TRANSPARENT, stroke: stroke(Some(*s), opacity) }));
                    }
                }
            }
            // A solid line is a line segment, which epaint snaps to the pixel grid as egui's own
            // lines are (a path along two points is not snapped).
            Item::Line { from, to, stroke: s } if s.dash.is_none() => {
                shapes.push(Shape::line_segment([pos(*from, shift), pos(*to, shift)], Stroke::new(s.width, color(s.color, opacity))));
            }
            Item::Line { from, to, stroke: s } => {
                stroke_shapes(vec![pos(*from, shift), pos(*to, shift)], false, *s, opacity, &mut shapes);
            }
            Item::Path { path, fill, stroke: s } => {
                let subs = subpaths(path, shift, tolerance);
                if let Some(f) = fill {
                    path_fill(&subs, f, scale, opacity, &mut shapes);
                }
                if let Some(s) = s {
                    for (points, closed) in subs {
                        if points.len() >= 2 {
                            stroke_shapes(points, closed, *s, opacity, &mut shapes);
                        }
                    }
                }
            }
            Item::Mesh(mesh) => shapes.push(Shape::mesh(mesh_of(mesh, shift, opacity))),
            Item::Shadow { rect: r, radii, offset, blur, spread, color: c } => {
                let r = rect(*r, shift).translate(*offset).expand(*spread);
                let radii = CornerRadii {
                    top_left: radii.top_left + spread,
                    top_right: radii.top_right + spread,
                    bottom_right: radii.bottom_right + spread,
                    bottom_left: radii.bottom_left + spread,
                };
                shapes.push(Shape::Rect(RectShape::filled(r, corners(radii), color(*c, opacity)).with_blur_width(*blur)));
            }
            Item::VGradient { rect: r, top, bottom } => {
                // Premultiplied vertex colours interpolate across the quad, as egui's
                // `Shape::gradient_rect`.
                let r = rect(*r, shift);
                let (top, bottom) = (color(*top, opacity), color(*bottom, opacity));
                let mut mesh = Mesh::with_texture(FONT_TEXTURE);
                for (p, c) in [(r.left_top(), top), (r.right_top(), top), (r.right_bottom(), bottom), (r.left_bottom(), bottom)] {
                    mesh.colored_vertex(p, c);
                }
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                shapes.push(Shape::mesh(mesh));
            }
            Item::Image { key, rect: r, uv, tint } => {
                shapes.push(Shape::image(image_texture(*key), rect(*r, shift), *uv, color(*tint, opacity)));
            }
            Item::ImageMask { key, rect: r, uv, color: c } => {
                out.masks.push(*key);
                shapes.push(Shape::image(mask_texture(*key), rect(*r, shift), *uv, color(*c, opacity)));
            }
            Item::Text { layout, origin, color: c, highlight, .. } => {
                let Some(shaped) = layout.downcast_ref::<Shaped>() else {
                    continue;
                };
                let fresh = env.text.reshaped(shaped);
                let shaped = fresh.as_ref().unwrap_or(shaped);
                let mut galley = shaped.galley.clone();
                if let Some(h) = highlight {
                    highlight_galley(&mut galley, shaped, h, opacity);
                }
                let mut text = TextShape::new(pos(*origin, shift), galley, Color32::WHITE);
                if let Some(c) = c {
                    text = text.with_override_text_color(color(*c, opacity));
                } else if opacity < 1.0 {
                    text = text.with_opacity_factor(opacity);
                }
                shapes.push(Shape::Text(text));
            }
            Item::Surface { key, rect: r } => {
                let r = rect(*r, shift);
                flush(&mut raw, &clips.outlines, &mut out.prims);
                out.prims.push(Prim { clip_rect: clips.rect, bounds: r.intersect(clips.rect), piece: Piece::Surface { key: *key, rect: r } });
            }
        }
        for shape in shapes.drain(..) {
            env.tessellator.tessellate_clipped_shape(ClippedShape::new(clips.rect, shape), &mut raw);
        }
    }
    flush(&mut raw, &clips.outlines, &mut out.prims);
    out.masks.sort_unstable();
    out.masks.dedup();
    out
}

/// Moves the meshes in `raw` to `prims`, clipped to the rounded `outlines`.
fn flush(raw: &mut Vec<ClippedPrimitive>, outlines: &[Vec<Pos2>], prims: &mut Vec<Prim>) {
    for p in raw.drain(..) {
        let Primitive::Mesh(mut mesh) = p.primitive else { continue };
        for outline in outlines {
            clip_to_outline(&mut mesh, outline);
        }
        if mesh.is_empty() {
            continue;
        }
        prims.push(Prim { clip_rect: p.clip_rect, bounds: mesh.calc_bounds(), piece: Piece::Mesh(mesh) });
    }
}

fn mesh_of(mesh: &DisplayMesh, shift: Vec2, opacity: f32) -> Mesh {
    let texture = mesh.texture.map_or(FONT_TEXTURE, image_texture);
    let mut out = Mesh::with_texture(texture);
    let textured = mesh.texture.is_some();
    out.vertices = mesh
        .vertices
        .iter()
        .map(|v| epaint::Vertex { pos: pos(v.pos, shift), uv: if textured { v.uv } else { epaint::WHITE_UV }, color: color(v.color, opacity) })
        .collect();
    out.indices = mesh.indices.clone();
    out
}

/// The fill of a path: convex single contours through epaint, anything else as trapezoids with
/// an anti-aliasing fringe.
fn path_fill(subs: &[(Vec<Pos2>, bool)], fill: &Fill, scale: f32, opacity: f32, out: &mut Vec<Shape>) {
    let c = color(fill.color, opacity);
    if fill.convex {
        for (points, _) in subs {
            if points.len() >= 3 {
                out.push(Shape::convex_polygon(points.clone(), c, Stroke::NONE));
            }
        }
        return;
    }
    let contours: Vec<Vec<Pos2>> = subs.iter().filter(|(p, _)| p.len() >= 3).map(|(p, _)| p.clone()).collect();
    let mut mesh = Mesh::with_texture(FONT_TEXTURE);
    for [a, b, d] in concave::triangulate(&contours, fill.rule) {
        let base = mesh.vertices.len() as u32;
        for p in [a, b, d] {
            mesh.colored_vertex(p, c);
        }
        mesh.add_triangle(base, base + 1, base + 2);
    }
    for [p, q, q2, p2] in concave::fringe(&contours, fill.rule, 1.0 / scale) {
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(p, c);
        mesh.colored_vertex(q, c);
        mesh.colored_vertex(q2, Color32::TRANSPARENT);
        mesh.colored_vertex(p2, Color32::TRANSPARENT);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    if !mesh.is_empty() {
        out.push(Shape::mesh(mesh));
    }
}

/// Fills `h` behind the glyphs inside the galley's row meshes, each row's full height, as egui
/// paints a text selection (`text_selection::visuals::paint_text_selection`): the fill then sits
/// where the tessellator snaps the text, not at the unsnapped origin.
fn highlight_galley(galley: &mut Arc<Galley>, shaped: &Shaped, h: &TextHighlight, opacity: f32) {
    if h.range.is_empty() {
        return;
    }
    let fill = color(h.fill, opacity);
    let galley = Arc::make_mut(galley);
    let min = galley.layout_from_cursor(CCursor::new(shaped.char_index(h.range.start)));
    let max = galley.layout_from_cursor(CCursor::new(shaped.char_index(h.range.end)));
    for ri in min.row..=max.row {
        // Where the caret goes on this row if it is empty, row-relative.
        let empty_row_x = galley.pos_from_layout_cursor(&LayoutCursor { row: ri, column: CharIndex(0) }).min.x - galley.rows[ri].pos.x;
        let placed = &mut galley.rows[ri];
        let ends_with_newline = placed.ends_with_newline;
        let row = Arc::make_mut(&mut placed.row);
        let first = if ri == min.row { min.column } else { CharIndex(0) };
        let ranges: Vec<Rangef> = if ri == max.row {
            row.x_ranges(first..max.column).into_iter().collect()
        } else {
            let mut ranges: Vec<Rangef> = row.x_ranges(first..row.char_count_excluding_newline()).into_iter().collect();
            // A selected line break shows as half a row's height past the line's end.
            let newline = if ends_with_newline { row.height() * 0.5 } else { 0.0 };
            if row.rtl_paragraph {
                let left = row.glyphs.iter().map(|g| g.pos.x).reduce(f32::min).unwrap_or(empty_row_x);
                match ranges.first_mut() {
                    Some(range) if range.min <= left => range.min = left - newline,
                    _ if 0.0 < newline => ranges.insert(0, Rangef::new(left - newline, left)),
                    _ => {}
                }
            } else if !row.glyphs.iter().any(|g| g.is_rtl()) {
                if let Some(range) = ranges.last_mut() {
                    range.max = row.size.x + newline;
                }
            } else if 0.0 < newline {
                ranges.push(Rangef::new(row.size.x, row.size.x + newline));
            }
            ranges
        };
        let height = row.size.y;
        // After the row's background, before its glyphs.
        let glyphs_start = row.visuals.glyph_index_start;
        let mesh = &mut row.visuals.mesh;
        for range in ranges {
            mesh.add_colored_rect(ERect::from_x_y_ranges(range, 0.0..=height), fill);
            mesh.indices[glyphs_start..].rotate_right(6);
        }
        row.visuals.mesh_bounds = mesh.calc_bounds();
    }
}
