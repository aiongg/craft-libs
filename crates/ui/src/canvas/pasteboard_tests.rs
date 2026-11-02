//! The pasteboard and page frames.

use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, RawInput, pos2, vec2};
use kurbo::{Point, Rect};

use super::*;
use crate::theme::{Base, ToEgui, install, tokens_in_mode};
use craft_theme::Mode;

/// A 400 × 300 canvas at the screen origin showing document (0, 0)–(400, 300) at zoom 1.
fn view() -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0)
}

fn pages() -> Vec<PageFrame> {
    vec![PageFrame::new(Rect::new(50.0, 40.0, 150.0, 140.0)).name("One"), PageFrame::new(Rect::new(200.0, 100.0, 350.0, 260.0)).name("Two")]
}

fn area(q: &[Pos2; 4]) -> f64 {
    let r = egui::Rect::from_points(q);
    f64::from(r.width() * r.height())
}

#[test]
fn the_pasteboard_is_the_canvas_minus_the_pages() {
    let (view, pages) = (view(), pages());
    let quads = Pasteboard::quads(&view, &pages);
    let total: f64 = quads.iter().map(area).sum();
    let covered = 100.0 * 100.0 + 150.0 * 160.0;
    assert!((total - (400.0 * 300.0 - covered)).abs() < 1e-3, "total {total}");
    for page in &pages {
        let inner = page.rect.inflate(-0.5, -0.5);
        let probe = egui::Rect::from_min_max(pos2(inner.x0 as f32, inner.y0 as f32), pos2(inner.x1 as f32, inner.y1 as f32));
        assert!(quads.iter().all(|q| !egui::Rect::from_points(q).intersects(probe)), "a quad overlaps {:?}", page.name);
    }
    assert_eq!(Pasteboard::quads(&view, &[]).len(), 1, "no pages: the whole canvas");
}

fn secondary(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE }
}

fn primary(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// A pass showing the pasteboard with `events`.
fn pass(ctx: &egui::Context, events: Vec<Event>) -> PasteboardResponse {
    let input = RawInput { screen_rect: Some(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))), events, ..RawInput::default() };
    let mut out = PasteboardResponse::default();
    let pages = pages();
    ctx.run_ui(input, |ui| out = Pasteboard::new("pb", &pages).show(ui, &view())).drop_without_applying_deltas();
    out
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    ctx
}

#[test]
fn a_secondary_press_outside_the_pages_reports_its_point() {
    let ctx = context();
    pass(&ctx, vec![Event::PointerMoved(pos2(10.0, 10.0))]);
    let r = pass(&ctx, vec![secondary(pos2(10.0, 10.0), true)]);
    assert_eq!(r.context_at, Some(Point::new(10.0, 10.0)));
    let r = pass(&ctx, vec![secondary(pos2(10.0, 10.0), false), Event::PointerMoved(pos2(100.0, 100.0))]);
    assert_eq!(r.context_at, None, "no press");
    let r = pass(&ctx, vec![secondary(pos2(100.0, 100.0), true)]);
    assert_eq!(r.context_at, None, "inside a page");
}

#[test]
fn a_click_on_a_page_name_reports_the_page() {
    let ctx = context();
    // The name sits just above the page's top-left corner (50, 40).
    let at = pos2(56.0, 33.0);
    pass(&ctx, vec![Event::PointerMoved(at)]);
    pass(&ctx, vec![primary(at, true)]);
    let r = pass(&ctx, vec![primary(at, false)]);
    assert_eq!(r.name_clicked, Some(0));
    let away = pos2(300.0, 20.0);
    pass(&ctx, vec![Event::PointerMoved(away)]);
    pass(&ctx, vec![primary(away, true)]);
    assert_eq!(pass(&ctx, vec![primary(away, false)]).name_clicked, None);
}

#[test]
fn the_pasteboard_fill_is_the_canvas_colour_unless_set() {
    let ctx = context();
    let tokens = tokens_in_mode(&ctx, Mode::Light);
    let default = Pasteboard::style(&tokens, PasteboardColor::Default).fill;
    assert_eq!(default, tokens.color.surface.canvas.to_egui());
    assert_eq!(Pasteboard::style(&tokens, PasteboardColor::DarkGray).fill, Color32::from_gray(40));
    assert_eq!(Pasteboard::page_at(&pages(), Point::new(210.0, 110.0)), Some(1));
}

// The Menu key and Shift+F10 open the colour menu from the focused canvas, at the view's centre;
// an unfocused canvas leaves the keys alone.
#[test]
fn the_menu_keys_open_the_colour_menu_from_the_focused_canvas() {
    let ctx = context();
    let canvas = egui::Id::NULL.with("pasteboard-canvas");
    let key = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
    let run = |focused: bool, event: Event| {
        let input =
            RawInput { screen_rect: Some(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))), events: vec![event], ..RawInput::default() };
        let mut out = PasteboardResponse::default();
        let pages = pages();
        ctx.run_ui(input, |ui| {
            let _ = ui.interact(view().rect, canvas, egui::Sense::click_and_drag());
            if focused {
                ui.memory_mut(|m| m.request_focus(canvas));
            } else {
                ui.memory_mut(|m| m.surrender_focus(canvas));
            }
            out = Pasteboard::new("pb", &pages).canvas(super::CanvasId::from_id(canvas)).show(ui, &view());
        })
        .drop_without_applying_deltas();
        out
    };
    let centre = view().to_doc(view().rect.center());
    assert_eq!(run(false, key(Key::Menu, Modifiers::NONE)).context_at, None, "no focus");
    assert_eq!(run(true, key(Key::Menu, Modifiers::NONE)).context_at, Some(centre));
    assert_eq!(run(true, key(Key::F10, Modifiers::SHIFT)).context_at, Some(centre));
}

// An active artboard's edge is a 1 pt `accent.solid` outline and another board's is
// `overlay.artboard`; the other's colour is never drawn on it.
#[test]
fn artboard_edges_are_accent_when_active_and_overlay_otherwise() {
    use egui::Shape;
    let ctx = context();
    let strokes = |active: bool| {
        let input = RawInput { screen_rect: Some(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))), ..RawInput::default() };
        let boards = vec![PageFrame { active: Some(active), ..PageFrame::new(Rect::new(50.0, 40.0, 150.0, 140.0)) }];
        let mut colors = None;
        let output = ctx.run_ui(input, |ui| {
            let tokens = <crate::theme::Tokens as crate::theme::TokensOf>::of(ui);
            colors = Some((tokens.color.accent.solid.to_egui(), tokens.color.overlay.artboard.to_egui(), tokens.component.pasteboard.outline_width));
            let _ = Pasteboard::new("pb", &boards).show(ui, &view());
        });
        let strokes: Vec<egui::Stroke> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Rect(rect) => Some(rect.stroke),
                _ => None,
            })
            .collect();
        output.drop_without_applying_deltas();
        (strokes, colors.expect("a pass ran"))
    };
    let (active, (accent, overlay, width)) = strokes(true);
    assert!(active.iter().any(|s| s.color == accent && s.width == width), "{active:?}");
    assert!(active.iter().all(|s| s.color != overlay), "{active:?}");
    let (other, _) = strokes(false);
    assert!(other.iter().any(|s| s.color == overlay && s.width == width), "{other:?}");
    assert!(other.iter().all(|s| s.color != accent), "{other:?}");
}
