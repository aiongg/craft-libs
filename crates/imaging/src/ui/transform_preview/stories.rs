//! Stories of the transform preview (task ST-TR2): not registered. Registering them (task RG2) adds
//! `transform_preview::stories` to the crate's `register!` list, `TransformPreview` to `COMPONENTS`
//! and [`TransformPreviewStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The moving layer is `MockImage` (`ui::mock_image`) uploaded as a mip pyramid in which every
//! level below the first has its own tint, so the level the preview picks shows. It sits in the
//! middle of a blank document (the checkerboard). Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/transform-preview/maps` | The layer through a quad (rotate, skew, distort past the horizon, perspective), a warp mesh, a pin mesh (fake solver) and a function map; Bicubic and Nearest at three zooms; opacity; clip to the canvas; a styled variant with a fake `StyledSource` (a drop shadow drawn by the story) |
//!
//! [`Document`] (the blank document, its canvas and the layer's textures) is the one stage the
//! other warp stories (`warp::pin::stories`, `warp::perspective::stories`) share.

#![expect(dead_code, reason = "task RG2 registers the stories")]

use std::sync::{Arc, PoisonError};

use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::canvas::{CanvasView, PixelRect, RasterSource};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, Surface};
use craft_ui::typed_story;
use egui::{AsIdSalt, Color32, ColorImage, Response, Sense, Ui};

use super::{PreviewMap, PreviewPixels, PreviewQuality, PreviewTextures, StylePlan, StyledContent, StyledPreview, StyledSource, TransformPreview};
use crate::ui::mock_image::MockImage;
use crate::ui::warp::WarpMesh;
use crate::ui::warp::pin::stories::{IdwSolver, sample_pins};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["TransformPreview"];

craft_ui::words! {
    /// The captions of the transform preview stories, English by default.
    #[non_exhaustive]
    pub(crate) struct TransformPreviewStoryLabels {
        rotate: "transformPreviewStory.rotate" = "Rotate";
        skew: "transformPreviewStory.skew" = "Skew";
        distort: "transformPreviewStory.distort" = "Distort past the horizon";
        perspective: "transformPreviewStory.perspective" = "Perspective";
        warp: "transformPreviewStory.warp" = "Warp mesh";
        pins: "transformPreviewStory.pins" = "Pin mesh";
        function: "transformPreviewStory.function" = "Function map";
        bicubic: "transformPreviewStory.bicubic" = "Bicubic at {n}%";
        nearest: "transformPreviewStory.nearest" = "Nearest Neighbor at {n}%";
        opacity: "transformPreviewStory.opacity" = "Opacity {n}%";
        clip: "transformPreviewStory.clip" = "Clipped to the canvas";
        styled: "transformPreviewStory.styled" = "Layer style: drop shadow";
    }
}

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/transform-preview/maps",
            Level::Primitives,
            "TransformPreview",
            "Maps",
            MapsArgs {
                width: Document::DEFAULT_SIZE[0],
                height: Document::DEFAULT_SIZE[1],
                turn: 20.0,
                skew: 20.0,
                perspective: 25.0,
                bend: 30.0,
                opacity: 50.0,
            },
            show_maps
        )
        .on(Surface::Canvas),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct MapsArgs {
    /// The document's width, in pixels.
    #[arg(range = 200.0..=4000.0)]
    width: f64,
    /// The document's height, in pixels.
    #[arg(range = 200.0..=4000.0)]
    height: f64,
    /// The turn of the rotated box, in degrees.
    #[arg(range = -180.0..=180.0)]
    turn: f64,
    /// The skew of the skewed box, in degrees.
    #[arg(range = -60.0..=60.0)]
    skew: f64,
    /// How far the perspective box's far edge is narrowed, in percent of the width.
    #[arg(range = 0.0..=45.0)]
    perspective: f64,
    /// How far the warp lifts its top edge, in percent of the height.
    #[arg(range = -50.0..=50.0)]
    bend: f64,
    /// The opacity of the opacity case, in percent.
    #[arg(range = 0.0..=100.0)]
    opacity: f64,
}

