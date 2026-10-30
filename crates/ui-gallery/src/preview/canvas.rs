//! The canvas region: document tab, rulers, pasteboard with the document, scrollbars.
//!
//! `dock::show` gives this module the rect between the tool rail and the dock. Everything on
//! screen comes from the library (`TabStrip`, `Rulers`, `CanvasView`, `CanvasScrollbars`,
//! `Marquee`); the document's fills are the only local painting, as in the library's canvas story.

use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::canvas::{CanvasScrollbars, CanvasView, MarqueeDrag, MarqueeShape, RulerNames, RulerUnits, Rulers, ScrollbarNames};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::tab_strip::{Tab, TabKind, TabStrip};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{Color32, CursorIcon, Id, Key, Modifiers, PointerButton, Sense, Shape, Stroke, Ui, WidgetInfo};

use super::model::{Doc, Mock, Tool, View};

/// What one click of the Zoom tool multiplies the zoom by.
const ZOOM_STEP: f64 = 2.0;

/// A filled block of a layer on the page, in document units, with its colour and opacity.
#[derive(Clone, Debug, PartialEq)]
struct Fill {
    rect: Rect,
    color: [u8; 3],
    /// Percent.
    opacity: u8,
}

/// The fills the canvas paints on the page, bottom layer first: visible pixel layers in the
/// colour `Doc::shown_color` gives them, cut to the page. Hidden, empty and adjustment layers
/// paint nothing. A list rather than painter calls so a test can compare what two documents show.
fn fills(doc: &Doc) -> Vec<Fill> {
    let page = Rect::new(0.0, 0.0, f64::from(doc.size[0]), f64::from(doc.size[1]));
    (0..doc.layers.len())
        .rev()
        .filter_map(|index| {
            let layer = &doc.layers[index];
            let color = doc.shown_color(index).filter(|_| layer.visible)?;
            let [x, y, w, h] = layer.rect.map(f64::from);
            let rect = Rect::new(x, y, x + w, y + h).intersect(page);
            (rect.width() > 0.0 && rect.height() > 0.0).then_some(Fill { rect, color, opacity: layer.opacity })
        })
        .collect()
}

/// The view as the library draws it: `View::zoom` 1 is one device pixel per document pixel, so a
/// point shows `zoom / pixels_per_point` of them.
fn canvas_view(view: &View, rect: egui::Rect, ppp: f64) -> CanvasView {
    CanvasView::new(rect, Point::new(view.center[0], view.center[1]), view.zoom / ppp)
}

/// Multiplies the zoom by `factor` so the document point at `offset` (device pixels from the
/// region's middle) stays where it is.
fn zoom_about(view: &mut View, factor: f64, offset: [f64; 2]) {
    let before = view.zoom;
    view.set_zoom(before * factor);
    for (center, offset) in view.center.iter_mut().zip(offset) {
        *center += offset / before - offset / view.zoom;
    }
}

/// The document tab's title: `Untitled @ 99.2% (Shapes, RGB/8)`.
fn tab_title(mock: &Mock) -> String {
    let percent = format!("{:.1}", mock.view.zoom * 100.0);
    let percent = percent.strip_suffix(".0").unwrap_or(&percent);
    let layer = match mock.doc.selected.as_slice() {
        [] => String::new(),
        [_] => mock.doc.selected_layer().map(|l| format!("{}, ", l.name)).unwrap_or_default(),
        many => format!("{} layers, ", many.len()),
    };
    format!("{} @ {percent}% ({layer}RGB/8)", mock.doc.name)
}

/// `r` rounded to whole document pixels.
fn pixels(r: Rect) -> Rect {
    Rect::new(r.x0.round(), r.y0.round(), r.x1.round(), r.y1.round())
}

