//! [`Navigator`]: the preview box with the proxy, its pointer, wheel and keys, and the zoom row
//! under it.

use std::sync::Arc;

use egui::accesskit::{self, Action, Role};
use egui::{
    AsIdSalt, Color32, CursorIcon, IdSalt, Key, Modifiers, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2,
};
use kurbo::{Point, Size};

use super::labels::NavigatorLabels;
use super::options::{NavigatorSettings, ProxyColor};
use super::proxy::{preview_rect, proxy_quad, view_center_for, zoom_to_document_rect};
use super::style::NavigatorStyle;
use super::zoom_row::zoom_row;
use crate::canvas::navigate::{WheelAction, WheelNavigation};
use crate::canvas::textures::CachedTexture;
use crate::canvas::{CanvasView, Marquee, MarqueeLook, NavigationOptions, ZoomLadder, zoom};
use crate::controls::{IconButton, NumberField, Slider};
use crate::focus::key_owner::{KeyOwner, OwnerGate, owner};
use crate::localized::{LocaleNumbers, SharedLabels as _};
use crate::primitives::{FocusRing, Interactive};
use crate::theme::{Tokens, TokensOf as _};
use craft_theme::ControlSize;

/// An arrow key moves the view by this share of the visible span, Shift by [`SHIFT_STEP`], Page
/// Up and Page Down by [`PAGE_STEP`].
const ARROW_STEP: f32 = 0.1;
const SHIFT_STEP: f32 = 0.5;
const PAGE_STEP: f32 = 1.0;

/// A drag on the preview, kept in egui memory from its start to its release.
#[derive(Clone, Copy)]
struct Drag {
    /// The view at the press, put back by Esc.
    press: CanvasView,
    /// Where a Cmd/Ctrl-drag started: the drag draws a zoom rectangle instead of moving the view.
    zoom_from: Option<Pos2>,
    cancelled: bool,
}

/// A paint callback for [`NavigatorPreview::Paint`]: draws the picture into the fitted rect.
pub type NavigatorPaint<'a> = Box<dyn FnOnce(&Painter, Rect) + 'a>;

/// What the preview box shows: the document's picture, made and cached by the app.
pub enum NavigatorPreview<'a> {
    /// A texture from the app's own [`TextureCache`](crate::canvas::textures::TextureCache),
    /// drawn over the checkerboard into the fitted rect ([`preview_rect`](super::preview_rect)).
    /// Damage updates go through
    /// [`put_region`](crate::canvas::textures::TextureCache::put_region), so the id stays.
    Texture(CachedTexture),
    /// The app paints the picture itself into the fitted rect (PhotoCraft's GPU canvas callback);
    /// the checkerboard is not drawn.
    Paint(NavigatorPaint<'a>),
    /// No picture yet (PhotoCraft waits up to 3 passes for its GPU canvas): the checkerboard
    /// alone, with the proxy over it.
    Waiting,
}

