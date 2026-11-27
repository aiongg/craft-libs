//! Galleys and layout jobs.

use egui::epaint::text::FontsView;
use egui::text::LayoutJob;
use egui::{Context, Galley};
use std::sync::Arc;

/// Lays `job` out with the context's fonts. A section with no line height gets its face's row height rounded up to a
/// whole number of points: egui rounds a row to whole physical pixels, so a fractional height would give rows of
/// different logical heights at different interface scales (14.5 at scale 2, 15 at scale 1), and a wrapped text would
/// change height with the scale.
pub(crate) fn layout(ctx: &Context, job: LayoutJob) -> Arc<Galley> {
    #[cfg(feature = "testing")]
    crate::runtime::measure::count_text_layout(ctx);
    ctx.fonts_mut(|fonts| layout_with(fonts, job))
}

/// [`layout`] over fonts the caller already holds.
pub(crate) fn layout_with(fonts: &mut FontsView<'_>, mut job: LayoutJob) -> Arc<Galley> {
    for section in &mut job.sections {
        if section.format.line_height.is_none() {
            section.format.line_height = Some(fonts.row_height(&section.format.font_id).ceil());
        }
    }
    fonts.layout_job(job)
}

/// What a text handle laid out by the egui runtime holds.
pub(crate) struct EguiShaped {
    pub(crate) galley: Arc<Galley>,
}

/// The job of a paragraph of sections: the faces by family, the paragraph direction, rows at the left edge and then
/// placed by `row_align`, as `craft-ui-render-epaint` lays them out.
pub(crate) fn job_of(text: &craft_ui_rt::RichText) -> LayoutJob {
    use egui::epaint::text::{LayoutSection, TextDirection, TextFormat, TextWrapping};
    let color = |c: craft_theme::Rgba| egui::Color32::from_rgba_unmultiplied(c.0[0], c.0[1], c.0[2], c.0[3]);
    let mut job = LayoutJob {
        text: text.text.to_string(),
        direction: if text.rtl { TextDirection::RightToLeft } else { TextDirection::LeftToRight },
        halign: egui::Align::LEFT,
        halign_min_is_start: false,
        wrap: TextWrapping {
            max_width: text.wrap.max_width,
            max_rows: text.wrap.max_rows,
            break_anywhere: text.wrap.break_anywhere,
            overflow_character: text.wrap.overflow,
        },
        ..LayoutJob::default()
    };
    for section in &text.sections {
        let f = &section.format;
        let family = match &f.face {
            craft_ui_rt::FontFace::Slot(craft_theme::FontSlot::Mono) => egui::FontFamily::Monospace,
            craft_ui_rt::FontFace::Slot(craft_theme::FontSlot::UiSemibold) => egui::FontFamily::Name(crate::fonts::SEMIBOLD.into()),
            craft_ui_rt::FontFace::Slot(_) => egui::FontFamily::Proportional,
            craft_ui_rt::FontFace::Family(name) => egui::FontFamily::Name(name.as_ref().into()),
        };
        let mut format = TextFormat {
            font_id: egui::FontId::new(f.size, family),
            color: color(f.color),
            background: color(f.background),
            italics: f.italic,
            extra_letter_spacing: f.letter_spacing,
            line_height: f.line_height,
            ..TextFormat::default()
        };
        if let Some(line) = f.underline {
            format.underline = egui::Stroke::new(line.width, color(line.color));
        }
        if let Some(line) = f.strikethrough {
            format.strikethrough = egui::Stroke::new(line.width, color(line.color));
        }
        for &(tag, value) in &f.axes {
            format.coords.push(tag, value);
        }
        if !f.features.is_empty() {
            format.font_features = egui::epaint::text::FontFeatures::new(f.features.iter().copied());
        }
        job.sections.push(LayoutSection {
            leading_space: section.indent,
            byte_range: egui::epaint::text::ByteIndex(section.bytes.start)..egui::epaint::text::ByteIndex(section.bytes.end),
            format,
        });
    }
    job
}

/// The galley with each row placed within the widest by `share` of its free room, rounded to whole pixels, and its
/// rect starting at x = 0.
fn placed(mut galley: Arc<Galley>, share: f32, pixels_per_point: f32) -> Arc<Galley> {
    let shift = -galley.rect.min.x;
    let width = galley.rect.width();
    let align = share != 0.0 && galley.rows.iter().any(|placed| placed.size.x < width);
    if shift == 0.0 && !align {
        return galley;
    }
    let g = Arc::make_mut(&mut galley);
    let by = egui::vec2(shift, 0.0);
    for placed in &mut g.rows {
        placed.pos.x += shift;
        if align {
            let free = (width - placed.size.x).max(0.0);
            placed.pos.x = (free * share * pixels_per_point).round() / pixels_per_point;
        }
    }
    g.rect = g.rect.translate(by);
    g.mesh_bounds = g.mesh_bounds.translate(by);
    galley
}

