//! `Navigator`: the preview box with the proxy rectangle of the view. The picture is fitted inside the box; a press
//! moves the view, a drag of the proxy moves it by the distance dragged, a Cmd/Ctrl-drag zooms to the rectangle, the
//! wheel zooms about the view centre and the arrow keys move it.

use super::ProxyColor;
use super::proxy::{preview_rect, proxy_quad, view_center_for, zoom_to_document_rect};
use crate::canvas::{CanvasWords, NavigationOptions, WheelAction, WheelNavigation};
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, MeasureCx, Name, Outcome, PaintUi, PartsCx,
    Pointer as _, Props as _, Remember as _, Role, WordSetCx as _,
};
use crate::primitives::ContentPainterCx;
use crate::primitives::textures::CachedTexture;
use crate::runtime::access::Numeric;
use crate::runtime::env::Label;
use crate::runtime::focus::{KeepKeys, KeyUse};
use crate::style::{ComponentStyle, FocusLook, StyleColor, StyleCx};
use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{
    CancelReason, CanvasView, CursorIcon, CursorPriority, HitLevel, HitPriority, HitShape, Metrics, NavMove, PartKey, PressCancel, PressPhase, Sense,
};
use egui::{Color32, Pos2, Rect, Response, Stroke, StrokeKind, Vec2, pos2};
use std::ops::Deref;

const WELL: PartKey = PartKey::new("well");
const PROXY: PartKey = PartKey::new("proxy");
const HORIZONTAL: PartKey = PartKey::new("horizontal");
const VERTICAL: PartKey = PartKey::new("vertical");

/// An arrow key moves the view by this share of the visible span, Shift by `SHIFT_STEP`, Page Up and Page Down by
/// `PAGE_STEP`.
const ARROW_STEP: f32 = 0.1;
const SHIFT_STEP: f32 = 0.5;
const PAGE_STEP: f32 = 1.0;

/// A closure that paints the document's picture into the fitted rect.
pub struct NavigatorPaint<'a>(Box<dyn FnOnce(&ContentPainterCx<'_>, Rect) + 'a>);

impl<'a> NavigatorPaint<'a> {
    /// A painter that draws the picture with `paint`, given the content painter and the rect the picture fills.
    pub fn new(paint: impl FnOnce(&ContentPainterCx<'_>, Rect) + 'a) -> Self {
        NavigatorPaint(Box::new(paint))
    }
}

/// What the preview box shows: the document's picture, made and cached by the app.
#[non_exhaustive]
pub enum NavigatorPreview<'a> {
    /// A texture from the app's own texture store, drawn over the checkerboard into the fitted rect.
    Texture(CachedTexture),
    /// The app paints the picture itself into the fitted rect; the checkerboard is not drawn.
    Paint(NavigatorPaint<'a>),
    /// No picture yet: the checkerboard alone, with the proxy over it.
    None,
}

/// The press in progress, kept between passes.
#[derive(Clone, Default)]
struct Held {
    active: bool,
    /// The press draws a zoom rectangle (Cmd or Ctrl at the press).
    zoom: bool,
    /// The press started on the proxy: the view moves by the distance dragged.
    on_proxy: bool,
    from: Option<CanvasView>,
}

/// The style of the navigator.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct NavigatorStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// Between the box and the fitted document.
    inset: f32,
    proxy_width: f32,
    halo_width: f32,
    halo: StyleColor,
    radius: f32,
}

/// The preview box of the Navigator panel with the proxy rectangle of the view.
///
/// - A press on the box moves the view centre to the point pressed and follows the pointer; a press on the proxy moves
///   the view by the distance dragged. Escape puts the view back as it was at the press.
/// - With Cmd (macOS) or Ctrl held at the press, the drag draws a rectangle and the release zooms the view to it.
/// - The wheel zooms about the view centre, one notch at a time, whether or not Zoom with Scroll Wheel is on; a pinch
///   zooms the same way when the navigation options allow it. With Animated Zoom on, the output asks the app to ease
///   the zoom.
/// - Focused, the arrow keys move the view by a tenth of its span (Shift: half), Page Up and Page Down by a span.
/// - The box has two AccessKit scroll bars, the view centre in percent of the document; a set value moves the view.
/// - The proxy follows a rotated or mirrored view as a quad.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(content_painter)]
pub struct Navigator<'a> {
    #[common]
    common: Common,
    view: Bind<'a, CanvasView>,
    document: Size,
    /// The picture.
    #[prop]
    preview: NavigatorPreview<'a>,
    /// The app's navigation options.
    #[prop]
    navigation: NavigationOptions,
    /// The colour of the proxy rectangle.
    #[prop]
    proxy: ProxyColor,
    zoom_rect: Option<Rect>,
    animate: bool,
}

