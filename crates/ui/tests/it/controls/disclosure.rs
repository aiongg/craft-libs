//! The arrow every component shows for something that opens: where it points, and under right-to-left.

use craft_i18n::Direction;
use craft_ui::controls::disclosure::DisclosureGlyph;
use craft_ui::testing::trial;
use egui::{Rect, pos2, vec2};

/// The tip of the glyph's outline, relative to the centre of the rect it is drawn in.
fn tip(glyph: DisclosureGlyph, direction: Direction) -> egui::Vec2 {
    let rect = Rect::from_center_size(pos2(50.0, 50.0), vec2(16.0, 16.0));
    glyph.path(rect, direction)[1] - rect.center()
}

/// A closed disclosure points to the end of the line and mirrors under right-to-left; an open one and a dropdown
/// point down.
#[trial]
fn disclosure_points_to_the_end_when_closed_and_down_when_open() {
    assert!(tip(DisclosureGlyph::new(0.0), Direction::Ltr).x > 0.0, "closed, left to right: right");
    assert!(tip(DisclosureGlyph::new(0.0), Direction::Rtl).x < 0.0, "closed, right to left: left");
    for direction in [Direction::Ltr, Direction::Rtl] {
        let open = tip(DisclosureGlyph::new(1.0), direction);
        assert!(open.y > 0.0 && open.x.abs() < 1e-3, "open points down in {direction:?}");
    }
    let tree = DisclosureGlyph::filled(0.0, true);
    let submenu = DisclosureGlyph::filled(0.0, false);
    assert!(tip(tree, Direction::Ltr).x < tip(submenu, Direction::Ltr).x, "a tree row's triangle is smaller than a submenu's");
}
