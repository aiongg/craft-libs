//! The rt window's renderer: render-epaint's kept meshes, moved to window coordinates once per rt
//! frame into one list of `(clip, mesh)`, merged per clip and texture, which `show` adds to egui's
//! painter on every pass. Images are egui textures; text uses egui's fonts and atlas.
//!
//! Damage is not used: egui draws the whole window every pass, so the list always holds every
//! mesh of the window.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use craft_ui_render_epaint::{EpaintText, KeptMeshes, PieceKind, TextureIds, color_image, texture_options};
use craft_ui_rt::{
    FrameTree, ImageChange, ImageKey, ImageStore, ReadBack, ReadBackId, Rect, RectU, RenderStats, Renderer, SizeU, SurfaceHandle, WindowId,
};
use egui::{TextureHandle, TextureId};
use epaint::{Mesh, TessellationOptions};
use web_time::Instant;

use crate::fonts::EguiFonts;

/// An image's egui textures. Dropping a handle frees its texture.
struct Tex {
    image: TextureHandle,
    /// White with the image's alpha, for masks; made the first time a mesh draws the image as one.
    mask: Option<TextureHandle>,
}

struct Textures {
    ctx: egui::Context,
    by_key: HashMap<ImageKey, Tex>,
}

impl Textures {
    fn load(&self, key: ImageKey, images: &ImageStore, mask: bool) -> Option<TextureHandle> {
        let image = images.get(key)?;
        let size = [image.size.width as usize, image.size.height as usize];
        let name = format!("craft-rt-image-{}{}", key.0, if mask { "-mask" } else { "" });
        Some(self.ctx.load_texture(name, color_image(size, &image.pixels, mask), texture_options(images.options(key))))
    }
}

/// The ids kept meshes are tessellated with. A mask texture is made on first use.
struct Ids<'a> {
    textures: &'a RefCell<Textures>,
    images: &'a ImageStore,
}

impl TextureIds for Ids<'_> {
    fn image(&self, key: ImageKey) -> TextureId {
        self.textures.borrow().by_key.get(&key).map_or_else(TextureId::default, |t| t.image.id())
    }

    fn mask(&self, key: ImageKey) -> TextureId {
        let mut textures = self.textures.borrow_mut();
        match textures.by_key.get(&key) {
            None => return TextureId::default(),
            Some(Tex { mask: Some(mask), .. }) => return mask.id(),
            Some(_) => {}
        }
        let Some(mask) = textures.load(key, self.images, true) else { return TextureId::default() };
        let id = mask.id();
        if let Some(t) = textures.by_key.get_mut(&key) {
            t.mask = Some(mask);
        }
        id
    }
}

pub(crate) struct EguiPainter {
    text: EpaintText<EguiFonts>,
    options: TessellationOptions,
    scale: f32,
    /// Made at the first render, when egui's fonts exist.
    kept: Option<KeptMeshes>,
    textures: RefCell<Textures>,
    /// The window's meshes in window coordinates with their clip, in paint order.
    shapes: Vec<(Rect, Arc<Mesh>)>,
    vertices: u32,
}

impl EguiPainter {
    pub fn new(ctx: egui::Context) -> EguiPainter {
        let options = ctx.tessellation_options(|o| *o);
        EguiPainter {
            text: EpaintText::from_fonts(EguiFonts::new(ctx.clone())),
            options,
            scale: 1.0,
            kept: None,
            textures: RefCell::new(Textures { ctx, by_key: HashMap::new() }),
            shapes: Vec::new(),
            vertices: 0,
        }
    }

    /// Whether the kept meshes point into egui's current glyph atlas: egui's own text can grow
    /// or clear it between rt frames.
    pub fn is_current(&self) -> bool {
        self.kept.as_ref().is_none_or(|k| k.is_current(&self.text))
    }

    /// Adds the window's meshes to `painter`, each cut by its own clip and `painter`'s.
    pub fn emit(&self, painter: &egui::Painter) -> (u32, u32) {
        for (clip, mesh) in &self.shapes {
            painter.with_clip_rect(*clip).add(egui::Shape::Mesh(mesh.clone()));
        }
        (u32::try_from(self.shapes.len()).unwrap_or(u32::MAX), self.vertices)
    }

