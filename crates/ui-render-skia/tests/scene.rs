//! A window of cells that exercise every display item, each cell repainting on its own state.

use std::sync::Arc;

use craft_theme::FontSlot;
use craft_theme::{Appearance, Contrast, Mode, TextSize, builtins, resolve};
use craft_ui_render_skia::SkiaText;
use craft_ui_rt::{
    BuildCx, Clip, Constraints, CornerRadii, Dash, Element, Fill, FillRule, FontSpec, ImageKey, ImageOptions, LayoutCx, Mesh, Overflow, PaintCx,
    Rgba, RgbaImage, Runtime, Show, SizeU, State, Stroke, StrokeAlign, SurfaceKey, TextHighlight, TextLayout, TextSpec, Tokens, Vec2, Vertex,
    WindowId, hash_key, pos2, vec2,
};
use emath::Rect;

pub const WINDOW: WindowId = WindowId(1);
pub const CELL: Vec2 = vec2(100.0, 60.0);
pub const COLUMNS: usize = 6;
pub const ROWS: usize = 4;
pub const IMAGE: ImageKey = ImageKey(7);
pub const SURFACE: SurfaceKey = SurfaceKey(9);
pub const SURFACE_SIZE: SizeU = SizeU::new(100, 60);

pub fn tokens() -> Tokens {
    let appearance = Appearance {
        mode: Mode::Dark,
        contrast: Contrast::Standard,
        size: craft_theme::Size::Compact,
        text_size: TextSize::Small,
        reduced_motion: false,
    };
    resolve(&builtins()[0], appearance, None).tokens
}

pub fn viewport() -> Vec2 {
    vec2(CELL.x * COLUMNS as f32, CELL.y * ROWS as f32)
}

pub fn image_pixels(size: u32, phase: u8) -> RgbaImage {
    let mut img = RgbaImage::new(SizeU::new(size, size));
    for (i, px) in img.pixels.chunks_exact_mut(4).enumerate() {
        let (x, y) = ((i as u32 % size) as u8, (i as u32 / size) as u8);
        px.copy_from_slice(&[x.wrapping_mul(16).wrapping_add(phase), y.wrapping_mul(16), 200, 40 + x.wrapping_mul(12).min(200)]);
    }
    img
}

/// The text engine of the scene: Inter plus the Arabic and CJK test faces as fallbacks.
pub fn engine() -> SkiaText {
    let mut text = SkiaText::isolated();
    super::text::register_script_faces(&mut text);
    text
}

struct Grid {
    cells: Vec<State<u32>>,
    /// Cell kinds (index % 8) drawn as a plain rect.
    plain: &'static [usize],
}

impl Element for Grid {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        for (index, state) in self.cells.iter().enumerate() {
            cx.keyed(index, |cx| Cell { index, state: *state, plain: self.plain.contains(&(index % 8)) }.show(cx));
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(CELL));
            cx.place_child(child, vec2((i % COLUMNS) as f32 * CELL.x, (i / COLUMNS) as f32 * CELL.y));
        }
        c.max
    }
}

struct Cell {
    index: usize,
    state: State<u32>,
    plain: bool,
}

impl Element for Cell {
    type Output = ();
    type Local = Option<TextLayout>;

    fn props_key(&self) -> u64 {
        hash_key(&self.index)
    }

    fn init(&self, _: &mut BuildCx<'_>) -> Option<TextLayout> {
        None
    }

    fn layout(&self, local: &mut Option<TextLayout>, cx: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        if self.index % 8 == 5 && !self.plain {
            let spec = TextSpec {
                text: "Aa \u{627}\u{644}\u{645}\u{639}\u{631}\u{641}\u{629} \u{4E2D}\u{6587}".into(),
                font: FontSpec { slot: FontSlot::Ui, size: 12.0 },
                line_height: None,
                color: Rgba([230, 230, 230, 255]),
                max_width: Some(CELL.x - 8.0),
                overflow: Overflow::Ellipsis,
            };
            *local = Some(cx.text_layout(&spec));
        }
        CELL
    }

