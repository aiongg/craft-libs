//! [`SelectionOverlay`] (task SE-F7).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task SE-F7")]

use craft_ui::canvas::CanvasView;
use craft_ui::canvas::kurbo::Point;
use craft_ui::localized::{LocaleNumbers, Unit};
use egui::{AsIdSalt, Id, IdSalt, Response, Ui};

use crate::selection::{PreviewMode, QuickMaskLook};
use crate::trace::{MaskRect, MaskSource, Segment};

/// The longest side of the preview texture, in texels: a larger document is sampled every `k`-th
/// pixel (`k = ceil(long side / 1024)`).
// pc: crates/ui-egui/src/dialogs/color_range_ui.rs:47 `CANVAS_PREVIEW` (1024).
pub(super) const PREVIEW_TEXTURE_MAX: u32 = 1024;

craft_ui::words! {
    /// The accessible name and value of the selection overlay, English by default.
    #[non_exhaustive]
    pub struct SelectionOverlayLabels {
        /// The graphics object's name.
        name: "selectionOverlay.name" = "Selection";
        /// The value with nothing selected.
        nothing: "selectionOverlay.nothing" = "No selection";
        /// The value: the content bounds, each number with the document's unit.
        bounds: "selectionOverlay.bounds" = "{width} × {height} at {x}, {y}";
        /// Appended to the value while a preview shows: `{value}, {mode}`.
        with_mode: "selectionOverlay.withMode" = "{value}, {mode}";
        grayscale: "selectionOverlay.grayscale" = "Grayscale";
        black_matte: "selectionOverlay.blackMatte" = "Black Matte";
        white_matte: "selectionOverlay.whiteMatte" = "White Matte";
        quick_mask: "selectionOverlay.quickMask" = "Quick Mask";
    }
}

/// Where the overlay's outline comes from.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum OutlineSource<'a> {
    /// The app's mask: the overlay traces it ([`crate::trace::outline`]) and caches the
    /// segments, and builds the preview texture from it.
    Mask(&'a dyn MaskSource),
    /// Segments the app traced itself (in a job, PhotoCraft's Rayon trace), in document pixels of
    /// the full-size document, with the content `bounds` for the accessible value. Drawn as they
    /// are: no cache, no preview texture.
    Segments { segments: &'a [Segment], bounds: MaskRect },
}

impl<'a, M: MaskSource> From<&'a M> for OutlineSource<'a> {
    fn from(mask: &'a M) -> Self {
        OutlineSource::Mask(mask)
    }
}

/// What [`SelectionOverlay::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectionOverlayResponse {
    /// The overlay's id (its cache and its AccessKit node).
    pub id: Id,
    /// The selection's content bounds in full-size document pixels, `None` when nothing is
    /// selected.
    pub bounds: Option<MaskRect>,
    /// The trace ran this pass (the mask's revision, the step, the region or the reduction
    /// changed); `false` when the cached segments were drawn. For a performance readout and tests.
    pub retraced: bool,
}

/// The selection on the canvas, drawn once per pass before the tool's paint.
///
/// ```ignore
/// let mask = SurfaceMask(&doc.selection);                       // the app's MaskSource
/// SelectionOverlay::new(("selection", doc.id), &mask)
///     .visible(show.selection_edges && !polygon.replaces_selection())
///     .offset(floating_offset)                                  // a floating piece, a selection drag
///     .preview(PreviewMode::None, QuickMaskLook::default())
///     .show(ui, &canvas, &view);
/// ```
///
/// What it draws, bottom first:
/// - The preview ([`preview`](Self::preview)), when the mode is not `None` and the source is a
///   mask: one texture of [`PreviewMode::pixel`] per texel over the whole document, at most
///   1024 texels on the long side, rebuilt only when the mask's revision, the mode, the look or the
///   reduction changes, drawn over the document's rectangle with linear filtering.
/// - The ants ([`MarqueeLook::Ants`](craft_ui::canvas::MarqueeLook::Ants), the `marquee.*`
///   tokens) along the selection's boundary, when [`visible`](Self::visible). Hidden ants keep the
///   preview.
///
/// The trace, for a mask source: step [`trace_step`](crate::trace::trace_step) of the view's
/// device zoom (`view.zoom · pixels_per_point`) and the [`reduction`](Self::reduction); region
/// [`quantised_region`](crate::trace::quantised_region) of the view's visible bounds, so a small
/// pan reuses the trace; bounds the mask's content bounds (the whole canvas when
/// [`MaskSource::outside`] is above 0) within the region. With a [`mapped`](Self::mapped) outline
/// the whole selection is traced instead (a Free Transform may carry parts from outside the view
/// into it) with [`bounded_step`](crate::trace::bounded_step). Segments are scaled up by the
/// reduction. The overlay keeps one trace per id in context memory, keyed by the mask's revision,
/// the step, the region and the reduction, and retraces only when one of them changes.
///
/// Drawing: each segment end moves by the [`offset`](Self::offset), then through the
/// [`mapped`](Self::mapped) point map, then through the view, so the ants are crisp on device
/// pixels at any zoom and rotation. While the ants show, the overlay asks for a repaint every
/// `marquee.tick_ms` (100 ms) unless reduced motion stops the march.
///
/// AccessKit: one graphics object named "Selection", valued with the content bounds in the
/// document's unit ("120 × 80 at 40, 20", with [`value_in`](Self::value_in)) or "No selection";
/// a preview adds its mode's name. No focus; the ants are decoration.
pub struct SelectionOverlay<'a> {
    id_salt: IdSalt,
    source: OutlineSource<'a>,
    visible: bool,
    offset: [i32; 2],
    reduction: u32,
    map: Option<&'a dyn Fn(Point) -> Point>,
    preview: PreviewMode,
    look: QuickMaskLook,
    value_in: Option<(&'a LocaleNumbers, &'a Unit)>,
    labels: SelectionOverlayLabels,
}