/// The Navigator panel body: the preview box sized to the space it gets, the proxy rectangle,
/// and the zoom row (percent field, Zoom Out, slider, Zoom In) under it. It edits the
/// [`CanvasView`] it is given and reports what changed; the app writes the view back to its own
/// view type.
///
/// **What the user sees and does.**
/// - The zoom row is laid out first, at the bottom of the space; the preview box takes the height
///   above it in the same pass (no previous-pass height), at least
///   `navigator.min_preview`. The box is filled with `surface.canvas`; the document is fitted
///   inside with `navigator.inset` on each side ([`preview_rect`](super::preview_rect)), over a
///   checkerboard of `navigator.checker` squares in `Checkerboard`'s colours.
/// - The proxy is [`proxy_quad`](super::proxy_quad) of the view passed in (the animated one while
///   a zoom animation runs), in the [`ProxyColor`](super::ProxyColor) of the settings, 1.5 wide,
///   over a 1 pt `overlay.halo` outline, each edge clipped to the box less 1 pt; upright it is a
///   rectangle with `navigator.proxy_radius` corners.
/// - Press and drag on the preview: the view centre follows the pointer
///   ([`view_center_for`](super::view_center_for)), clamped to the document, every pass. A click
///   moves the centre to the clicked point. Esc during the drag puts the view back as it was at
///   the press.
/// - With Cmd (macOS) or Ctrl (elsewhere) held at the press, the drag draws marching ants
///   ([`Marquee`](crate::canvas::Marquee) with `MarqueeLook::Ants`) clipped to the fitted rect;
///   the release zooms to the rectangle ([`zoom_to_document_rect`](super::zoom_to_document_rect)).
///   The cursor is `ZoomIn` while the modifier is held over the preview.
/// - The wheel over the preview zooms about the view centre, one ×1.1 notch
///   ([`WheelNavigation::NOTCH`](crate::canvas::navigate::WheelNavigation::NOTCH)), eased over
///   passes when Animated Zoom is on; it zooms whether or not Zoom with Scroll Wheel is on, and
///   never pans. Shift + wheel steps the ladder. A notch past a zoom limit changes nothing and
///   calls [`WheelNavigation::settle`](crate::canvas::navigate::WheelNavigation::settle). A pinch
///   zooms the same way when [`NavigationOptions::pinch_zooms`] is on. The wheel outside the
///   preview does nothing here.
/// - The zoom row: see `zoom_row`. Buttons step the ladder and set
///   [`NavigatorOutput::animate_zoom`] when Animated Zoom is on; the field and slider set the zoom
///   without animation.
///
/// **Keyboard and focus.** The preview is one Tab stop and shows the focus ring. With it focused:
/// arrows move the view by 10 % of the visible span in the arrow's screen direction, Shift ×5
/// (half a span); Page Up and Page Down move by a span vertically; `+` and `-` step the ladder;
/// Home centres the document. Tab then visits the field, Zoom Out, the slider and Zoom In, in
/// that order. Keys go through [`focus::key_owner`](crate::focus::key_owner) so a focused text
/// field keeps its own.
///
/// **AccessKit.** The preview is a `Group` named [`NavigatorLabels::name`] with two `ScrollBar`
/// children ("Visible area horizontal position", "… vertical position"), numeric value 0 to 100
/// (the view centre in percent of the document), whose `Increment` and `Decrement` move the view
/// by the arrow-key step. The picture is a decorative `Image` named
/// [`NavigatorLabels::preview`]. The proxy has no node of its own.
///
/// **Direction.** The preview, proxy and ants never mirror
/// ([`Unmirrored::Image`](crate::direction::Unmirrored) and `Canvas`). The zoom row follows the
/// interface direction except the slider, whose value grows to the right (`SliderValue`), so in
/// right-to-left Zoom In comes before the slider and Zoom Out after it.
///
/// **What the app gets back.** [`NavigatorOutput`]: whether the view changed (every pass of a
/// drag), whether to animate it, whether the settings changed. The widget keeps the press view of
/// a drag and the ants' start in egui memory under its id. With no document the host shows its
/// own empty text and does not draw the widget.
#[must_use = "show the navigator with `show`"]
pub struct Navigator<'a> {
    pub(super) id_salt: IdSalt,
    pub(super) size: Size,
    pub(super) numbers: &'a LocaleNumbers,
    pub(super) preview: NavigatorPreview<'a>,
    pub(super) navigation: Option<&'a NavigationOptions>,
    pub(super) settings: Option<&'a mut NavigatorSettings>,
    pub(super) labels: Option<NavigatorLabels>,
    pub(super) control_size: ControlSize,
    pub(super) enabled: bool,
}

