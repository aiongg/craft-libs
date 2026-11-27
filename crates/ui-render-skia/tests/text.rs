//! Arabic and CJK text lay out with fallback faces; the system's fonts cover what no registered
//! face does.

use std::sync::Arc;

use craft_theme::{FontSlot, Rgba};
use craft_ui_render_skia::{FontFace, SkiaText};
use craft_ui_rt::{FontSpec, Overflow, TextEngine, TextSpec, pos2};

const ARABIC: &[u8] = include_bytes!("../../ui-contract/tests/fonts/NotoSansArabic-test.ttf");
const CJK: &[u8] = include_bytes!("../../ui-contract/tests/fonts/NotoSansCJKsc-test.otf");

/// Registers the Arabic and CJK test faces and chains them after Inter.
pub fn register_script_faces(text: &mut SkiaText) {
    text.register_face(&FontFace { family: "Noto Arabic".into(), bytes: Arc::from(ARABIC) });
    text.register_face(&FontFace { family: "Noto CJK".into(), bytes: Arc::from(CJK) });
    text.set_fallback_families(["Noto Arabic", "Noto CJK"]);
}

fn spec(text: &str, max_width: Option<f32>, overflow: Overflow) -> TextSpec {
    TextSpec { text: text.into(), font: FontSpec { slot: FontSlot::Ui, size: 16.0 }, line_height: None, color: Rgba([255; 4]), max_width, overflow }
}

/// Letters the Arabic test face has: "al-ma'rifa".
const PERSIAN: &str = "\u{627}\u{644}\u{645}\u{639}\u{631}\u{641}\u{629}";
const CJK_TEXT: &str = "\u{4E2D}\u{6587}";

#[test]
fn arabic_script_and_cjk_resolve_through_the_fallback_faces() {
    let mut with = SkiaText::isolated();
    register_script_faces(&mut with);
    let mut without = SkiaText::isolated();

    for text in [PERSIAN, CJK_TEXT, "PDF \u{639}\u{631}\u{628}\u{64A}\u{629} \u{4E2D}"] {
        let got = with.layout(&spec(text, None, Overflow::Clip), 1.0);
        let missing = without.layout(&spec(text, None, Overflow::Clip), 1.0);
        assert_eq!(with.unresolved_glyphs(&got), 0, "{text:?} has glyphs no face covers");
        assert!(without.unresolved_glyphs(&missing) > 0, "{text:?}: Inter alone covers it, so the trial proves nothing");
        assert!(got.size.x > 4.0 && got.size.y > 8.0, "{text:?}: {:?}", got.size);
    }
}

#[test]
fn right_to_left_text_runs_from_the_right_edge() {
    let mut text = SkiaText::isolated();
    register_script_faces(&mut text);
    let layout = text.layout(&spec(PERSIAN, None, Overflow::Clip), 1.0);
    let len = PERSIAN.len();
    let start = text.caret_rect(&layout, 0);
    let end = text.caret_rect(&layout, len);
    assert!(start.min.x > end.min.x, "the first character is at the right: {start:?} {end:?}");
    // A point near the right edge is before the first character, one near the left after the last.
    assert_eq!(text.hit_index(&layout, pos2(layout.size.x - 0.5, 4.0)), 0);
    assert_eq!(text.hit_index(&layout, pos2(0.5, 4.0)), len);
}

#[test]
fn wrapping_and_ellipsis_work_on_cjk() {
    let mut text = SkiaText::isolated();
    register_script_faces(&mut text);
    let line = text.layout(&spec("\u{4E2D}\u{6587}\u{4E2D}\u{6587}\u{4E2D}\u{6587}", None, Overflow::Clip), 1.0);
    let cut = text.layout(&spec("\u{4E2D}\u{6587}\u{4E2D}\u{6587}\u{4E2D}\u{6587}", Some(line.size.x * 0.6), Overflow::Ellipsis), 1.0);
    assert!(cut.size.x <= line.size.x * 0.6 + 0.5, "{} against {}", cut.size.x, line.size.x);
    assert_eq!(cut.lines, 1);
}

/// Emoji come from the machine's fonts, which only the default engine consults; the layout must
/// still succeed where there is no emoji font.
#[test]
fn emoji_use_the_system_fonts_when_the_machine_has_one() {
    let mut text = SkiaText::new();
    let layout = text.layout(&spec("a\u{1F600}b", None, Overflow::Clip), 1.0);
    assert!(layout.size.x > 4.0);
    eprintln!("emoji glyphs without any face: {}", text.unresolved_glyphs(&layout));
}