    fn collect(&mut self, frame: &FrameTree<'_>, stats: &mut RenderStats) {
        let kept = self.kept.get_or_insert_with(|| KeptMeshes::new(&self.text, self.scale, self.options));
        kept.begin(frame, &self.text);
        let ids = Ids { textures: &self.textures, images: frame.images() };
        let text = &mut self.text;
        let mut shapes: Vec<(Rect, Mesh)> = Vec::with_capacity(self.shapes.len());
        frame.walk(|draw| {
            stats.draws += 1;
            let entry = kept.draw(&draw, text, &ids, stats);
            for piece in entry.pieces() {
                // App-drawn surfaces need a host hook; the embedded content draws none.
                let PieceKind::Mesh(mesh) = piece.kind else { continue };
                if !piece.clip_rect.is_positive() {
                    continue;
                }
                let moved = entry.moved(mesh);
                if moved.is_empty() {
                    continue;
                }
                match shapes.last_mut() {
                    Some((clip, last)) if *clip == piece.clip_rect && last.texture_id == moved.texture_id => last.append(moved),
                    _ => shapes.push((piece.clip_rect, moved)),
                }
            }
        });
        self.vertices = u32::try_from(shapes.iter().map(|(_, m)| m.vertices.len()).sum::<usize>()).unwrap_or(u32::MAX);
        self.shapes = shapes.into_iter().map(|(clip, mesh)| (clip, Arc::new(mesh))).collect();
    }
}

impl Renderer for EguiPainter {
    type Text = EpaintText<EguiFonts>;

    fn text(&mut self) -> &mut EpaintText<EguiFonts> {
        &mut self.text
    }

    fn set_target(&mut self, _: WindowId, _: SurfaceHandle, _: SizeU, scale: f32) {
        if scale != self.scale {
            self.scale = scale;
            self.kept = None;
        }
    }

    fn lose_target(&mut self, _: WindowId) {
        self.kept = None;
        self.shapes.clear();
        self.vertices = 0;
    }

    fn sync_images(&mut self, images: &ImageStore) -> u32 {
        let mut synced = 0;
        let textures = self.textures.get_mut();
        for change in images.changes() {
            match *change {
                ImageChange::Set(key) => {
                    let Some(image) = textures.load(key, images, false) else { continue };
                    let had_mask = textures.by_key.get(&key).is_some_and(|t| t.mask.is_some());
                    let mask = if had_mask { textures.load(key, images, true) } else { None };
                    textures.by_key.insert(key, Tex { image, mask });
                    synced += 1;
                }
                ImageChange::Pixels(key, rect) => {
                    let Some(image) = images.get(key) else { continue };
                    let whole = RectU::of_size(image.size);
                    let Some(tex) = textures.by_key.get_mut(&key).filter(|_| !rect.is_empty() && rect.within(whole)) else {
                        // Nothing to update in: the whole image goes up instead.
                        if let Some(image) = textures.load(key, images, false) {
                            textures.by_key.insert(key, Tex { image, mask: None });
                        }
                        synced += 1;
                        continue;
                    };
                    let stride = image.size.width as usize * 4;
                    let row = rect.width() as usize * 4;
                    let mut bytes = Vec::with_capacity(row * rect.height() as usize);
                    for y in rect.y0..rect.y1 {
                        let at = y as usize * stride + rect.x0 as usize * 4;
                        bytes.extend_from_slice(&image.pixels[at..at + row]);
                    }
                    let (size, at) = ([rect.width() as usize, rect.height() as usize], [rect.x0 as usize, rect.y0 as usize]);
                    let options = texture_options(images.options(key));
                    tex.image.set_partial(at, color_image(size, &bytes, false), options);
                    if let Some(mask) = &mut tex.mask {
                        mask.set_partial(at, color_image(size, &bytes, true), options);
                    }
                    synced += 1;
                }
                ImageChange::Removed(key) => {
                    textures.by_key.remove(&key);
                }
            }
        }
        synced
    }

    fn render(&mut self, _: WindowId, frame: &FrameTree<'_>, _damage: &[Rect]) -> RenderStats {
        let t = Instant::now();
        let mut stats = RenderStats::default();
        self.collect(frame, &mut stats);
        if !self.is_current() {
            // Shaping again grew the atlas, which moves every glyph coordinate: start over once.
            stats = RenderStats::default();
            self.collect(frame, &mut stats);
        }
        stats.prepare_ms = t.elapsed().as_secs_f64() * 1000.0;
        stats
    }

    fn request_read_back(&mut self, _: WindowId, _: Option<RectU>) -> ReadBackId {
        ReadBackId(0)
    }

    /// egui owns the pixels.
    fn poll_read_back(&mut self, _: ReadBackId) -> ReadBack {
        ReadBack::Failed
    }
}