/// A galley as an engine layout, its handle holding the galley.
pub(crate) fn rt_layout(galley: Arc<Galley>) -> craft_ui_rt::TextLayout {
    let size = galley.rect.size();
    let baseline = galley.rows.first().map_or(0.0, |placed| match placed.row.glyphs.first() {
        Some(glyph) => placed.pos.y + glyph.pos.y,
        None => placed.pos.y + placed.row.size.y,
    });
    let lines = galley.rows.len();
    craft_ui_rt::TextLayout { handle: craft_ui_rt::TextHandle::new(Arc::new(EguiShaped { galley })), size, baseline, lines }
}

/// The metrics of a layout the egui runtime made.
pub(crate) fn rt_metrics(layout: &craft_ui_rt::TextLayout) -> craft_ui_rt::TextMetrics {
    let Some(shaped) = layout.handle.downcast_ref::<EguiShaped>() else {
        return craft_ui_rt::TextMetrics { text: Arc::from(""), rows: Vec::new(), ink: egui::Rect::NOTHING, elided: false };
    };
    let galley = &shaped.galley;
    let rows = galley
        .rows
        .iter()
        .map(|placed| craft_ui_rt::TextRow {
            rect: egui::Rect::from_min_size(placed.pos, placed.row.size),
            glyphs: placed
                .row
                .glyphs
                .iter()
                .map(|g| craft_ui_rt::GlyphPlace { chr: g.chr, x: g.pos.x, advance: g.advance_width, rtl: g.is_rtl() })
                .collect(),
        })
        .collect();
    let ink = if galley.mesh_bounds.is_finite() { galley.mesh_bounds } else { egui::Rect::NOTHING };
    craft_ui_rt::TextMetrics { text: galley.job.text.as_str().into(), rows, ink, elided: galley.elided }
}

/// The egui runtime's text engine and image store: text through the context's fonts. Pictures recorded by key are
/// the renderers' (the egui runtime paints no display list images).
impl crate::primitives::Services for Context {
    fn scale(&self) -> f32 {
        self.pixels_per_point()
    }

    fn fonts(&self) -> crate::fonts::StackFacts {
        crate::fonts::stack_facts(self)
    }

    fn layout(&self, text: &craft_ui_rt::RichText) -> craft_ui_rt::TextLayout {
        rt_layout(placed(layout(self, job_of(text)), text.row_align, self.pixels_per_point()))
    }

    fn metrics(&self, layout: &craft_ui_rt::TextLayout) -> craft_ui_rt::TextMetrics {
        rt_metrics(layout)
    }

    fn egui(&self) -> Option<&Context> {
        Some(self)
    }

    fn max_image_side(&self) -> u32 {
        self.input(|input| input.max_texture_side) as u32
    }

    fn face_over_interface(&self, key: &str, load: &mut dyn FnMut() -> Option<crate::fonts::FaceBytes>) -> crate::fonts::FaceState {
        crate::fonts::document::DocumentFaces::over_interface(self, key, load)
    }

    fn face(&self, key: &str, job: crate::primitives::FaceJob<'_>) -> crate::fonts::FaceState {
        crate::fonts::document::DocumentFaces::face(self, key, job)
    }

    fn face_bytes(&self, key: &str) -> Option<crate::fonts::FaceBytes> {
        crate::fonts::document::DocumentFaces::bytes(self, key)
    }

    fn image(&self, _key: craft_ui_rt::ImageKey, _options: craft_ui_rt::ImageOptions, _make: &mut dyn FnMut() -> Arc<craft_ui_rt::RgbaImage>) {}
}

