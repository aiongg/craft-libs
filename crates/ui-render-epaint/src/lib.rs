//! craft-ui-rt's `Renderer` and `TextEngine` on epaint's tessellator and egui-wgpu.
//!
//! Each node's two display lists are tessellated into primitives in the node's own coordinates
//! and kept per (node, part) with the list's version. A frame translates the kept primitives to
//! the node's window position as it uploads them, so a scroll or a move re-tessellates nothing.
//!
//! Every window has its own canvas texture, which outlives the frame. A frame clears and redraws
//! only the damage rects: it uploads the primitives that reach into them, scissored to them, and
//! leaves every other pixel as the last frame drew it. A window gets the canvas copied onto its
//! swapchain image. epaint snaps shapes and glyph positions to the pixel grid while tessellating,
//! so a node whose origin is not on a whole pixel is tessellated with its fractional pixel offset
//! (`shift`) added and moved back by the whole-pixel remainder; the snapping then lands where it
//! does when the shape is tessellated in window coordinates, as the egui build does.
//!
//! One wgpu device, queue and font atlas serve all windows; the device is the one an app-drawn
//! surface ([`SurfaceHook`]) draws on. Windows come from the platform as raw window handles
//! (craft-ui-rt's [`NativeWindow`]); this crate links no window system.
//!
//! The device, the queue, the font atlas and the images survive `lose_target`.

#![allow(clippy::chunks_exact_to_as_chunks)]

mod clip;
mod concave;
mod paint;
mod readback;
mod surface;
mod text;
mod timer;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use craft_theme::Rgba;
use craft_ui_rt::{
    FrameTree, ImageChange, ImageKey, ImageOptions, ImageStore, NativeWindow, Part, ReadBack, ReadBackId, RectU, RenderStats, Renderer, RgbaImage,
    SizeU, SurfaceHandle, WindowId, damage_pixels,
};
use egui_wgpu::{RenderState, RendererOptions, ScreenDescriptor, WgpuConfiguration, WgpuSetup};
use emath::{Rect as ERect, Vec2};
use epaint::textures::{TextureFilter, TextureOptions};
use epaint::{ClippedPrimitive, ColorImage, ImageDelta, PaintCallback, Primitive, TessellationOptions, Tessellator};

use crate::paint::{Env, Piece, Prim, mask_texture};
use crate::readback::PendingRead;
use crate::timer::Timer;

pub use egui_wgpu::wgpu;
pub use epaint::text::{FontDefinitions, TextOptions};
pub use surface::{SurfaceHook, SurfaceInfo, SurfacePrepare};
pub use text::{CONDENSED_FAMILY, EpaintText, SEMIBOLD_FAMILY};

pub type Error = Box<dyn std::error::Error>;

#[derive(Clone)]
pub struct EpaintOptions {
    pub text: TextOptions,
    pub tessellation: TessellationOptions,
    pub renderer: RendererOptions,
    pub config: WgpuConfiguration,
    /// What a frame starts from, straight alpha.
    pub clear: Rgba,
    /// Offscreen windows: write GPU timestamps at the start and end of each frame's render pass
    /// and wait for them, which fills [`FrameTimes::gpu_ms`]. Asks for the device's timestamp
    /// feature where the adapter has it. For benchmarks, never for windows.
    pub gpu_timing: bool,
}

impl Default for EpaintOptions {
    fn default() -> Self {
        EpaintOptions {
            text: TextOptions::default(),
            tessellation: TessellationOptions::default(),
            renderer: RendererOptions::default(),
            config: WgpuConfiguration::default(),
            clear: Rgba([0, 0, 0, 0]),
            gpu_timing: false,
        }
    }
}

impl EpaintOptions {
    /// The options of egui's snapshot renderer: no dithering and predictable texture filtering,
    /// so offscreen pixels compare exactly.
    pub fn predictable() -> Self {
        EpaintOptions { renderer: RendererOptions::PREDICTABLE, ..Self::default() }
    }
}

/// What the last frame cost beyond [`RenderStats`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTimes {
    /// Clearing, command encoding and submission, in milliseconds.
    pub submit_ms: f64,
    /// The render pass on the GPU, when [`EpaintOptions::gpu_timing`] is on and the frame was
    /// offscreen.
    pub gpu_ms: Option<f64>,
}

/// What the last `sync_images` uploaded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Images uploaded whole.
    pub full: u32,
    /// Sub-rect uploads.
    pub partial: u32,
    /// Texels written to image textures.
    pub texels: u64,
}

