//! Hidden characters.

use super::marks::*;

// Space, tab, U+2028 and U+00A0 map to · » ¬ °; a letter has no mark.
#[test]
fn characters_map_to_their_marks() {
    let marks: Vec<_> = [' ', '\t', '\u{2028}', '\u{A0}'].into_iter().map(|c| HiddenMark::of(c).map(HiddenMark::glyph)).collect();
    assert_eq!(marks, [Some("·"), Some("»"), Some("¬"), Some("°")]);
    assert_eq!(HiddenMark::of('a'), None);
    assert_eq!(HiddenMark::of('\n'), None);
}

// The marks are sized by the style handed in, not by whatever the context holds: ports 20 pt wide in a 100 pt frame, and one text shape per hidden character.
#[test]
fn frames_paint_with_the_style_they_are_given() {
    use egui::{Context, LayerId, RawInput, Shape};
    use kurbo::{Point, Rect};

    use craft_ui::canvas::CanvasView;
    use craft_ui::theme::current;

    let ctx = Context::default();
    let mut style = TextFrameMarks::style(&current(&ctx));
    style.port = 20.0;
    let view = CanvasView::new(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 200.0)), Point::new(50.0, 50.0), 1.0);
    let frames = [FrameExtras { bounds: Rect::new(0.0, 0.0, 100.0, 100.0), threaded_in: false, threaded_out: true, overset: false }];
    let marks = [PlacedMark { mark: HiddenMark::Space, at: Point::new(10.0, 10.0), centred: true, ascent: 12.0 }];
    let mut output = ctx.run_pass(RawInput::default(), |ctx| {
        let painter = ctx.layer_painter(LayerId::background());
        TextFrameMarks::new(&view, &frames, style).hidden_characters(&marks).paint(&painter);
    });
    let (mut ports, mut texts) = (Vec::new(), 0);
    output.textures_delta.clear();
    for clipped in &output.shapes {
        match &clipped.shape {
            Shape::Rect(rect) if rect.fill == egui::Color32::WHITE => ports.push(rect.rect.width()),
            Shape::Text(_) => texts += 1,
            _ => {}
        }
    }
    assert_eq!(ports, [20.0, 20.0]);
    assert_eq!(texts, 1);
}
