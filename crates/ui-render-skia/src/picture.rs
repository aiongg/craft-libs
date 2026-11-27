//! Display list to a recorded Skia picture, in the node's local logical coordinates; the renderer
//! plays it back under `scale * translate(origin)`.

use std::collections::HashSet;

use craft_theme::Rgba;
use craft_ui_rt::{Clip, CornerRadii, DisplayList, FillRule, ImageKey, ImageOptions, Item, Media, Mesh, Stroke, StrokeAlign, SurfaceKey};
use emath::{Pos2, Rect, Vec2};
use kurbo::{BezPath, PathEl};
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::gradient::{Colors as GradientColors, Gradient, Interpolation, shaders};
use skia_safe::paint::Style;
use skia_safe::vertices::VertexMode;
use skia_safe::{
    BlendMode, BlurStyle, Canvas, ClipOp, Color, Color4f, FilterMode, MaskFilter, MipmapMode, Paint, Path, PathBuilder, PathEffect, PathFillType,
    Picture, PictureRecorder, Point, RRect, SamplingOptions, TileMode, Vector, Vertices, color_filters,
};

use crate::images::Images;
use crate::surface::Surfaces;
use crate::text;

/// A recorded list and what it depends on beyond itself.
pub(crate) struct Recorded {
    pub picture: Option<Picture>,
    /// Images and surfaces the list draws (also the ones that were missing): a change to one
    /// drops the picture.
    pub uses: HashSet<Media>,
    /// The picture's sampling depends on the target scale (an image whose magnify and minify
    /// filters differ, or that has mipmaps), so a scale change records it again.
    pub scale_sensitive: bool,
}

struct Recorder<'a> {
    canvas: &'a Canvas,
    opacity: f32,
    scale: f32,
    images: &'a Images,
    surfaces: &'a Surfaces,
    uses: HashSet<Media>,
    scale_sensitive: bool,
    /// Saves the list's own pushes made, so a stray pop does nothing.
    depth: usize,
}

fn color(c: Rgba, opacity: f32) -> Color4f {
    let [r, g, b, a] = c.0;
    Color4f::new(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, f32::from(a) / 255.0 * opacity)
}

fn visible(c: Rgba) -> bool {
    c.0[3] != 0
}

fn rect(r: Rect) -> skia_safe::Rect {
    skia_safe::Rect::new(r.min.x, r.min.y, r.max.x, r.max.y)
}

fn point(p: Pos2) -> Point {
    Point::new(p.x, p.y)
}

fn kpoint(p: kurbo::Point) -> Point {
    Point::new(p.x as f32, p.y as f32)
}

fn degenerate(r: Rect) -> bool {
    !(r.width() > 0.0 && r.height() > 0.0)
}

fn fill_paint(c: Rgba, opacity: f32) -> Paint {
    let mut p = Paint::new(color(c, opacity), None);
    p.set_anti_alias(true);
    p
}

fn stroke_paint(s: &Stroke, width: f32, opacity: f32) -> Paint {
    let mut p = fill_paint(s.color, opacity);
    p.set_style(Style::Stroke);
    p.set_stroke_width(width);
    if let Some(d) = s.dash {
        p.set_path_effect(PathEffect::dash(&[d.on, d.off], d.offset));
    }
    p
}

/// Skia strokes are centred on the outline; the other alignments move the outline by half the
/// width. Returns the offset to grow the shape by.
fn outset(s: &Stroke) -> f32 {
    match s.align {
        StrokeAlign::Inside => -s.width / 2.0,
        StrokeAlign::Middle => 0.0,
        StrokeAlign::Outside => s.width / 2.0,
    }
}

fn rrect(r: Rect, radii: CornerRadii, grow: f32) -> RRect {
    // Square corners stay square; round ones follow the offset.
    let g = |v: f32| {
        let v = if v <= 0.0 { 0.0 } else { (v + grow).max(0.0) };
        Vector::new(v, v)
    };
    RRect::new_rect_radii(rect(r.expand(grow)), &[g(radii.top_left), g(radii.top_right), g(radii.bottom_right), g(radii.bottom_left)])
}

fn path(p: &BezPath, rule: FillRule) -> Path {
    let mut b = PathBuilder::new_with_fill_type(match rule {
        FillRule::NonZero => PathFillType::Winding,
        FillRule::EvenOdd => PathFillType::EvenOdd,
    });
    for el in p.elements() {
        match *el {
            PathEl::MoveTo(a) => b.move_to(kpoint(a)),
            PathEl::LineTo(a) => b.line_to(kpoint(a)),
            PathEl::QuadTo(a, c) => b.quad_to(kpoint(a), kpoint(c)),
            PathEl::CurveTo(a, c, d) => b.cubic_to(kpoint(a), kpoint(c), kpoint(d)),
            PathEl::ClosePath => b.close(),
        };
    }
    b.detach()
}