impl<'a> Navigator<'a> {
    /// A navigator for a document of `size` (document units); `numbers` formats the zoom field.
    /// `id_salt` keeps its drag, focus and the zoom field's draft.
    pub fn new(id_salt: impl AsIdSalt, size: Size, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            size,
            numbers,
            preview: NavigatorPreview::Waiting,
            navigation: None,
            settings: None,
            labels: None,
            control_size: ControlSize::Sm,
            enabled: true,
        }
    }

    /// The picture; [`NavigatorPreview::Waiting`] by default.
    pub fn preview(mut self, preview: NavigatorPreview<'a>) -> Self {
        self.preview = preview;
        self
    }

    /// The app's navigation preferences (Animated Zoom, Zoom with Trackpad Pinch, Enable
    /// Gestures); [`NavigationOptions::default`] without a call.
    pub fn navigation(mut self, navigation: &'a NavigationOptions) -> Self {
        self.navigation = Some(navigation);
        self
    }

    /// The persisted section: the proxy colour. Without a call the proxy is
    /// [`ProxyColor::Default`](super::ProxyColor::Default) and the output never reports a
    /// settings change.
    pub fn settings(mut self, settings: &'a mut NavigatorSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// The words; the installed words' [`NavigatorLabels`] without a call.
    pub fn labels(mut self, labels: NavigatorLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The zoom row's control size; [`ControlSize::Sm`] by default (the library's
    /// `NumberField`, `IconButton` and `Slider` at that size, the field at the library's width).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.control_size = size;
        self
    }

    /// Whether the user can move or zoom the view (default true); disabled, the preview and row
    /// draw dimmed and take no input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Lays out the zoom row at the bottom of the available rect, the preview box above it, reads
    /// this pass's pointer, wheel and keys, and edits `view`. Events are handled in this pass, in
    /// order: the zoom row, then the preview's drag, wheel and keys.
    pub fn show(self, ui: &mut Ui, view: &mut CanvasView) -> NavigatorOutput {
        // pc: crates/ui-egui/src/panels/navigator.rs:3-150 `navigator` (box, fit, picture, ⌘-drag, click/drag, wheel, proxy, row);
        // new: row laid out first (L7), keys and AccessKit scroll bars (L4), Esc restores (L5), rotated proxy (L3).
        let id_salt = self.id_salt;
        ui.push_id(id_salt, |ui| self.show_in(ui, view)).inner
    }

    fn show_in(self, ui: &mut Ui, view: &mut CanvasView) -> NavigatorOutput {
        let Navigator { size, numbers, preview, navigation, settings, labels, control_size, enabled, .. } = self;
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = Navigator::style(&tokens);
        let labels = labels.map_or_else(|| NavigatorLabels::shared_ctx(&ctx), Arc::new);
        let navigation = navigation.copied().unwrap_or_default();
        let proxy_color = settings.as_deref().map_or_else(ProxyColor::default, |s| s.options.proxy_color).color(&tokens);
        let ppp = ctx.pixels_per_point();
        if !enabled {
            ui.disable();
        }

        // The row's height is known from the control tokens, so the preview takes the rest of the
        // height in this pass, and the preview comes first in the focus order.
        let row_height = NumberField::style(&tokens, control_size)
            .field
            .height
            .max(IconButton::style(&tokens, control_size).side)
            .max(Slider::style(&tokens, control_size).height);
        let available = ui.available_size();
        let room = available.y - row_height - style.row_gap;
        let height = if room.is_finite() { room.max(style.min_preview) } else { style.min_preview };
        let width = if available.x.is_finite() { available.x } else { height };

        ui.spacing_mut().item_spacing.y = 0.0;
        let response = Interactive::new("preview", Role::Group, labels.name.as_str())
            .sense(Sense::click_and_drag())
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact(ui, vec2(width, height))
            .finish(ui);
        let frame = response.rect;
        let fitted = preview_rect(frame, size, style.inset);
        let painter = ui.painter_at(frame);
        painter.rect_filled(frame, style.radius, style.fill);
        match preview {
            NavigatorPreview::Texture(texture) => {
                paint_checker(&painter, fitted, &style);
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                painter.image(texture.id, fitted, uv, Color32::WHITE); // design: allow(identity tint)
            }
            NavigatorPreview::Paint(paint) => paint(&painter.with_clip_rect(painter.clip_rect().intersect(fitted)), fitted),
            NavigatorPreview::Waiting => paint_checker(&painter, fitted, &style),
        }
        let nodes = ui.new_child(UiBuilder::new().max_rect(frame).accessibility_parent(response.id));
        Interactive::with_id(response.id.with("image"), Role::Image, labels.preview.as_str())
            .sense(Sense::hover())
            .state_layer(false)
            .focus_ring(false)
            .interact_at(&nodes, fitted)
            .finish(&nodes);

        let mut changed = false;
        if enabled {
            let command = ctx.input(|i| i.modifiers.command);
            changed |= pointer(ui, &response, view, size, fitted, &painter, command, ppp);
            if command && response.hovered() {
                ctx.set_cursor_icon(CursorIcon::ZoomIn);
            }
            changed |= wheel(&ctx, &response, view, size, &navigation, ppp);
            changed |= keys(&ctx, &response, view, size, ppp);
            changed |= scroll_bars(&nodes, &response, frame, view, size, &labels);
        }
        FocusRing::paint_radius(ui, frame, style.radius, &response);

        ui.add_space(style.row_gap);
        let row = zoom_row(ui, view, size, numbers, &labels, &style, control_size, navigation.animated_zoom);
        changed |= row.changed;
        paint_proxy(&painter, view, size, fitted, frame, &style, proxy_color);

        NavigatorOutput {
            response,
            view_changed: changed,
            animate_zoom: row.animate,
            settings_changed: false,
            preview_rect: fitted,
            row_rect: row.rect,
        }
    }
}