/// Draws the canvas region into the rect `ui` has left and runs its pointer tools.
pub fn show(ui: &mut Ui, mock: &mut Mock) {
    let ctx = ui.ctx().clone();
    let tokens = Tokens::of(ui);
    let pasteboard = tokens.color.surface.canvas.to_egui();
    let ruler_size = tokens.component.ruler.size;
    drop(tokens);
    let id = ui.scope_id();

    let tab = Tab::new(0u8, tab_title(mock)).modified(mock.doc.modified);
    let _ = TabStrip::new(id.with("documents"), "Documents", TabKind::Document).selected(&0u8).show(ui, &[tab]);

    let full = ui.available_rect_before_wrap();
    let rect = if mock.view.rulers { egui::Rect::from_min_max(full.min + egui::vec2(ruler_size, ruler_size), full.max) } else { full };
    ui.allocate_rect(full, Sense::hover());
    let ppp = f64::from(ui.pixels_per_point());
    // The document opens fitted, and Fit on Screen asks again.
    let fitted = id.with("fitted");
    if mock.view.fit_pending || !ctx.data(|d| d.get_temp::<bool>(fitted)).unwrap_or(false) {
        mock.view.fit([f64::from(rect.width()) * ppp, f64::from(rect.height()) * ppp], mock.doc.size);
        ctx.data_mut(|d| d.insert_temp(fitted, true));
    }

    let response = ui.interact(rect, id.with("canvas"), Sense::click_and_drag() | Sense::FOCUSABLE);
    response.widget_info(|| WidgetInfo::labeled(Role::Canvas, ui.is_enabled(), "Document"));
    let tool = mock.tools.current;
    let cv = canvas_view(&mock.view, rect, ppp);
    let from_middle = |p: egui::Pos2| {
        let d = (p - rect.center()) * ppp as f32;
        [f64::from(d.x), f64::from(d.y)]
    };

    // Ctrl or Cmd with the wheel (and a pinch) zooms about the pointer; the plain wheel scrolls.
    if response.hovered() {
        let (zoom, scroll) = ctx.input(|i| (f64::from(i.zoom_delta()), i.smooth_scroll_delta));
        if (zoom - 1.0).abs() > f64::EPSILON
            && let Some(pointer) = response.hover_pos()
        {
            zoom_about(&mut mock.view, zoom, from_middle(pointer));
        } else if scroll != egui::Vec2::ZERO {
            mock.view.center[0] -= f64::from(scroll.x) / cv.zoom;
            mock.view.center[1] -= f64::from(scroll.y) / cv.zoom;
        }
    }
    match tool {
        Tool::Hand => {
            if response.dragged_by(PointerButton::Primary) {
                let delta = response.drag_delta();
                mock.view.center[0] -= f64::from(delta.x) / cv.zoom;
                mock.view.center[1] -= f64::from(delta.y) / cv.zoom;
                ui.set_cursor_icon(CursorIcon::Grabbing);
            } else if response.hovered() {
                ui.set_cursor_icon(CursorIcon::Grab);
            }
        }
        Tool::Zoom => {
            let out = ctx.input(|i| i.modifiers.alt);
            if response.hovered() {
                ui.set_cursor_icon(if out { CursorIcon::ZoomOut } else { CursorIcon::ZoomIn });
            }
            if response.clicked_by(PointerButton::Primary)
                && let Some(pointer) = response.interact_pointer_pos()
            {
                let factor = if out { 1.0 / ZOOM_STEP } else { ZOOM_STEP };
                zoom_about(&mut mock.view, factor, from_middle(pointer));
            }
        }
        Tool::RectMarquee if response.hovered() => ui.set_cursor_icon(CursorIcon::Crosshair),
        _ => {}
    }

    // The document stays on the canvas (overscroll off).
    let page = Rect::new(0.0, 0.0, f64::from(mock.doc.size[0]), f64::from(mock.doc.size[1]));
    let mut cv = canvas_view(&mock.view, rect, ppp);
    if CanvasScrollbars::clamp(&mut cv, page) {
        mock.view.center = [cv.center.x, cv.center.y];
    }

    // Pasteboard, page and layers.
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, pasteboard);
    painter.add(Shape::convex_polygon(cv.quad(page).to_vec(), Color32::WHITE, Stroke::NONE));
    for fill in fills(&mock.doc) {
        let [r, g, b] = fill.color;
        let alpha = (u16::from(fill.opacity) * 255 / 100) as u8;
        painter.add(Shape::convex_polygon(cv.quad(fill.rect).to_vec(), Color32::from_rgba_unmultiplied(r, g, b, alpha), Stroke::NONE));
    }

    // The marquee tool and its selection, which outlives the tool.
    let numbers = LocaleNumbers::of(ui);
    let key = id.with("marquee-selection");
    let mut selection: Option<Rect> = ctx.data(|d| d.get_temp(key));
    let mut live = None;
    if tool == Tool::RectMarquee {
        let drag = MarqueeDrag::new("marquee").shape(MarqueeShape::Rect).show(ui, &response, &cv);
        if drag.clicked {
            selection = None;
            mock.status.clear();
        }
        if let Some(EditEvent::Commit { value, changed: true, .. }) = &drag.event {
            selection = Some(pixels(value.rect()));
        }
        live = drag.rect.map(pixels);
    }
    if selection.is_some() && (response.has_focus() || response.hovered()) && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        selection = None;
        mock.status.clear();
    }
    if let Some(r) = live.or(selection) {
        mock.status = format!("Marquee {} x {} px", r.width(), r.height());
        MarqueeShape::Rect.marquee(r, "Selection").bounds_value(&numbers, &Unit::new("{n} px")).show(ui, &response, &cv);
    }
    ctx.data_mut(|d| {
        if let Some(r) = selection {
            d.insert_temp(key, r);
        } else {
            d.remove::<Rect>(key);
        }
    });

    let names = ScrollbarNames::new("Horizontal scroll bar", "Vertical scroll bar");
    if CanvasScrollbars::new(id.with("scrollbars"), names).show(ui, &mut cv, page).changed {
        mock.view.center = [cv.center.x, cv.center.y];
    }

    if mock.view.rulers {
        let zero_key = id.with("ruler-zero");
        let mut zero: Point = ctx.data(|d| d.get_temp(zero_key)).unwrap_or(Point::ZERO);
        let units = RulerUnits::pixels((*numbers).clone());
        let names = RulerNames::new("Horizontal ruler", "Vertical ruler", "Ruler origin");
        let rulers = Rulers::new(id.with("rulers"), &units, &units, names).zero(zero).show(ui, full, &cv, None);
        if rulers.reset_origin {
            zero = Point::ZERO;
        }
        if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = rulers.origin {
            zero = value;
        }
        ctx.data_mut(|d| d.insert_temp(zero_key, zero));
    }
}

