//! The Navigator's pure geometry: the proxy quad, the zoom to a rectangle, the proxy colours.

use craft_theme::{Appearance, Contrast, Mode, Size as SizeClass, TextSize, builtins, resolve};
use egui::{Color32, Rect, pos2, vec2};
use kurbo::{Point, Size};

use super::{NavigatorLabels, NavigatorOptions, NavigatorSettings, ProxyColor, preview_rect, proxy_quad, view_center_for, zoom_to_document_rect};
use crate::canvas::CanvasView;
use crate::theme::{FontFamilies, ToEgui as _, UiTokens};

const DOC: Size = Size::new(2000.0, 1500.0);

/// The 216 × 166 box fits the document at a tenth, inside the 8 point inset.
fn preview() -> Rect {
    preview_rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(216.0, 166.0)), DOC, 8.0)
}

fn view(rotation: f64) -> CanvasView {
    CanvasView { rotation, ..CanvasView::new(Rect::from_min_size(pos2(50.0, 20.0), vec2(800.0, 600.0)), Point::new(1000.0, 750.0), 1.0) }
}

fn near(a: egui::Pos2, b: egui::Pos2) -> bool {
    (a - b).length() < 1e-3
}

// Rotated and flipped views give the canvas corners mapped into the preview; upright gives the
// axis-aligned rectangle of PhotoCraft's size and centre.
#[test]
fn navigator_proxy_is_a_quad_on_a_rotated_view() {
    assert_eq!(preview(), Rect::from_min_max(pos2(8.0, 8.0), pos2(208.0, 158.0)));
    // PhotoCraft: 800 × 600 points at zoom 1 are 80 × 60 preview points round the centre (108, 83).
    let upright = proxy_quad(&view(0.0), DOC, preview());
    let expected = [pos2(68.0, 53.0), pos2(148.0, 53.0), pos2(148.0, 113.0), pos2(68.0, 113.0)];
    assert!(upright.iter().zip(expected).all(|(a, b)| near(*a, b)), "{upright:?}");
    // A pixel aspect of 2 shows half as many document columns.
    let wide = proxy_quad(&CanvasView { aspect: 2.0, ..view(0.0) }, DOC, preview());
    assert!(near(wide[1], pos2(128.0, 53.0)) && near(wide[0], pos2(88.0, 53.0)), "{wide:?}");
    // A quarter turn swaps the extents; the quad stays axis-aligned.
    let turned = proxy_quad(&view(90.0), DOC, preview());
    let bounds = Rect::from_points(&turned);
    assert!((bounds.width() - 60.0).abs() < 1e-3 && (bounds.height() - 80.0).abs() < 1e-3, "{bounds:?}");
    // Any other angle keeps the edge lengths and the right angles.
    let [a, b, c, d] = proxy_quad(&view(30.0), DOC, preview());
    assert!(((b - a).length() - 80.0).abs() < 1e-3 && ((c - b).length() - 60.0).abs() < 1e-3);
    assert!((b - a).dot(c - b).abs() < 1e-2 && near(a + (c - b), d));
    assert!((a.x - b.x).abs() > 1.0 && (a.y - b.y).abs() > 1.0, "tilted, not axis-aligned");
    // Mirrored, the corners keep the canvas's order, so the left corner comes out on the right.
    let flipped = proxy_quad(&CanvasView { flip_x: true, ..view(0.0) }, DOC, preview());
    assert!(near(flipped[0], pos2(148.0, 53.0)) && near(flipped[1], pos2(68.0, 53.0)), "{flipped:?}");
    // A click maps back to the document and stays inside it.
    let middle = view_center_for(preview(), DOC, pos2(108.0, 83.0));
    assert!((middle.x - 1000.0).abs() < 1e-2 && (middle.y - 750.0).abs() < 1e-2, "{middle:?}");
    assert_eq!(view_center_for(preview(), DOC, pos2(-40.0, 400.0)), Point::new(0.0, 1500.0));
}

fn zoom_view(zoom: f64) -> CanvasView {
    CanvasView::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0)), Point::new(1000.0, 750.0), zoom)
}