/// The zooms of the interpolation cases, in percent: the first is far enough out for a mip level.
const ZOOMS: [(&str, &str, f64); 3] =
    [("bicubic-25", "nearest-25", 25.0), ("bicubic-100", "nearest-100", 100.0), ("bicubic-400", "nearest-400", 400.0)];
/// The zoom of the clip case, as a fraction of the zoom that fits the document in the canvas.
const CLIP_FIT: f64 = 0.5;
/// How far the clipped box reaches past the layer on each side, as a fraction of the document's size.
const CLIP_REACH: f64 = 0.4;
/// The drop shadow's offset, as a fraction of the document's width.
const SHADOW_OFFSET: f64 = 1.0 / 60.0;
/// The tints of the mip levels below the first.
const TINTS: [[u8; 3]; 4] = [[255, 80, 60], [60, 200, 90], [70, 110, 255], [240, 200, 40]];

/// A blank document with a layer over its middle half: the stage of the warp stories. The layer's
/// pixels are `MockImage` drawn at the layer's size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Document {
    /// The document's size in pixels.
    pub(crate) size: [u32; 2],
}

/// How a canvas is zoomed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Zoom {
    /// The whole document fits the canvas.
    Fit,
    /// Device pixels per document pixel, in percent.
    Percent(f64),
    /// A fraction of the zoom that fits the document.
    OfFit(f64),
}

/// A transparent document of a given size: the checkerboard shows through.
struct Blank([u32; 2]);

impl RasterSource for Blank {
    fn size(&self) -> [u32; 2] {
        self.0
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, _region: PixelRect, _factor: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::TRANSPARENT);
    }
}

impl Document {
    /// The default size in pixels (3:2).
    pub(crate) const DEFAULT_SIZE: [f64; 2] = [960.0, 640.0];

    /// A document of `width` × `height` pixels.
    pub(crate) fn new(width: f64, height: f64) -> Self {
        let pixels = |v: f64| v.round().clamp(16.0, 8000.0) as u32;
        Self { size: [pixels(width), pixels(height)] }
    }

    /// The layer's frame: the middle half of the document.
    pub(crate) fn layer(&self) -> Rect {
        let (w, h) = (f64::from(self.size[0]), f64::from(self.size[1]));
        Rect::new(w / 4.0, h / 4.0, w * 0.75, h * 0.75)
    }

    /// The layer's size in pixels.
    pub(crate) fn layer_size(&self) -> [u32; 2] {
        let layer = self.layer();
        [layer.width().round().max(1.0) as u32, layer.height().round().max(1.0) as u32]
    }

    /// A canvas `width` points wide showing the document, with the layer drawn by the caller.
    pub(crate) fn canvas(&self, ui: &mut Ui, id_salt: impl AsIdSalt, width: f32, zoom: Zoom, centre: Option<Point>) -> (Response, CanvasView) {
        let fit = f64::from(width) / f64::from(self.size[0]) * 100.0 * f64::from(ui.pixels_per_point());
        let percent = match zoom {
            Zoom::Fit => None,
            Zoom::Percent(percent) => Some(percent),
            Zoom::OfFit(fraction) => Some(fit * fraction),
        };
        MockImage::new(self.size).canvas_of(ui, id_salt, width, percent, centre, &Blank(self.size))
    }

    /// [`canvas`](Self::canvas) whose response senses presses and drags, for an overlay.
    pub(crate) fn interactive_canvas(&self, ui: &mut Ui, id_salt: impl AsIdSalt, width: f32) -> (Response, CanvasView) {
        let (response, view) = self.canvas(ui, id_salt, width, Zoom::Fit, None);
        let sensing = ui.interact(response.rect, response.id.with("sense"), Sense::click_and_drag() - Sense::FOCUSABLE);
        (sensing, view)
    }