#[cfg(test)]
mod tests {
    use craft_theme::{ModeChoice, SystemAppearance};
    use craft_ui::inspect::AccessTree;
    use craft_ui::theme::install;
    use egui::accesskit::Role;
    use egui::{Context, Event, MouseWheelSource, MouseWheelUnit, Pos2, RawInput, TouchPhase, vec2};

    use super::*;
    use crate::Appearance;

    /// The canvas region alone in a window, with the gallery's theme.
    struct Host {
        ctx: Context,
        appearance: Appearance,
        mock: Mock,
    }

    impl Host {
        fn new() -> Self {
            let ctx = Context::default();
            ctx.enable_accesskit();
            let appearance = Appearance { theme: "craft.artcraft".to_owned(), mode: ModeChoice::Dark, ..Appearance::default() };
            appearance.configure(&ctx, None, false, SystemAppearance::default());
            Self { ctx, appearance, mock: Mock::default() }
        }

        fn frame(&mut self, events: Vec<Event>) -> Option<AccessTree> {
            let screen = egui::Rect::from_min_size(Pos2::ZERO, vec2(900.0, 640.0));
            let raw = RawInput { screen_rect: Some(screen), events, ..RawInput::default() };
            let base = self.appearance.base(SystemAppearance::default());
            let output = self.ctx.run_ui(raw, |ui| {
                install(ui.ctx(), &base);
                show(ui, &mut self.mock);
            });
            let tree = output.platform_output.accesskit_update.as_ref().and_then(AccessTree::from_update);
            output.drop_without_applying_deltas();
            tree
        }