/// Records `list` with node opacity `opacity` multiplied into every colour (the runtime defines it
/// as a per-draw factor; [`Item::PushOpacity`] groups composite as layers). `bounds` is the list's
/// ink in local coordinates. `scale` picks image filters.
pub(crate) fn record(list: &DisplayList, opacity: f32, scale: f32, images: &Images, surfaces: &Surfaces, bounds: Rect) -> Recorded {
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(rect(bounds), false);
    let mut r = Recorder { canvas, opacity, scale, images, surfaces, uses: HashSet::new(), scale_sensitive: false, depth: 0 };
    for item in &list.items {
        r.item(item);
    }
    let Recorder { uses, scale_sensitive, canvas, depth, .. } = r;
    for _ in 0..depth {
        canvas.restore();
    }
    Recorded { picture: recorder.finish_recording_as_picture(None), uses, scale_sensitive }
}

impl Recorder<'_> {
    fn item(&mut self, item: &Item) {
        match item {
            Item::Rect { rect, radii, fill, stroke } => self.rect(*rect, *radii, *fill, *stroke),
            Item::Circle { center, radius, fill, stroke } => self.circle(*center, *radius, *fill, *stroke),
            Item::Line { from, to, stroke } => {
                if visible(stroke.color) && stroke.width > 0.0 {
                    self.canvas.draw_line(point(*from), point(*to), &stroke_paint(stroke, stroke.width, self.opacity));
                }
            }
            Item::Path { path, fill, stroke } => self.path(path, *fill, *stroke),
            Item::Mesh(mesh) => self.mesh(mesh),
            Item::Shadow { rect, radii, offset, blur, spread, color } => self.shadow(*rect, *radii, *offset, *blur, *spread, *color),
            Item::Image { key, rect, uv, tint } => self.image(*key, *rect, *uv, *tint, false),
            Item::ImageMask { key, rect, uv, color } => self.image(*key, *rect, *uv, *color, true),
            Item::VGradient { rect, top, bottom } => self.gradient(*rect, *top, *bottom),
            Item::Text { layout, origin, color, highlight, angle, .. } => {
                if *angle == 0.0 {
                    self.text(layout, *origin, *color, highlight.as_ref());
                } else {
                    self.canvas.save();
                    self.canvas.rotate(angle.to_degrees(), Some(point(*origin)));
                    self.text(layout, *origin, *color, highlight.as_ref());
                    self.canvas.restore();
                }
            }
            Item::Surface { key, rect } => self.surface(*key, *rect),
            // ui-rt puts a slot's items in its place; an unfilled slot paints nothing.
            Item::Slot(_) => {}
            Item::PushClip(clip) => self.push_clip(*clip),
            Item::PopClip | Item::PopOpacity => self.pop(),
            Item::PushOpacity { opacity, bounds } => {
                self.canvas.save_layer_alpha_f(rect(*bounds), *opacity);
                self.depth += 1;
            }
        }
    }

    fn pop(&mut self) {
        if self.depth > 0 {
            self.canvas.restore();
            self.depth -= 1;
        }
    }

    fn push_clip(&mut self, clip: Clip) {
        self.canvas.save();
        self.depth += 1;
        if clip.radii.is_zero() {
            // A hard edge, as epaint's scissor; the frame snaps node clips to pixels.
            self.canvas.clip_rect(rect(clip.rect), ClipOp::Intersect, false);
        } else {
            self.canvas.clip_rrect(rrect(clip.rect, clip.radii, 0.0), ClipOp::Intersect, true);
        }
    }

    fn rect(&mut self, r: Rect, radii: CornerRadii, fill: Rgba, stroke: Option<Stroke>) {
        if visible(fill) {
            self.canvas.draw_rrect(rrect(r, radii, 0.0), &fill_paint(fill, self.opacity));
        }
        if let Some(s) = stroke.filter(|s| visible(s.color) && s.width > 0.0) {
            self.canvas.draw_rrect(rrect(r, radii, outset(&s)), &stroke_paint(&s, s.width, self.opacity));
        }
    }

    fn circle(&mut self, center: Pos2, radius: f32, fill: Rgba, stroke: Option<Stroke>) {
        if visible(fill) {
            self.canvas.draw_circle(point(center), radius, &fill_paint(fill, self.opacity));
        }
        if let Some(s) = stroke.filter(|s| visible(s.color) && s.width > 0.0) {
            let r = (radius + outset(&s)).max(0.0);
            self.canvas.draw_circle(point(center), r, &stroke_paint(&s, s.width, self.opacity));
        }
    }

    fn path(&mut self, p: &BezPath, fill: Option<craft_ui_rt::Fill>, stroke: Option<Stroke>) {
        let sk = path(p, fill.map_or(FillRule::NonZero, |f| f.rule));
        if let Some(f) = fill.filter(|f| visible(f.color)) {
            self.canvas.draw_path(&sk, &fill_paint(f.color, self.opacity));
        }
        let Some(s) = stroke.filter(|s| visible(s.color) && s.width > 0.0) else { return };
        match s.align {
            StrokeAlign::Middle | StrokeAlign::Outside => {
                self.canvas.draw_path(&sk, &stroke_paint(&s, s.width, self.opacity));
            }
            // Inside: a double-width stroke clipped to the path's interior.
            StrokeAlign::Inside => {
                self.canvas.save();
                self.canvas.clip_path(&sk, ClipOp::Intersect, true);
                self.canvas.draw_path(&sk, &stroke_paint(&s, s.width * 2.0, self.opacity));
                self.canvas.restore();
            }
        }
    }

    fn mesh(&mut self, mesh: &Mesh) {
        if mesh.vertices.is_empty() || mesh.indices.len() < 3 {
            return;
        }
        let image = mesh.texture.and_then(|k| {
            self.uses.insert(Media::Image(k));
            self.images.get(k).map(|s| (s.image.clone(), s.options))
        });
        let (w, h) = image.as_ref().map_or((1.0, 1.0), |(i, _)| (i.width() as f32, i.height() as f32));
        let positions: Vec<Point> = mesh.vertices.iter().map(|v| point(v.pos)).collect();
        let texs: Vec<Point> = mesh.vertices.iter().map(|v| Point::new(v.uv.x * w, v.uv.y * h)).collect();
        let colors: Vec<Color> = mesh
            .vertices
            .iter()
            .map(|v| {
                let [r, g, b, a] = v.color.0;
                Color::from_argb((f32::from(a) * self.opacity).round() as u8, r, g, b)
            })
            .collect();
        let tris = mesh.indices.len() / 3 * 3;
        let vertices = if mesh.vertices.len() <= usize::from(u16::MAX) {
            let indices: Vec<u16> = mesh.indices[..tris].iter().map(|&i| i as u16).collect();
            Vertices::new_copy(VertexMode::Triangles, &positions, &texs, &colors, Some(&indices))
        } else {
            // More vertices than a 16-bit index reaches: expand to a plain triangle list.
            let at = |i: u32| i as usize;
            let (p, t, c): (Vec<_>, Vec<_>, Vec<_>) =
                mesh.indices[..tris].iter().map(|&i| (positions[at(i)], texs[at(i)], colors[at(i)])).collect::<Vec<_>>().into_iter().fold(
                    (Vec::new(), Vec::new(), Vec::new()),
                    |mut acc, (p, t, c)| {
                        acc.0.push(p);
                        acc.1.push(t);
                        acc.2.push(c);
                        acc
                    },
                );
            Vertices::new_copy(VertexMode::Triangles, &p, &t, &c, None)
        };
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        let mode = match image {
            Some((image, options)) => {
                let sampling = self.sampling(options, (1.0, 1.0), (1.0, 1.0));
                paint.set_shader(image.to_shader((TileMode::Clamp, TileMode::Clamp), sampling, None));
                BlendMode::Modulate
            }
            // Skia blends the paint (the shader) as the source over the vertex colours: Dst keeps only the colours.
            None => BlendMode::Dst,
        };
        self.canvas.draw_vertices(&vertices, mode, &paint);
    }

    fn shadow(&mut self, r: Rect, radii: CornerRadii, offset: Vec2, blur: f32, spread: f32, c: Rgba) {
        if !visible(c) {
            return;
        }
        let mut paint = fill_paint(c, self.opacity);
        if blur > 0.0 {
            // A Gaussian whose extent is `blur`, as CSS box-shadow: sigma is half of it.
            paint.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, blur / 2.0, true));
        }
        self.canvas.draw_rrect(rrect(r.translate(offset), radii, spread), &paint);
    }

    fn gradient(&mut self, r: Rect, top: Rgba, bottom: Rgba) {
        if degenerate(r) {
            return;
        }
        // Premultiplied sRGB interpolation, as the epaint mesh's vertex colours.
        let stops = [color(top, self.opacity), color(bottom, self.opacity)];
        let gradient = Gradient::new(GradientColors::new(&stops, None, TileMode::Clamp, None), Interpolation::from_flags(1));
        let shader = shaders::linear_gradient((Point::new(r.min.x, r.min.y), Point::new(r.min.x, r.max.y)), &gradient, None);
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_shader(shader);
        self.canvas.draw_rect(rect(r), &paint);
    }

    /// The sampling for an image of `options` drawn from `src` pixels to `dst` logical points.
    fn sampling(&mut self, options: ImageOptions, dst: (f32, f32), src: (f32, f32)) -> SamplingOptions {
        let magnified = dst.0 * self.scale > src.0 || dst.1 * self.scale > src.1;
        let filter = if magnified { options.magnify } else { options.minify };
        self.scale_sensitive |= options.magnify != options.minify || options.mipmaps;
        let filter = match filter {
            craft_ui_rt::Filter::Linear => FilterMode::Linear,
            craft_ui_rt::Filter::Nearest => FilterMode::Nearest,
        };
        let mips = if options.mipmaps && !magnified { MipmapMode::Linear } else { MipmapMode::None };
        SamplingOptions::new(filter, mips)
    }

    /// Draws image `key`'s `uv` part into `r`. A tint multiplies the image (premultiplied), as the
    /// epaint vertex colour does; a mask colour is painted through the image's alpha.
    fn image(&mut self, key: ImageKey, r: Rect, uv: Rect, tint: Rgba, mask: bool) {
        self.uses.insert(Media::Image(key));
        let Some((image, options)) = self.images.get(key).map(|s| (s.image.clone(), s.options)) else { return };
        let (w, h) = (image.width() as f32, image.height() as f32);
        let src = skia_safe::Rect::new(uv.min.x * w, uv.min.y * h, uv.max.x * w, uv.max.y * h);
        if degenerate(r) || !(src.width() > 0.0 && src.height() > 0.0) {
            return;
        }
        let sampling = self.sampling(options, (r.width(), r.height()), (src.width(), src.height()));
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        if mask {
            paint.set_color_filter(color_filters::blend(color(tint, self.opacity).to_color(), BlendMode::SrcIn));
        } else if tint.0[..3] == [255, 255, 255] {
            paint.set_alpha_f(f32::from(tint.0[3]) / 255.0 * self.opacity);
        } else {
            paint.set_color_filter(color_filters::blend(color(tint, self.opacity).to_color(), BlendMode::Modulate));
        }
        if options.wrap == craft_ui_rt::Wrap::Repeat {
            // The image tiled from its uv origin: a shader carries the repeat past the image's edge.
            let (sx, sy) = (r.width() / src.width(), r.height() / src.height());
            let mut local = skia_safe::Matrix::scale((sx, sy));
            local.post_translate((r.min.x - src.left * sx, r.min.y - src.top * sy));
            paint.set_shader(image.to_shader((TileMode::Repeat, TileMode::Repeat), sampling, Some(&local)));
            self.canvas.draw_rect(rect(r), &paint);
            return;
        }
        self.canvas.draw_image_rect_with_sampling_options(&image, Some((&src, SrcRectConstraint::Fast)), rect(r), sampling, &paint);
    }

    fn text(&mut self, layout: &craft_ui_rt::TextHandle, origin: Pos2, override_color: Option<Rgba>, highlight: Option<&craft_ui_rt::TextHighlight>) {
        let Some(sh) = text::downcast(layout) else { return };
        if let Some(h) = highlight {
            let paint = fill_paint(h.fill, self.opacity);
            for r in sh.highlight(h.range.clone()) {
                self.canvas.draw_rect(rect(r.translate(origin.to_vec2())), &paint);
            }
        }
        let Some(blob) = &sh.blob else { return };
        let paint = fill_paint(override_color.unwrap_or(sh.color), self.opacity);
        self.canvas.draw_text_blob(blob, point(origin), &paint);
    }

    /// The surface's tiles, each scaled from surface pixels into `r`.
    fn surface(&mut self, key: SurfaceKey, r: Rect) {
        self.uses.insert(Media::Surface(key));
        let Some((spec, tiles)) = self.surfaces.tiles(key) else { return };
        if degenerate(r) || spec.size.width == 0 || spec.size.height == 0 {
            return;
        }
        let (sx, sy) = (r.width() / spec.size.width as f32, r.height() / spec.size.height as f32);
        let sampling = self.sampling(spec.options, (r.width(), r.height()), (spec.size.width as f32, spec.size.height as f32));
        let mut paint = Paint::default();
        // Tiles meet on shared edges; anti-aliasing there would show seams.
        paint.set_anti_alias(false);
        paint.set_alpha_f(self.opacity);
        for (image, t) in tiles {
            let dst =
                skia_safe::Rect::new(r.min.x + t.x0 as f32 * sx, r.min.y + t.y0 as f32 * sy, r.min.x + t.x1 as f32 * sx, r.min.y + t.y1 as f32 * sy);
            self.canvas.draw_image_rect_with_sampling_options(image, None, dst, sampling, &paint);
        }
    }
}