/// Where a window's canvas goes after a frame.
enum Kind {
    Offscreen,
    Window { native: NativeWindow, surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration, blitter: wgpu::util::TextureBlitter },
}

/// A kept piece list with the version and shift it was made for.
struct Kept {
    version: u64,
    shift: Vec2,
    prims: Vec<Prim>,
    masks: Vec<ImageKey>,
}

struct WindowState {
    kind: Kind,
    /// The window's pixels, kept from frame to frame.
    canvas: wgpu::Texture,
    size_px: SizeU,
    scale: f32,
    tessellator: Tessellator,
    /// The atlas generation and size the tessellator and kept primitives belong to.
    atlas: (u64, [usize; 2]),
    kept: HashMap<(craft_ui_rt::NodeId, Part), Kept>,
}

pub struct EpaintRenderer {
    gpu: RenderState,
    options: EpaintOptions,
    text: EpaintText,
    windows: HashMap<WindowId, WindowState>,
    /// The surface made while choosing the adapter, until its window's `set_target` takes it.
    first_surface: Option<(NativeWindow, wgpu::Surface<'static>)>,
    /// Image keys with a texture, to tell a changed rect of an uploaded image from a new image.
    uploaded: HashSet<ImageKey>,
    /// Image keys that also have an alpha-only texture ([`craft_ui_rt::Item::ImageMask`]).
    masks: HashSet<ImageKey>,
    hooks: surface::Hooks,
    /// Clear-colour pixels for the largest damage rect so far.
    clear_px: Vec<u8>,
    timer: Option<Timer>,
    times: FrameTimes,
    synced: SyncReport,
    reads: HashMap<ReadBackId, PendingRead>,
    next_read: u64,
}

impl EpaintRenderer {
    /// Creates the device. `fonts` are the app's faces (craft's font stack flattened). `first`
    /// is a window the adapter must be able to draw to; `None` picks any adapter, for headless
    /// runs.
    pub async fn create(options: EpaintOptions, fonts: FontDefinitions, first: Option<&NativeWindow>) -> Result<Self, Error> {
        let mut config = options.config.clone();
        if options.gpu_timing
            && let WgpuSetup::CreateNew(setup) = &mut config.wgpu_setup
        {
            let base = setup.device_descriptor.clone();
            setup.device_descriptor = Arc::new(move |adapter| {
                let mut descriptor = base(adapter);
                if adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
                    descriptor.required_features |= wgpu::Features::TIMESTAMP_QUERY;
                }
                descriptor
            });
        }
        let instance = config.wgpu_setup.new_instance().await;
        let first_surface = match first {
            Some(native) => Some((native.clone(), instance.create_surface(native.clone())?)),
            None => None,
        };
        let gpu = RenderState::create(&config, &instance, first_surface.as_ref().map(|(_, s)| s), options.renderer).await?;
        let timer = if options.gpu_timing { Timer::new(&gpu) } else { None };
        Ok(EpaintRenderer {
            timer,
            gpu,
            text: EpaintText::new(fonts, options.text),
            options,
            windows: HashMap::new(),
            first_surface,
            uploaded: HashSet::new(),
            masks: HashSet::new(),
            hooks: HashMap::new(),
            clear_px: Vec::new(),
            times: FrameTimes::default(),
            synced: SyncReport::default(),
            reads: HashMap::new(),
            next_read: 0,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn create_blocking(options: EpaintOptions, fonts: FontDefinitions, first: Option<&NativeWindow>) -> Result<Self, Error> {
        pollster::block_on(Self::create(options, fonts, first))
    }

    /// The device every window and every [`SurfaceHook`] draws on.
    pub fn device(&self) -> &wgpu::Device {
        &self.gpu.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.gpu.queue
    }

    /// The format of every window's canvas, and so of what a hook draws into.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.gpu.target_format
    }

    /// Registers the app's drawing for `Item::Surface` items with `key`.
    pub fn set_surface_hook(&mut self, key: craft_ui_rt::SurfaceKey, hook: Arc<dyn SurfaceHook>) {
        self.hooks.insert(key, hook);
    }

    pub fn remove_surface_hook(&mut self, key: craft_ui_rt::SurfaceKey) {
        self.hooks.remove(&key);
    }

    /// Cost of the last frame drawn beyond [`RenderStats`].
    pub fn last_times(&self) -> FrameTimes {
        self.times
    }

    /// What the last `sync_images` uploaded.
    pub fn last_sync(&self) -> SyncReport {
        self.synced
    }

    /// The glyph atlas generation (see [`EpaintText::atlas_generation`]).
    pub fn atlas_generation(&self) -> u64 {
        self.text.atlas_generation()
    }

    fn configure(&self, state: &mut WindowState) {
        if let Kind::Window { surface, config, .. } = &mut state.kind {
            config.width = state.size_px.width;
            config.height = state.size_px.height;
            surface.configure(&self.gpu.device, config);
        }
    }

    fn make_canvas(&self, size_px: SizeU) -> wgpu::Texture {
        self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("craft-ui-render-epaint canvas"),
            size: wgpu::Extent3d { width: size_px.width, height: size_px.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.gpu.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    fn window_kind(&mut self, native: &NativeWindow, size_px: SizeU) -> Option<Kind> {
        let surface = match self.first_surface.take() {
            Some((n, s)) if n.same(native) => s,
            _ => match self.gpu.instance.create_surface(native.clone()) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("surface creation failed: {e}");
                    return None;
                }
            },
        };
        let Some(mut config) = surface.get_default_config(&self.gpu.adapter, size_px.width, size_px.height) else {
            log::error!("the adapter cannot draw to this surface");
            return None;
        };
        let surface_options = self.options.config.surface;
        config.format = self.gpu.target_format;
        config.present_mode = surface_options.present_mode;
        config.desired_maximum_frame_latency = surface_options.desired_maximum_frame_latency.unwrap_or(2);
        let blitter = wgpu::util::TextureBlitter::new(&self.gpu.device, config.format);
        Some(Kind::Window { native: native.clone(), surface, config, blitter })
    }

    /// Clears `rects` of the window's canvas to the clear colour.
    fn clear(&mut self, canvas: &wgpu::Texture, rects: &[RectU]) {
        let [r, g, b, a] = self.options.clear.0;
        let pre = |c: u8| ((u32::from(c) * u32::from(a) + 127) / 255) as u8;
        let pixel = match self.gpu.target_format {
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => [pre(b), pre(g), pre(r), a],
            _ => [pre(r), pre(g), pre(b), a],
        };
        let most = rects.iter().map(|r| r.width() as usize * r.height() as usize * 4).max().unwrap_or(0);
        if self.clear_px.len() < most || self.clear_px.get(..4) != Some(&pixel[..]) {
            self.clear_px = pixel.repeat(most / 4);
        }
        for r in rects {
            self.gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: canvas,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: r.x0, y: r.y0, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &self.clear_px[..r.width() as usize * r.height() as usize * 4],
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(r.width() * 4), rows_per_image: Some(r.height()) },
                wgpu::Extent3d { width: r.width(), height: r.height(), depth_or_array_layers: 1 },
            );
        }
    }