impl<'a> SelectionOverlay<'a> {
    /// The selection of `source` (a `&impl MaskSource`, or [`OutlineSource::Segments`]) under
    /// `id_salt`: ants shown, no offset, no reduction, no map, no preview.
    pub fn new(id_salt: impl AsIdSalt, source: impl Into<OutlineSource<'a>>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            source: source.into(),
            visible: true,
            offset: [0, 0],
            reduction: 1,
            map: None,
            preview: PreviewMode::None,
            look: QuickMaskLook::default(),
            value_in: None,
            labels: SelectionOverlayLabels::default(),
        }
    }

    /// Whether the ants show (View › Show › Selection Edges, and not while a New Polygonal or
    /// Magnetic Lasso outline replaces the selection). Hidden ants keep the preview. Default on.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Draw the ants `offset` whole document pixels from where the selection is: a floating piece,
    /// a selection drag in progress ([`SelectionDrag::delta`](crate::ui::select::SelectionDrag)),
    /// layers moved together with the selection.
    pub fn offset(mut self, offset: [i32; 2]) -> Self {
        self.offset = offset;
        self
    }

    /// The source is reduced `k` times (Select and Mask's preview proxy): its pixels are `k`
    /// document pixels each way. Default 1; 0 counts as 1.
    pub fn reduction(mut self, k: u32) -> Self {
        self.reduction = k.max(1);
        self
    }

    /// Draw the outline through `map` (document point to document point): where a Free Transform
    /// box puts the selection (`|p| homography.apply(p)` from `TransformSession::homography`) or
    /// the app's warp map in the Warp phase. The whole selection is traced, not only the visible
    /// part.
    pub fn mapped(mut self, map: &'a dyn Fn(Point) -> Point) -> Self {
        self.map = Some(map);
        self
    }

    /// The Selection Preview (Color Range's modes, `look` for its Quick Mask) or, with
    /// [`PreviewMode::QuickMask`] and the user's Quick Mask Options, the Quick Mask tint (Edit in
    /// Quick Mask Mode, the Type Mask tools). Default [`PreviewMode::None`].
    pub fn preview(mut self, mode: PreviewMode, look: QuickMaskLook) -> Self {
        self.preview = mode;
        self.look = look;
        self
    }

    /// Format the accessible value's numbers with `numbers` in `unit` (the rulers' unit). Without
    /// it the value is in pixels with the context's numbers.
    pub fn value_in(mut self, numbers: &'a LocaleNumbers, unit: &'a Unit) -> Self {
        self.value_in = Some((numbers, unit));
        self
    }

    /// Translated words (English by default).
    pub fn labels(mut self, labels: SelectionOverlayLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Draws the overlay over `canvas` (the canvas's response, for the clip rect and the AccessKit
    /// parent) with `view`.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> SelectionOverlayResponse {
        // pc: crates/ui-egui/src/canvas.rs:231-321 `paint_selection_outline` (step :248, region
        // :261-278, whole trace under a transform :266-281, cache key :282-290, scale by `k`
        // :298-302, offset :309-313, repaint :318); crates/ui-egui/src/dialogs/color_range_ui.rs:592-636
        // `paint_selection_preview` (texture, reuse by key, drawn over the image rect).
        todo!()
    }
}