/// Press, drag and click on the preview: the view centre follows the pointer, Esc puts the view
/// back, and with Cmd/Ctrl held at the press the drag draws a zoom rectangle and the release
/// zooms to it. Whether the view changed.
#[expect(clippy::too_many_arguments, reason = "one pass of the preview's pointer input")]
fn pointer(ui: &Ui, response: &Response, view: &mut CanvasView, size: Size, fitted: Rect, painter: &Painter, command: bool, ppp: f32) -> bool {
    let ctx = ui.ctx();
    let id = response.id.with("drag");
    let mut drag = ctx.data(|d| d.get_temp::<Option<Drag>>(id)).flatten();
    if response.drag_started() {
        let origin = ctx.input(|i| i.pointer.press_origin()).or(response.interact_pointer_pos());
        drag = Some(Drag { press: *view, zoom_from: origin.filter(|_| command), cancelled: false });
    }
    let before = *view;
    let pointer = response.interact_pointer_pos();
    match drag.as_mut() {
        Some(drag) => {
            if !drag.cancelled && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                drag.cancelled = true;
                view.center = drag.press.center;
                view.zoom = drag.press.zoom;
            }
            if !drag.cancelled && (response.dragged() || response.drag_stopped()) {
                match (drag.zoom_from, pointer) {
                    (None, Some(at)) => view.center = view_center_for(fitted, size, at),
                    (Some(from), Some(to)) => zoom_rectangle(ui, response, view, size, fitted, painter, (from, to), ppp),
                    _ => {}
                }
            }
        }
        None if response.clicked() => {
            if let Some(at) = pointer {
                view.center = view_center_for(fitted, size, at);
            }
        }
        None => {}
    }
    let kept = drag.filter(|_| response.dragged() && !response.drag_stopped());
    ctx.data_mut(|d| d.insert_temp(id, kept));
    *view != before
}

/// The Cmd/Ctrl-drag from `from` to `to`: marching ants clipped to the picture, and on release a
/// zoom to the rectangle.
#[expect(clippy::too_many_arguments, reason = "one pass of the preview's pointer input")]
fn zoom_rectangle(
    ui: &Ui,
    response: &Response,
    view: &mut CanvasView,
    size: Size,
    fitted: Rect,
    painter: &Painter,
    (from, to): (Pos2, Pos2),
    ppp: f32,
) {
    let r = Rect::from_two_pos(from, to).intersect(fitted);
    if response.drag_stopped() {
        let scale = fitted.width() / size.width.max(1.0) as f32;
        let doc = |p: Pos2| ((p.x - fitted.min.x) / scale, (p.y - fitted.min.y) / scale);
        let ((x0, y0), (x1, y1)) = (doc(r.min), doc(r.max));
        zoom_to_document_rect(view, kurbo::Rect::new(f64::from(x0), f64::from(y0), f64::from(x1), f64::from(y1)), size, ppp);
    } else if r.is_positive() {
        let screen = kurbo::Rect::new(f64::from(r.min.x), f64::from(r.min.y), f64::from(r.max.x), f64::from(r.max.y));
        let time = ui.input(|i| i.time);
        Marquee::rect(screen, "").paint_screen(painter, fitted, MarqueeLook::Ants, time);
    }
}