// `zoom_to_document_rect`: the 16-points-per-pixel case, the pixels-per-point and aspect case, a
// rectangle clipped to the document, the under-2-pixel rung step, and NaN and off-document no-ops
// (PhotoCraft's `navigator_zoom_to_fits_the_rectangle_in_the_canvas`, panels.rs:362).
#[test]
fn navigator_ctrl_drag_zooms_to_the_rectangle() {
    // 800 × 600 points over 50 × 37.5 units: 16 points per unit.
    let mut v = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut v, kurbo::Rect::new(100.0, 100.0, 150.0, 137.5), DOC, 1.0));
    assert!((v.zoom - 16.0).abs() < 1e-9 && v.center == Point::new(125.0, 118.75), "{v:?}");
    // At 2 pixels per point the same points per unit are twice the device zoom, which is what
    // the view stores divided back.
    let mut v = zoom_view(1.0);
    zoom_to_document_rect(&mut v, kurbo::Rect::new(0.0, 0.0, 400.0, 300.0), DOC, 2.0);
    assert!((v.zoom - 2.0).abs() < 1e-9, "{v:?}");
    // A pixel aspect of 2 doubles the columns across the canvas width.
    let mut v = CanvasView { aspect: 2.0, ..zoom_view(1.0) };
    zoom_to_document_rect(&mut v, kurbo::Rect::new(0.0, 0.0, 400.0, 300.0), DOC, 1.0);
    assert!((v.zoom - 1.0).abs() < 1e-9, "{v:?}");
    // A rectangle past the document is clipped to it and centred on what is left.
    let mut v = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut v, kurbo::Rect::new(-100.0, -100.0, 400.0, 300.0), DOC, 1.0));
    assert!((v.zoom - 2.0).abs() < 1e-9 && v.center == Point::new(200.0, 150.0), "{v:?}");
    // Under 2 units on a side: one ladder rung in (100 % to 200 %), centred on the rectangle.
    let mut v = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut v, kurbo::Rect::new(100.0, 100.0, 101.0, 150.0), DOC, 1.0));
    assert!((v.zoom - 2.0).abs() < 1e-9 && v.center == Point::new(100.5, 125.0), "{v:?}");
    // NaN edges and a rectangle off the document do nothing.
    for r in
        [kurbo::Rect::new(f64::NAN, 0.0, 100.0, 100.0), kurbo::Rect::new(3000.0, 0.0, 3100.0, 100.0), kurbo::Rect::new(0.0, 0.0, f64::INFINITY, 1.0)]
    {
        let mut v = zoom_view(1.0);
        assert!(!zoom_to_document_rect(&mut v, r, DOC, 1.0), "{r:?}");
        assert_eq!(v, zoom_view(1.0));
    }
}

fn tokens() -> UiTokens {
    let appearance =
        Appearance { mode: Mode::Dark, contrast: Contrast::Standard, size: SizeClass::Compact, text_size: TextSize::Small, reduced_motion: false };
    UiTokens::new(resolve(&builtins()[0], appearance, None).tokens, appearance, Mode::Dark, FontFamilies::default())
}

// Each `ProxyColor` resolves; `Default` reads `navigator.proxy`; `Custom` keeps its value; the
// settings round-trip through JSON as `{"proxyColor": ..}`.
#[test]
fn proxy_color_choice_overrides_the_token() {
    let tokens = tokens();
    let navigator = &tokens.component.navigator;
    assert_eq!(ProxyColor::Default.color(&tokens), navigator.proxy.to_egui());
    let named = &ProxyColor::ALL[1..];
    for (color, token) in named.iter().zip(&navigator.proxy_colors) {
        assert_eq!(color.color(&tokens), token.to_egui(), "{color:?}");
    }
    let shown: Vec<Color32> = named.iter().map(|c| c.color(&tokens)).collect();
    assert!(shown.iter().enumerate().all(|(i, a)| shown[i + 1..].iter().all(|b| a != b)), "the named colours differ");
    assert_eq!(ProxyColor::Custom([1, 2, 3]).color(&tokens), Color32::from_rgb(1, 2, 3));

    let labels = NavigatorLabels::default();
    assert_eq!(ProxyColor::Violet.label(&labels), "Violet");
    assert_eq!(ProxyColor::Custom([0; 3]).label(&labels), "Custom…");

    let red = NavigatorSettings { options: NavigatorOptions { proxy_color: ProxyColor::Red } };
    assert_eq!(serde_json::to_string(&red).unwrap(), r#"{"proxyColor":"Red"}"#);
    let custom = NavigatorSettings { options: NavigatorOptions { proxy_color: ProxyColor::Custom([1, 2, 3]) } };
    let json = serde_json::to_string(&custom).unwrap();
    assert_eq!(json, r#"{"proxyColor":{"Custom":[1,2,3]}}"#);
    assert_eq!(serde_json::from_str::<NavigatorSettings>(&json).unwrap(), custom);
    assert_eq!(serde_json::from_str::<NavigatorSettings>(r#"{"other":1}"#).unwrap(), NavigatorSettings::default());
}
