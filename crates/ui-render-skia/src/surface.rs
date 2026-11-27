//! App-drawn surfaces as CPU tiles.
//!
//! The app registers a [`SurfaceProducer`] for a [`SurfaceKey`] and tells the renderer what changed
//! ([`crate::SkiaRenderer::invalidate_surface`]). The renderer keeps the surface as a grid of raster
//! tiles. A change re-fills only the tiles it touches, and only the changed part of each, writes the
//! pixels into the tile in place (no picture holds the old tile at that point, so Skia does not
//! copy it) and snapshots it again. A surface item then draws its tiles scaled into the item's rect.

use std::collections::{HashMap, HashSet};

use craft_ui_rt::{ImageOptions, Media, RectU, SizeU, SurfaceKey};
use skia_safe::{AlphaType, ColorType, Image, ImageInfo, Surface, surfaces};

/// Tile edge in surface pixels.
const TILE: u32 = 512;

/// What an app implements to supply a surface's pixels.
pub trait SurfaceProducer {
    /// Writes the pixels of `rect` (surface pixels) into `out`: tightly packed rows of
    /// premultiplied RGBA, `rect`'s width times height times 4 bytes. A document canvas is opaque,
    /// where premultiplied and straight alpha are the same bytes.
    fn fill(&mut self, rect: RectU, out: &mut [u8]);
}

#[derive(Clone, Copy, Debug)]
pub struct SurfaceSpec {
    /// The surface's size in its own pixels.
    pub size: SizeU,
    /// How it is sampled when drawn larger or smaller than its pixels.
    pub options: ImageOptions,
}

struct Tile {
    rect: RectU,
    surface: Surface,
    image: Option<Image>,
}

pub(crate) struct Entry {
    spec: SurfaceSpec,
    producer: Box<dyn SurfaceProducer>,
    tiles: Vec<Tile>,
    dirty: Vec<RectU>,
    all_dirty: bool,
}

#[derive(Default)]
pub(crate) struct Surfaces {
    map: HashMap<SurfaceKey, Entry>,
    scratch: Vec<u8>,
    /// Surfaces the last ingest filled since the frame began: a frame's runtime-side invalidation
    /// of one of these is the same change, not a new whole-surface one.
    ingested_this_frame: HashSet<SurfaceKey>,
}

/// What one ingest did, for the R5 measurement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ingested {
    /// Pixels the producer filled.
    pub pixels: u64,
    pub fill_ms: f64,
    pub write_ms: f64,
}

fn info(w: u32, h: u32) -> ImageInfo {
    ImageInfo::new((w as i32, h as i32), ColorType::RGBA8888, AlphaType::Premul, None)
}

impl Surfaces {
    pub fn register(&mut self, key: SurfaceKey, spec: SurfaceSpec, producer: Box<dyn SurfaceProducer>) {
        let mut tiles = Vec::new();
        for y in (0..spec.size.height).step_by(TILE as usize) {
            for x in (0..spec.size.width).step_by(TILE as usize) {
                let rect = RectU::new(x, y, (x + TILE).min(spec.size.width), (y + TILE).min(spec.size.height));
                if let Some(surface) = surfaces::raster(&info(rect.width(), rect.height()), None, None) {
                    tiles.push(Tile { rect, surface, image: None });
                }
            }
        }
        self.map.insert(key, Entry { spec, producer, tiles, dirty: Vec::new(), all_dirty: true });
    }

    pub fn remove(&mut self, key: SurfaceKey) -> bool {
        self.map.remove(&key).is_some()
    }

    /// Marks `rect` (the whole surface when `None`) as changed.
    pub fn invalidate(&mut self, key: SurfaceKey, rect: Option<RectU>) {
        let Some(e) = self.map.get_mut(&key) else { return };
        match rect {
            Some(r) if !e.all_dirty => e.dirty.push(r),
            Some(_) => {}
            None => e.all_dirty = true,
        }
    }

    /// The runtime reports `key` changed: when the app told the renderer what part (and that is
    /// already ingested this frame) nothing more is needed, else the whole surface changed.
    pub fn invalidate_unless_marked(&mut self, key: SurfaceKey) {
        if self.ingested_this_frame.contains(&key) {
            return;
        }
        if let Some(e) = self.map.get_mut(&key)
            && !e.all_dirty
            && e.dirty.is_empty()
        {
            e.all_dirty = true;
        }
    }

    /// A new frame begins: nothing is ingested yet.
    pub fn begin_frame(&mut self) {
        self.ingested_this_frame.clear();
    }

    pub fn has_pending(&self) -> bool {
        self.map.values().any(|e| e.all_dirty || !e.dirty.is_empty())
    }

    /// Re-fills what changed since the last ingest. Pictures that show a changed surface must be
    /// dropped before this call, so the tiles are written in place.
    pub fn pending(&self) -> impl Iterator<Item = Media> + '_ {
        self.map.iter().filter(|(_, e)| e.all_dirty || !e.dirty.is_empty()).map(|(k, _)| Media::Surface(*k))
    }

    pub fn ingest(&mut self, changed: &mut HashSet<Media>) -> Ingested {
        let mut out = Ingested::default();
        let scratch = &mut self.scratch;
        for (key, e) in &mut self.map {
            if !e.all_dirty && e.dirty.is_empty() {
                continue;
            }
            changed.insert(Media::Surface(*key));
            self.ingested_this_frame.insert(*key);
            let whole = RectU::of_size(e.spec.size);
            let regions: Vec<RectU> = if e.all_dirty { vec![whole] } else { std::mem::take(&mut e.dirty) };
            e.all_dirty = false;
            e.dirty.clear();
            for tile in &mut e.tiles {
                tile.image = None;
            }
            for region in regions {
                for tile in &mut e.tiles {
                    fill_tile(&mut *e.producer, tile, region, scratch, &mut out);
                }
            }
            for tile in &mut e.tiles {
                if tile.image.is_none() {
                    tile.image = Some(tile.surface.image_snapshot());
                }
            }
        }
        out
    }

    /// The tiles of `key` as (image, source-pixel rect) pairs, and the surface's size and options.
    pub fn tiles(&self, key: SurfaceKey) -> Option<(SurfaceSpec, impl Iterator<Item = (&Image, RectU)> + '_)> {
        let e = self.map.get(&key)?;
        Some((e.spec, e.tiles.iter().filter_map(|t| Some((t.image.as_ref()?, t.rect)))))
    }
}

/// Fills the part of `region` inside `tile`.
fn fill_tile(producer: &mut dyn SurfaceProducer, tile: &mut Tile, region: RectU, scratch: &mut Vec<u8>, out: &mut Ingested) {
    let part = RectU::new(region.x0.max(tile.rect.x0), region.y0.max(tile.rect.y0), region.x1.min(tile.rect.x1), region.y1.min(tile.rect.y1));
    if part.is_empty() {
        return;
    }
    let (w, h) = (part.width(), part.height());
    scratch.resize(w as usize * h as usize * 4, 0);
    let t = web_time::Instant::now();
    producer.fill(part, scratch);
    out.fill_ms += t.elapsed().as_secs_f64() * 1000.0;
    let t = web_time::Instant::now();
    tile.image = None;
    let at = ((part.x0 - tile.rect.x0) as i32, (part.y0 - tile.rect.y0) as i32);
    if !tile.surface.canvas().write_pixels(&info(w, h), scratch, w as usize * 4, at) {
        log::error!("skia could not write {w}x{h} surface pixels");
    }
    out.write_ms += t.elapsed().as_secs_f64() * 1000.0;
    out.pixels += u64::from(w) * u64::from(h);
}