    /// The layer's textures, uploaded once per layer size: `MockImage` at every mip level, each
    /// level below the first tinted.
    pub(crate) fn textures(&self, ui: &Ui) -> Arc<PreviewTextures> {
        let id = ui.make_persistent_id("transform-story-textures");
        let size = self.layer_size();
        if let Some((kept, textures)) = ui.data(|d| d.get_temp::<([u32; 2], Arc<PreviewTextures>)>(id))
            && kept == size
        {
            return textures;
        }
        let levels = pyramid(size);
        let pixels = PreviewPixels::new(levels[0].as_ref().clone(), [1.0, 1.0]);
        let textures = Arc::new(PreviewTextures::new(ui.ctx(), "transform-story", pixels));
        *textures.images.lock().unwrap_or_else(PoisonError::into_inner) = levels;
        ui.data_mut(|d| d.insert_temp(id, (size, Arc::clone(&textures))));
        textures
    }
}

/// `MockImage` at `size` and every halving of it down to one texel, the levels below the first
/// tinted.
fn pyramid(size: [u32; 2]) -> Vec<Arc<ColorImage>> {
    let picture = MockImage::new(size);
    let mut levels = Vec::new();
    let mut factor = 1u32;
    loop {
        let (w, h) = (size[0].div_ceil(factor).max(1), size[1].div_ceil(factor).max(1));
        let mut image = ColorImage::filled([w as usize, h as usize], Color32::TRANSPARENT);
        picture.read(PixelRect::of_size(size), factor, &mut image);
        tint(&mut image, levels.len());
        levels.push(Arc::new(image));
        if w <= 1 && h <= 1 {
            break;
        }
        factor *= 2;
    }
    levels
}

/// Mixes level `level`'s tint into `image` (the picture is opaque, so premultiplied is the same).
fn tint(image: &mut ColorImage, level: usize) {
    if level == 0 {
        return;
    }
    let tint = TINTS[(level - 1) % TINTS.len()];
    let mix = |c: u8, t: u8| ((u16::from(c) * 3 + u16::from(t) * 2) / 5) as u8;
    for pixel in &mut image.pixels {
        let [r, g, b, a] = pixel.to_array();
        *pixel = Color32::from_rgba_premultiplied(mix(r, tint[0]), mix(g, tint[1]), mix(b, tint[2]), a);
    }
}

fn corners(r: Rect) -> [Point; 4] {
    [Point::new(r.x0, r.y0), Point::new(r.x1, r.y0), Point::new(r.x1, r.y1), Point::new(r.x0, r.y1)]
}

fn rotated(r: Rect, degrees: f64) -> [Point; 4] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let c = r.center();
    corners(r).map(|p| {
        let (dx, dy) = (p.x - c.x, p.y - c.y);
        Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
    })
}

fn skewed(r: Rect, degrees: f64) -> [Point; 4] {
    let shift = degrees.to_radians().tan() * r.height() / 2.0;
    let [tl, tr, br, bl] = corners(r);
    [Point::new(tl.x + shift, tl.y), Point::new(tr.x + shift, tr.y), Point::new(br.x - shift, br.y), Point::new(bl.x - shift, bl.y)]
}

/// The bottom corners crossed: a Distort dragged past the horizon.
fn crossed(r: Rect) -> [Point; 4] {
    let [tl, tr, br, bl] = corners(r);
    let inset = r.width() * 0.1;
    [tl, tr, Point::new(bl.x + inset, br.y), Point::new(br.x - inset, bl.y)]
}

/// The top edge narrowed by `percent` of the width on each side.
fn narrowed(r: Rect, percent: f64) -> [Point; 4] {
    let inset = r.width() * percent / 100.0;
    let [tl, tr, br, bl] = corners(r);
    [Point::new(tl.x + inset, tl.y), Point::new(tr.x - inset, tr.y), br, bl]
}