    /// Copies the canvas onto the window's next swapchain image.
    fn present(&self, state: &mut WindowState, encoder: &mut wgpu::CommandEncoder) -> Option<wgpu::SurfaceTexture> {
        use wgpu::CurrentSurfaceTexture as C;
        let got = {
            let Kind::Window { surface, .. } = &state.kind else { return None };
            surface.get_current_texture()
        };
        let got = match got {
            C::Outdated | C::Lost => {
                self.configure(state);
                let Kind::Window { surface, .. } = &state.kind else { return None };
                surface.get_current_texture()
            }
            other => other,
        };
        let frame = match got {
            C::Success(t) | C::Suboptimal(t) => t,
            other => {
                log::debug!("frame skipped: {other:?}");
                return None;
            }
        };
        let Kind::Window { blitter, .. } = &state.kind else { return None };
        let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        blitter.copy(&self.gpu.device, encoder, &view(&state.canvas), &view(&frame.texture));
        Some(frame)
    }

    fn upload_image(&self, key: ImageKey, image: &RgbaImage, options: ImageOptions, mask: bool) {
        let size = [image.size.width as usize, image.size.height as usize];
        let color = color_image(size, &image.pixels, mask);
        let id = if mask { mask_texture(key) } else { paint::image_texture(key) };
        let delta = ImageDelta::full(color, texture_options(options));
        self.gpu.renderer.write().update_texture(&self.gpu.device, &self.gpu.queue, id, &delta);
    }

