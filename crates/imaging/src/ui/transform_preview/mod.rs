//! The moving pixels of a transform drawn through a quad, a mesh or a map ([`TransformPreview`]).
//!
//! The app reads its layer's pixels once per session into [`PreviewPixels`] and uploads them as
//! [`PreviewTextures`] (a mip chain built on demand). Every pass, [`TransformPreview`] draws them
//! through the session's map under the guides and the box: a [quad](PreviewMap::quad) for Free
//! Transform, a [warp mesh](PreviewMap::mesh), a [pin mesh](PreviewMap::pins) for Puppet Warp, or
//! the app's [function](PreviewMap::function) for Perspective Warp. A layer with a style is
//! composited by the app's [`StyledSource`] through [`StyledPreview`] instead, within a time
//! budget. The preview never changes the document.
//!
//! ```no_run
//! use craft_imaging::ui::transform_preview::{PreviewMap, PreviewPixels, PreviewTextures, TransformPreview};
//! # fn demo(ui: &mut egui::Ui, view: &craft_ui::canvas::CanvasView, image: egui::ColorImage, quad: [kurbo::Point; 4]) {
//! // Once per session:
//! let textures = PreviewTextures::new(ui.ctx(), "ft-preview", PreviewPixels::new(image, [1.0, 1.0]));
//! // Every pass, under the guides and the box:
//! TransformPreview::new("ft-preview", &textures).map(PreviewMap::quad(quad)).opacity(0.8).clip([800, 600]).show(ui, view);
//! # }
//! ```

mod maps;
pub(crate) mod stories;
mod styled;
mod textures;

#[cfg(test)]
mod maps_tests;
#[cfg(test)]
mod styled_tests;
#[cfg(test)]
mod textures_tests;

use egui::epaint::Vertex;
use egui::{AsIdSalt, Color32, Mesh, Painter, TextureId, Ui};
use kurbo::Point;

use craft_ui::canvas::CanvasView;

pub use maps::PreviewMap;
use maps::{PreviewMesh, clip_to_canvas};
pub use styled::{StylePlan, StyledBudget, StyledContent, StyledPreview, StyledSource};
pub use textures::{PreviewDisplay, PreviewMask, PreviewPixels, PreviewTextures, TextureChoice};

/// The preview's algorithm constants (LC23, LC24): choices of how finely to sample, not look, so
/// not tokens. Values are PhotoCraft's.
pub struct PreviewQuality;

impl PreviewQuality {
    /// Cells per side of a quad's preview mesh: keeps perspective previews straight
    /// (PhotoCraft `transform_tool/draw.rs:114`).
    pub const QUAD_CELLS: usize = 24;
    /// Cells per side of a function map's preview mesh (PhotoCraft `perspective_ui.rs:57`).
    pub const PLANE_CELLS: usize = 40;
    /// Target screen points per segment of a warp mesh's preview (PhotoCraft `warp_preview.rs:11`).
    pub const SEGMENT_PX: f64 = 6.0;
    /// Fewest and most segments one warp patch gets along an axis.
    pub const MIN_PER_CELL: usize = 4;
    #[allow(missing_docs)]
    pub const MAX_PER_CELL: usize = 96;
    /// Most samples along either axis of a warp mesh.
    pub const MAX_PER_AXIS: usize = 240;
    /// Deepest mip level: 4096 times smaller (PhotoCraft `transform_tex.rs:75`).
    pub const MIP_LEVELS: u32 = 12;
    /// A source pixel shown at this many screen pixels or more counts as zoomed in.
    pub const CRISP_ZOOM: f32 = 2.0;
    /// At most this enlargement by the transform keeps the zoomed-in pixels crisp.
    pub const CRISP_SCALE: f32 = 1.05;
    /// Below this many screen pixels per source pixel a mip level is used.
    pub const MIP_BELOW: f32 = 0.5;
}

/// A painter that draws over the preview: the app's layers above the moving one.
type Above<'a> = Box<dyn FnOnce(&Painter) + 'a>;

/// The moving pixels of a transform drawn through a map, as image content under the guides and
/// the box (PhotoCraft draws them before the guides so a guide stays visible across the layer).
///
/// - The texture level and filter follow the on-screen size of a source pixel
///   ([`PreviewTextures::choose`]): a mip level when shrinking below ½, level 0 bilinear around
///   1:1, level 0 nearest at zoom ≥ 2 with the transform enlarging by at most 1.05, and always
///   level 0 nearest with [`nearest`](Self::nearest).
/// - The mesh is clipped to the canvas in document space ([`clip`](Self::clip)), also under a
///   turned view; quad cells with a corner past the horizon (a Distort dragged behind the eye)
///   are left out; triangles are drawn from the most displaced to the least, so a fold keeps its
///   layers whole.
/// - [`opacity`](Self::opacity) tints every vertex white with that alpha (layer opacity × fill).
/// - With [`styled`](Self::styled) and a plan that fits, the styled composite is drawn in place of
///   the mesh (quad maps only); otherwise the plain mesh.
/// - No AccessKit node: image content.
pub struct TransformPreview<'a> {
    textures: &'a PreviewTextures,
    map: Option<PreviewMap<'a>>,
    opacity: f32,
    nearest: bool,
    clip: Option<[u32; 2]>,
    display: Option<&'a PreviewDisplay>,
    styled: Option<(&'a StyledPreview, &'a dyn StyledSource)>,
    dragging: bool,
    budget: StyledBudget,
    above: Option<Above<'a>>,
}

