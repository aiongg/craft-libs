//! Surface kinds, edge styles and corner mirroring.

use craft_i18n::Direction;
use craft_theme::{Contrast, ControlSize, Mode, Variant};
use craft_ui::primitives::Surface;
use craft_ui::style::{Corners, SurfaceEdge, SurfaceKind};
use craft_ui::testing::style::{tokens, tokens_named, with_cx};
use craft_ui::testing::{Frames, trial};
use egui::epaint::RectShape;
use egui::{Rect, Shape, pos2};

const RECT: Rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(90.0, 40.0));

#[trial]
fn start_corners_follow_the_reading_direction_and_bevel_lighting_does_not_mirror() {
    let frames = Frames::new();
    let flat = tokens(Mode::Light, Contrast::Standard);
    with_cx(&frames.ctx, &flat, Variant::SECONDARY, ControlSize::Md, |cx| {
        let style = cx.surface(SurfaceKind::Raised);
        let radius = |direction| {
            let shapes = Surface::shapes(&style, None, RECT, Corners::Start, direction, 1.0);
            shapes.iter().find_map(|s| match s {
                Shape::Rect(RectShape { corner_radius, fill, .. }) if *fill != egui::Color32::TRANSPARENT => Some(*corner_radius),
                _ => None,
            })
        };
        let (ltr, rtl) = (radius(Direction::Ltr), radius(Direction::Rtl));
        assert!(ltr.is_some_and(|r| r.nw > 0 && r.sw > 0 && r.ne == 0 && r.se == 0), "LTR rounds the left side");
        assert!(rtl.is_some_and(|r| r.ne > 0 && r.se > 0 && r.nw == 0 && r.sw == 0), "RTL rounds the right side");
    });
    let classic = tokens_named("classic", Mode::Light, Contrast::Standard);
    with_cx(&frames.ctx, &classic, Variant::SECONDARY, ControlSize::Md, |cx| {
        let style = cx.surface(SurfaceKind::Raised);
        let under = |direction| Surface::shapes(&style, None, RECT, Corners::All, direction, 1.0);
        assert_eq!(under(Direction::Ltr), under(Direction::Rtl), "the bevel is lighting, not reading order");
    });
}