    /// Alpha-only textures for the images the frame draws as masks that have none yet.
    fn upload_masks(&mut self, needed: &HashSet<ImageKey>, images: &ImageStore) {
        for &key in needed {
            if self.masks.contains(&key) {
                continue;
            }
            if let Some(image) = images.get(key) {
                self.upload_image(key, image, images.options(key), true);
                self.masks.insert(key);
            }
        }
    }

    /// The kept primitives that reach into a damage rect, moved to window coordinates and
    /// scissored to that rect. Lists that paint outside every rect are not tessellated.
    fn primitives(
        &mut self,
        state: &mut WindowState,
        frame: &FrameTree<'_>,
        damage: &[RectU],
        stats: &mut RenderStats,
        needed: &mut HashSet<ImageKey>,
    ) -> Vec<ClippedPrimitive> {
        let atlas = (self.text.atlas_generation(), self.text.atlas_size());
        if state.atlas != atlas {
            state.atlas = atlas;
            state.kept.clear();
            state.tessellator = self.text.tessellator(state.scale, self.options.tessellation);
        }
        for id in &frame.changes().removed {
            state.kept.remove(&(*id, Part::Content));
            state.kept.remove(&(*id, Part::Overlay));
        }
        let mut out = Vec::new();
        if damage.is_empty() {
            return out;
        }
        let erect = |r: emath::Rect| r;
        let damage: Vec<ERect> = damage.iter().map(|r| erect(r.to_logical(state.scale))).collect();
        let scale = f64::from(state.scale);
        let WindowState { kept, tessellator, .. } = state;
        let text = &mut self.text;
        let hooks = &self.hooks;
        frame.walk(|draw| {
            stats.draws += 1;
            if !damage.iter().any(|d| d.intersects(draw.ink)) {
                return;
            }
            let frac = |v: f32| {
                let px = f64::from(v) * scale;
                ((px - px.floor()) / scale) as f32
            };
            let shift = Vec2::new(frac(draw.origin.x), frac(draw.origin.y));
            let key = (draw.node, draw.part);
            let fresh = kept.get(&key).is_some_and(|k| k.version == draw.version && k.shift == shift);
            if fresh {
                stats.reused += 1;
            } else {
                let done = paint::tessellate(draw.list, shift, &mut Env { tessellator, text, scale: scale as f32 });
                kept.insert(key, Kept { version: draw.version, shift, prims: done.prims, masks: done.masks });
                stats.prepared += 1;
            }
            let entry = &kept[&key];
            needed.extend(entry.masks.iter().copied());
            let delta = Vec2::new(draw.origin.x - shift.x, draw.origin.y - shift.y);
            let outlines: Vec<Vec<emath::Pos2>> = draw.rounded.iter().map(|c| clip::rounded_outline(c.rect, c.radii, scale as f32)).collect();
            for prim in &entry.prims {
                let clip_rect = prim.clip_rect.translate(delta).intersect(draw.clip);
                let bounds = prim.bounds.translate(delta).intersect(clip_rect);
                match &prim.piece {
                    Piece::Mesh(mesh) => {
                        let mut moved: Option<epaint::Mesh> = None;
                        for d in &damage {
                            let scissor = clip_rect.intersect(*d);
                            if !scissor.is_positive() || !bounds.intersects(*d) {
                                continue;
                            }
                            if moved.is_none() {
                                let mut m = mesh.clone();
                                for v in &mut m.vertices {
                                    v.pos += delta;
                                    if draw.opacity < 1.0 {
                                        v.color = v.color.gamma_multiply(draw.opacity);
                                    }
                                }
                                for outline in &outlines {
                                    clip::clip_to_outline(&mut m, outline);
                                }
                                moved = Some(m);
                            }
                            let Some(m) = moved.as_ref().filter(|m| !m.is_empty()) else { break };
                            out.push(ClippedPrimitive { clip_rect: scissor, primitive: Primitive::Mesh(m.clone()) });
                        }
                    }
                    Piece::Surface { key, rect } => {
                        let Some(hook) = hooks.get(key) else { continue };
                        let rect = rect.translate(delta);
                        for d in &damage {
                            let scissor = clip_rect.intersect(*d);
                            if !scissor.is_positive() || !rect.intersects(*d) {
                                continue;
                            }
                            let callback: PaintCallback = egui_wgpu::Callback::new_paint_callback(rect, surface::Forward(hook.clone(), rect));
                            out.push(ClippedPrimitive { clip_rect: scissor, primitive: Primitive::Callback(callback) });
                        }
                    }
                }
            }
        });
        out
    }

