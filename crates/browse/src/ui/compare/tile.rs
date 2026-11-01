//! `CompareTile`: one fitted image with zoom and pan shared across tiles, and its pending and
//! failed pills.

use craft_theme::TypeRole;
use craft_ui::canvas::CanvasView;
use craft_ui::canvas::navigate::{NavigationOptions, WheelNavigation, pan};
use craft_ui::canvas::textures::{CachedTexture, Revision, TextureCache, TextureCacheOptions};
use craft_ui::canvas::zoom::fit_view;
use craft_ui::controls::Spinner;
use craft_ui::direction;
use craft_ui::icons::IconSize;
use craft_ui::localized::Labels as _;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::text::LayoutJob;
use egui::{
    AsIdSalt, Color32, ColorImage, CornerRadius, CursorIcon, IdSalt, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2,
};
use kurbo::{Point, Size};

use super::{CompareCaption, CompareLabels, CompareStyle};
use crate::ui::grid::TileStyle;

/// The picture a compare tile shows.
pub enum CompareImage<'a> {
    /// A texture from the app's own `TextureCache`, with the image's full size in pixels (for Fit
    /// and 100 %).
    Texture { texture: CachedTexture, size: [u32; 2] },
    /// A picture the tile uploads into its own `TextureCache` (`get_or_put` keyed by `key` and
    /// `revision`; `make` runs once per revision).
    Picture { key: u64, revision: Revision, size: [u32; 2], make: Box<dyn FnOnce() -> ColorImage + 'a> },
    /// Nothing yet: the thumbnail placeholder.
    None,
}

/// The host's render state for the tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderState {
    /// The picture is current.
    Ready,
    /// A render is running; `overdue` once it has taken longer than the host's wait, which shows
    /// the "Rendering…" pill (`surface.scrim` with a `Spinner`).
    Pending { overdue: bool },
    /// The render failed: the "Render failed" pill (`status.bad.solid`) at once, the message as
    /// its tooltip.
    Failed(String),
}

/// What [`CompareTile::show`] returns.
#[derive(Debug)]
pub struct CompareResponse {
    /// The tile.
    pub response: Response,
    /// A click: the host makes the pane active.
    pub clicked: bool,
    /// A double click: the host toggles Fit and 100 % ([`CompareTile::toggle_zoom`]).
    pub double_clicked: bool,
    /// The shared view changed this pass (zoom or pan).
    pub view_changed: bool,
    /// Where the picture was drawn.
    pub image_rect: Rect,
}

/// The host's drawing over the picture.
type Overlay<'a> = Box<dyn FnOnce(&Painter, &CanvasView, Rect) + 'a>;

/// One compare pane (N8.08): the image fitted into the area less the caption strip, zoomed and
/// panned through a `CanvasView` the tiles share (wheel, pinch, drag through
/// `canvas::WheelNavigation` and `canvas::pan`; the tile works on a copy whose `rect` is its own
/// image rect, and writes back `center` and `zoom`, so equal panes stay in step), the caption,
/// the pills, the rejected veil (`tile.veil`), and the active ring (`tile.ring.selected` 2 pt with
/// its halo). `.survey(true)`: no zoom or pan, always fitted. Window renders are the host's,
/// drawn through `.overlay(..)` over the picture. AccessKit: an `Image` named by
/// `CompareLabels::tile_name` (the caption read as text), the pill text as its description.
pub struct CompareTile<'a> {
    id_salt: IdSalt,
    view: &'a mut CanvasView,
    image: CompareImage<'a>,
    caption: Option<CompareCaption>,
    state: RenderState,
    active: bool,
    rejected: bool,
    survey: bool,
    overlay: Option<Overlay<'a>>,
    labels: Option<CompareLabels>,
}