        fn settle(&mut self) -> AccessTree {
            for _ in 0..3 {
                self.frame(Vec::new());
            }
            self.frame(Vec::new()).expect("an AccessKit tree")
        }
    }

    fn has_name(tree: &AccessTree, name: &str) -> bool {
        tree.walk(tree.root()).iter().any(|(id, _)| tree.name(*id).as_deref() == Some(name))
    }

    #[test]
    fn hiding_a_layer_changes_what_is_painted() {
        let mut doc = Doc::default();
        // The seed document paints Background and Shapes; Sky is hidden.
        assert_eq!(fills(&doc).len(), 2);
        let id_of = |doc: &Doc, name: &str| doc.layers.iter().find(|l| l.name == name).map(|l| l.id).expect("a seed layer");
        let sky = id_of(&doc, "Sky");
        assert!(doc.toggle_visible(sky));
        assert_eq!(fills(&doc).len(), 3);
        let shapes = id_of(&doc, "Shapes");
        doc.toggle_visible(shapes);
        assert!(fills(&doc).iter().all(|f| f.color != [240, 140, 40]));
    }

    #[test]
    fn the_first_draw_fits_the_document() {
        let mut host = Host::new();
        assert!((host.mock.view.zoom - 1.0).abs() < f64::EPSILON, "the model starts at 100%");
        host.frame(Vec::new());
        let fitted = host.mock.view.zoom;
        assert!(fitted < 0.95, "fitted to the region, not 100%: {fitted}");
        assert_eq!(host.mock.view.center, [600.0, 400.0]);
        // Fit on Screen answers the pending request on the next draw.
        host.mock.view.set_zoom(3.0);
        host.mock.view.fit_pending = true;
        host.frame(Vec::new());
        assert!((host.mock.view.zoom - fitted).abs() < 1e-9);
        assert!(!host.mock.view.fit_pending);
    }

    #[test]
    fn ctrl_wheel_zooms_and_keeps_the_point_under_the_pointer() {
        let mut host = Host::new();
        host.settle();
        host.mock.view.set_zoom(1.0);
        host.mock.view.center = [600.0, 400.0];
        let tree = host.settle();
        let canvas = tree
            .walk(tree.root())
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| tree.name(*id).as_deref() == Some("Document") && tree.node(*id).is_some_and(|n| n.role() == Role::Canvas))
            .expect("the canvas node");
        let rect = tree.bounds(canvas).expect("canvas bounds");
        let pointer = rect.center() + vec2(120.0, -70.0);
        let doc_at = |m: &Mock| canvas_view(&m.view, rect, 1.0).to_doc(pointer);
        host.frame(vec![Event::PointerMoved(pointer)]);
        let (zoom, point) = (host.mock.view.zoom, doc_at(&host.mock));
        let wheel = Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, 60.0),
            modifiers: Modifiers::CTRL,
            phase: TouchPhase::Move,
            source: MouseWheelSource::Wheel,
        };
        host.frame(vec![wheel]);
        assert!(host.mock.view.zoom > zoom, "Ctrl+wheel up zooms in: {zoom} -> {}", host.mock.view.zoom);
        let after = doc_at(&host.mock);
        assert!((after.x - point.x).abs() < 1e-6 && (after.y - point.y).abs() < 1e-6, "{point:?} moved to {after:?}");
    }

    #[test]
    fn turning_rulers_off_removes_the_ruler_nodes() {
        let mut host = Host::new();
        let tree = host.settle();
        assert!(has_name(&tree, "Horizontal ruler") && has_name(&tree, "Vertical ruler"));
        host.mock.view.rulers = false;
        let tree = host.settle();
        assert!(!has_name(&tree, "Horizontal ruler") && !has_name(&tree, "Vertical ruler"));
    }
}