    fn render_window(&mut self, state: &mut WindowState, frame: &FrameTree<'_>, damage: &[emath::Rect], stats: &mut RenderStats) {
        let damage = damage_pixels(damage, state.scale, state.size_px);
        let t = web_time::Instant::now();
        let mut needed = HashSet::new();
        let mut prims = self.primitives(state, frame, &damage, stats, &mut needed);
        if self.text.atlas_size() != state.atlas.1 {
            // Shaping again grew the atlas, which moves every glyph coordinate: start over once.
            *stats = RenderStats::default();
            needed.clear();
            prims = self.primitives(state, frame, &damage, stats, &mut needed);
        }
        stats.prepare_ms = t.elapsed().as_secs_f64() * 1000.0;
        if damage.is_empty() {
            return;
        }
        if let Some(delta) = self.text.atlas_delta() {
            self.gpu.renderer.write().update_texture(&self.gpu.device, &self.gpu.queue, paint::FONT_TEXTURE, &delta);
        }
        self.upload_masks(&needed, frame.images());

        let t = web_time::Instant::now();
        self.clear(&state.canvas, &damage);
        let timed = matches!(state.kind, Kind::Offscreen) && self.timer.is_some();
        let screen = ScreenDescriptor { size_in_pixels: [state.size_px.width, state.size_px.height], pixels_per_point: state.scale };
        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("craft-ui-render-epaint") });
        let extra = {
            let view = state.canvas.create_view(&wgpu::TextureViewDescriptor::default());
            let mut gpu = self.gpu.renderer.write();
            let extra = gpu.update_buffers(&self.gpu.device, &self.gpu.queue, &mut encoder, &prims, &screen);
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("craft-ui-render-epaint"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: self.timer.as_ref().filter(|_| timed).map(Timer::writes),
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            gpu.render(&mut pass, &prims, &screen);
            extra
        };
        if let Some(timer) = self.timer.as_ref().filter(|_| timed) {
            timer.resolve(&mut encoder);
        }
        let presented = self.present(state, &mut encoder);
        self.gpu.queue.submit(extra.into_iter().chain(Some(encoder.finish())));
        if let Some(t) = presented {
            self.gpu.queue.present(t);
        }
        self.times = FrameTimes {
            submit_ms: t.elapsed().as_secs_f64() * 1000.0,
            gpu_ms: self.timer.as_ref().filter(|_| timed).and_then(|timer| timer.read(&self.gpu.device)),
        };
    }
}

fn texture_options(options: ImageOptions) -> TextureOptions {
    let filter = |f: craft_ui_rt::Filter| match f {
        craft_ui_rt::Filter::Linear => TextureFilter::Linear,
        craft_ui_rt::Filter::Nearest => TextureFilter::Nearest,
    };
    // `options.mipmaps` is not applied: egui-wgpu textures have one level.
    TextureOptions { magnification: filter(options.magnify), minification: filter(options.minify), ..TextureOptions::LINEAR }
}

/// The sRGB straight-alpha `rgba` as epaint's premultiplied image; for a mask, white with the
/// alpha kept.
fn color_image(size: [usize; 2], rgba: &[u8], mask: bool) -> ColorImage {
    if mask {
        let white: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [255, 255, 255, p[3]]).collect();
        ColorImage::from_rgba_unmultiplied(size, &white)
    } else {
        ColorImage::from_rgba_unmultiplied(size, rgba)
    }
}

impl Renderer for EpaintRenderer {
    type Text = EpaintText;

    fn text(&mut self) -> &mut EpaintText {
        &mut self.text
    }

    fn set_target(&mut self, window: WindowId, target: SurfaceHandle, size_px: SizeU, scale: f32) {
        let old = self.windows.remove(&window);
        if size_px.width == 0 || size_px.height == 0 {
            return;
        }
        let kind = match target {
            SurfaceHandle::Offscreen => Kind::Offscreen,
            SurfaceHandle::Native(_) => {
                let Some(native) = target.native() else {
                    log::error!("the native surface is not a craft_ui_rt::NativeWindow");
                    return;
                };
                match old.as_ref().map(|o| &o.kind) {
                    Some(Kind::Window { native: n, .. }) if n.same(native) => {
                        let Some(Kind::Window { native, surface, config, blitter }) = old.map(|o| o.kind) else { return };
                        Kind::Window { native, surface, config, blitter }
                    }
                    _ => match self.window_kind(native, size_px) {
                        Some(kind) => kind,
                        None => return,
                    },
                }
            }
        };
        let atlas = (self.text.atlas_generation(), self.text.atlas_size());
        let mut state = WindowState {
            kind,
            canvas: self.make_canvas(size_px),
            size_px,
            scale,
            tessellator: self.text.tessellator(scale, self.options.tessellation),
            atlas,
            kept: HashMap::new(),
        };
        self.configure(&mut state);
        self.windows.insert(window, state);
    }

