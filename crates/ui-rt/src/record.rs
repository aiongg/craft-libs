//! A renderer with no GPU: it records what each frame drew, for checks of what the runtime
//! produces. Its text engine gives every character the same advance.

use std::collections::HashMap;
use std::sync::Arc;

use craft_theme::Rgba;
use emath::{Pos2, Rect, Vec2};

use crate::display::{DisplayList, ImageStore, Item, RgbaImage};
use crate::frame::{FrameTree, Part};
use crate::geom::{RectU, SizeU, is_empty};
use crate::id::{NodeId, WindowId};
use crate::platform::SurfaceHandle;
use crate::render::{ReadBack, ReadBackId, RenderStats, Renderer};
use crate::text::{TextEngine, TextHandle, TextLayout, TextSpec};

/// One drawn display list, resolved to window space.
#[derive(Clone, Debug)]
pub struct RecordedDraw {
    pub node: NodeId,
    pub part: Part,
    pub version: u64,
    pub origin: Vec2,
    pub clip: Rect,
    pub opacity: f32,
    pub list: Arc<DisplayList>,
}

#[derive(Clone, Debug)]
pub struct RecordedFrame {
    pub window: WindowId,
    pub draws: Vec<RecordedDraw>,
    pub damage: Vec<Rect>,
    pub repainted: Vec<NodeId>,
    pub moved: Vec<NodeId>,
    pub removed: Vec<NodeId>,
    pub full: bool,
}

/// Records every window's frames. Read-backs are ready at the first poll, as a coarse image of
/// the window's last frame.
#[derive(Default)]
pub struct RecordingRenderer {
    text: FixedText,
    targets: HashMap<WindowId, (SizeU, f32)>,
    /// Versions seen per node, so `prepared` counts only lists that changed.
    seen: HashMap<(NodeId, Part), u64>,
    pub frames: Vec<RecordedFrame>,
    /// Image changes applied, in order.
    pub image_syncs: Vec<crate::display::ImageChange>,
    read_backs: HashMap<ReadBackId, ReadBack>,
    next_read_back: u64,
}

impl RecordingRenderer {
    /// The last frame of any window.
    pub fn last(&self) -> Option<&RecordedFrame> {
        self.frames.last()
    }

    /// The last frame of `window`.
    pub fn last_of(&self, window: WindowId) -> Option<&RecordedFrame> {
        self.frames.iter().rev().find(|f| f.window == window)
    }
}

impl Renderer for RecordingRenderer {
    type Text = FixedText;

    fn text(&mut self) -> &mut FixedText {
        &mut self.text
    }

    fn set_target(&mut self, window: WindowId, _: SurfaceHandle, size_px: SizeU, scale: f32) {
        self.targets.insert(window, (size_px, scale));
    }

    fn lose_target(&mut self, window: WindowId) {
        self.targets.remove(&window);
    }

    fn sync_images(&mut self, images: &ImageStore) -> u32 {
        self.image_syncs.extend_from_slice(images.changes());
        u32::try_from(images.changes().len()).unwrap_or(u32::MAX)
    }

    fn render(&mut self, window: WindowId, frame: &FrameTree<'_>, damage: &[Rect]) -> RenderStats {
        let mut stats = RenderStats::default();
        let changes = frame.changes();
        for id in &changes.removed {
            self.seen.remove(&(*id, Part::Content));
            self.seen.remove(&(*id, Part::Overlay));
        }
        let mut out = RecordedFrame {
            window,
            draws: Vec::new(),
            damage: damage.to_vec(),
            repainted: changes.repainted.clone(),
            moved: changes.moved.clone(),
            removed: changes.removed.clone(),
            full: changes.full,
        };
        frame.walk(|d| {
            stats.draws += 1;
            if self.seen.insert((d.node, d.part), d.version) == Some(d.version) {
                stats.reused += 1;
            } else {
                stats.prepared += 1;
            }
            out.draws.push(RecordedDraw {
                node: d.node,
                part: d.part,
                version: d.version,
                origin: d.origin,
                clip: d.clip,
                opacity: d.opacity,
                list: d.list.clone(),
            });
        });
        self.frames.push(out);
        stats
    }

