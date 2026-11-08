//! `SizeLimits` and `DockHeights`.

use craft_ui::contract::SizeLimits;
use craft_ui::docking::DockHeights;
use craft_ui::testing::trial;
use egui::Vec2;

#[trial]
fn limits_and_heights_refuse_values_that_are_not_finite_or_out_of_range() {
    assert!(SizeLimits::default().valid());
    assert!(SizeLimits::new(Vec2::new(100.0, 50.0), Vec2::new(400.0, 300.0)).valid());
    for bad in [
        SizeLimits::new(Vec2::new(f32::NAN, 0.0), Vec2::splat(10.0)),
        SizeLimits::new(Vec2::ZERO, Vec2::new(f32::INFINITY, 10.0)),
        SizeLimits::new(Vec2::new(-1.0, 0.0), Vec2::splat(10.0)),
        SizeLimits::new(Vec2::splat(20.0), Vec2::splat(10.0)),
        SizeLimits::at_most(Vec2::splat(SizeLimits::BOUND + 1.0)),
    ] {
        assert!(!bad.valid(), "{bad:?}");
    }
    assert!(DockHeights::new(200.0).compact(120.0).fill(400.0).valid());
    for bad in [DockHeights::new(f32::NAN), DockHeights::new(-1.0), DockHeights::new(10.0).fill(SizeLimits::BOUND + 1.0)] {
        assert!(!bad.valid(), "{bad:?}");
    }
}
