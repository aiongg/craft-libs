//! Display lists tessellated once and kept per (node, part), with the list version and sub-pixel
//! shift they were made for, and moved to window coordinates as they are drawn. The wgpu renderer
//! keeps one set per window; a host that draws the meshes itself (an egui app embedding an rt
//! window) keeps one the same way.

use std::collections::HashMap;

use craft_ui_rt::{Draw, FrameTree, ImageKey, NodeId, Part, RenderStats, SurfaceKey};
use emath::{Pos2, Rect as ERect, Vec2};
use epaint::{Mesh, TessellationOptions, Tessellator};

use crate::clip;
use crate::paint::{self, Env, Piece, Prim, Scratch, TextureIds};
use crate::text::{EpaintText, FontsAccess};

/// A kept piece list with the version and shift it was made for.
struct Kept {
    version: u64,
    shift: Vec2,
    prims: Vec<Prim>,
    masks: Vec<ImageKey>,
}

/// One window's kept meshes, at one scale.
pub struct KeptMeshes {
    scale: f32,
    options: TessellationOptions,
    tessellator: Tessellator,
    /// The atlas generation and size the tessellator and kept meshes belong to.
    atlas: (u64, [usize; 2]),
    kept: HashMap<(NodeId, Part), Kept>,
    /// Buffers every list uses and leaves empty, so a list allocates none of its own.
    scratch: Scratch,
}

fn atlas_of<F: FontsAccess>(text: &EpaintText<F>) -> (u64, [usize; 2]) {
    (text.atlas_generation(), text.atlas_size())
}

impl KeptMeshes {
    /// An empty set for a window at `scale` physical pixels per point.
    pub fn new<F: FontsAccess>(text: &EpaintText<F>, scale: f32, options: TessellationOptions) -> KeptMeshes {
        KeptMeshes {
            scale,
            options,
            tessellator: text.tessellator(scale, options),
            atlas: atlas_of(text),
            kept: HashMap::new(),
            scratch: Scratch::default(),
        }
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Whether the kept meshes point into `text`'s current glyph atlas (same generation and size).
    /// A host whose own text can grow or clear the atlas between rt frames checks it before it
    /// draws the meshes again.
    pub fn is_current<F: FontsAccess>(&self, text: &EpaintText<F>) -> bool {
        self.atlas == atlas_of(text)
    }

    /// Starts drawing `frame`: forgets the lists of the nodes it removed, and every list when the
    /// glyph atlas moved since they were made.
    pub fn begin<F: FontsAccess>(&mut self, frame: &FrameTree<'_>, text: &EpaintText<F>) {
        if !self.is_current(text) {
            self.atlas = atlas_of(text);
            self.kept.clear();
            self.tessellator = text.tessellator(self.scale, self.options);
        }
        for id in &frame.changes().removed {
            self.kept.remove(&(*id, Part::Content));
            self.kept.remove(&(*id, Part::Overlay));
        }
    }

    /// `draw`'s kept pieces, tessellated now when its list version or sub-pixel shift changed.
    /// Counts the list in `stats` as prepared or reused. `ids` name the image textures new
    /// meshes draw from.
    pub fn draw<'k, F: FontsAccess>(
        &'k mut self,
        draw: &Draw<'_>,
        text: &mut EpaintText<F>,
        ids: &dyn TextureIds,
        stats: &mut RenderStats,
    ) -> KeptDraw<'k> {
        let scale = f64::from(self.scale);
        // epaint snaps to the pixel grid while tessellating, so a node off the grid is tessellated
        // with its fractional pixel offset and moved by the whole-pixel remainder.
        let frac = |v: f32| {
            let px = f64::from(v) * scale;
            ((px - px.floor()) / scale) as f32
        };
        let shift = Vec2::new(frac(draw.origin.x), frac(draw.origin.y));
        let key = (draw.node, draw.part);
        let fresh = self.kept.get(&key).is_some_and(|k| k.version == draw.version && k.shift == shift);
        if fresh {
            stats.reused += 1;
        } else {
            let mut env = Env { tessellator: &mut self.tessellator, text, ids, scale: self.scale, scratch: &mut self.scratch };
            let done = paint::tessellate(draw.list, shift, &mut env);
            self.kept.insert(key, Kept { version: draw.version, shift, prims: done.prims, masks: done.masks });
            stats.prepared += 1;
        }
        let outlines = draw.rounded.iter().map(|c| clip::rounded_outline(c.rect, c.radii, self.scale)).collect();
        let entry = &self.kept[&key];
        KeptDraw {
            delta: Vec2::new(draw.origin.x - shift.x, draw.origin.y - shift.y),
            clip: draw.clip,
            opacity: draw.opacity,
            outlines,
            prims: &entry.prims,
            masks: &entry.masks,
        }
    }
}

/// One draw's kept pieces and what moves them to window coordinates.
pub struct KeptDraw<'k> {
    delta: Vec2,
    clip: ERect,
    opacity: f32,
    /// The rounded clips above and at the node, as window-coordinate outlines.
    outlines: Vec<Vec<Pos2>>,
    prims: &'k [Prim],
    masks: &'k [ImageKey],
}

/// One kept piece, placed in window coordinates.
pub struct KeptPiece<'k> {
    /// The piece's scissor cut by the draw's clip.
    pub clip_rect: ERect,
    /// What the piece can touch, inside `clip_rect`.
    pub bounds: ERect,
    pub kind: PieceKind<'k>,
}

pub enum PieceKind<'k> {
    /// A mesh in the node's coordinates; [`KeptDraw::moved`] places it.
    Mesh(&'k Mesh),
    /// The app-drawn surface `key` inside `rect`.
    Surface { key: SurfaceKey, rect: ERect },
}

impl<'k> KeptDraw<'k> {
    /// Images the list draws as masks, which need their alpha-only texture.
    pub fn masks(&self) -> &'k [ImageKey] {
        self.masks
    }

    /// The pieces in paint order.
    pub fn pieces(&self) -> impl Iterator<Item = KeptPiece<'k>> + '_ {
        let prims: &'k [Prim] = self.prims;
        prims.iter().map(move |prim| {
            let clip_rect = prim.clip_rect.translate(self.delta).intersect(self.clip);
            let bounds = prim.bounds.translate(self.delta).intersect(clip_rect);
            let kind = match &prim.piece {
                Piece::Mesh(mesh) => PieceKind::Mesh(mesh),
                Piece::Surface { key, rect } => PieceKind::Surface { key: *key, rect: rect.translate(self.delta) },
            };
            KeptPiece { clip_rect, bounds, kind }
        })
    }

    /// `mesh` (one of the pieces') moved to window coordinates, with the draw's opacity and
    /// rounded clips applied. Empty when the clips cut all of it.
    pub fn moved(&self, mesh: &Mesh) -> Mesh {
        let mut m = mesh.clone();
        for v in &mut m.vertices {
            v.pos += self.delta;
            if self.opacity < 1.0 {
                v.color = v.color.gamma_multiply(self.opacity);
            }
        }
        for outline in &self.outlines {
            clip::clip_to_outline(&mut m, outline);
        }
        m
    }
}