impl<'a> Navigator<'a> {
    /// A navigator named `label` that edits `view`, over a document of `document` units.
    pub fn new(label: impl Into<Label>, view: Bind<'a, CanvasView>, document: Size) -> Self {
        Navigator {
            common: Common::new(),
            view,
            document,
            preview: NavigatorPreview::None,
            navigation: NavigationOptions::default(),
            proxy: ProxyColor::Default,
            zoom_rect: None,
            animate: false,
        }
        .label(label)
    }

    fn quad(&self, view: &CanvasView, rect: Rect, inset: f32) -> Vec<Pos2> {
        clip_polygon(&proxy_quad(view, self.document, preview_rect(rect, self.document, inset)), rect)
    }
}

/// The part of the convex polygon `points` inside `rect` (Sutherland and Hodgman).
fn clip_polygon(points: &[Pos2], rect: Rect) -> Vec<Pos2> {
    let edges: [fn(Pos2, Rect) -> f32; 4] = [|p, r| p.x - r.min.x, |p, r| r.max.x - p.x, |p, r| p.y - r.min.y, |p, r| r.max.y - p.y];
    let mut out = points.to_vec();
    for inside in edges {
        let input = std::mem::take(&mut out);
        for (at, &current) in input.iter().enumerate() {
            let previous = input[(at + input.len() - 1) % input.len()];
            let (a, b) = (inside(previous, rect), inside(current, rect));
            if (a >= 0.0) != (b >= 0.0) {
                let t = a / (a - b);
                out.push(previous + (current - previous) * t);
            }
            if b >= 0.0 {
                out.push(current);
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// The part of the segment `a`-`b` inside `rect` (Liang and Barsky), if any.
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

/// `view` with its centre at `center` inside the document.
fn centred(view: &CanvasView, center: Point, document: Size) -> CanvasView {
    let mut next = *view;
    next.center = Point::new(center.x.clamp(0.0, document.width), center.y.clamp(0.0, document.height));
    next
}

impl Component for Navigator<'_> {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::BlockArrows);
    type Style = NavigatorStyle;
    type Output = NavigatorOutput;

    fn style(cx: &StyleCx<'_>, _: NavigatorStyleKey) -> NavigatorStyle {
        NavigatorStyle {
            metrics: Metrics::new().min_size(Vec2::new(cx.tokens.size.plot.width, cx.tokens.size.plot.height)),
            focus: cx.focus_look(),
            inset: cx.tokens.space.gap.inline,
            proxy_width: cx.tokens.stroke.medium,
            halo_width: cx.tokens.stroke.thin,
            halo: cx.color(cx.tokens.color.overlay.halo),
            radius: cx.tokens.shape.radius.sm,
        }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn sense(&self) -> Sense {
        Sense::FOCUS | Sense::WHEEL | Sense::WHEEL_ZOOM
    }

    fn bind(&mut self) -> Option<BindMut<'_, CanvasView>> {
        Some(self.view.reborrow())
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(WELL, rect).sense(Sense::CLICK | Sense::DRAG).pointer_only(
            "the arrow keys move the view; the zoom field and Zoom In and Zoom Out set the zoom, the zoom rectangle has no numeric form",
        );
        for bar in [HORIZONTAL, VERTICAL] {
            cx.part(bar, rect).sense(Sense::HOVER).priority(HitPriority::new(HitLevel::Background, 0)).keyboard_via(WELL);
        }
        let Some(view) = cx.value().copied() else { return };
        let inset = cx.style.inset;
        let quad = self.quad(&view, rect, inset);
        if quad.len() >= 3 {
            cx.part(PROXY, HitShape::Polygon(quad))
                .sense(Sense::DRAG)
                .priority(HitPriority::new(HitLevel::Handle, 0))
                .cursor(CursorIcon::Grab)
                .drag_cursor(CursorIcon::Grabbing)
                .keyboard_via(WELL);
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(view) = cx.value().copied() else { return };
        let rect = cx.rect;
        let document = self.document;
        let ppp = cx.env.scale().0;
        let fitted = preview_rect(rect, document, cx.style.inset);
        let scale = f64::from(fitted.width() / document.width.max(1.0) as f32);

        if let Some(wheel) = cx.take_wheel() {
            let options = NavigationOptions { zoom_with_wheel: true, ..self.navigation };
            let action = cx.memory::<WheelNavigation>().read(&wheel, &options);
            if let Some(action @ (WheelAction::Zoom(_) | WheelAction::SnapZoom(_))) = action {
                let mut next = view;
                WheelNavigation::apply(&mut next, action, view.rect.center(), document, ppp);
                next.center = view.center;
                if next != view {
                    self.animate = self.navigation.animated_zoom;
                    cx.set(next);
                    return;
                }
            }
        }

        let proxy = cx.part_if_declared(PROXY).map(|part| (part.press, part.hovered));
        let well = (cx.part(WELL).press, cx.part(WELL).hovered);
        let (press, hovered, on_proxy) = match proxy {
            Some((press, hovered)) if press.phase.is_press() => (press, hovered, true),
            Some((_, true)) => (well.0, true, true),
            _ => (well.0, well.1, false),
        };
        if hovered && press.mods.cmd && !press.phase.is_press() {
            cx.cursor(CursorIcon::ZoomIn, CursorPriority::Part);
        }
        let mut held = cx.memory::<Held>().clone();
        if press.went_down
            && let Some(start) = press.phase.start()
        {
            held = Held { active: true, zoom: press.mods.cmd, on_proxy, from: Some(view) };
            if !held.zoom {
                cx.begin();
                if !on_proxy {
                    cx.propose(centred(&view, view_center_for(fitted, document, start.origin), document));
                }
            }
        }
        if held.active {
            let from = held.from.unwrap_or(view);
            let (zooming, moves_by_drag) = (held.zoom, held.on_proxy);
            let at = |pos: Pos2, origin: Pos2| {
                if moves_by_drag {
                    let moved = (pos - origin) / scale as f32;
                    centred(&from, Point::new(from.center.x + f64::from(moved.x), from.center.y + f64::from(moved.y)), document)
                } else {
                    centred(&from, view_center_for(fitted, document, pos), document)
                }
            };
            match press.phase {
                PressPhase::Drag { start, pos, .. } if zooming => {
                    self.zoom_rect = Some(Rect::from_two_pos(start.origin, pos).intersect(fitted));
                }
                PressPhase::Drag { start, pos, .. } => cx.propose(at(pos, start.origin)),
                PressPhase::Release { start, pos, .. } if zooming => {
                    let r = Rect::from_two_pos(start.origin, pos).intersect(fitted);
                    let to_doc = |p: Pos2| ((p.x - fitted.min.x) / scale as f32, (p.y - fitted.min.y) / scale as f32);
                    let ((x0, y0), (x1, y1)) = (to_doc(r.min), to_doc(r.max));
                    let mut next = view;
                    if zoom_to_document_rect(
                        &mut next,
                        craft_behavior::kurbo::Rect::new(f64::from(x0), f64::from(y0), f64::from(x1), f64::from(y1)),
                        document,
                        ppp,
                    ) {
                        cx.set(next);
                    }
                    held.active = false;
                }
                PressPhase::Release { start, pos, .. } => {
                    cx.propose(at(pos, start.origin));
                    cx.commit();
                    held.active = false;
                }
                PressPhase::Cancel { reason, .. } => {
                    if !zooming {
                        cx.cancel(if reason == PressCancel::Escape { CancelReason::Escape } else { CancelReason::Blur });
                    }
                    held.active = false;
                }
                _ => {}
            }
        }
        *cx.memory::<Held>() = held;

        let span = view.rect.size();
        let rtl = cx.flow.is_rtl();
        let mut delta = Vec2::ZERO;
        while let Some((mv, mods)) = cx.intents().take_move() {
            let share = if mods.extend { SHIFT_STEP } else { ARROW_STEP };
            let side = if rtl { -1.0 } else { 1.0 };
            match mv {
                NavMove::Back => delta.x -= side * span.x * share,
                NavMove::Forward => delta.x += side * span.x * share,
                NavMove::Up => delta.y -= span.y * share,
                NavMove::Down => delta.y += span.y * share,
                NavMove::PageUp => delta.y -= span.y * PAGE_STEP,
                NavMove::PageDown => delta.y += span.y * PAGE_STEP,
                NavMove::Start | NavMove::End => {}
                _ => {}
            }
        }
        let mut next = view;
        if delta != Vec2::ZERO {
            let by = view.unmap_vec(delta);
            next = centred(&view, Point::new(view.center.x + by.x, view.center.y + by.y), document);
        }
        if let Some(percent) = cx.intents().take_set_value_at(HORIZONTAL) {
            next.center.x = (percent / 100.0).clamp(0.0, 1.0) * document.width;
        }
        if let Some(percent) = cx.intents().take_set_value_at(VERTICAL) {
            next.center.y = (percent / 100.0).clamp(0.0, 1.0) * document.height;
        }
        if next != view {
            cx.set(next);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rect = cx.rect;
        let style = cx.style;
        let fitted = preview_rect(rect, self.document, style.inset);
        let preview = std::mem::replace(&mut self.preview, NavigatorPreview::None);
        let painter = ui.content_painter(cx);
        painter.plot_frame(rect);
        match preview {
            NavigatorPreview::Texture(texture) => {
                painter.checker(fitted);
                painter.picture(texture.id, fitted, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
            }
            NavigatorPreview::Paint(NavigatorPaint(paint)) => paint(&painter, fitted),
            NavigatorPreview::None => painter.checker(fitted),
        }
        let Some(view) = cx.value().copied() else { return };
        let raw = &painter.painter;
        let clip = rect.shrink(style.halo_width);
        let halo = Stroke::new(style.proxy_width + 2.0 * style.halo_width, style.halo.egui());
        let line = Stroke::new(style.proxy_width, Color32::from(self.proxy.color()));
        let quad = proxy_quad(&view, self.document, fitted);
        if view.axis_aligned() {
            let proxy = Rect::from_points(&quad).intersect(clip);
            if proxy.is_positive() {
                let radius = egui::CornerRadius::same(style.radius.round().clamp(0.0, 255.0) as u8);
                raw.rect_stroke(proxy, radius, halo, StrokeKind::Middle);
                raw.rect_stroke(proxy, radius, line, StrokeKind::Middle);
            }
        } else {
            let edges: Vec<[Pos2; 2]> = (0..4).filter_map(|at| clip_segment(quad[at], quad[(at + 1) % 4], clip)).collect();
            for stroke in [halo, line] {
                for edge in &edges {
                    raw.line_segment(*edge, stroke);
                }
            }
        }
        if let Some(zoom) = self.zoom_rect.filter(|zoom| zoom.is_positive()) {
            painter.marching_ants(&[zoom.left_top(), zoom.right_top(), zoom.right_bottom(), zoom.left_bottom()], true, 0.0);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let Some(view) = cx.value().copied() else { return };
        let words = cx.words::<CanvasWords>();
        let percent = |center: f64, extent: f64| (center / extent.max(1.0) * 100.0).clamp(0.0, 100.0);
        for (key, name, value) in [
            (HORIZONTAL, &words.navigator_horizontal, percent(view.center.x, self.document.width)),
            (VERTICAL, &words.navigator_vertical, percent(view.center.y, self.document.height)),
        ] {
            let node = access.node(key, Role::SCROLL_BAR);
            node.name = Name::Text(name.as_str().to_owned());
            node.numeric = Some(Numeric::new(value, 0.0..=100.0));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> NavigatorOutput {
        NavigatorOutput { view: cx.edit_response(), animate_zoom: self.animate }
    }
}

/// What the navigator reports in a pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct NavigatorOutput {
    /// The view: a drag's gesture, a wheel notch, a key or a zoom rectangle as one change.
    pub view: EditResponse<CanvasView>,
    /// The change was a wheel zoom with Animated Zoom on: the app starts a `ZoomAnimation` from the view before it.
    pub animate_zoom: bool,
}

impl Deref for NavigatorOutput {
    type Target = EditResponse<CanvasView>;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl Outcome for NavigatorOutput {
    type Value = CanvasView;

    fn response(&self) -> &Response {
        &self.view.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.view.story_facts()
    }
}