    fn request_read_back(&mut self, window: WindowId, rect: Option<RectU>) -> ReadBackId {
        self.next_read_back += 1;
        let id = ReadBackId(self.next_read_back);
        let result = match self.targets.get(&window) {
            Some(&(size, scale)) => {
                let full = self.picture(window, size, scale);
                match rect {
                    None => ReadBack::Ready(full),
                    Some(r) if r.within(RectU::of_size(size)) => ReadBack::Ready(crop(&full, r)),
                    Some(_) => ReadBack::Failed,
                }
            }
            None => ReadBack::Failed,
        };
        self.read_backs.insert(id, result);
        id
    }

    fn poll_read_back(&mut self, id: ReadBackId) -> ReadBack {
        self.read_backs.remove(&id).unwrap_or(ReadBack::Failed)
    }
}

fn crop(img: &RgbaImage, r: RectU) -> RgbaImage {
    let mut out = RgbaImage::new(SizeU::new(r.width(), r.height()));
    let (stride, row) = (img.size.width as usize * 4, r.width() as usize * 4);
    for (i, y) in (r.y0..r.y1).enumerate() {
        let at = y as usize * stride + r.x0 as usize * 4;
        out.pixels[i * row..(i + 1) * row].copy_from_slice(&img.pixels[at..at + row]);
    }
    out
}

impl RecordingRenderer {
    /// A coarse image of `window`'s last frame: rects, images and text boxes as flat fills,
    /// clipped, no anti-aliasing, corners square. Enough to compare layouts, not looks.
    fn picture(&self, window: WindowId, size: SizeU, scale: f32) -> RgbaImage {
        let mut img = RgbaImage::new(size);
        let Some(frame) = self.last_of(window) else {
            return img;
        };
        let s = scale;
        for d in &frame.draws {
            let mut clips = vec![d.clip];
            for item in &d.list.items {
                let clip = clips.last().copied().unwrap_or(d.clip);
                let fill = |img: &mut RgbaImage, r: Rect, c: Rgba| fill_rect(img, r.translate(d.origin).intersect(clip), c, s);
                match item {
                    Item::Rect { rect, fill: c, .. } => fill(&mut img, *rect, *c),
                    Item::Circle { center, radius, fill: c, .. } => fill(&mut img, Rect::from_center_size(*center, Vec2::splat(radius * 2.0)), *c),
                    Item::Image { rect, tint: c, .. } | Item::ImageMask { rect, color: c, .. } => fill(&mut img, *rect, *c),
                    Item::Text { layout, origin, color, .. } => {
                        if let Some(t) = layout.downcast_ref::<FixedLayout>() {
                            fill(&mut img, Rect::from_min_size(*origin, t.size), color.unwrap_or(t.color).faded(0.5));
                        }
                    }
                    Item::PushClip(c) => clips.push(c.rect.translate(d.origin).intersect(clip)),
                    Item::PopClip => {
                        clips.pop();
                    }
                    Item::Line { .. }
                    | Item::Path { .. }
                    | Item::Mesh(_)
                    | Item::Shadow { .. }
                    | Item::Surface { .. }
                    | Item::VGradient { .. }
                    | Item::PushOpacity { .. }
                    | Item::PopOpacity => {}
                }
            }
        }
        img
    }
}

/// Source-over of a flat colour onto the pixels `r` (logical points) covers.
fn fill_rect(img: &mut RgbaImage, r: Rect, c: Rgba, scale: f32) {
    if is_empty(r) || c.0[3] == 0 {
        return;
    }
    let (w, h) = (img.size.width as f32, img.size.height as f32);
    let x0 = (r.min.x * scale).round().clamp(0.0, w) as usize;
    let x1 = (r.max.x * scale).round().clamp(0.0, w) as usize;
    let y0 = (r.min.y * scale).round().clamp(0.0, h) as usize;
    let y1 = (r.max.y * scale).round().clamp(0.0, h) as usize;
    let a = u32::from(c.0[3]);
    let stride = img.size.width as usize * 4;
    for y in y0..y1 {
        for x in x0..x1 {
            let p = &mut img.pixels[y * stride + x * 4..y * stride + x * 4 + 4];
            for (d, s) in p.iter_mut().zip(c.0).take(3) {
                *d = ((u32::from(s) * a + u32::from(*d) * (255 - a)) / 255) as u8;
            }
            p[3] = (a + u32::from(p[3]) * (255 - a) / 255) as u8;
        }
    }
}