    fn paint(&self, text: &Option<TextLayout>, cx: &mut PaintCx<'_>) {
        let v = self.state.get(cx);
        let tone = (40 + (v * 37 % 160)) as u8;
        let r = Rect::from_min_size(pos2(4.0, 4.0), CELL - vec2(8.0, 8.0));
        let bg = Rgba([tone, 90, 255 - tone, 255]);
        if self.plain {
            cx.fill_rect(r, bg);
            return;
        }
        match self.index % 8 {
            0 => {
                cx.rect(
                    r,
                    CornerRadii::same(8.0),
                    bg,
                    Some(Stroke::new(3.0, Rgba([250, 220, 60, 255])).aligned(StrokeAlign::Middle).dashed(Dash {
                        on: 5.0,
                        off: 3.0,
                        offset: v as f32,
                    })),
                );
                cx.circle(pos2(50.0, 30.0), 10.0 + (v % 5) as f32, Rgba([20, 20, 20, 200]), Some(Stroke::new(1.5, Rgba([255; 4]))));
            }
            1 => {
                cx.shadow(r.shrink(8.0), CornerRadii::same(6.0), vec2(0.0, 3.0), 8.0, 0.0, Rgba([0, 0, 0, 160]));
                cx.rect(r.shrink(8.0), CornerRadii::same(6.0), bg, None);
            }
            2 => {
                cx.vgradient(r, bg, Rgba([10, 10, 10, 255]));
                cx.line(pos2(6.0, 6.0 + v as f32 % 40.0), pos2(90.0, 50.0), Stroke::new(2.0, Rgba([255, 255, 255, 220])));
            }
            3 => {
                let vert = |x: f32, y: f32, c: [u8; 4]| Vertex { pos: pos2(x, y), uv: pos2(x / 100.0, y / 60.0), color: Rgba(c) };
                cx.mesh(Arc::new(Mesh {
                    vertices: vec![
                        vert(10.0, 10.0, [255, 0, 0, 255]),
                        vert(90.0, 10.0, [0, 255, 0, 255]),
                        vert(10.0, 50.0, [0, 0, 255, 255]),
                        vert(90.0, 50.0 - (v % 10) as f32, [255, 255, 0, 128]),
                    ],
                    indices: vec![0, 1, 2, 1, 2, 3],
                    texture: None,
                }));
            }
            4 => {
                cx.image(IMAGE, Rect::from_min_size(pos2(6.0, 6.0), vec2(40.0, 40.0)), Rgba([255, 255, 255, 255]));
                cx.image_mask(IMAGE, Rect::from_min_size(pos2(54.0, 6.0), vec2(40.0, 40.0)), bg);
                cx.image(IMAGE, Rect::from_min_size(pos2(6.0, 48.0), vec2(8.0, 8.0)), Rgba([255, 128, 128, 200]));
            }
            5 => {
                cx.fill_rect(r, Rgba([30, 30, 34, 255]));
                if let Some(layout) = text {
                    let highlight = TextHighlight { range: 0..(2 + (v as usize % 3)), fill: Rgba([60, 90, 200, 255]) };
                    cx.text_highlighted(layout, pos2(8.0, 10.0), None, highlight);
                }
            }
            6 => {
                cx.push_opacity(0.5 + (v % 3) as f32 * 0.1, r);
                cx.fill_rect(Rect::from_min_size(pos2(6.0, 6.0), vec2(50.0, 40.0)), Rgba([255, 60, 60, 255]));
                cx.fill_rect(Rect::from_min_size(pos2(36.0, 14.0), vec2(50.0, 40.0)), Rgba([60, 60, 255, 255]));
                cx.pop_opacity();
                cx.push_clip(Clip { rect: Rect::from_min_size(pos2(60.0, 4.0), vec2(36.0, 36.0)), radii: CornerRadii::same(12.0) });
                let mut star = craft_ui_rt::BezPath::new();
                for (i, (x, y)) in [(78.0, 2.0), (90.0, 40.0), (62.0, 16.0), (94.0, 16.0), (66.0, 40.0)].into_iter().enumerate() {
                    if i == 0 {
                        star.move_to((x, y));
                    } else {
                        star.line_to((x, y));
                    }
                }
                star.close_path();
                cx.path(
                    Arc::new(star),
                    Some(Fill { color: Rgba([60, 200, 90, 255]), rule: FillRule::EvenOdd, convex: false }),
                    Some(Stroke::new(2.0, Rgba([255; 4]))),
                );
                cx.pop_clip();
            }
            _ => {
                cx.fill_rect(r, Rgba([12, 12, 12, 255]));
                cx.surface(SURFACE, r);
                cx.rect(r, CornerRadii::ZERO, Rgba([0; 4]), Some(Stroke::new(1.0, Rgba([255, 255, 255, 120]))));
            }
        }
    }
}

/// One runtime with the scene's window, and the states that repaint its cells.
pub struct Scene {
    pub runtime: Runtime,
    pub cells: Vec<State<u32>>,
}

/// The scene; the cell kinds in `plain` draw only a rect. Kind 1 is the blurred shadow, 5 the text.
pub fn scene(scale: f32, plain: &'static [usize]) -> Scene {
    let mut runtime = Runtime::new(tokens());
    runtime.add_window(WINDOW, viewport(), scale);
    let cells: Vec<State<u32>> = (0..COLUMNS * ROWS).map(|_| runtime.store_mut().create(0u32)).collect();
    runtime.images_mut().insert_with(IMAGE, Arc::new(image_pixels(16, 0)), ImageOptions::default());
    runtime.set_root(WINDOW, Grid { cells: cells.clone(), plain });
    runtime.invalidate_window(WINDOW);
    Scene { runtime, cells }
}

/// The physical size of the scene's window at `scale`.
pub fn target_size(scale: f32) -> SizeU {
    SizeU::from_logical(viewport(), scale)
}