/// The wheel over the preview: one notch of the navigation zoom about the view centre, whether or
/// not Zoom with Scroll Wheel is on, never a pan. Whether the view changed.
fn wheel(ctx: &egui::Context, response: &Response, view: &mut CanvasView, size: Size, navigation: &NavigationOptions, ppp: f32) -> bool {
    if !response.hovered() {
        return false;
    }
    let options = NavigationOptions { zoom_with_wheel: true, ..*navigation };
    let Some(action) = WheelNavigation::read(ctx, response, &options) else { return false };
    let current = zoom::device_zoom(view, ppp);
    let next = match action {
        WheelAction::Zoom(factor) => ZoomLadder::clamp(current * factor, size),
        WheelAction::SnapZoom(steps) => (0..steps.unsigned_abs()).fold(current, |z, _| ZoomLadder::step(z, steps.signum(), size)),
        _ => return false,
    };
    if (next / current - 1.0).abs() <= 1e-12 {
        WheelNavigation::settle(ctx, response);
        return false;
    }
    let center = view.center;
    zoom::zoom_about(view, view.rect.center(), next, false, ppp);
    view.center = center;
    true
}

/// Keys with the preview focused: arrows move the view by a tenth of its span (Shift: half), Page
/// Up and Down by a span, `+` and `-` step the ladder, Home centres the document. Whether the
/// view changed.
fn keys(ctx: &egui::Context, response: &Response, view: &mut CanvasView, size: Size, ppp: f32) -> bool {
    if !response.has_focus() {
        return false;
    }
    // The arrows are the preview's, not egui's focus movement.
    let arrows = egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..egui::EventFilter::default() };
    ctx.memory_mut(|m| m.set_focus_lock_filter(response.id, arrows));
    if !matches!(owner(ctx, &OwnerGate::default()), KeyOwner::Widget | KeyOwner::App) {
        return false;
    }
    let take = |modifiers: Modifiers, key: Key| ctx.input_mut(|i| i.consume_key(modifiers, key));
    let before = *view;
    let span = view.rect.size();
    let mut delta = Vec2::ZERO;
    for (key, direction) in
        [(Key::ArrowLeft, vec2(-1.0, 0.0)), (Key::ArrowRight, vec2(1.0, 0.0)), (Key::ArrowUp, vec2(0.0, -1.0)), (Key::ArrowDown, vec2(0.0, 1.0))]
    {
        if take(Modifiers::SHIFT, key) {
            delta += direction * span * SHIFT_STEP;
        } else if take(Modifiers::NONE, key) {
            delta += direction * span * ARROW_STEP;
        }
    }
    if take(Modifiers::NONE, Key::PageUp) {
        delta.y -= span.y * PAGE_STEP;
    }
    if take(Modifiers::NONE, Key::PageDown) {
        delta.y += span.y * PAGE_STEP;
    }
    if delta != Vec2::ZERO {
        move_view(view, view.unmap_vec(delta), size);
    }
    if take(Modifiers::NONE, Key::Plus) || take(Modifiers::NONE, Key::Equals) || take(Modifiers::SHIFT, Key::Equals) {
        zoom::step_view(view, 1, size, ppp);
    }
    if take(Modifiers::NONE, Key::Minus) {
        zoom::step_view(view, -1, size, ppp);
    }
    if take(Modifiers::NONE, Key::Home) {
        view.center = Point::new(size.width / 2.0, size.height / 2.0);
    }
    *view != before
}

/// Moves the view centre by `by` document units, within the document.
fn move_view(view: &mut CanvasView, by: kurbo::Vec2, size: Size) {
    let center = view.center + by;
    view.center = Point::new(center.x.clamp(0.0, size.width), center.y.clamp(0.0, size.height));
}

/// The two AccessKit scroll bars of the preview: the view centre in percent of the document, and
/// `Increment` and `Decrement` moving the view by an arrow-key step. Whether the view changed.
fn scroll_bars(nodes: &Ui, response: &Response, frame: Rect, view: &mut CanvasView, size: Size, labels: &NavigatorLabels) -> bool {
    let before = *view;
    let visible = view.visible();
    for horizontal in [true, false] {
        let (name, orientation, center, extent, span) = if horizontal {
            (&labels.horizontal, accesskit::Orientation::Horizontal, view.center.x, size.width, visible.width())
        } else {
            (&labels.vertical, accesskit::Orientation::Vertical, view.center.y, size.height, visible.height())
        };
        let extent = extent.max(1.0);
        let step = span * f64::from(ARROW_STEP);
        let id = response.id.with(("scroll", horizontal));
        let bar = Interactive::with_id(id, Role::ScrollBar, name.as_str())
            .sense(Sense::hover())
            .state_layer(false)
            .focus_ring(false)
            .interact_at(nodes, frame)
            .numeric((center / extent * 100.0).clamp(0.0, 100.0), 0.0..=100.0, step / extent * 100.0)
            .finish(nodes);
        nodes.ctx().accesskit_node_builder(bar.id, |node| node.set_orientation(orientation));
        let requests = |action: Action| nodes.input(|i| i.num_accesskit_action_requests(id, action)) as f64;
        let moved = step * (requests(Action::Increment) - requests(Action::Decrement));
        if moved != 0.0 {
            move_view(view, if horizontal { kurbo::Vec2::new(moved, 0.0) } else { kurbo::Vec2::new(0.0, moved) }, size);
        }
    }
    *view != before
}