/// What [`FixedText`] keeps per layout.
#[derive(Debug)]
pub struct FixedLayout {
    pub text: Arc<str>,
    pub advance: f32,
    /// Characters per row; line breaks also end a row.
    pub per_line: usize,
    pub line: f32,
    pub size: Vec2,
    pub color: Rgba,
}

/// Every character advances half the font size; lines are the line height (1.25 x size when the
/// spec has none). One line unless the spec wraps. A stack it is given changes no metric; it is
/// kept and counted, as a real engine keeps it.
#[derive(Default)]
pub struct FixedText {
    stack: Option<crate::text::FontStack>,
    stack_generation: u64,
}

impl TextEngine for FixedText {
    fn set_stack(&mut self, stack: &crate::text::FontStack) {
        self.stack = Some(stack.clone());
        self.stack_generation += 1;
    }

    fn stack_generation(&self) -> u64 {
        self.stack_generation
    }

    fn stack(&self) -> Option<&crate::text::FontStack> {
        self.stack.as_ref()
    }

    fn layout(&mut self, spec: &TextSpec, _scale: f32) -> TextLayout {
        let size = spec.font.size;
        let advance = size * 0.5;
        let line = spec.line_height.unwrap_or(size * 1.25);
        let text = crate::text::overflow_text(spec, |t| t.chars().count() as f32 * advance);
        let chars = text.chars().count();
        let per_line =
            spec.max_width.filter(|_| spec.overflow == crate::text::Overflow::Wrap).map_or(usize::MAX, |w| ((w / advance).floor() as usize).max(1));
        let lines = if chars == 0 { 1 } else { chars.div_ceil(per_line) };
        let width = advance * chars.min(per_line) as f32;
        let layout = FixedLayout { text, advance, per_line, line, size: Vec2::new(width, line * lines as f32), color: spec.color };
        let out_size = layout.size;
        TextLayout { handle: TextHandle::new(Arc::new(layout)), size: out_size, baseline: size, lines }
    }

    fn hit_index(&mut self, layout: &TextLayout, pos: Pos2) -> usize {
        let Some(t) = layout.handle.downcast_ref::<FixedLayout>() else {
            return 0;
        };
        let n = ((pos.x / t.advance).round().max(0.0) as usize).min(t.text.chars().count());
        t.text.char_indices().nth(n).map_or(t.text.len(), |(i, _)| i)
    }

    fn caret_rect(&mut self, layout: &TextLayout, index: usize) -> Rect {
        let Some(t) = layout.handle.downcast_ref::<FixedLayout>() else {
            return Rect::NOTHING;
        };
        let n = t.text.get(..index).map_or(0, |s| s.chars().count());
        let x = t.advance * n as f32;
        Rect::from_min_max(Pos2::new(x, 0.0), Pos2::new(x + 1.0, t.size.y))
    }

    /// Rows of at most `per_line` characters, broken at line breaks too; one glyph per character
    /// (a break has none).
    fn metrics(&mut self, layout: &TextLayout) -> crate::text::TextMetrics {
        let Some(t) = layout.handle.downcast_ref::<FixedLayout>() else {
            return crate::text::TextMetrics { text: Arc::from(""), rows: Vec::new(), ink: Rect::NOTHING, elided: false };
        };
        let mut rows = Vec::new();
        for line in t.text.split('\n') {
            let chars: Vec<char> = line.chars().collect();
            let chunks: Vec<&[char]> = if chars.is_empty() { vec![&[][..]] } else { chars.chunks(t.per_line.max(1)).collect() };
            for chunk in chunks {
                let top = rows.len() as f32 * t.line;
                let glyphs = chunk
                    .iter()
                    .enumerate()
                    .map(|(i, c)| crate::text::GlyphPlace { chr: *c, x: i as f32 * t.advance, advance: t.advance, rtl: false })
                    .collect();
                let rect = Rect::from_min_size(Pos2::new(0.0, top), Vec2::new(chunk.len() as f32 * t.advance, t.line));
                rows.push(crate::text::TextRow { rect, glyphs });
            }
        }
        let ink = rows.iter().fold(Rect::NOTHING, |ink, row| ink.union(row.rect));
        crate::text::TextMetrics { text: t.text.clone(), rows, ink, elided: false }
    }
}