/// A one-patch warp mesh whose top edge bulges up by `percent` of the height.
fn bent(r: Rect, percent: f64) -> WarpMesh {
    let mut mesh = WarpMesh::identity(r, 1, 1);
    let (width, height) = (mesh.width(), mesh.height());
    let lift = r.height() * percent / 100.0;
    for (index, point) in mesh.points.iter_mut().enumerate() {
        let (i, j) = (index % width, index / width);
        let inside = if i == 0 || i + 1 == width { 0.0 } else { 1.0 };
        point.y -= lift * inside * (1.0 - j as f64 / (height - 1) as f64);
    }
    mesh
}

fn percent(numbers: &LocaleNumbers, n: f64) -> String {
    numbers.format(n, 0..=0)
}

fn caption(text: &str, n: &str) -> String {
    craft_i18n::fmt(text, &[("n", n)])
}

/// A caption and a canvas under it; `paint` draws the preview into the canvas's view.
fn cell(ui: &mut Ui, doc: &Document, text: &str, salt: &'static str, zoom: Zoom, centre: Option<Point>, paint: impl FnOnce(&Ui, &CanvasView)) {
    ui.add(Text::new(text).content(Content::Muted));
    let width = ui.available_width();
    let (_, view) = doc.canvas(ui, ("transform-story", salt), width, zoom, centre);
    paint(ui, &view);
}

/// The layer through `map`, clipped to the document.
pub(crate) fn draw(ui: &Ui, view: &CanvasView, doc: &Document, textures: &PreviewTextures, map: PreviewMap<'_>, opacity: f32, nearest: bool) {
    TransformPreview::new("transform-story", textures).map(map).opacity(opacity).nearest(nearest).clip(doc.size).show(ui, view);
}

/// A drop shadow offset down and right, drawn by the story in place of the app's compositor.
struct Shadow {
    /// In document pixels.
    offset: f64,
}

impl StyledSource for Shadow {
    fn revision(&self) -> u64 {
        1
    }

    fn pixel_cost_ns(&self) -> f64 {
        20.0
    }

    fn reach(&self, scale: f64) -> i32 {
        (self.offset * scale).ceil() as i32 + 1
    }

    fn compose(&self, plan: &StylePlan, content: &StyledContent) -> Option<ColorImage> {
        let shown = plan.shown;
        let (w, h) = (shown.width() as usize, shown.height() as usize);
        let shift = (self.offset * plan.scale).round() as i32;
        let columns = content.rect.width() as usize;
        let texel = |x: i32, y: i32| -> [f32; 4] {
            if x < content.rect.x0 || y < content.rect.y0 || x >= content.rect.x1 || y >= content.rect.y1 {
                return [0.0; 4];
            }
            let index = (y - content.rect.y0) as usize * columns + (x - content.rect.x0) as usize;
            content.texels.get(index).copied().unwrap_or([0.0; 4])
        };
        let mut out = ColorImage::filled([w, h], Color32::TRANSPARENT);
        for (row, line) in out.pixels.chunks_mut(w.max(1)).enumerate() {
            let y = shown.y0 + row as i32;
            for (column, pixel) in line.iter_mut().enumerate() {
                let x = shown.x0 + column as i32;
                let [r, g, b, a] = texel(x, y);
                let shadow = texel(x - shift, y - shift)[3] * 0.5;
                let alpha = a + shadow * (1.0 - a);
                let to_byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                // The shadow is black: it adds alpha and no colour.
                *pixel = Color32::from_rgba_premultiplied(to_byte(r * a), to_byte(g * a), to_byte(b * a), to_byte(alpha));
            }
        }
        Some(out)
    }
}