/// The proxy of `view` over the preview: upright, a rounded rectangle; turned or mirrored, the quad
/// with each edge clipped to the box less the halo's width. A halo outline under the coloured line.
fn paint_proxy(painter: &Painter, view: &CanvasView, size: Size, fitted: Rect, frame: Rect, style: &NavigatorStyle, color: Color32) {
    let quad = proxy_quad(view, size, fitted);
    let clip = frame.shrink(style.halo_width);
    let halo = Stroke::new(style.proxy_width + 2.0 * style.halo_width, style.halo);
    let line = Stroke::new(style.proxy_width, color);
    if view.axis_aligned() {
        let rect = Rect::from_points(&quad).intersect(clip);
        if rect.is_positive() {
            painter.rect_stroke(rect, style.proxy_radius, halo, StrokeKind::Middle);
            painter.rect_stroke(rect, style.proxy_radius, line, StrokeKind::Middle);
        }
        return;
    }
    let edges: Vec<[Pos2; 2]> = (0..4).filter_map(|i| clip_segment(quad[i], quad[(i + 1) % 4], clip)).collect();
    for stroke in [halo, line] {
        for edge in &edges {
            painter.line_segment(*edge, stroke);
        }
    }
}

/// The part of the segment `a`–`b` inside `rect` (Liang–Barsky), if any.
fn clip_segment(a: Pos2, b: Pos2, rect: Rect) -> Option<[Pos2; 2]> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
    for (p, q) in [(-d.x, a.x - rect.min.x), (d.x, rect.max.x - a.x), (-d.y, a.y - rect.min.y), (d.y, rect.max.y - a.y)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 <= t1).then(|| [a + d * t0, a + d * t1])
}

/// What [`Navigator::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct NavigatorOutput {
    /// The preview box: its focus and hover.
    pub response: Response,
    /// The view was changed this pass (a drag pass, a click, a notch, a key, the row). The app
    /// writes it back to its own view and clears its fit-pending and zoom anchor.
    pub view_changed: bool,
    /// The change was a Zoom In or Zoom Out button with Animated Zoom on: the app starts
    /// [`ZoomAnimation`](crate::canvas::ZoomAnimation) from the view before the change.
    pub animate_zoom: bool,
    /// The settings changed (never from the body today; the field exists for the dialog and
    /// later options): the app persists [`NavigatorSettings`].
    pub settings_changed: bool,
    /// Where the document picture was drawn (PhotoCraft's `last_navigator_preview` test hook).
    pub preview_rect: Rect,
    /// The zoom row (PhotoCraft's `last_navigator_controls` test hook).
    pub row_rect: Rect,
}

/// Paints a checkerboard of `style.checker`-point squares in `style.checker_colors` over `rect`,
/// starting at its top-left corner (`CheckerSize` offers 4, 8 and 16 only).
fn paint_checker(painter: &Painter, rect: Rect, style: &NavigatorStyle) {
    // pc: crates/ui-egui/src/widgets/color.rs:29 `checker`, called with 6.0 at panels/navigator.rs:25, :28.
    let painter = painter.with_clip_rect(painter.clip_rect().intersect(rect));
    let [light, dark] = style.checker_colors;
    painter.rect_filled(rect, 0.0, light);
    let side = style.checker;
    if side <= 0.0 {
        return;
    }
    let (columns, rows) = ((rect.width() / side).ceil() as usize, (rect.height() / side).ceil() as usize);
    for row in 0..rows {
        for column in (1 - row % 2..columns).step_by(2) {
            let square = Rect::from_min_size(rect.min + vec2(column as f32 * side, row as f32 * side), vec2(side, side));
            painter.rect_filled(square, 0.0, dark);
        }
    }
}