impl<'a> TransformPreview<'a> {
    /// The preview of `textures`; nothing is drawn until a [`map`](Self::map) is set. The preview
    /// registers no widget, so `id_salt` only keeps the builder's usual first parameter.
    pub fn new(_id_salt: impl AsIdSalt, textures: &'a PreviewTextures) -> Self {
        Self {
            textures,
            map: None,
            opacity: 1.0,
            nearest: false,
            clip: None,
            display: None,
            styled: None,
            dragging: false,
            budget: StyledBudget::default(),
            above: None,
        }
    }

    /// Where the source goes.
    pub fn map(mut self, map: PreviewMap<'a>) -> Self {
        self.map = Some(map);
        self
    }

    /// The layer's opacity times its fill, 0 to 1; 1 by default.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Nearest Neighbor interpolation: level 0 with nearest filtering at every zoom. The app
    /// passes the draw session's `Interpolation::NearestNeighbor` as a bool: the imaging library
    /// cannot name the drawing library's type (decisions §2.1).
    pub fn nearest(mut self, nearest: bool) -> Self {
        self.nearest = nearest;
        self
    }

    /// The document's size in pixels: nothing is drawn past its edges.
    pub fn clip(mut self, canvas: [u32; 2]) -> Self {
        self.clip = Some(canvas);
        self
    }

    /// The monitor conversion of the displayed upload; the mip levels stay raw.
    pub fn display(mut self, display: &'a PreviewDisplay) -> Self {
        self.display = Some(display);
        self
    }

    /// A styled layer: composite through the app's `source` and keep the result in `preview`.
    pub fn styled(mut self, preview: &'a StyledPreview, source: &'a dyn StyledSource) -> Self {
        self.styled = Some((preview, source));
        self
    }

    /// The pointer is down: the styled composite uses the drag budget.
    pub fn dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    /// How long a styled rebuild may take; PhotoCraft's times by default.
    pub fn styled_budget(mut self, budget: StyledBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Draws the app's layers above the moving one after it, in the same clip.
    pub fn above(mut self, paint: impl FnOnce(&Painter) + 'a) -> Self {
        self.above = Some(Box::new(paint));
        self
    }

    /// Draws the preview into the canvas area of `view`.
    pub fn show(self, ui: &Ui, view: &CanvasView) {
        // pc: crates/ui-egui/src/tools/transform_tool/draw.rs:113 (the quad mesh), :118 (texture
        // pick), :83 (`front_vertices`), :395 (`clip_to_canvas`);
        // crates/ui-egui/src/tools/perspective_ui.rs:263 (`draw_content`, function map);
        // crates/ui-egui/src/tools/puppet_ui.rs:348 (`draw_content`, pin mesh).
        let Some(map) = self.map else { return };
        let ctx = ui.ctx();
        let painter = ui.painter_at(view.rect);
        let zoom = view.zoom as f32 * ctx.pixels_per_point();
        let scale = map.source_scale(self.textures.size());
        let tint = Color32::from_white_alpha((self.opacity.clamp(0.0, 1.0) * 255.0).round() as u8);

        let styled = match (&self.styled, &map, self.clip) {
            (Some((preview, source)), PreviewMap::Quad(quad), Some(canvas)) => {
                let frame = styled::StyledFrame {
                    textures: self.textures,
                    quad: *quad,
                    quad_scale: scale,
                    nearest: self.nearest,
                    zoom,
                    dragging: self.dragging,
                    canvas,
                    budget: self.budget,
                };
                preview.texture(ctx, &frame, *source, self.display)
            }
            _ => None,
        };
        if let Some((texture, rect)) = styled {
            let corners = [(rect.x0, rect.y0, 0.0, 0.0), (rect.x1, rect.y0, 1.0, 0.0), (rect.x1, rect.y1, 1.0, 1.0), (rect.x0, rect.y1, 0.0, 1.0)];
            let mesh = PreviewMesh {
                points: corners.iter().map(|c| Point::new(c.0, c.1)).collect(),
                rest: corners.iter().map(|c| Point::new(c.0, c.1)).collect(),
                uv: corners.iter().map(|c| [c.2, c.3]).collect(),
                indices: vec![0, 1, 2, 0, 2, 3],
            };
            painter.add(screen_mesh(mesh, texture, Color32::WHITE, self.clip, view));
        } else {
            let choice = PreviewTextures::choose(zoom, scale, self.nearest);
            let (texture, uv) = self.textures.pick(ctx, choice, self.display);
            let mesh = map.build(view.zoom * f64::from(ctx.pixels_per_point()), uv);
            painter.add(screen_mesh(mesh, texture, tint, self.clip, view));
        }
        if let Some(above) = self.above {
            above(&painter);
        }
    }
}

/// `mesh` clipped to the canvas in document space and mapped through `view` to an egui mesh.
fn screen_mesh(mesh: PreviewMesh, texture: TextureId, tint: Color32, canvas: Option<[u32; 2]>, view: &CanvasView) -> Mesh {
    let mesh = match canvas {
        Some(canvas) => clip_to_canvas(mesh, canvas),
        None => mesh,
    };
    let mut out = Mesh::with_texture(texture);
    out.vertices =
        mesh.points.iter().zip(&mesh.uv).map(|(p, uv)| Vertex { pos: view.to_screen(*p), uv: egui::pos2(uv[0], uv[1]), color: tint }).collect();
    out.indices = mesh.indices;
    out
}