fn show_maps(ui: &mut Ui, args: &mut MapsArgs) {
    let labels = TransformPreviewStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let doc = Document::new(args.width, args.height);
    let textures = doc.textures(ui);
    let textures = textures.as_ref();
    let layer = doc.layer();
    let centre = layer.center();
    let turn = args.turn;

    // The four boxes of Free Transform.
    let boxes = [
        [(&labels.rotate, "rotate", rotated(layer, turn)), (&labels.skew, "skew", skewed(layer, args.skew))],
        [(&labels.distort, "distort", crossed(layer)), (&labels.perspective, "perspective", narrowed(layer, args.perspective))],
    ];
    for row in boxes {
        ui.columns(2, |columns| {
            for (column, (text, salt, quad)) in columns.iter_mut().zip(row) {
                cell(column, &doc, text, salt, Zoom::Fit, None, |ui, view| draw(ui, view, &doc, textures, PreviewMap::quad(quad), 1.0, false));
            }
        });
    }

    // The other maps: a warp mesh, a pin mesh on the fake solver and a function.
    let mesh = bent(layer, args.bend);
    let solver = IdwSolver::solved(layer, &sample_pins(layer));
    let ripple = |p: Point| {
        let wave = (p.y - layer.y0) / layer.height() * std::f64::consts::TAU * 1.5;
        Point::new(p.x + layer.width() * 0.05 * wave.sin(), p.y)
    };
    ui.columns(3, |columns| {
        cell(&mut columns[0], &doc, &labels.warp, "warp", Zoom::Fit, None, |ui, view| {
            draw(ui, view, &doc, textures, PreviewMap::mesh(&mesh), 1.0, false);
        });
        cell(&mut columns[1], &doc, &labels.pins, "pins", Zoom::Fit, None, |ui, view| {
            draw(ui, view, &doc, textures, PreviewMap::pins(solver.view(), layer), 1.0, false);
        });
        cell(&mut columns[2], &doc, &labels.function, "function", Zoom::Fit, None, |ui, view| {
            draw(ui, view, &doc, textures, PreviewMap::function(layer, PreviewQuality::PLANE_CELLS, &ripple), 1.0, false);
        });
    });

    // Interpolation at three zooms: the pyramid's tints show at the farthest.
    let turned = PreviewMap::quad(rotated(layer, turn));
    for nearest in [false, true] {
        let text = if nearest { &labels.nearest } else { &labels.bicubic };
        ui.columns(ZOOMS.len(), |columns| {
            for (column, (bicubic_salt, nearest_salt, zoom)) in columns.iter_mut().zip(ZOOMS) {
                let salt = if nearest { nearest_salt } else { bicubic_salt };
                let text = caption(text, &percent(&numbers, zoom));
                cell(column, &doc, &text, salt, Zoom::Percent(zoom), Some(centre), |ui, view| {
                    draw(ui, view, &doc, textures, turned, 1.0, nearest);
                });
            }
        });
    }

    // Opacity, the clip to the canvas and a styled layer.
    let styled_id = ui.make_persistent_id("transform-story-styled");
    let styled = ui.data_mut(|d| Arc::clone(d.get_temp_mut_or_insert_with(styled_id, || Arc::new(StyledPreview::new("transform-story-styled")))));
    let shadow = Shadow { offset: f64::from(doc.size[0]) * SHADOW_OFFSET };
    let reach = layer.inflate(f64::from(doc.size[0]) * CLIP_REACH, f64::from(doc.size[1]) * CLIP_REACH);
    ui.columns(3, |columns| {
        let text = caption(&labels.opacity, &percent(&numbers, args.opacity));
        cell(&mut columns[0], &doc, &text, "opacity", Zoom::Fit, None, |ui, view| {
            draw(ui, view, &doc, textures, turned, (args.opacity / 100.0) as f32, false);
        });
        cell(&mut columns[1], &doc, &labels.clip, "clip", Zoom::OfFit(CLIP_FIT), None, |ui, view| {
            draw(ui, view, &doc, textures, PreviewMap::quad(rotated(reach, turn)), 1.0, false);
        });
        cell(&mut columns[2], &doc, &labels.styled, "styled", Zoom::Fit, None, |ui, view| {
            let map = PreviewMap::quad(rotated(layer, turn));
            TransformPreview::new("transform-story", textures).map(map).clip(doc.size).styled(&styled, &shadow).show(ui, view);
        });
    });
}