/// Paints display items recorded by a canvas whose origin is the window's on an egui painter: what the egui runtime
/// shows of content painted through `Canvas` (cursor art, drag ghosts). Pictures by image key are not painted.
pub(crate) fn paint_items(painter: &egui::Painter, items: &[craft_ui_rt::Item]) {
    use craft_ui_rt::{Item, StrokeAlign};
    use egui::epaint::{CircleShape, RectShape, Shape, StrokeKind};
    let color = |c: craft_theme::Rgba, opacity: f32| {
        let c = egui::Color32::from_rgba_unmultiplied(c.0[0], c.0[1], c.0[2], c.0[3]);
        if opacity < 1.0 { c.gamma_multiply(opacity) } else { c }
    };
    let corners = |r: craft_ui_rt::CornerRadii| egui::CornerRadius {
        nw: r.top_left.round() as u8,
        ne: r.top_right.round() as u8,
        se: r.bottom_right.round() as u8,
        sw: r.bottom_left.round() as u8,
    };
    let kind = |a: StrokeAlign| match a {
        StrokeAlign::Inside => StrokeKind::Inside,
        StrokeAlign::Middle => StrokeKind::Middle,
        StrokeAlign::Outside => StrokeKind::Outside,
    };
    let mut clips = vec![painter.clone()];
    let mut opacity = vec![1.0_f32];
    for item in items {
        let p = clips.last().expect("the base painter stays").clone();
        let o = *opacity.last().expect("the base opacity stays");
        let stroke = |s: &Option<craft_ui_rt::Stroke>| s.map_or(egui::Stroke::NONE, |s| egui::Stroke::new(s.width, color(s.color, o)));
        match item {
            Item::Rect { rect, radii, fill, stroke: s } => {
                let k = s.map_or(StrokeKind::Inside, |s| kind(s.align));
                p.add(RectShape::new(*rect, corners(*radii), color(*fill, o), stroke(s), k));
            }
            Item::Circle { center, radius, fill, stroke: s } => {
                p.add(CircleShape { center: *center, radius: *radius, fill: color(*fill, o), stroke: stroke(s) });
            }
            Item::Line { from, to, stroke: s } => {
                p.line_segment([*from, *to], egui::Stroke::new(s.width, color(s.color, o)));
            }
            Item::Path { path, fill, stroke: s } => {
                let points: Vec<egui::Pos2> = path
                    .elements()
                    .iter()
                    .filter_map(|el| match el {
                        craft_behavior::kurbo::PathEl::MoveTo(q) | craft_behavior::kurbo::PathEl::LineTo(q) => {
                            Some(egui::pos2(q.x as f32, q.y as f32))
                        }
                        _ => None,
                    })
                    .collect();
                let closed = path.elements().iter().any(|el| matches!(el, craft_behavior::kurbo::PathEl::ClosePath));
                if let Some(f) = fill {
                    p.add(Shape::convex_polygon(points.clone(), color(f.color, o), egui::Stroke::NONE));
                }
                if let Some(line) = s {
                    let mut path = egui::epaint::PathStroke::from(stroke(s));
                    path.kind = kind(line.align);
                    p.add(egui::epaint::PathShape { points, closed, fill: egui::Color32::TRANSPARENT, stroke: path });
                }
            }
            Item::Mesh(mesh) => {
                let mut out = mesh.texture.map_or_else(egui::Mesh::default, |key| egui::Mesh::with_texture(egui::TextureId::User(key.0)));
                let uv = |v: &craft_ui_rt::Vertex| if mesh.texture.is_some() { v.uv } else { egui::epaint::WHITE_UV };
                out.vertices = mesh.vertices.iter().map(|v| egui::epaint::Vertex { pos: v.pos, uv: uv(v), color: color(v.color, o) }).collect();
                out.indices = mesh.indices.clone();
                p.add(Shape::mesh(out));
            }
            Item::Shadow { rect, radii, offset, blur, spread, color: c } => {
                p.add(RectShape::filled(rect.translate(*offset).expand(*spread), corners(*radii), color(*c, o)).with_blur_width(*blur));
            }
            Item::Text { layout, origin, .. } => {
                if let Some(shaped) = layout.downcast_ref::<EguiShaped>() {
                    let mut shape = egui::epaint::TextShape::new(*origin, shaped.galley.clone(), egui::Color32::PLACEHOLDER);
                    shape.opacity_factor = o;
                    p.add(shape);
                }
            }
            Item::PushClip(clip) => clips.push(p.with_clip_rect(clip.rect.intersect(p.clip_rect()))),
            Item::PopClip => {
                if clips.len() > 1 {
                    clips.pop();
                }
            }
            Item::PushOpacity { opacity: share, .. } => opacity.push(o * share),
            Item::PopOpacity => {
                if opacity.len() > 1 {
                    opacity.pop();
                }
            }
            _ => {}
        }
    }
}

/// Paints what `canvas` (origin at the window's) recorded on `painter` when it goes.
pub(crate) struct FlushOnDrop<'p, 'c>(pub(crate) &'p egui::Painter, pub(crate) &'p crate::primitives::Canvas<'c>);

impl Drop for FlushOnDrop<'_, '_> {
    fn drop(&mut self) {
        let (content, overlay) = self.1.take();
        paint_items(self.0, &content.items);
        paint_items(self.0, &overlay.items);
    }
}

/// Runs `paint` on a canvas over `painter`'s context and clip, then paints what it recorded on `painter`.
#[doc(hidden)]
pub fn on_painter<R>(painter: &egui::Painter, paint: impl FnOnce(&crate::primitives::Canvas<'_>) -> R) -> R {
    let canvas = crate::primitives::Canvas::owned(painter.ctx().clone(), egui::Pos2::ZERO, painter.clip_rect());
    let out = paint(&canvas);
    let (content, overlay) = canvas.finish();
    paint_items(painter, &content.items);
    paint_items(painter, &overlay.items);
    out
}