impl<'a> CompareTile<'a> {
    /// A tile showing `image` through the shared `view`.
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut CanvasView, image: CompareImage<'a>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:213 (`photo_tile`).
        Self {
            id_salt: IdSalt::new(id_salt),
            view,
            image,
            caption: None,
            state: RenderState::Ready,
            active: false,
            rejected: false,
            survey: false,
            overlay: None,
            labels: None,
        }
    }

    /// The caption strip.
    #[must_use]
    pub fn caption(self, caption: CompareCaption) -> Self {
        // new: builder.
        Self { caption: Some(caption), ..self }
    }

    /// The render state (default `Ready`).
    #[must_use]
    pub fn state(self, state: RenderState) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:328 (the pills).
        Self { state, ..self }
    }

    /// The active pane: the ring.
    #[must_use]
    pub fn active(self, active: bool) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:298 (the active tile's ring).
        Self { active, ..self }
    }

    /// The rejected veil.
    #[must_use]
    pub fn rejected(self, rejected: bool) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:292 (black α110 veil).
        Self { rejected, ..self }
    }

    /// Survey: fitted, no zoom or pan.
    #[must_use]
    pub fn survey(self, survey: bool) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:232 (zoom and pan only in Compare and Reference).
        Self { survey, ..self }
    }

    /// The host's drawing over the picture (a window render), with the tile's view and image rect.
    #[must_use]
    pub fn overlay(self, overlay: impl FnOnce(&Painter, &CanvasView, Rect) + 'a) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:446 (`draw_window`).
        Self { overlay: Some(Box::new(overlay)), ..self }
    }

    /// The words.
    #[must_use]
    pub fn labels(self, labels: CompareLabels) -> Self {
        // new: builder.
        Self { labels: Some(labels), ..self }
    }

    /// Draws the tile in `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> CompareResponse {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:213 (`photo_tile`).
        let Self { id_salt, view, image, caption, state, active, rejected, survey, overlay, labels } = self;
        let tokens = Tokens::of(ui);
        let style = CompareStyle::of(&tokens);
        let tile = TileStyle::of(&tokens);
        let labels = labels.unwrap_or_else(|| CompareLabels::of(ui));
        let ctx = ui.ctx().clone();
        let ppp = ctx.pixels_per_point();
        let id = ui.scope_id().with(id_salt);
        let response = ui.interact(rect, id, if survey { Sense::click() } else { Sense::click_and_drag() });

        let caption_height = if caption.is_some() { style.caption } else { 0.0 };
        let image_area = Rect::from_min_max(rect.min, pos2(rect.right(), (rect.bottom() - caption_height).max(rect.top())));
        let (texture, size) = match image {
            CompareImage::Texture { texture, size } => (Some(texture), Some(size)),
            CompareImage::Picture { key, revision, size, make } => {
                let cache = TextureCache::<u64>::in_context(&ctx, ("compare-textures", id_salt), TextureCacheOptions::frames());
                (Some(cache.get_or_put(&ctx, key, revision, make)), Some(size))
            }
            CompareImage::None => (None, None),
        };

        // The tile works on a copy whose rect is its own image rect and writes back centre and zoom.
        let before = *view;
        let mut shown = before;
        shown.rect = image_area;
        let document = size.map(|[w, h]| Size::new(f64::from(w.max(1)), f64::from(h.max(1))));
        if let Some(document) = document {
            let mark_id = id.with("fitted");
            let mut fit = shown;
            fit_view(&mut fit, document, ppp);
            // A view this tile left at Fit follows the area when it changes.
            let left_fitted = ctx
                .data(|d| d.get_temp::<(f64, f64, f64)>(mark_id))
                .is_some_and(|(zoom, x, y)| close(zoom, shown.zoom) && close(x, shown.center.x) && close(y, shown.center.y));
            if survey || left_fitted || !(shown.zoom.is_finite() && shown.zoom > 0.0) {
                shown.zoom = fit.zoom;
                shown.center = fit.center;
            }
            if !survey {
                let prefs = NavigationOptions { zoom_with_wheel: true, animated_zoom: false, ..NavigationOptions::default() };
                if let Some(action) = WheelNavigation::read(&ctx, &response, &prefs) {
                    let pointer = response.hover_pos().unwrap_or_else(|| image_area.center());
                    WheelNavigation::apply(&mut shown, action, pointer, document, &prefs, ppp);
                }
                if response.dragged() {
                    pan(&mut shown, response.drag_delta());
                    ctx.set_cursor_icon(CursorIcon::Grabbing);
                } else if response.hovered() {
                    ctx.set_cursor_icon(CursorIcon::Grab);
                }
                view.zoom = shown.zoom;
                view.center = shown.center;
                if close(shown.zoom, fit.zoom) && close(shown.center.x, fit.center.x) && close(shown.center.y, fit.center.y) {
                    ctx.data_mut(|d| d.insert_temp(mark_id, (shown.zoom, shown.center.x, shown.center.y)));
                } else {
                    ctx.data_mut(|d| d.remove::<(f64, f64, f64)>(mark_id));
                }
            }
        }
        let display = match document {
            Some(document) => Rect::from_two_pos(shown.to_screen(Point::ZERO), shown.to_screen(Point::new(document.width, document.height))),
            None => image_area,
        };
        let visible = display.intersect(image_area);
        let visible = if visible.is_positive() { visible } else { image_area };

        let mut painted = None;
        let mut pill_text = None;
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(image_area);
            match texture {
                Some(texture) => {
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    painter.image(texture.id, display, uv, Color32::WHITE); // design: allow(identity tint)
                }
                None => {
                    painter.rect_filled(image_area, 0.0, tokens.component.thumbnail.placeholder.to_egui());
                }
            }
            if let Some(overlay) = overlay {
                overlay(&painter, &shown, display);
            }
            if rejected {
                painter.rect_filled(visible, 0.0, style.veil);
            }
            if active {
                let ring = ui.painter_at(rect.expand(tile.ring_width + tile.ring_halo_width));
                ring.rect_stroke(visible, 0.0, Stroke::new(tile.ring_width, style.ring), StrokeKind::Outside);
                ring.rect_stroke(visible.expand(tile.ring_width), 0.0, Stroke::new(tile.ring_halo_width, style.ring_halo), StrokeKind::Outside);
            }
            let pill = match &state {
                RenderState::Pending { overdue: true } => Some((labels.rendering.clone(), style.pending_fill, style.pill_text, true, None)),
                RenderState::Failed(message) => {
                    Some((labels.render_failed.clone(), style.failed_fill, style.failed_text, false, Some(message.clone())))
                }
                RenderState::Ready | RenderState::Pending { overdue: false } => None,
            };
            if let Some((text, fill, ink, spinning, tip)) = pill {
                let rtl = direction::is_rtl(ui);
                let pad = tokens.space.pad.control;
                let gap = tokens.space.gap.inline;
                let spinner = IconSize::Sm.points(&tokens);
                let galley = painter.layout_job(LayoutJob::single_section(text.clone(), role_format(&tokens, TypeRole::Caption, ink)));
                let lead = if spinning { spinner + gap } else { 0.0 };
                let pill_size = vec2(galley.size().x + lead + 2.0 * pad, galley.size().y.max(spinner) + pad);
                let min = pos2(visible.center().x - pill_size.x / 2.0, visible.top() + tokens.space.gap.row);
                let pill_rect = Rect::from_min_size(min, pill_size);
                painter.rect_filled(pill_rect, CornerRadius::from((pill_size.y / 2.0).min(tokens.shape.radius.pill)), fill);
                let (spinner_x, text_x) = if rtl {
                    (pill_rect.right() - pad - spinner, pill_rect.left() + pad)
                } else {
                    (pill_rect.left() + pad, pill_rect.right() - pad - galley.size().x)
                };
                if spinning {
                    let slot = Rect::from_min_size(pos2(spinner_x, pill_rect.center().y - spinner / 2.0), vec2(spinner, spinner));
                    Spinner::paint(ui, slot, ink);
                }
                painter.galley(pos2(text_x, pill_rect.center().y - galley.size().y / 2.0), galley, ink);
                if let Some(tip) = tip {
                    let hover = ui.interact(pill_rect.intersect(image_area), id.with("pill"), Sense::hover());
                    hover.on_hover_text(tip);
                }
                pill_text = Some(text);
            }
            if let Some(caption) = &caption {
                let top = (display.bottom() + tile.caption_gap).clamp(image_area.top(), image_area.bottom());
                let (left, right) = if direction::is_rtl(ui) {
                    (rect.left(), display.right().clamp(rect.left(), rect.right()))
                } else {
                    (display.left().clamp(rect.left(), rect.right()), rect.right())
                };
                let strip = Rect::from_min_max(pos2(left, top), pos2(right, top + style.caption));
                painted = Some(caption.paint(ui, strip, &style, &labels));
            }
        }

        let name = painted.map(|p| p.name).unwrap_or_default();
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Image, ui.is_enabled(), &name));
        if let Some(text) = pill_text {
            ctx.accesskit_node_builder(response.id, |node| node.set_description(text));
        }
        CompareResponse {
            clicked: response.clicked(),
            double_clicked: response.double_clicked(),
            view_changed: !(close(view.zoom, before.zoom) && close(view.center.x, before.center.x) && close(view.center.y, before.center.y)),
            image_rect: display,
            response,
        }
    }

    /// Toggles `view` between Fit (`canvas::fit_view` for an image of `size` in `area`) and 100 %
    /// (one device pixel per image pixel, about the area's centre). `area` is the image area: the
    /// tile's rect less the caption strip (`CompareStyle::caption`).
    pub fn toggle_zoom(view: &mut CanvasView, size: [u32; 2], area: Rect, pixels_per_point: f32) {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:520 (double click toggles Fit and 100 %).
        let document = Size::new(f64::from(size[0].max(1)), f64::from(size[1].max(1)));
        let centre = view.to_doc(area.center());
        view.rect = area;
        let mut fit = *view;
        fit_view(&mut fit, document, pixels_per_point);
        if close(view.zoom, fit.zoom) {
            view.zoom = CanvasView::points_per_unit(1.0, pixels_per_point);
            view.center = centre;
        } else {
            view.zoom = fit.zoom;
            view.center = fit.center;
        }
    }
}

/// Equal up to the rounding of a zoom or a centre.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}