    fn lose_target(&mut self, window: WindowId) {
        self.windows.remove(&window);
        self.first_surface = None;
    }

    fn sync_images(&mut self, images: &ImageStore) -> u32 {
        self.synced = SyncReport::default();
        // Between layout and drawing: flushes galleys no layout used this frame, and clears the
        // glyph atlas if it filled up (layouts shaped before are shaped again when drawn).
        self.text.begin_pass();
        for change in images.changes() {
            match *change {
                ImageChange::Set(key) => {
                    let Some(image) = images.get(key) else { continue };
                    let options = images.options(key);
                    self.upload_image(key, image, options, false);
                    if self.masks.contains(&key) {
                        self.upload_image(key, image, options, true);
                    }
                    self.uploaded.insert(key);
                    self.synced.full += 1;
                    self.synced.texels += u64::from(image.size.width) * u64::from(image.size.height);
                }
                ImageChange::Pixels(key, rect) => {
                    let Some(image) = images.get(key) else { continue };
                    if !self.uploaded.contains(&key) || !rect.within(RectU::of_size(image.size)) || rect.is_empty() {
                        // Nothing to update in: the whole image goes up instead.
                        self.upload_image(key, image, images.options(key), false);
                        self.uploaded.insert(key);
                        self.synced.full += 1;
                        self.synced.texels += u64::from(image.size.width) * u64::from(image.size.height);
                        continue;
                    }
                    let stride = image.size.width as usize * 4;
                    let row = rect.width() as usize * 4;
                    let mut bytes = Vec::with_capacity(row * rect.height() as usize);
                    for y in rect.y0..rect.y1 {
                        let at = y as usize * stride + rect.x0 as usize * 4;
                        bytes.extend_from_slice(&image.pixels[at..at + row]);
                    }
                    let size = [rect.width() as usize, rect.height() as usize];
                    let at = [rect.x0 as usize, rect.y0 as usize];
                    let options = texture_options(images.options(key));
                    let mut gpu = self.gpu.renderer.write();
                    gpu.update_texture(
                        &self.gpu.device,
                        &self.gpu.queue,
                        paint::image_texture(key),
                        &ImageDelta::partial(at, color_image(size, &bytes, false), options),
                    );
                    if self.masks.contains(&key) {
                        gpu.update_texture(
                            &self.gpu.device,
                            &self.gpu.queue,
                            mask_texture(key),
                            &ImageDelta::partial(at, color_image(size, &bytes, true), options),
                        );
                    }
                    self.synced.partial += 1;
                    self.synced.texels += rect.area();
                }
                ImageChange::Removed(key) => {
                    let mut gpu = self.gpu.renderer.write();
                    gpu.free_texture(&paint::image_texture(key));
                    gpu.free_texture(&mask_texture(key));
                    self.uploaded.remove(&key);
                    self.masks.remove(&key);
                }
            }
        }
        self.synced.full + self.synced.partial
    }

    fn render(&mut self, window: WindowId, frame: &FrameTree<'_>, damage: &[emath::Rect]) -> RenderStats {
        let mut stats = RenderStats::default();
        let Some(mut state) = self.windows.remove(&window) else {
            return stats;
        };
        self.render_window(&mut state, frame, damage, &mut stats);
        self.windows.insert(window, state);
        stats
    }

    fn request_read_back(&mut self, window: WindowId, rect: Option<RectU>) -> ReadBackId {
        let id = ReadBackId(self.next_read);
        self.next_read += 1;
        if let Some(state) = self.windows.get(&window)
            && let Some(read) = readback::start(&self.gpu, &state.canvas, state.size_px, rect)
        {
            self.reads.insert(id, read);
        }
        id
    }

    fn poll_read_back(&mut self, id: ReadBackId) -> ReadBack {
        let Some(read) = self.reads.get(&id) else {
            return ReadBack::Failed;
        };
        let result = readback::poll(&self.gpu, read);
        if !matches!(result, ReadBack::Pending) {
            self.reads.remove(&id);
        }
        result
    }
}
