//! [`GradientSwatch`] in a headless egui context: the click, AccessKit and the painted ramp.

use craft_theme::Srgba;
use egui::accesskit::{HasPopup, Role};
use egui::{Color32, Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, vec2};

use super::{ColorStop, GradientStops, GradientSwatch};
use crate::theme::{Base, install};

fn frame(ctx: &Context, events: Vec<Event>, stops: &GradientStops, clicked: &mut bool) -> (Rect, egui::FullOutput) {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0))), events, ..Default::default() };
    let mut rect = Rect::NOTHING;
    let output = ctx.run_ui(input, |ui| {
        let response = GradientSwatch::new("fill", "Gradient picker", stops).width(96.0).has_popup(true).show(ui);
        *clicked |= response.clicked();
        rect = response.rect;
    });
    (rect, output)
}

#[test]
fn a_gradient_swatch_is_a_button_that_paints_its_end_colours_and_reports_a_click() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let stops = GradientStops::new(vec![ColorStop::new(0.0, Srgba::rgb(1.0, 0.0, 0.0)), ColorStop::new(1.0, Srgba::rgb(0.0, 0.0, 1.0))]);
    let mut clicked = false;
    let (rect, output) = frame(&ctx, vec![], &stops, &mut clicked);
    assert_eq!(rect.width(), 96.0);

    let nodes = output.platform_output.accesskit_update.clone().unwrap().nodes;
    let node = nodes.iter().map(|(_, n)| n).find(|n| n.label() == Some("Gradient picker")).expect("a named node");
    assert_eq!((node.role(), node.has_popup()), (Role::Button, Some(HasPopup::Dialog)));

    // The leftmost and rightmost vertices of the ramp mesh carry the end colours.
    let colors: Vec<(f32, Color32)> = output
        .shapes
        .iter()
        .filter_map(|c| if let Shape::Mesh(m) = &c.shape { Some(m.clone()) } else { None })
        .flat_map(|m| m.vertices.iter().map(|v| (v.pos.x, v.color)).collect::<Vec<_>>())
        .collect();
    let leftmost = colors.iter().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap().1;
    let rightmost = colors.iter().max_by(|a, b| a.0.total_cmp(&b.0)).unwrap().1;
    assert_eq!((leftmost, rightmost), (Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 0, 255)));
    output.drop_without_applying_deltas();

    let at = rect.center();
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    for events in [vec![Event::PointerMoved(at), button(true)], vec![button(false)]] {
        frame(&ctx, events, &stops, &mut clicked).1.drop_without_applying_deltas();
    }
    assert!(clicked);
}
